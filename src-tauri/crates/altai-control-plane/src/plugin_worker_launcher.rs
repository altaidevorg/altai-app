//! CP-08 real worker processes behind the supervision core (package 072,
//! PR 2). [`WorkerSupervisor`](crate::WorkerSupervisor) decides policy over
//! observations; this module supplies the reality those observations
//! describe: launch a plugin's worker process, watch it exit, stop it on
//! purpose, and reconcile the two — every exit is reported to the
//! supervisor, every `Restart` directive is acted on immediately, and an
//! exhausted budget leaves the plugin down.
//!
//! The launcher does not decide *how* plugin code runs: the host hands it
//! a command builder per manifest, so the same supervision drives a
//! packaged binary, an interpreter, or a test double.
//!
//! PR 3 adds probing: with a [`HealthProbePolicy`](crate::HealthProbePolicy),
//! reconcile also asks a live worker how it feels over its stdio
//! transport — an answer is the `Healthy` observation the supervision
//! core was designed around, silence inside the window kills the worker
//! and is observed as the crash it is. Without a policy, a process that
//! has not exited is considered running.
//!
//! PR 5 adds the remaining capability pair over the same wire: webhook
//! deliveries (at-most-once like jobs, in their own ledger) and scoped
//! secrets — deliberately *not* at-most-once, because a relaunched
//! worker is a fresh process that must be handed its secrets again. The
//! host remembers every brokered value and re-hands the unhanded ones on
//! each reconcile, so a restart is transparent to the plugin.

use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use altai_control_protocol::{PluginCapability, PluginManifest};
use serde_json::Value;

use crate::plugin_worker::{
    HealthProbePolicy, WorkerDirective, WorkerError, WorkerHealth, WorkerObservation,
    WorkerRestartPolicy, WorkerSupervisor,
};
use crate::plugin_worker_jobs::{
    DispatchLedger, DispatchOutcome, DispatchState, JobRequest, JobResult,
};
use crate::plugin_worker_secrets::{
    SecretAck, SecretHandoff, SecretHandoffOutcome, SecretString,
};
use crate::plugin_worker_transport::{StdioWorkerTransport, WorkerFrame};
use crate::plugin_worker_webhooks::{WebhookAck, WebhookDelivery};

/// One secret this host has brokered for the worker: the value (kept
/// for restarts — a relaunched process needs it again) and whether the
/// current process has been given it.
struct SecretProvision {
    value: SecretString,
    handed_to_process: bool,
}

/// How to launch one plugin's worker process. Implemented by the host
/// environment; [`CommandWorkerLauncher`] is the std-process default.
pub trait WorkerLauncher: Send + Sync {
    fn launch(&self, manifest: &PluginManifest) -> Result<WorkerProcess, WorkerError>;
}

/// Builds the command for one plugin's worker. The host owns how plugin
/// code executes; the launcher only runs what it is handed.
pub type WorkerCommandBuilder =
    Box<dyn Fn(&PluginManifest) -> Command + Send + Sync>;

/// Launches workers as child processes of the current process.
pub struct CommandWorkerLauncher {
    build: WorkerCommandBuilder,
}

impl CommandWorkerLauncher {
    pub fn new(build: WorkerCommandBuilder) -> Self {
        Self { build }
    }
}

