//! The semantic bridge from ALTAI App CSS tokens to terminal theme roles.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;

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

/// User-facing theme selection for `altai agent --theme` / `ALTAI_TUI_THEME`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalThemeMode {
    Auto,
    Dark,
    Light,
    NoColor,
}

impl TerminalThemeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
            Self::NoColor => "no-color",
        }
    }

    /// Parse a CLI or environment theme token.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "no-color" | "nocolor" | "plain" => Some(Self::NoColor),
            _ => None,
        }
    }
}

/// Concrete sRGB role colors derived once from `src/styles/globals.css` OKLCH tokens.
/// Values are checked in (not hand-tuned ANSI) so terminal and desktop stay aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// Resolved truecolor roles for one appearance mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedTerminalColors {
    pub canvas: Rgb,
    pub panel: Rgb,
    pub raised: Rgb,
    pub overlay: Rgb,
    pub text: Rgb,
    pub muted_text: Rgb,
    pub active: Rgb,
    pub focus: Rgb,
    pub success: Rgb,
    pub warning: Rgb,
    pub info: Rgb,
    pub error: Rgb,
    pub border: Rgb,
}

/// Dark IDE surface — near-black canvas, acid-lime active.
pub const DARK_TERMINAL_COLORS: ResolvedTerminalColors = ResolvedTerminalColors {
    canvas: Rgb::new(7, 7, 8),
    panel: Rgb::new(12, 13, 14),
    raised: Rgb::new(18, 19, 20),
    overlay: Rgb::new(22, 23, 25),
    text: Rgb::new(240, 242, 244),
    muted_text: Rgb::new(137, 140, 146),
    active: Rgb::new(181, 234, 38),
    focus: Rgb::new(93, 114, 149),
    success: Rgb::new(81, 198, 114),
    warning: Rgb::new(245, 174, 57),
    info: Rgb::new(75, 174, 237),
    error: Rgb::new(248, 75, 75),
    border: Rgb::new(55, 56, 60),
};

/// Light semantic counterpart of the dark reference.
pub const LIGHT_TERMINAL_COLORS: ResolvedTerminalColors = ResolvedTerminalColors {
    canvas: Rgb::new(251, 252, 252),
    panel: Rgb::new(255, 255, 255),
    raised: Rgb::new(244, 245, 247),
    overlay: Rgb::new(255, 255, 255),
    text: Rgb::new(19, 22, 28),
    muted_text: Rgb::new(81, 85, 92),
    active: Rgb::new(154, 211, 53),
    focus: Rgb::new(79, 100, 134),
    success: Rgb::new(0, 127, 53),
    warning: Rgb::new(201, 105, 0),
    info: Rgb::new(0, 106, 175),
    error: Rgb::new(212, 9, 36),
    border: Rgb::new(220, 222, 225),
};

/// Effective appearance after resolving auto / NO_COLOR / env.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveTerminalAppearance {
    Dark,
    Light,
    NoColor,
}

impl EffectiveTerminalAppearance {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::NoColor => "no-color",
        }
    }

    pub fn colors(self) -> Option<&'static ResolvedTerminalColors> {
        match self {
            Self::Dark => Some(&DARK_TERMINAL_COLORS),
            Self::Light => Some(&LIGHT_TERMINAL_COLORS),
            Self::NoColor => None,
        }
    }
}

/// Resolve CLI theme + `ALTAI_TUI_THEME` + `NO_COLOR` into an effective appearance.
///
/// Precedence: `NO_COLOR` always wins → explicit CLI mode → `ALTAI_TUI_THEME` →
/// `auto` (COLORFGBG / default dark).
pub fn resolve_terminal_appearance(
    cli_theme: TerminalThemeMode,
    no_color_env: bool,
    altai_theme_env: Option<&str>,
    colorfgbg: Option<&str>,
) -> EffectiveTerminalAppearance {
    if no_color_env || cli_theme == TerminalThemeMode::NoColor {
        return EffectiveTerminalAppearance::NoColor;
    }

    let selected = match cli_theme {
        TerminalThemeMode::Auto => altai_theme_env
            .and_then(TerminalThemeMode::parse)
            .unwrap_or(TerminalThemeMode::Auto),
        other => other,
    };

    match selected {
        TerminalThemeMode::Dark => EffectiveTerminalAppearance::Dark,
        TerminalThemeMode::Light => EffectiveTerminalAppearance::Light,
        TerminalThemeMode::NoColor => EffectiveTerminalAppearance::NoColor,
        TerminalThemeMode::Auto => detect_auto_appearance(colorfgbg),
    }
}

