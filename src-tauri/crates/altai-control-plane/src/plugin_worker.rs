//! CP-08 plugin worker supervision (package 072, PR 1). An application
//! plugin runs a worker process; a crash in that process must be a fact
//! the host observes and answers by policy — never a reason the control
//! plane itself dies or spins. This module owns that policy as a pure
//! state machine over observations: the host (a later PR) launches real
//! processes, feeds what it saw here, and acts on the directive it gets
//! back. Nothing here performs process I/O.

use altai_control_protocol::{PluginId, PluginKind, PluginManifest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerError {
    /// Agent-content plugins declare no runtime, so there is nothing to
    /// supervise.
    NotAnApplication { plugin_id: PluginId },
    /// The observation cannot follow the state the worker is in.
    InvalidObservation { plugin_id: PluginId, reason: String },
    /// The worker process could not be launched.
    Launch { plugin_id: PluginId, reason: String },
    /// Interacting with a launched worker process failed.
    Process { reason: String },
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "plugin worker error: {self:?}")
    }
}
impl std::error::Error for WorkerError {}

/// Restart bound for one supervised session. The budget is total, not
/// rolling: a plugin that crashes its way through it stays down until a
/// human or an upgrade re-supervises, which is the crash-isolation
/// guarantee — a crashloop cannot spin forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerRestartPolicy {
    pub max_restarts: u32,
}

impl WorkerRestartPolicy {
    pub const fn new(max_restarts: u32) -> Self {
        Self { max_restarts }
    }
}

/// What the host observed its worker process do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerObservation {
    /// The process answered a health probe.
    Healthy,
    /// The process died; `reason` carries the exit story for surfacing.
    Crashed { reason: String },
    /// The host asked the process to stop (user uninstall, shutdown…).
    StoppedByHost,
}

/// The worker's supervision state, as the host may surface it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerHealth {
    /// Launched, not yet probed.
    Starting,
    /// Answered a health probe.
    Healthy,
    /// Dead. `exhausted` marks a budget spent — no further restarts this
    /// session.
    Crashed {
        restarts: u32,
        exhausted: bool,
        reason: String,
    },
    /// Stopped on purpose; restarts are not offered.
    Stopped,
}

/// What the host should do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerDirective {
    /// Nothing to do.
    None,
    /// Relaunch the process.
    Restart,
    /// Restart budget spent: leave it down and surface the crash.
    GiveUp,
}

/// One application plugin's supervision session. Pure: the same
/// observation sequence always yields the same health and directives.
#[derive(Debug, Clone)]
pub struct WorkerSupervisor {
    manifest: PluginManifest,
    policy: WorkerRestartPolicy,
    state: WorkerHealth,
    restarts: u32,
}

impl WorkerSupervisor {
    /// Only application plugins have a worker to supervise; the manifest's
    /// kind is the contract the registry already verified (071).
    pub fn new(
        manifest: PluginManifest,
        policy: WorkerRestartPolicy,
    ) -> Result<Self, WorkerError> {
        if manifest.kind != PluginKind::Application {
            return Err(WorkerError::NotAnApplication {
                plugin_id: manifest.plugin_id.clone(),
            });
        }
        Ok(Self {
            manifest,
            policy,
            state: WorkerHealth::Starting,
            restarts: 0,
        })
    }

    pub fn plugin_id(&self) -> &PluginId {
        &self.manifest.plugin_id
    }

    pub fn health(&self) -> &WorkerHealth {
        &self.state
    }