impl WorkerLauncher for CommandWorkerLauncher {
    fn launch(&self, manifest: &PluginManifest) -> Result<WorkerProcess, WorkerError> {
        let launch_error = |reason: String| WorkerError::Launch {
            plugin_id: manifest.plugin_id.clone(),
            reason,
        };
        let mut command = (self.build)(manifest);
        // The stdio channel is the health-probe wire (and later the job
        // wire); only stderr is host-irrelevant.
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(|error| launch_error(error.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| launch_error("worker stdin was not piped".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| launch_error("worker stdout was not piped".into()))?;
        Ok(WorkerProcess {
            child,
            transport: StdioWorkerTransport::new(stdin, stdout),
        })
    }
}

/// One launched worker process. Waiting is polling, not blocking: the
/// owner reconciles on its own schedule.
pub struct WorkerProcess {
    child: Child,
    transport: StdioWorkerTransport,
}

impl WorkerProcess {
    /// `Some` once the process has ended, `None` while it runs.
    pub fn try_wait(&mut self) -> Result<Option<String>, WorkerError> {
        match self.child.try_wait() {
            Ok(Some(status)) => Ok(Some(format!("worker exited with {status}"))),
            Ok(None) => Ok(None),
            Err(error) => Err(WorkerError::Process {
                reason: error.to_string(),
            }),
        }
    }

    /// Ask this worker how it is, waiting up to `window` for its answer.
    /// `Ok(false)`: no answer came back.
    pub fn probe(&mut self, window: Duration) -> Result<bool, WorkerError> {
        self.transport.probe(window)
    }

    /// Send one job request to this worker.
    pub fn send_job(&mut self, request: &JobRequest) -> Result<(), WorkerError> {
        self.transport.send(&WorkerFrame::JobRequest(request.clone()))
    }

    /// Wait for one job's result; results for other jobs arriving
    /// meanwhile are stashed by the transport for their own awaiters.
    pub fn await_job_result(&mut self, job_id: &str, window: Duration) -> Option<JobResult> {
        self.transport.await_result(job_id, window)
    }

    /// Send one webhook delivery to this worker.
    pub fn send_webhook(&mut self, delivery: &WebhookDelivery) -> Result<(), WorkerError> {
        self.transport
            .send(&WorkerFrame::WebhookDelivery(delivery.clone()))
    }

    /// Wait for one webhook delivery's ack.
    pub fn await_webhook_ack(&mut self, delivery_id: &str, window: Duration) -> Option<WebhookAck> {
        self.transport.await_webhook_ack(delivery_id, window)
    }

    /// Send one scoped secret to this worker.
    pub fn send_secret(&mut self, handoff: &SecretHandoff) -> Result<(), WorkerError> {
        self.transport
            .send(&WorkerFrame::SecretHandoff(handoff.clone()))
    }

    /// Wait for one secret's ack.
    pub fn await_secret_ack(&mut self, name: &str, window: Duration) -> Option<SecretAck> {
        self.transport.await_secret_ack(name, window)
    }

    /// Terminate the process and reap it, so a killed worker does not
    /// linger. Deliberately infallible: killing an already-exited
    /// process is the no-op it should be — the fact "this process is
    /// done" does not change.
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A supervised worker end to end: the process and the policy over it in
/// one place. Every [`reconcile`](Self::reconcile) reports exits the
/// supervisor has not seen, probes a live worker when the policy says
/// so, and acts on the directive it returns.
pub struct SupervisedWorker {
    supervisor: WorkerSupervisor,
    launcher: Arc<dyn WorkerLauncher>,
    manifest: PluginManifest,
    process: Option<WorkerProcess>,
    probe: Option<HealthProbePolicy>,
    launched_at: Instant,
    jobs: DispatchLedger,
    webhooks: DispatchLedger,
    secrets: std::collections::HashMap<String, SecretProvision>,
}

impl std::fmt::Debug for SupervisedWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Processes and launchers have no meaningful debug shape; the
        // supervision state is what diagnostics want.
        f.debug_struct("SupervisedWorker")
            .field("plugin_id", self.plugin_id())
            .field("health", self.supervisor.health())
            .field("has_process", &self.process.is_some())
            .finish()
    }
}

impl SupervisedWorker {
    pub fn start(
        manifest: PluginManifest,
        policy: WorkerRestartPolicy,
        probe: Option<HealthProbePolicy>,
        launcher: Arc<dyn WorkerLauncher>,
    ) -> Result<Self, WorkerError> {
        let supervisor = WorkerSupervisor::new(manifest.clone(), policy)?;
        let process = launcher.launch(&manifest)?;
        Ok(Self {
            supervisor,
            launcher,
            manifest,
            process: Some(process),
            probe,
            launched_at: Instant::now(),
            jobs: DispatchLedger::new(),
            webhooks: DispatchLedger::new(),
            secrets: std::collections::HashMap::new(),
        })
    }

    pub fn health(&self) -> &WorkerHealth {
        self.supervisor.health()
    }

    pub fn plugin_id(&self) -> &altai_control_protocol::PluginId {
        self.supervisor.plugin_id()
    }

    /// Report any not-yet-seen exit — a process that has died is never
    /// probed, its exit is the observation — then, if the worker lives
    /// and a probe policy is set and its startup grace has passed, ask
    /// how it feels. Act on the supervisor's directive: a `Restart`
    /// relaunches here, an exhausted budget leaves the plugin down.
    pub fn reconcile(&mut self) -> Result<WorkerDirective, WorkerError> {
        let exited = match self.process.as_mut() {
            Some(process) => process.try_wait()?,
            None => None,
        };
        let Some(reason) = exited else {
            return self.reconcile_alive();
        };
        // try_wait already reaped it; nothing to stop.
        self.process = None;
        self.observe_death_and_maybe_relaunch(reason)
    }

    fn reconcile_alive(&mut self) -> Result<WorkerDirective, WorkerError> {
        self.re_provision_secrets()?;
        let Some(policy) = self.probe else {
            return Ok(WorkerDirective::None);
        };
        if self.launched_at.elapsed() < policy.startup_grace {
            // Still starting: a worker booting is not yet silent.
            return Ok(WorkerDirective::None);
        }
        let answered = match self.process.as_mut() {
            Some(process) => process.probe(policy.probe_window)?,
            None => return Ok(WorkerDirective::None),
        };
        if answered {
            return self.supervisor.observe(WorkerObservation::Healthy);
        }
        // Silent while alive: kill it (it has not exited by itself), then
        // let the death be the crash fact it is.
        if let Some(mut process) = self.process.take() {
            process.stop();
        }
        self.observe_death_and_maybe_relaunch(
            "worker did not answer its health probe".into(),
        )
    }

    fn observe_death_and_maybe_relaunch(
        &mut self,
        reason: String,
    ) -> Result<WorkerDirective, WorkerError> {
        let directive = self
            .supervisor
            .observe(WorkerObservation::Crashed { reason })?;
        if directive == WorkerDirective::Restart {
            self.process = Some(self.launcher.launch(&self.manifest)?);
            self.launched_at = Instant::now();
            // A fresh process has been given nothing: every secret must
            // be handed again, and the next reconcile's sweep does.
            for provision in self.secrets.values_mut() {
                provision.handed_to_process = false;
            }
        }
        Ok(directive)
    }