/// Convenience wrapper that reads `NO_COLOR`, `ALTAI_TUI_THEME`, and `COLORFGBG`.
pub fn resolve_terminal_appearance_from_env(
    cli_theme: TerminalThemeMode,
) -> EffectiveTerminalAppearance {
    let no_color = matches!(env::var_os("NO_COLOR"), Some(value) if !value.is_empty());
    let altai_theme = env::var("ALTAI_TUI_THEME").ok();
    let colorfgbg = env::var("COLORFGBG").ok();
    resolve_terminal_appearance(
        cli_theme,
        no_color,
        altai_theme.as_deref(),
        colorfgbg.as_deref(),
    )
}

fn detect_auto_appearance(colorfgbg: Option<&str>) -> EffectiveTerminalAppearance {
    // COLORFGBG is typically `fg;bg` with 0–15 ANSI indexes. High bg ⇒ light terminal.
    if let Some(raw) = colorfgbg {
        if let Some(bg) = raw
            .split(';')
            .nth(1)
            .and_then(|part| part.trim().parse::<u8>().ok())
        {
            if bg >= 8 {
                return EffectiveTerminalAppearance::Light;
            }
            return EffectiveTerminalAppearance::Dark;
        }
    }
    EffectiveTerminalAppearance::Dark
}

/// Layout density breakpoints for the Task Session TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalLayoutDensity {
    /// Below 80 columns: single pane + persistent status.
    Narrow,
    /// 80–119 columns: transcript with tabbed secondary panes.
    Medium,
    /// 120+ columns: transcript plus focused secondary pane.
    Wide,
}

impl TerminalLayoutDensity {
    pub fn from_cols(cols: u16) -> Self {
        if cols < 80 {
            Self::Narrow
        } else if cols < 120 {
            Self::Medium
        } else {
            Self::Wide
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Narrow => "narrow",
            Self::Medium => "medium",
            Self::Wide => "wide",
        }
    }
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

    #[test]
    fn no_color_always_wins() {
        assert_eq!(
            resolve_terminal_appearance(TerminalThemeMode::Dark, true, None, None),
            EffectiveTerminalAppearance::NoColor
        );
        assert_eq!(
            resolve_terminal_appearance(TerminalThemeMode::Light, false, Some("dark"), None),
            EffectiveTerminalAppearance::Light
        );
    }

    #[test]
    fn env_theme_applies_only_for_auto() {
        assert_eq!(
            resolve_terminal_appearance(TerminalThemeMode::Auto, false, Some("light"), None),
            EffectiveTerminalAppearance::Light
        );
        assert_eq!(
            resolve_terminal_appearance(TerminalThemeMode::Dark, false, Some("light"), None),
            EffectiveTerminalAppearance::Dark
        );
    }

    #[test]
    fn colorfgbg_detects_light_background() {
        assert_eq!(
            resolve_terminal_appearance(TerminalThemeMode::Auto, false, None, Some("0;15")),
            EffectiveTerminalAppearance::Light
        );
        assert_eq!(
            resolve_terminal_appearance(TerminalThemeMode::Auto, false, None, Some("15;0")),
            EffectiveTerminalAppearance::Dark
        );
    }

    #[test]
    fn layout_density_breakpoints() {
        assert_eq!(
            TerminalLayoutDensity::from_cols(79),
            TerminalLayoutDensity::Narrow
        );
        assert_eq!(
            TerminalLayoutDensity::from_cols(80),
            TerminalLayoutDensity::Medium
        );
        assert_eq!(
            TerminalLayoutDensity::from_cols(119),
            TerminalLayoutDensity::Medium
        );
        assert_eq!(
            TerminalLayoutDensity::from_cols(120),
            TerminalLayoutDensity::Wide
        );
    }

    #[test]
    fn dark_active_is_acid_lime_family() {
        // Primary brand accent must stay in the lime/yellow-green band.
        assert!(DARK_TERMINAL_COLORS.active.g > DARK_TERMINAL_COLORS.active.r);
        assert!(DARK_TERMINAL_COLORS.active.g > DARK_TERMINAL_COLORS.active.b);
    }
}