    /// Feed one observation, get the next directive. Idempotent for
    /// repeated terminal facts: a stop observed twice, or an exit arriving
    /// after the host already asked to stop, is the same single fact.
    pub fn observe(&mut self, observation: WorkerObservation) -> Result<WorkerDirective, WorkerError> {
        let invalid = |reason: &str| WorkerError::InvalidObservation {
            plugin_id: self.manifest.plugin_id.clone(),
            reason: reason.to_string(),
        };
        match observation {
            WorkerObservation::Healthy => match &self.state {
                WorkerHealth::Starting
                | WorkerHealth::Healthy
                | WorkerHealth::Crashed {
                    exhausted: false, ..
                } => {
                    // Healthy-from-crashed is the restart completing: the
                    // relaunched process answered its first probe.
                    self.state = WorkerHealth::Healthy;
                    Ok(WorkerDirective::None)
                }
                // A given-up or host-stopped worker cannot become healthy
                // again within this session; a new process is a new session.
                WorkerHealth::Crashed {
                    exhausted: true, ..
                }
                | WorkerHealth::Stopped => Err(invalid("healthy observation for a worker that is not running")),
            },
            WorkerObservation::Crashed { reason } => match self.state.clone() {
                WorkerHealth::Starting
                | WorkerHealth::Healthy
                // A replacement can die before its first probe: the
                // crash-before-healthy sequence is legal and consumes
                // budget like any other crash.
                | WorkerHealth::Crashed {
                    exhausted: false, ..
                } => {
                    if self.restarts < self.policy.max_restarts {
                        self.restarts += 1;
                        self.state = WorkerHealth::Crashed {
                            restarts: self.restarts,
                            exhausted: false,
                            reason: reason.clone(),
                        };
                        Ok(WorkerDirective::Restart)
                    } else {
                        self.state = WorkerHealth::Crashed {
                            restarts: self.restarts,
                            exhausted: true,
                            reason,
                        };
                        Ok(WorkerDirective::GiveUp)
                    }
                }
                // An exit racing the host's stop request is the stop
                // completing, not a new crash.
                WorkerHealth::Stopped => Ok(WorkerDirective::None),
                WorkerHealth::Crashed { exhausted: true, .. } => Err(invalid(
                    "crash observed for an already given-up worker",
                )),
            },
            WorkerObservation::StoppedByHost => {
                self.state = WorkerHealth::Stopped;
                Ok(WorkerDirective::None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{PluginCapability, PluginVersion};

    fn manifest(kind: PluginKind) -> PluginManifest {
        PluginManifest {
            plugin_id: PluginId::new("plug_1"),
            kind,
            version: PluginVersion::new(1, 0, 0),
            display_name: "Test plugin".into(),
            capabilities: match kind {
                PluginKind::Application => vec![PluginCapability::Jobs],
                PluginKind::AgentContent => Vec::new(),
            },
        }
    }

    fn supervisor(max_restarts: u32) -> WorkerSupervisor {
        WorkerSupervisor::new(manifest(PluginKind::Application), WorkerRestartPolicy::new(max_restarts)).unwrap()
    }

    #[test]
    fn an_agent_content_plugin_has_no_worker_to_supervise() {
        let error = WorkerSupervisor::new(
            manifest(PluginKind::AgentContent),
            WorkerRestartPolicy::new(3),
        )
        .unwrap_err();
        assert_eq!(error, WorkerError::NotAnApplication {
            plugin_id: PluginId::new("plug_1"),
        });
    }

    #[test]
    fn a_new_worker_starts_and_goes_healthy_on_its_first_probe() {
        let mut supervisor = supervisor(3);
        assert_eq!(supervisor.health(), &WorkerHealth::Starting);
        assert_eq!(
            supervisor.observe(WorkerObservation::Healthy).unwrap(),
            WorkerDirective::None
        );
        assert_eq!(supervisor.health(), &WorkerHealth::Healthy);
    }

    #[test]
    fn a_crash_within_budget_directs_a_restart() {
        let mut supervisor = supervisor(2);
        supervisor.observe(WorkerObservation::Healthy).unwrap();

        assert_eq!(
            supervisor
                .observe(WorkerObservation::Crashed { reason: "segfault".into() })
                .unwrap(),
            WorkerDirective::Restart
        );
        assert_eq!(
            supervisor.health(),
            &WorkerHealth::Crashed {
                restarts: 1,
                exhausted: false,
                reason: "segfault".into(),
            }
        );

        // The relaunched process reports healthy, then crashes again —
        // still within budget.
        supervisor.observe(WorkerObservation::Healthy).unwrap();
        assert_eq!(
            supervisor
                .observe(WorkerObservation::Crashed { reason: "oom".into() })
                .unwrap(),
            WorkerDirective::Restart
        );
    }

    #[test]
    fn a_replacement_that_dies_before_its_first_probe_consumes_budget() {
        // Real supervision found this sequence: a replacement can exit
        // before it ever answers a probe, so a crash can follow a crash.
        // Each death burns budget; only the spent budget is terminal.
        let mut supervisor = supervisor(2);
        supervisor.observe(WorkerObservation::Healthy).unwrap();
        assert_eq!(
            supervisor
                .observe(WorkerObservation::Crashed { reason: "first".into() })
                .unwrap(),
            WorkerDirective::Restart
        );
        assert_eq!(
            supervisor
                .observe(WorkerObservation::Crashed { reason: "replacement died".into() })
                .unwrap(),
            WorkerDirective::Restart
        );
        assert_eq!(
            supervisor.health(),
            &WorkerHealth::Crashed {
                restarts: 2,
                exhausted: false,
                reason: "replacement died".into(),
            }
        );
        // Budget spent: the next crash is terminal.
        assert_eq!(
            supervisor
                .observe(WorkerObservation::Crashed { reason: "third".into() })
                .unwrap(),
            WorkerDirective::GiveUp
        );
    }

    #[test]
    fn a_crashloop_exhausts_the_budget_and_gives_up_terminally() {
        let mut supervisor = supervisor(1);
        supervisor.observe(WorkerObservation::Healthy).unwrap();
        assert_eq!(
            supervisor
                .observe(WorkerObservation::Crashed { reason: "first".into() })
                .unwrap(),
            WorkerDirective::Restart
        );
        supervisor.observe(WorkerObservation::Healthy).unwrap();

        assert_eq!(
            supervisor
                .observe(WorkerObservation::Crashed { reason: "second".into() })
                .unwrap(),
            WorkerDirective::GiveUp
        );
        assert_eq!(
            supervisor.health(),
            &WorkerHealth::Crashed {
                restarts: 1,
                exhausted: true,
                reason: "second".into(),
            }
        );

        // No third chance within the session.
        assert!(matches!(
            supervisor.observe(WorkerObservation::Healthy),
            Err(WorkerError::InvalidObservation { .. })
        ));
    }

    #[test]
    fn a_host_stop_is_deliberate_and_terminal() {
        let mut supervisor = supervisor(3);
        supervisor.observe(WorkerObservation::Healthy).unwrap();
        assert_eq!(
            supervisor.observe(WorkerObservation::StoppedByHost).unwrap(),
            WorkerDirective::None
        );
        assert_eq!(supervisor.health(), &WorkerHealth::Stopped);

        // An exit racing the stop is the stop completing: no restart offer.
        assert_eq!(
            supervisor
                .observe(WorkerObservation::Crashed { reason: "exit during stop".into() })
                .unwrap(),
            WorkerDirective::None
        );
        // And a repeated stop observation is the same single fact.
        assert_eq!(
            supervisor.observe(WorkerObservation::StoppedByHost).unwrap(),
            WorkerDirective::None
        );
    }

    #[test]
    fn a_zero_restart_policy_never_restarts() {
        let mut supervisor = supervisor(0);
        assert_eq!(
            supervisor
                .observe(WorkerObservation::Crashed { reason: "immediate".into() })
                .unwrap(),
            WorkerDirective::GiveUp
        );
        assert_eq!(
            supervisor.health(),
            &WorkerHealth::Crashed {
                restarts: 0,
                exhausted: true,
                reason: "immediate".into(),
            }
        );
    }
}