    /// Stop on purpose: kill the process if any, then record the stop.
    /// Stopping twice is the same single fact.
    pub fn stop(&mut self) -> Result<(), WorkerError> {
        if let Some(mut process) = self.process.take() {
            process.stop();
        }
        self.supervisor.observe(WorkerObservation::StoppedByHost)?;
        Ok(())
    }

    /// Dispatch one job to this worker. The `job_id` is the idempotency
    /// key: it is sent at most once, ever. Requires the `Jobs`
    /// capability. A dead worker refuses without recording (nothing was
    /// sent, so a later dispatch is safe); a send that fails after
    /// recording stays `Dispatched` — ambiguous delivery is surfaced as
    /// a job that never completes, never silently retried.
    pub fn dispatch_job(
        &mut self,
        job_id: &str,
        payload: Value,
    ) -> Result<DispatchOutcome, WorkerError> {
        if !self
            .manifest
            .capabilities
            .contains(&PluginCapability::Jobs)
        {
            return Err(WorkerError::CapabilityMissing {
                plugin_id: self.manifest.plugin_id.clone(),
                capability: PluginCapability::Jobs,
            });
        }
        if self.jobs.state(job_id).is_some() {
            return Ok(DispatchOutcome::AlreadyKnown);
        }
        let Some(process) = self.process.as_mut() else {
            return Ok(DispatchOutcome::WorkerDown);
        };
        // Record before sending: the at-most-once guarantee. A crash
        // between these lines cannot lead to a second send.
        self.jobs.record_dispatch(job_id);
        process.send_job(&JobRequest {
            job_id: job_id.to_string(),
            payload,
        })?;
        Ok(DispatchOutcome::Sent)
    }

