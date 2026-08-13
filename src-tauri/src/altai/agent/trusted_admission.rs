//! CP-08-08 host-only composition for one authorized attempt admission.
//!
//! This deliberately returns an in-process value. It is not a Tauri command
//! and is never serialized, because it contains the host-resolved credential.

use altai_agent_service::AttemptExecutionRequest;
use altai_control_plane::AgentRepository;
use altai_control_protocol::{Attempt, RunBinding, SessionId};
use tauri::AppHandle;

use super::{
    attempt_adapter::{adapt_attempt, TrustedAttemptInput},
    execution_profile::resolve_authorized_execution_profile,
    trusted_profile::{resolve_trusted_execution_profile, TrustedExecutionProfile},
};
use crate::modules::secrets::SecretsState;

/// All data needed to admit a canonical attempt into the execution runtime.
/// `profile` must remain on the native side because it carries an API key.
#[allow(dead_code)] // CP-08-09 calls this from the native start command.
pub struct TrustedAttemptAdmission {
    pub execution: AttemptExecutionRequest,
    pub profile: TrustedExecutionProfile,
    pub instructions: String,
}

/// Compose one native-only execution admission from durable records.
///
/// The caller provides no model, provider, endpoint, or API key: those values
/// are derived from the attempt's owner revision and the OS secret store.
#[allow(dead_code)] // staged with its CP-08-09 caller to keep the seam testable.
#[allow(clippy::too_many_arguments)]
pub fn prepare_trusted_attempt_admission(
    app: &AppHandle,
    secrets_state: &SecretsState,
    agents: &dyn AgentRepository,
    attempt: Attempt,
    run_binding: RunBinding,
    session_id: SessionId,
    prompt: String,
    context_pack: String,
    permission_policy: String,
) -> Result<TrustedAttemptAdmission, String> {
    let authorized = resolve_authorized_execution_profile(agents, &attempt)
        .map_err(|error| format!("Attempt profile authorization failed: {error:?}"))?;
    let profile = resolve_trusted_execution_profile(
        app,
        secrets_state,
        authorized.revision.model.as_deref(),
        &permission_policy,
    )?;
    let execution = adapt_attempt(TrustedAttemptInput {
        attempt,
        run_binding,
        session_id,
        prompt,
        context_pack,
        permission_policy: profile.permission_mode.clone(),
    })
    .map_err(|error| error.to_string())?;
    Ok(TrustedAttemptAdmission {
        execution,
        profile,
        instructions: authorized.revision.instructions,
    })
}
