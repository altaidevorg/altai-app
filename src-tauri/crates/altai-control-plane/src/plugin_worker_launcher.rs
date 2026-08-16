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
//! packaged binary, an interpreter, or a test double. Health *probing*
//! (asking a live worker how it feels) needs the worker IPC transport and
//! is a later package-072 PR; here a process that has not exited is
//! running, and every exit is a crash observation.

use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use altai_control_protocol::PluginManifest;

use crate::plugin_worker::{WorkerDirective, WorkerError, WorkerHealth, WorkerObservation, WorkerRestartPolicy, WorkerSupervisor};

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
        let mut command = (self.build)(manifest);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = command.spawn().map_err(|error| WorkerError::Launch {
            plugin_id: manifest.plugin_id.clone(),
            reason: error.to_string(),
        })?;
        Ok(WorkerProcess { child })
    }
}

/// One launched worker process. Waiting is polling, not blocking: the
/// owner reconciles on its own schedule.
pub struct WorkerProcess {
    child: Child,
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

    /// Terminate the process. Killing an already-exited process is not an
    /// error: the fact "this process is done" does not change.
    pub fn stop(&mut self) -> Result<(), WorkerError> {
        self.child.kill().map_err(|error| WorkerError::Process {
            reason: error.to_string(),
        })
    }
}

/// A supervised worker end to end: the process and the policy over it in
/// one place. Every [`reconcile`](Self::reconcile) reports exits the
/// supervisor has not seen and acts on the directive it returns.
pub struct SupervisedWorker {
    supervisor: WorkerSupervisor,
    launcher: Arc<dyn WorkerLauncher>,
    manifest: PluginManifest,
    process: Option<WorkerProcess>,
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
        launcher: Arc<dyn WorkerLauncher>,
    ) -> Result<Self, WorkerError> {
        let supervisor = WorkerSupervisor::new(manifest.clone(), policy)?;
        let process = launcher.launch(&manifest)?;
        Ok(Self {
            supervisor,
            launcher,
            manifest,
            process: Some(process),
        })
    }

    pub fn health(&self) -> &WorkerHealth {
        self.supervisor.health()
    }

    pub fn plugin_id(&self) -> &altai_control_protocol::PluginId {
        self.supervisor.plugin_id()
    }

    /// Report any not-yet-seen exit, then act on the supervisor's
    /// directive: a `Restart` relaunches here, an exhausted budget leaves
    /// the plugin down, and a live process reconciles to `None`.
    pub fn reconcile(&mut self) -> Result<WorkerDirective, WorkerError> {
        let exited = match self.process.as_mut() {
            Some(process) => process.try_wait()?,
            None => None,
        };
        let Some(reason) = exited else {
            return Ok(WorkerDirective::None);
        };
        self.process = None;
        let directive = self
            .supervisor
            .observe(WorkerObservation::Crashed { reason })?;
        if directive == WorkerDirective::Restart {
            self.process = Some(self.launcher.launch(&self.manifest)?);
        }
        Ok(directive)
    }

    /// Stop on purpose: kill the process if any, then record the stop.
    /// Stopping twice is the same single fact.
    pub fn stop(&mut self) -> Result<(), WorkerError> {
        if let Some(mut process) = self.process.take() {
            process.stop()?;
        }
        self.supervisor.observe(WorkerObservation::StoppedByHost)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{PluginCapability, PluginId, PluginKind, PluginVersion};
    use std::time::Duration;

    fn manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: PluginId::new("plug_1"),
            kind: PluginKind::Application,
            version: PluginVersion::new(1, 0, 0),
            display_name: "Test plugin".into(),
            capabilities: vec![PluginCapability::Jobs],
        }
    }

    /// Runs this test binary again in a controlled child mode, so exit
    /// observation is exercised against real process exits on every
    /// platform CI runs on.
    fn child_command(mode: &'static str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "plugin_worker_launcher::tests::child_mode"])
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
            _ => {}
        }
    }

    fn launcher(mode: &'static str) -> Arc<CommandWorkerLauncher> {
        Arc::new(CommandWorkerLauncher::new(Box::new(move |_| {
            child_command(mode)
        })))
    }

    /// A launcher whose first worker crashes and whose replacements hang:
    /// the stand-in for a plugin whose bad build dies and whose relaunched
    /// process stays up.
    fn crash_once_launcher() -> Arc<CommandWorkerLauncher> {
        let crashed = std::sync::atomic::AtomicBool::new(false);
        Arc::new(CommandWorkerLauncher::new(Box::new(move |_| {
            if crashed.swap(true, std::sync::atomic::Ordering::SeqCst) {
                child_command("hang")
            } else {
                child_command("crash")
            }
        })))
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

    #[test]
    fn a_live_process_reconciles_to_none() {
        let mut worker = SupervisedWorker::start(
            manifest(),
            WorkerRestartPolicy::new(3),
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
            crash_once_launcher(),
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
}