    /// Wait for one job's result, recording it when it arrives. A
    /// completed job answers instantly from the ledger; an unknown job
    /// has nothing to await; a result that never comes leaves the job
    /// `Dispatched` — visible, not failed.
    pub fn await_job_result(
        &mut self,
        job_id: &str,
        window: Duration,
    ) -> Result<Option<JobResult>, WorkerError> {
        if let Some(DispatchState::Completed { ok, output }) = self.jobs.state(job_id) {
            return Ok(Some(JobResult {
                job_id: job_id.to_string(),
                ok: *ok,
                output: output.clone(),
            }));
        }
        if self.jobs.state(job_id).is_none() {
            return Ok(None);
        }
        let Some(process) = self.process.as_mut() else {
            return Ok(None);
        };
        match process.await_job_result(job_id, window) {
            Some(result) => {
                self.jobs.record_result(result.clone());
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    /// What the host knows about one job id.
    pub fn job_state(&self, job_id: &str) -> Option<&DispatchState> {
        self.jobs.state(job_id)
    }

    /// What the host knows about one webhook delivery id.
    pub fn webhook_state(&self, delivery_id: &str) -> Option<&DispatchState> {
        self.webhooks.state(delivery_id)
    }

    /// Hand one inbound webhook to this worker. Same at-most-once
    /// contract as [`dispatch_job`](Self::dispatch_job), keyed by
    /// `delivery_id` in its own ledger. Requires the `Webhooks`
    /// capability.
    pub fn deliver_webhook(
        &mut self,
        delivery_id: &str,
        event: &str,
        payload: Value,
    ) -> Result<DispatchOutcome, WorkerError> {
        if !self.manifest.capabilities.contains(&PluginCapability::Webhooks) {
            return Err(WorkerError::CapabilityMissing {
                plugin_id: self.manifest.plugin_id.clone(),
                capability: PluginCapability::Webhooks,
            });
        }
        if self.webhooks.state(delivery_id).is_some() {
            return Ok(DispatchOutcome::AlreadyKnown);
        }
        let Some(process) = self.process.as_mut() else {
            return Ok(DispatchOutcome::WorkerDown);
        };
        // Record before sending: the at-most-once guarantee.
        self.webhooks.record_dispatch(delivery_id);
        process.send_webhook(&WebhookDelivery {
            delivery_id: delivery_id.to_string(),
            event: event.to_string(),
            payload,
        })?;
        Ok(DispatchOutcome::Sent)
    }

    /// Wait for one webhook delivery's ack; a completed delivery answers
    /// instantly from the ledger. `Ok(None)`: no ack in `window`.
    pub fn await_webhook_ack(
        &mut self,
        delivery_id: &str,
        window: Duration,
    ) -> Result<Option<WebhookAck>, WorkerError> {
        if let Some(DispatchState::Completed { ok, .. }) = self.webhooks.state(delivery_id) {
            return Ok(Some(WebhookAck {
                delivery_id: delivery_id.to_string(),
                ok: *ok,
            }));
        }
        if self.webhooks.state(delivery_id).is_none() {
            return Ok(None);
        }
        let Some(process) = self.process.as_mut() else {
            return Ok(None);
        };
        match process.await_webhook_ack(delivery_id, window) {
            Some(ack) => {
                self.webhooks
                    .record_completion(delivery_id, ack.ok, Value::Null);
                Ok(Some(ack))
            }
            None => Ok(None),
        }
    }

    /// Hand one scoped secret to this worker. Requires the
    /// `ScopedSecrets` capability. The value is remembered so a
    /// relaunched process is re-provisioned — secrets are per-process
    /// state, not at-most-once dispatches.
    pub fn hand_secret(
        &mut self,
        name: &str,
        value: SecretString,
        ack_window: Duration,
    ) -> Result<SecretHandoffOutcome, WorkerError> {
        if !self
            .manifest
            .capabilities
            .contains(&PluginCapability::ScopedSecrets)
        {
            return Err(WorkerError::CapabilityMissing {
                plugin_id: self.manifest.plugin_id.clone(),
                capability: PluginCapability::ScopedSecrets,
            });
        }
        if self
            .secrets
            .get(name)
            .is_some_and(|provision| provision.handed_to_process)
        {
            return Ok(SecretHandoffOutcome::AlreadyProvided);
        }
        // Remember before sending, unhanded: a send that fails or a
        // worker that dies racing it leaves the secret pending for the
        // reconcile sweep, not lost.
        self.secrets.insert(
            name.to_string(),
            SecretProvision {
                value,
                handed_to_process: false,
            },
        );
        let Some(process) = self.process.as_mut() else {
            return Ok(SecretHandoffOutcome::WorkerDown);
        };
        let handoff = SecretHandoff {
            name: name.to_string(),
            value: self
                .secrets
                .get(name)
                .expect("the provision was just inserted")
                .value
                .clone(),
        };
        process.send_secret(&handoff)?;
        let confirmed = process.await_secret_ack(name, ack_window).is_some();
        if let Some(provision) = self.secrets.get_mut(name) {
            provision.handed_to_process = true;
        }
        Ok(SecretHandoffOutcome::Delivered { confirmed })
    }

    /// Give every unhanded secret to the live process: after a restart
    /// nothing has been handed, and a send that raced an exit is simply
    /// retried here. Idempotent — handed secrets are not re-sent.
    fn re_provision_secrets(&mut self) -> Result<(), WorkerError> {
        let pending: Vec<String> = self
            .secrets
            .iter()
            .filter(|(_, provision)| !provision.handed_to_process)
            .map(|(name, _)| name.clone())
            .collect();
        for name in pending {
            let Some(process) = self.process.as_mut() else {
                return Ok(());
            };
            let Some(provision) = self.secrets.get(&name) else {
                continue;
            };
            let handoff = SecretHandoff {
                name: name.clone(),
                value: provision.value.clone(),
            };
            if process.send_secret(&handoff).is_ok() {
                if let Some(provision) = self.secrets.get_mut(&name) {
                    provision.handed_to_process = true;
                }
            }
            // A failed send stays pending: the next reconcile retries,
            // and the exit path (if the worker died) resets everything.
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_worker_transport::WorkerFrame;
    use altai_control_protocol::{PluginCapability, PluginId, PluginKind, PluginVersion};
    use std::io::{BufRead, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: PluginId::new("plug_1"),
            kind: PluginKind::Application,
            version: PluginVersion::new(1, 0, 0),
            display_name: "Test plugin".into(),
            capabilities: vec![
                PluginCapability::Jobs,
                PluginCapability::Webhooks,
                PluginCapability::ScopedSecrets,
            ],
            ui: None,
        }
    }

    /// Runs this test binary again in a controlled child mode, so exits
    /// and probes are exercised against real processes on every platform
    /// CI runs on. `--nocapture` keeps the child's stdout on the pipe:
    /// libtest would otherwise swallow it into its capture buffer.
    fn child_command(mode: &'static str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "plugin_worker_launcher::tests::child_mode",
                "--nocapture",
            ])
            .env("WORKER_CHILD_MODE", mode);
        command
    }

    #[test]
    fn child_mode() {
        // Reentered as a child process: act out the mode, then die. In a
        // normal suite run (no env var) this test is a no-op that passes.
        match std::env::var("WORKER_CHILD_MODE").as_deref() {
            Ok("crash") => std::process::exit(7),
            Ok("hang") => std::thread::sleep(Duration::from_secs(60)),
            Ok("probe") => serve_probes(),
            Ok("jobs") => serve_full_protocol(false),
            Ok("jobs_exit_after_secret") => serve_full_protocol(true),
            _ => {}
        }
    }

    /// The probe-mode child: answer health probes over real stdio until
    /// the host closes the pipe.
    fn serve_probes() {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if matches!(WorkerFrame::from_line(&line), Some(WorkerFrame::HealthProbe)) {
                let reply = WorkerFrame::HealthOk.to_line();
                if writeln!(stdout, "{reply}").and_then(|()| stdout.flush()).is_err() {
                    break;
                }
            }
        }
        std::process::exit(0);
    }

    /// The jobs-mode child: a full-protocol worker. It counts what it
    /// receives — job requests, webhook deliveries, secret hand-offs —
    /// and reports those counts when a job's payload asks
    /// (`{"report_requests": true}` and siblings), so tests can prove
    /// how many of each actually reached the worker; other jobs get
    /// their payload echoed back. In `jobs_exit_after_secret` mode it
    /// answers one secret hand-off and then dies: the stand-in for a
    /// worker that crashes right after taking a secret.
    fn serve_full_protocol(exit_after_secret: bool) {
        let mut requests = 0u64;
        let mut deliveries = 0u64;
        let mut handoffs = 0u64;
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            let mut die_now = false;
            let reply = match WorkerFrame::from_line(&line) {
                Some(WorkerFrame::HealthProbe) => WorkerFrame::HealthOk.to_line(),
                Some(WorkerFrame::JobRequest(request)) => {
                    requests += 1;
                    let output = if request.payload.get("report_requests")
                        == Some(&serde_json::json!(true))
                    {
                        serde_json::json!({ "requests": requests })
                    } else if request.payload.get("report_deliveries")
                        == Some(&serde_json::json!(true))
                    {
                        serde_json::json!({ "deliveries": deliveries })
                    } else if request.payload.get("report_handoffs")
                        == Some(&serde_json::json!(true))
                    {
                        serde_json::json!({ "handoffs": handoffs })
                    } else {
                        request.payload
                    };
                    WorkerFrame::JobResult(JobResult {
                        job_id: request.job_id,
                        ok: true,
                        output,
                    })
                    .to_line()
                }
                Some(WorkerFrame::WebhookDelivery(delivery)) => {
                    deliveries += 1;
                    WorkerFrame::WebhookAck(WebhookAck {
                        delivery_id: delivery.delivery_id,
                        ok: true,
                    })
                    .to_line()
                }
                Some(WorkerFrame::SecretHandoff(handoff)) => {
                    handoffs += 1;
                    die_now = exit_after_secret;
                    WorkerFrame::SecretAck(SecretAck {
                        name: handoff.name,
                    })
                    .to_line()
                }
                _ => continue,
            };
            if writeln!(stdout, "{reply}")
                .and_then(|()| stdout.flush())
                .is_err()
            {
                break;
            }
            if die_now {
                std::process::exit(0);
            }
        }
        std::process::exit(0);
    }

