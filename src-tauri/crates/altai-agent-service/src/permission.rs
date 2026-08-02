//! ALTAI UI permission-mode → IsanAgent shell-policy mode mapping.
//!
//! Host-neutral: any host that surfaces the ALTAI permission-mode selector
//! (`ask` / `auto-edit` / `plan` / `bypass`) can reuse this mapping instead of
//! re-implementing the fail-safe defaults.

use isanagent::config::ShellPolicyMode;

/// Map the ALTAI UI permission mode to an IsanAgent shell-policy mode for interactive sessions.
///
/// This maps only the **shell / code-execution** dimension. File edits are
/// gated separately via [`permission_mode_to_edit_mode`]; the two are
/// independent because "auto-edit" should auto-apply file changes while still
/// prompting for shell commands.
/// - `ask`, `auto-edit`, and `plan` → `Ask`: code-exec / destructive-shell still
///   require approval. `plan` keeps shell read-only-with-approval so the agent
///   can run `git status` / `ls` while planning but cannot silently mutate.
/// - `bypass` → `Allow`: no prompts (UI-gated behind an explicit Settings
///   toggle + warning).
/// - unknown / None → leaves the on-disk config default untouched (which defaults to `Ask`).
///
/// Fail-safe: any unrecognized value returns `None`, so it can never silently downgrade to
/// `Allow`.
pub fn permission_mode_to_shell_mode(mode: Option<&str>) -> Option<ShellPolicyMode> {
    match mode.map(str::trim) {
        Some("ask")
        | Some("ask_before_edit")
        | Some("ask-before-edit")
        | Some("auto-edit")
        | Some("auto_edit")
        | Some("auto")
        | Some("edit_automatically")
        | Some("plan") => Some(ShellPolicyMode::Ask),
        Some("bypass") | Some("bypass_permissions") => Some(ShellPolicyMode::Allow),
        _ => None,
    }
}

/// Map the UI permission mode to the **file-edit** policy mode.
///
/// This is independent from [`permission_mode_to_shell_mode`] because the two
/// surfaces have different risk profiles:
/// - `ask` → `Ask`: edits require an approval card with a diff preview.
/// - `auto-edit` → `Allow`: edits apply silently. Shell still requires approval
///   (see [`permission_mode_to_shell_mode`]) — "auto-edit" never auto-approves
///   shell. This is the Cursor-style default for users who trust file changes
///   but want to keep a human in the loop on commands.
/// - `plan` → `Deny`: no mutations at all. The crate's gate surfaces the
///   `plan mode active — finalize or apply the plan first` error to the model,
///   which keeps it read-only.
/// - `bypass` → `Allow`: no prompts (UI-gated behind an explicit Settings toggle).
/// - unknown / None → returns `None` so the on-disk config default is preserved
///   (which is `Ask`). Fail-safe: an unrecognized value can never silently
///   downgrade to `Allow`.
pub fn permission_mode_to_edit_mode(mode: Option<&str>) -> Option<ShellPolicyMode> {
    match mode.map(str::trim) {
        Some("ask") | Some("ask_before_edit") | Some("ask-before-edit") => {
            Some(ShellPolicyMode::Ask)
        }
        Some("auto-edit") | Some("auto_edit") | Some("auto") | Some("edit_automatically") => {
            Some(ShellPolicyMode::Allow)
        }
        Some("plan") => Some(ShellPolicyMode::Deny),
        Some("bypass") | Some("bypass_permissions") => Some(ShellPolicyMode::Allow),
        _ => None,
    }
}

#[cfg(test)]
mod permission_mode_tests {
    use super::*;

    #[test]
    fn only_bypass_allows_shell() {
        // ask and auto-edit must still gate shell/code (UI contract: auto-edit auto-approves
        // edits only). bypass is the sole mode that maps to Allow.
        assert_eq!(
            permission_mode_to_shell_mode(Some("ask")),
            Some(ShellPolicyMode::Ask)
        );
        assert_eq!(
            permission_mode_to_shell_mode(Some("auto-edit")),
            Some(ShellPolicyMode::Ask)
        );
        assert_eq!(
            permission_mode_to_shell_mode(Some("bypass")),
            Some(ShellPolicyMode::Allow)
        );
        // Unknown / empty must not downgrade to Allow — leave the on-disk default.
        assert_eq!(permission_mode_to_shell_mode(Some("nonsense")), None);
        assert_eq!(permission_mode_to_shell_mode(None), None);
    }

    #[test]
    fn edit_mode_ask_requires_approval() {
        assert_eq!(
            permission_mode_to_edit_mode(Some("ask")),
            Some(ShellPolicyMode::Ask)
        );
        assert_eq!(
            permission_mode_to_edit_mode(Some("ask_before_edit")),
            Some(ShellPolicyMode::Ask)
        );
        assert_eq!(
            permission_mode_to_edit_mode(Some("ask-before-edit")),
            Some(ShellPolicyMode::Ask)
        );
    }

    #[test]
    fn edit_mode_auto_edit_allows_silently() {
        assert_eq!(
            permission_mode_to_edit_mode(Some("auto-edit")),
            Some(ShellPolicyMode::Allow)
        );
        assert_eq!(
            permission_mode_to_edit_mode(Some("auto_edit")),
            Some(ShellPolicyMode::Allow)
        );
        assert_eq!(
            permission_mode_to_edit_mode(Some("auto")),
            Some(ShellPolicyMode::Allow)
        );
        assert_eq!(
            permission_mode_to_edit_mode(Some("edit_automatically")),
            Some(ShellPolicyMode::Allow)
        );
    }

    #[test]
    fn edit_mode_plan_denies_mutations() {
        assert_eq!(
            permission_mode_to_edit_mode(Some("plan")),
            Some(ShellPolicyMode::Deny)
        );
    }

    #[test]
    fn edit_mode_bypass_allows() {
        assert_eq!(
            permission_mode_to_edit_mode(Some("bypass")),
            Some(ShellPolicyMode::Allow)
        );
        assert_eq!(
            permission_mode_to_edit_mode(Some("bypass_permissions")),
            Some(ShellPolicyMode::Allow)
        );
    }

    #[test]
    fn edit_mode_unknown_or_none_preserves_default() {
        assert_eq!(permission_mode_to_edit_mode(Some("nonsense")), None);
        assert_eq!(permission_mode_to_edit_mode(None), None);
    }
}
