//! Permission mode mapping shared by CLI adapters (mirrors desktop runtime policy).

/// Host-facing permission modes for ALTAI CLI / Desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionPolicyMode {
    Ask,
    AutoEdit,
    Plan,
    Bypass,
}

impl PermissionPolicyMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::AutoEdit => "auto-edit",
            Self::Plan => "plan",
            Self::Bypass => "bypass",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ask" | "ask_before_edit" | "ask-before-edit" => Some(Self::Ask),
            "auto-edit" | "auto_edit" | "auto" | "edit_automatically" => Some(Self::AutoEdit),
            "plan" => Some(Self::Plan),
            "bypass" | "bypass_permissions" => Some(Self::Bypass),
            _ => None,
        }
    }
}

/// Shell / edit policy pair applied to IsanAgent `ResolvedShellPolicy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellEditPolicyModes {
    /// `ask` | `deny` | `allow`
    pub shell: &'static str,
    /// `ask` | `deny` | `allow`
    pub edit: &'static str,
}

/// Map ALTAI permission modes to IsanAgent shell/edit policy strings.
///
/// Parity with desktop `permission_mode_to_shell_mode` /
/// `permission_mode_to_edit_mode`: `plan` keeps shell at `ask` (destructive
/// commands still prompt) while denying file edits.
pub fn shell_edit_modes_for(permission: PermissionPolicyMode) -> ShellEditPolicyModes {
    match permission {
        PermissionPolicyMode::Ask => ShellEditPolicyModes {
            shell: "ask",
            edit: "ask",
        },
        PermissionPolicyMode::AutoEdit => ShellEditPolicyModes {
            shell: "ask",
            edit: "allow",
        },
        PermissionPolicyMode::Plan => ShellEditPolicyModes {
            shell: "ask",
            edit: "deny",
        },
        PermissionPolicyMode::Bypass => ShellEditPolicyModes {
            shell: "allow",
            edit: "allow",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_keeps_shell_ask_and_denies_edits() {
        let modes = shell_edit_modes_for(PermissionPolicyMode::Plan);
        assert_eq!(modes.shell, "ask");
        assert_eq!(modes.edit, "deny");
    }

    #[test]
    fn bypass_allows_both() {
        let modes = shell_edit_modes_for(PermissionPolicyMode::Bypass);
        assert_eq!(modes.shell, "allow");
        assert_eq!(modes.edit, "allow");
    }
}