    fn launcher(mode: &'static str) -> Arc<CommandWorkerLauncher> {
        Arc::new(CommandWorkerLauncher::new(Box::new(move |_| {
            child_command(mode)
        })))
    }

    /// A launcher that runs the given modes in order, repeating the last:
    /// the stand-in for a plugin whose bad build dies or hangs and whose
    /// replacement behaves.
    fn sequencing_launcher(modes: &'static [&'static str]) -> Arc<CommandWorkerLauncher> {
        let step = AtomicUsize::new(0);
        Arc::new(CommandWorkerLauncher::new(Box::new(move |_| {
            let index = step.fetch_add(1, Ordering::SeqCst).min(modes.len() - 1);
            child_command(modes[index])
        })))
    }

    /// Generous enough for a slow CI child to boot and answer.
    fn probing(grace_ms: u64, window: Duration) -> Option<HealthProbePolicy> {
        Some(HealthProbePolicy {
            startup_grace: Duration::from_millis(grace_ms),
            probe_window: window,
        })
    }

    /// A crash-mode child exits on its own schedule, so the first
    /// reconcile can race it. Real supervision reconciles on a loop;
    /// these tests do the same, bounded.
    fn reconcile_until_decided(
        worker: &mut SupervisedWorker,
        attempts: u32,
    ) -> WorkerDirective {
        for _ in 0..attempts {
            match worker.reconcile().unwrap() {
                WorkerDirective::None => std::thread::sleep(Duration::from_millis(20)),
                decided => return decided,
            }
        }
        panic!("worker did not decide within {attempts} reconcile attempts");
    }

