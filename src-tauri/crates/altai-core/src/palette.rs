//! The semantic bridge from ALTAI App CSS tokens to terminal theme roles.

use serde::Deserialize;
use std::collections::BTreeMap;

const PALETTE_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../shared/altai-terminal-palette.json"
));

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TerminalPaletteManifest {
    pub schema_version: u16,
    pub source: String,
    pub description: String,
    pub modes: BTreeMap<String, BTreeMap<String, String>>,
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteError {
    InvalidJson(String),
    UnsupportedSchema(u16),
    MissingMode(&'static str),
    MissingRole {
        mode: &'static str,
        role: &'static str,
    },
}

impl std::fmt::Display for PaletteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(f, "invalid ALTAI terminal palette JSON: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(f, "unsupported ALTAI terminal palette schema: {version}")
            }
            Self::MissingMode(mode) => write!(f, "terminal palette is missing {mode} mode"),
            Self::MissingRole { mode, role } => {
                write!(f, "terminal palette is missing {role} in {mode} mode")
            }
        }
    }
}

impl std::error::Error for PaletteError {}

/// Load and validate the checked-in terminal role manifest.
pub fn load_terminal_palette() -> Result<TerminalPaletteManifest, PaletteError> {
    let palette: TerminalPaletteManifest = serde_json::from_str(PALETTE_SOURCE)
        .map_err(|error| PaletteError::InvalidJson(error.to_string()))?;

    if palette.schema_version != 1 {
        return Err(PaletteError::UnsupportedSchema(palette.schema_version));
    }

    for mode in ["dark", "light"] {
        let roles = palette
            .modes
            .get(mode)
            .ok_or(PaletteError::MissingMode(mode))?;
        for role in ["canvas", "panel", "text", "active", "focus", "error"] {
            if !roles.contains_key(role) {
                return Err(PaletteError::MissingRole { mode, role });
            }
        }
    }

    Ok(palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_the_required_modes_and_roles() {
        let palette = load_terminal_palette().expect("palette should be valid");
        assert_eq!(palette.source, "src/styles/globals.css");
        assert_eq!(palette.modes["dark"]["active"], "--primary");
        assert!(palette
            .fallbacks
            .iter()
            .any(|fallback| fallback == "no-color"));
    }
}