    fn reconcile_until_healthy(worker: &mut SupervisedWorker, attempts: u32) {
        for _ in 0..attempts {
            worker.reconcile().unwrap();
            if matches!(worker.health(), WorkerHealth::Healthy) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("worker never answered healthy within {attempts} reconcile attempts");
    }

    #[test]
    fn a_live_process_reconciles_to_none() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            None,
            launcher("hang"),
        )
        .unwrap();
        assert_eq!(worker.reconcile().unwrap(), WorkerDirective::None);
        assert_eq!(worker.health(), &WorkerHealth::Starting);
        worker.stop().unwrap();
    }

    #[test]
    fn a_crash_within_budget_restarts_the_process_for_real() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(2),
            None,
            sequencing_launcher(&["crash", "hang"]),
        )
        .unwrap();

        // The crash is observed, the directive is Restart, and the
        // replacement process is already launched — a hang-mode child, so
        // the next reconcile is quiet.
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::Restart
        );
        assert!(matches!(worker.health(), WorkerHealth::Crashed { restarts: 1, .. }));
        // Give the relaunched (hanging) child a moment to exist.
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(worker.reconcile().unwrap(), WorkerDirective::None);
        worker.stop().unwrap();
        assert_eq!(worker.health(), &WorkerHealth::Stopped);
    }

    #[test]
    fn a_replacement_that_dies_immediately_keeps_consuming_budget() {
        // Every launch crashes: the replacements never report healthy,
        // each death is a new crash observation, and the budget drains.
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(2),
            None,
            launcher("crash"),
        )
        .unwrap();

        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::Restart
        );
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::Restart
        );
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::GiveUp
        );
        assert!(matches!(
            worker.health(),
            WorkerHealth::Crashed {
                restarts: 2,
                exhausted: true,
                ..
            }
        ));
        assert!(worker.process.is_none());
    }

    #[test]
    fn an_exhausted_budget_leaves_the_plugin_down() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(0),
            None,
            launcher("crash"),
        )
        .unwrap();

        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::GiveUp
        );
        assert!(matches!(
            worker.health(),
            WorkerHealth::Crashed { exhausted: true, .. }
        ));
        // No replacement was launched.
        assert!(worker.process.is_none());
        // Reconciling a down worker is quiet, not an error.
        assert_eq!(worker.reconcile().unwrap(), WorkerDirective::None);
    }

    #[test]
    fn stopping_twice_is_one_fact() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            None,
            launcher("hang"),
        )
        .unwrap();
        worker.stop().unwrap();
        worker.stop().unwrap();
        assert_eq!(worker.health(), &WorkerHealth::Stopped);
    }

    #[test]
    fn an_agent_content_manifest_cannot_start_a_worker() {
        let mut manifest = manifest();
        manifest.kind = PluginKind::AgentContent;
        manifest.capabilities = Vec::new();
        let error = SupervisedWorker::start(
            manifest.clone(),
            WorkerRestartPolicy::new(3),
            None,
            launcher("hang"),
        )
        .unwrap_err();
        assert_eq!(
            error,
            WorkerError::NotAnApplication {
                plugin_id: PluginId::new("plug_1"),
            }
        );
    }

    #[test]
    fn an_answering_worker_becomes_healthy_through_a_real_probe() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            probing(0, Duration::from_secs(5)),
            launcher("probe"),
        )
        .unwrap();
        reconcile_until_healthy(&mut worker, 100);
        assert_eq!(worker.health(), &WorkerHealth::Healthy);
        worker.stop().unwrap();
        assert_eq!(worker.health(), &WorkerHealth::Stopped);
    }

    #[test]
    fn a_silent_worker_is_killed_and_its_silence_is_a_crash_fact() {
        // The hang child never answers a probe; with no restart budget,
        // its silence takes the plugin down.
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(0),
            probing(0, Duration::from_millis(150)),
            launcher("hang"),
        )
        .unwrap();
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::GiveUp
        );
        assert!(matches!(
            worker.health(),
            WorkerHealth::Crashed {
                exhausted: true,
                reason,
                ..
            } if reason.contains("health probe"),
        ));
        assert!(worker.process.is_none());
    }

    #[test]
    fn a_silent_worker_is_replaced_by_one_that_answers() {
        // First launch hangs, replacement answers: the probe observes the
        // silence, the restart observes the answer — Healthy-from-Crashed
        // from real evidence, exactly the transition the supervision
        // core designed for restarts.
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(2),
            probing(0, Duration::from_millis(150)),
            sequencing_launcher(&["hang", "probe"]),
        )
        .unwrap();
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::Restart
        );
        reconcile_until_healthy(&mut worker, 100);
        assert_eq!(worker.health(), &WorkerHealth::Healthy);
        worker.stop().unwrap();
    }

    #[test]
    fn a_fresh_worker_is_not_probed_before_its_startup_grace() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(0),
            probing(10_000, Duration::from_millis(150)),
            launcher("hang"),
        )
        .unwrap();
        assert_eq!(worker.reconcile().unwrap(), WorkerDirective::None);
        // Still starting: not probed, so not yet silent.
        assert_eq!(worker.health(), &WorkerHealth::Starting);
        worker.stop().unwrap();
    }

    #[test]
    fn an_exiting_worker_is_observed_through_its_exit_not_a_probe() {
        // With probing on, a crash child still reports through its exit:
        // exits are checked before any probe is sent.
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(2),
            probing(0, Duration::from_secs(5)),
            sequencing_launcher(&["crash", "hang"]),
        )
        .unwrap();
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::Restart
        );
        assert!(matches!(
            worker.health(),
            WorkerHealth::Crashed { restarts: 1, .. }
        ));
        worker.stop().unwrap();
    }

    #[test]
    fn a_job_round_trips_over_real_stdio() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            None,
            launcher("jobs"),
        )
        .unwrap();

        assert_eq!(
            worker
                .dispatch_job("job_1", serde_json::json!({"input": 7}))
                .unwrap(),
            DispatchOutcome::Sent
        );
        let result = worker
            .await_job_result("job_1", Duration::from_secs(5))
            .unwrap()
            .expect("the jobs child answers");
        assert!(result.ok);
        assert_eq!(result.output, serde_json::json!({"input": 7}));
        assert!(matches!(
            worker.job_state("job_1"),
            Some(DispatchState::Completed { ok: true, .. })
        ));

        // The id is burned: a re-dispatch sends nothing.
        assert_eq!(
            worker
                .dispatch_job("job_1", serde_json::json!({"input": 8}))
                .unwrap(),
            DispatchOutcome::AlreadyKnown
        );
        // And a completed job answers from the ledger, instantly.
        let replayed = worker
            .await_job_result("job_1", Duration::from_millis(1))
            .unwrap()
            .expect("completed jobs answer from the ledger");
        assert_eq!(replayed.output, serde_json::json!({"input": 7}));
        worker.stop().unwrap();
    }

    #[test]
    fn a_re_dispatch_never_reaches_the_worker() {
        // The child counts its requests and reports the count, so a
        // re-sent job would show up as one request too many.
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            None,
            launcher("jobs"),
        )
        .unwrap();
        worker
            .dispatch_job("job_1", serde_json::json!({"input": 1}))
            .unwrap();
        worker
            .await_job_result("job_1", Duration::from_secs(5))
            .unwrap()
            .expect("first dispatch completes");
        worker
            .dispatch_job("job_1", serde_json::json!({"input": 2}))
            .unwrap();

        worker
            .dispatch_job("job_count", serde_json::json!({"report_requests": true}))
            .unwrap();
        let report = worker
            .await_job_result("job_count", Duration::from_secs(5))
            .unwrap()
            .expect("the report job completes");
        // job_1 once + the report itself: the re-dispatch added nothing.
        assert_eq!(report.output, serde_json::json!({ "requests": 2 }));
        worker.stop().unwrap();
    }

    #[test]
    fn a_worker_without_the_jobs_capability_refuses_dispatch() {
        let mut manifest = manifest();
        manifest.capabilities = Vec::new();
        let mut worker = SupervisedWorker::start(
            manifest,
            WorkerRestartPolicy::new(3),
            None,
            launcher("jobs"),
        )
        .unwrap();
        assert_eq!(
            worker.dispatch_job("job_1", serde_json::json!({})).unwrap_err(),
            WorkerError::CapabilityMissing {
                plugin_id: PluginId::new("plug_1"),
                capability: PluginCapability::Jobs,
            }
        );
        worker.stop().unwrap();
    }

    #[test]
    fn dispatch_to_a_dead_worker_records_nothing_and_says_so() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(0),
            None,
            launcher("crash"),
        )
        .unwrap();
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::GiveUp
        );
        assert_eq!(
            worker.dispatch_job("job_1", serde_json::json!({})).unwrap(),
            DispatchOutcome::WorkerDown
        );
        // Nothing was sent, so nothing was recorded: the id is not
        // burned, a later dispatch to a healthy worker is safe.
        assert_eq!(worker.job_state("job_1"), None);
    }

    #[test]
    fn a_result_that_arrives_during_a_probe_is_not_lost() {
        // Dispatch, then reconcile (probe): the result can land while the
        // probe waits. Either receive order must deliver the result.
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            probing(0, Duration::from_secs(5)),
            launcher("jobs"),
        )
        .unwrap();
        worker
            .dispatch_job("job_1", serde_json::json!({"input": 1}))
            .unwrap();
        worker.reconcile().unwrap();
        let result = worker
            .await_job_result("job_1", Duration::from_secs(5))
            .unwrap()
            .expect("the result survives the probe");
        assert!(result.ok);
        worker.stop().unwrap();
    }
    #[test]
    fn a_webhook_round_trips_over_real_stdio() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            None,
            launcher("jobs"),
        )
        .unwrap();

        assert_eq!(
            worker
                .deliver_webhook("wh_1", "issue.opened", serde_json::json!({"n": 1}))
                .unwrap(),
            DispatchOutcome::Sent
        );
        let ack = worker
            .await_webhook_ack("wh_1", Duration::from_secs(5))
            .unwrap()
            .expect("the jobs child acks deliveries");
        assert!(ack.ok);
        assert!(matches!(
            worker.webhook_state("wh_1"),
            Some(DispatchState::Completed { ok: true, .. })
        ));

        // The delivery id is burned: a re-delivery sends nothing.
        assert_eq!(
            worker
                .deliver_webhook("wh_1", "issue.opened", serde_json::json!({"n": 2}))
                .unwrap(),
            DispatchOutcome::AlreadyKnown
        );
        // A completed delivery answers from the ledger, instantly.
        let replayed = worker
            .await_webhook_ack("wh_1", Duration::from_millis(1))
            .unwrap()
            .expect("completed deliveries answer from the ledger");
        assert!(replayed.ok);
        worker.stop().unwrap();
    }

    #[test]
    fn a_re_delivered_webhook_never_reaches_the_worker() {
        // The child counts its deliveries and reports the count, so a
        // re-sent delivery would show up as one delivery too many.
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            None,
            launcher("jobs"),
        )
        .unwrap();
        worker
            .deliver_webhook("wh_1", "issue.opened", serde_json::json!({"n": 1}))
            .unwrap();
        worker
            .await_webhook_ack("wh_1", Duration::from_secs(5))
            .unwrap()
            .expect("first delivery completes");
        // Already known: nothing is sent for the re-delivery.
        worker
            .deliver_webhook("wh_1", "issue.opened", serde_json::json!({"n": 2}))
            .unwrap();

        worker
            .deliver_webhook("wh_2", "issue.closed", serde_json::json!({"n": 3}))
            .unwrap();
        worker
            .await_webhook_ack("wh_2", Duration::from_secs(5))
            .unwrap()
            .expect("the second delivery completes");
        worker
            .dispatch_job("count", serde_json::json!({"report_deliveries": true}))
            .unwrap();
        let report = worker
            .await_job_result("count", Duration::from_secs(5))
            .unwrap()
            .expect("the report job completes");
        // wh_1 once + wh_2 once: the re-delivery added nothing.
        assert_eq!(report.output, serde_json::json!({ "deliveries": 2 }));
        worker.stop().unwrap();
    }

    #[test]
    fn a_worker_without_the_webhooks_capability_refuses_delivery() {
        let mut manifest = manifest();
        manifest.capabilities = vec![PluginCapability::Jobs];
        let mut worker = SupervisedWorker::start(
            manifest,
            WorkerRestartPolicy::new(3),
            None,
            launcher("jobs"),
        )
        .unwrap();
        assert_eq!(
            worker
                .deliver_webhook("wh_1", "issue.opened", serde_json::json!({}))
                .unwrap_err(),
            WorkerError::CapabilityMissing {
                plugin_id: PluginId::new("plug_1"),
                capability: PluginCapability::Webhooks,
            }
        );
        worker.stop().unwrap();
    }

    #[test]
    fn a_webhook_ack_that_arrives_during_a_probe_is_not_lost() {
        // Deliver, then reconcile (probe): the ack can land while the
        // probe waits. Either receive order must deliver the ack.
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            probing(0, Duration::from_secs(5)),
            launcher("jobs"),
        )
        .unwrap();
        worker
            .deliver_webhook("wh_1", "issue.opened", serde_json::json!({"n": 1}))
            .unwrap();
        worker.reconcile().unwrap();
        let ack = worker
            .await_webhook_ack("wh_1", Duration::from_secs(5))
            .unwrap()
            .expect("the ack survives the probe");
        assert!(ack.ok);
        worker.stop().unwrap();
    }

    #[test]
    fn a_webhook_to_a_dead_worker_records_nothing_and_says_so() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(0),
            None,
            launcher("crash"),
        )
        .unwrap();
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::GiveUp
        );
        assert_eq!(
            worker
                .deliver_webhook("wh_1", "issue.opened", serde_json::json!({}))
                .unwrap(),
            DispatchOutcome::WorkerDown
        );
        assert_eq!(worker.webhook_state("wh_1"), None);
    }

    #[test]
    fn a_secret_is_acked_once_and_not_re_sent_to_the_same_process() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            None,
            launcher("jobs"),
        )
        .unwrap();
        assert_eq!(
            worker
                .hand_secret(
                    "api_token",
                    SecretString::new("super-secret-key"),
                    Duration::from_secs(5)
                )
                .unwrap(),
            SecretHandoffOutcome::Delivered { confirmed: true }
        );
        // The same live process already has it: not sent again.
        assert_eq!(
            worker
                .hand_secret(
                    "api_token",
                    SecretString::new("super-secret-key"),
                    Duration::from_secs(5)
                )
                .unwrap(),
            SecretHandoffOutcome::AlreadyProvided
        );
        worker
            .dispatch_job("count", serde_json::json!({"report_handoffs": true}))
            .unwrap();
        let report = worker
            .await_job_result("count", Duration::from_secs(5))
            .unwrap()
            .expect("the report job completes");
        assert_eq!(report.output, serde_json::json!({ "handoffs": 1 }));
        worker.stop().unwrap();
    }

    #[test]
    fn a_secret_to_a_dead_worker_is_remembered_and_not_burned() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(0),
            None,
            launcher("crash"),
        )
        .unwrap();
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::GiveUp
        );
        assert_eq!(
            worker
                .hand_secret(
                    "api_token",
                    SecretString::new("super-secret-key"),
                    Duration::from_secs(5)
                )
                .unwrap(),
            SecretHandoffOutcome::WorkerDown
        );
        // Unlike a dispatch, the secret is remembered — a returning
        // worker must be given it, and AlreadyProvided is not claimed
        // for a hand-off that never happened.
        assert_eq!(
            worker
                .hand_secret(
                    "api_token",
                    SecretString::new("super-secret-key"),
                    Duration::from_secs(5)
                )
                .unwrap(),
            SecretHandoffOutcome::WorkerDown
        );
    }

    #[test]
    fn a_worker_without_the_secrets_capability_refuses_a_hand_off() {
        let mut manifest = manifest();
        manifest.capabilities = vec![PluginCapability::Jobs];
        let mut worker = SupervisedWorker::start(
            manifest,
            WorkerRestartPolicy::new(3),
            None,
            launcher("jobs"),
        )
        .unwrap();
        assert_eq!(
            worker
                .hand_secret(
                    "api_token",
                    SecretString::new("super-secret-key"),
                    Duration::from_secs(5)
                )
                .unwrap_err(),
            WorkerError::CapabilityMissing {
                plugin_id: PluginId::new("plug_1"),
                capability: PluginCapability::ScopedSecrets,
            }
        );
        worker.stop().unwrap();
    }

    #[test]
    fn a_restarted_worker_is_handed_its_secrets_again() {
        // Launch 1 crashes; launch 2 takes the secret, acks it, and
        // dies; launch 3 behaves. The hand-off to launch 3 must happen
        // without the host being told again — restart transparency.
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
            None,
            sequencing_launcher(&["crash", "jobs_exit_after_secret", "jobs"]),
        )
        .unwrap();
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::Restart
        );
        assert_eq!(
            worker
                .hand_secret(
                    "api_token",
                    SecretString::new("super-secret-key"),
                    Duration::from_secs(5)
                )
                .unwrap(),
            SecretHandoffOutcome::Delivered { confirmed: true }
        );
        // The second child acked and exited: its death is observed and
        // the well-behaved replacement takes over.
        assert_eq!(
            reconcile_until_decided(&mut worker, 100),
            WorkerDirective::Restart
        );
        // The reconcile sweep re-hands the secret to the new process.
        worker.reconcile().unwrap();

        worker
            .dispatch_job("count", serde_json::json!({"report_handoffs": true}))
            .unwrap();
        let report = worker
            .await_job_result("count", Duration::from_secs(5))
            .unwrap()
            .expect("the report job completes");
        // Exactly one hand-off in this process: not zero (transparently
        // re-handed), not two (only re-handed once).
        assert_eq!(report.output, serde_json::json!({ "handoffs": 1 }));
        worker.stop().unwrap();
    }
}
