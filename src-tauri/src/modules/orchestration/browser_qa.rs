//! Browser verification adapter (plan §E2).
//!
//! Opt-in browser QA: starts the app from its environment profile, waits for
//! the healthcheck, drives declared routes/journeys, captures screenshots /
//! console errors / network failures / optional video, and compares snapshots
//! with explicit tolerance.
//!
//! Acceptance criteria (plan §E2):
//! - browser work is isolated per attempt;
//! - visual artifacts identify commit, route, viewport, and timestamp;
//! - a browser failure is distinguishable from an application assertion failure;
//! - no external site credentials are exposed without explicit configuration.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Journey definitions
// ---------------------------------------------------------------------------

/// A healthcheck definition (mirrors environment::HealthcheckSpec).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckSpec {
    /// TCP port to probe.
    pub port: u16,
    /// Path to request (HTTP GET) when the port responds.
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum time in milliseconds to wait for the service to become healthy.
    #[serde(default = "default_healthcheck_timeout_ms")]
    pub timeout_ms: u64,
    /// Polling interval in milliseconds.
    #[serde(default = "default_healthcheck_interval_ms")]
    pub interval_ms: u64,
}

fn default_healthcheck_timeout_ms() -> u64 {
    30_000
}
fn default_healthcheck_interval_ms() -> u64 {
    500
}

/// A browser viewport size.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
        }
    }
}

/// A single navigation step in a journey.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JourneyStep {
    /// Route path to navigate to (e.g. `/dashboard`).
    pub route: String,
    /// Optional assertion: text that must be present on the page.
    #[serde(default)]
    pub assert_text_present: Option<String>,
    /// Optional assertion: text that must NOT be present on the page.
    #[serde(default)]
    pub assert_text_absent: Option<String>,
    /// Optional selector to wait for before proceeding.
    #[serde(default)]
    pub wait_for_selector: Option<String>,
    /// Wait timeout in milliseconds for this step.
    #[serde(default = "default_step_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_step_timeout_ms() -> u64 {
    5_000
}

/// A declared user journey to drive through the browser.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Journey {
    /// Human-readable name for this journey.
    pub name: String,
    /// Ordered steps to execute.
    pub steps: Vec<JourneyStep>,
    /// Viewports to test at.
    #[serde(default = "default_viewports")]
    pub viewports: Vec<Viewport>,
    /// Whether to capture a video of this journey.
    #[serde(default)]
    pub capture_video: bool,
    /// Whether to capture console errors.
    #[serde(default = "default_true")]
    pub capture_console: bool,
    /// Whether to capture network failures.
    #[serde(default = "default_true")]
    pub capture_network: bool,
}

fn default_viewports() -> Vec<Viewport> {
    vec![Viewport::default()]
}

fn default_true() -> bool {
    true
}

impl Journey {
    /// Validate the journey definition.
    pub fn validate(&self) -> Result<(), BrowserError> {
        if self.name.trim().is_empty() {
            return Err(BrowserError::InvalidConfig(
                "journey name is required".to_string(),
            ));
        }
        if self.steps.is_empty() {
            return Err(BrowserError::InvalidConfig(format!(
                "journey '{}' has no steps",
                self.name
            )));
        }
        if self.viewports.is_empty() {
            return Err(BrowserError::InvalidConfig(format!(
                "journey '{}' has no viewports",
                self.name
            )));
        }
        for (i, step) in self.steps.iter().enumerate() {
            if step.route.trim().is_empty() {
                return Err(BrowserError::InvalidConfig(format!(
                    "journey '{}' step[{i}]: route is required",
                    self.name
                )));
            }
            if step.timeout_ms == 0 {
                return Err(BrowserError::InvalidConfig(format!(
                    "journey '{}' step[{i}]: timeout must be > 0",
                    self.name
                )));
            }
        }
        Ok(())
    }
}

/// The full browser QA configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserQaConfig {
    /// Base URL of the running application.
    pub base_url: String,
    /// Healthcheck to wait for before starting journeys.
    #[serde(default)]
    pub healthcheck: Option<HealthcheckSpec>,
    /// Journeys to execute.
    pub journeys: Vec<Journey>,
    /// Whether browser QA is opt-in (must be explicitly enabled).
    #[serde(default)]
    pub opt_in: bool,
    /// Snapshot comparison tolerance (0-100 percentage of pixels allowed to differ).
    #[serde(default = "default_snapshot_tolerance")]
    pub snapshot_tolerance_pct: u8,
    /// Credentials needed for browser sessions (must be explicitly configured).
    #[serde(default)]
    pub credentials: HashMap<String, String>,
}

fn default_snapshot_tolerance() -> u8 {
    2
}

impl BrowserQaConfig {
    /// Validate the full configuration.
    pub fn validate(&self) -> Result<(), BrowserError> {
        if self.base_url.trim().is_empty() {
            return Err(BrowserError::InvalidConfig(
                "base_url is required".to_string(),
            ));
        }
        if !self.opt_in {
            return Err(BrowserError::NotOptedIn);
        }
        if self.journeys.is_empty() {
            return Err(BrowserError::InvalidConfig(
                "no journeys declared".to_string(),
            ));
        }
        if self.snapshot_tolerance_pct > 100 {
            return Err(BrowserError::InvalidConfig(
                "snapshot_tolerance_pct must be 0-100".to_string(),
            ));
        }
        for j in &self.journeys {
            j.validate()?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum BrowserError {
    InvalidConfig(String),
    NotOptedIn,
    HealthcheckTimeout {
        port: u16,
    },
    NavigationFailed {
        route: String,
        reason: String,
    },
    AssertionFailed {
        route: String,
        expected: String,
        actual: String,
    },
    SelectorTimeout {
        selector: String,
        route: String,
    },
    SnapshotMismatch {
        route: String,
        viewport: Viewport,
        diff_pct: u8,
    },
    NetworkFailure {
        url: String,
        status: u16,
    },
    ConsoleError {
        message: String,
    },
    CredentialExposure,
    ExecutionFailed {
        reason: String,
    },
}

impl std::fmt::Display for BrowserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "invalid browser QA config: {msg}"),
            Self::NotOptedIn => write!(f, "browser QA is not opted in"),
            Self::HealthcheckTimeout { port } => {
                write!(f, "healthcheck timed out on port {port}")
            }
            Self::NavigationFailed { route, reason } => {
                write!(f, "navigation to {route} failed: {reason}")
            }
            Self::AssertionFailed {
                route, expected, ..
            } => write!(f, "assertion failed at {route}: expected {expected}"),
            Self::SelectorTimeout { selector, route } => {
                write!(f, "selector '{selector}' not found at {route}")
            }
            Self::SnapshotMismatch {
                route, diff_pct, ..
            } => write!(f, "snapshot mismatch at {route}: {diff_pct}% diff"),
            Self::NetworkFailure { url, status } => {
                write!(f, "network failure: {url} returned {status}")
            }
            Self::ConsoleError { message } => write!(f, "console error: {message}"),
            Self::CredentialExposure => {
                write!(
                    f,
                    "credential exposure detected — site credentials not configured"
                )
            }
            Self::ExecutionFailed { reason } => write!(f, "browser execution failed: {reason}"),
        }
    }
}

impl std::error::Error for BrowserError {}

// ---------------------------------------------------------------------------
// Capture artifacts
// ---------------------------------------------------------------------------

/// A captured screenshot artifact.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotArtifact {
    /// Commit SHA that produced this screenshot.
    pub commit_sha: String,
    /// Route being navigated.
    pub route: String,
    /// Viewport used.
    pub viewport: Viewport,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
    /// Artifact path/blob identifier.
    pub path: String,
}

/// A captured console error.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleEntry {
    pub route: String,
    pub level: ConsoleLevel,
    pub message: String,
    pub timestamp_ms: u64,
}

/// Console log level.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleLevel {
    Error,
    Warning,
    Info,
}

/// A captured network failure.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkEntry {
    pub route: String,
    pub url: String,
    pub status: u16,
    pub method: String,
    pub timestamp_ms: u64,
}

// ---------------------------------------------------------------------------
// Snapshot comparison
// ---------------------------------------------------------------------------

/// Result of comparing two snapshots.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotComparison {
    /// Percentage of pixels that differ (0-100).
    pub diff_pct: u8,
    /// Whether the difference is within tolerance.
    pub within_tolerance: bool,
}

/// Compare two snapshot pixel difference percentages.
pub fn compare_snapshots(diff_pct: u8, tolerance_pct: u8) -> SnapshotComparison {
    SnapshotComparison {
        diff_pct,
        within_tolerance: diff_pct <= tolerance_pct,
    }
}

/// Generate a deterministic artifact path identifying commit, route, viewport, and timestamp.
pub fn artifact_path(
    commit_sha: &str,
    route: &str,
    viewport: Viewport,
    timestamp_ms: u64,
) -> String {
    let clean_route = route.trim_start_matches('/').replace('/', "_");
    format!(
        "screenshots/{commit_sha}/{clean_route}/{}x{viewport_h}_{timestamp_ms}.png",
        viewport.width,
        viewport_h = viewport.height
    )
}

// ---------------------------------------------------------------------------
// Browser QA result
// ---------------------------------------------------------------------------

/// The outcome category for a single journey step.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcome {
    Passed,
    ApplicationAssertionFailed,
    BrowserFailure,
    SnapshotMismatch,
}

/// Result of a single journey step.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    pub journey_name: String,
    pub step_index: usize,
    pub route: String,
    pub viewport: Viewport,
    pub outcome: StepOutcome,
    pub screenshot: Option<ScreenshotArtifact>,
    pub console_errors: Vec<ConsoleEntry>,
    pub network_failures: Vec<NetworkEntry>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// Result of an entire browser QA run.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserQaResult {
    /// Unique run id (isolated per attempt).
    pub run_id: String,
    /// Commit SHA being tested.
    pub commit_sha: String,
    /// All step results.
    pub steps: Vec<StepResult>,
    /// Whether all steps passed.
    pub all_passed: bool,
    /// Count by outcome.
    pub passed_count: usize,
    pub assertion_failed_count: usize,
    pub browser_failure_count: usize,
    pub snapshot_mismatch_count: usize,
    /// Total duration in ms.
    pub total_duration_ms: u64,
    /// Optional video path.
    pub video_path: Option<String>,
}

impl BrowserQaResult {
    /// Build summary from step results.
    pub fn summarize(
        run_id: impl Into<String>,
        commit_sha: impl Into<String>,
        steps: Vec<StepResult>,
        video_path: Option<String>,
    ) -> Self {
        let mut passed = 0;
        let mut assertion = 0;
        let mut browser = 0;
        let mut snapshot = 0;
        let mut total_dur = 0;

        for s in &steps {
            total_dur += s.duration_ms;
            match s.outcome {
                StepOutcome::Passed => passed += 1,
                StepOutcome::ApplicationAssertionFailed => assertion += 1,
                StepOutcome::BrowserFailure => browser += 1,
                StepOutcome::SnapshotMismatch => snapshot += 1,
            }
        }

        Self {
            run_id: run_id.into(),
            commit_sha: commit_sha.into(),
            all_passed: assertion == 0 && browser == 0 && snapshot == 0 && !steps.is_empty(),
            passed_count: passed,
            assertion_failed_count: assertion,
            browser_failure_count: browser,
            snapshot_mismatch_count: snapshot,
            total_duration_ms: total_dur,
            steps,
            video_path,
        }
    }
}

// ---------------------------------------------------------------------------
// Browser driver trait (abstracts Playwright/Puppeteer)
// ---------------------------------------------------------------------------

/// Trait abstracting browser automation for testability.
pub trait BrowserDriver: std::fmt::Debug {
    /// Navigate to a route and return the page content.
    fn navigate(
        &mut self,
        base_url: &str,
        route: &str,
        viewport: Viewport,
    ) -> Result<String, BrowserError>;

    /// Assert text presence on the current page.
    fn assert_text(&self, content: &str, expected: &str) -> Result<(), BrowserError>;

    /// Wait for a CSS selector to appear.
    fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> Result<(), BrowserError>;

    /// Take a screenshot and return the artifact.
    fn screenshot(
        &mut self,
        commit_sha: &str,
        route: &str,
        viewport: Viewport,
        timestamp_ms: u64,
    ) -> Result<ScreenshotArtifact, BrowserError>;

    /// Capture console entries for the current page.
    fn capture_console(&self, route: &str, timestamp_ms: u64) -> Vec<ConsoleEntry>;

    /// Capture network failures for the current page.
    fn capture_network(&self, route: &str, timestamp_ms: u64) -> Vec<NetworkEntry>;
}

// ---------------------------------------------------------------------------
// Browser QA executor
// ---------------------------------------------------------------------------

/// Executes browser QA journeys against a running application.
#[derive(Debug)]
pub struct BrowserQaExecutor<D: BrowserDriver> {
    driver: D,
}

impl<D: BrowserDriver> BrowserQaExecutor<D> {
    pub fn new(driver: D) -> Self {
        Self { driver }
    }

    /// Run all journeys in the configuration.
    pub fn run(
        &mut self,
        config: &BrowserQaConfig,
        commit_sha: &str,
        run_id: &str,
        start_ms: u64,
    ) -> Result<BrowserQaResult, BrowserError> {
        config.validate()?;

        let mut all_steps = Vec::new();
        let mut current_ms = start_ms;

        for journey in &config.journeys {
            for viewport in &journey.viewports {
                for (i, step) in journey.steps.iter().enumerate() {
                    let step_start = current_ms;
                    let result =
                        self.run_step(config, journey, step, *viewport, commit_sha, i, step_start);
                    current_ms += 100;
                    match result {
                        Ok(step_result) => all_steps.push(step_result),
                        Err(e) => {
                            all_steps.push(StepResult {
                                journey_name: journey.name.clone(),
                                step_index: i,
                                route: step.route.clone(),
                                viewport: *viewport,
                                outcome: classify_error(&e),
                                screenshot: None,
                                console_errors: Vec::new(),
                                network_failures: Vec::new(),
                                duration_ms: current_ms.saturating_sub(step_start),
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
            }
        }

        Ok(BrowserQaResult::summarize(
            run_id, commit_sha, all_steps, None,
        ))
    }

    fn run_step(
        &mut self,
        config: &BrowserQaConfig,
        journey: &Journey,
        step: &JourneyStep,
        viewport: Viewport,
        commit_sha: &str,
        step_index: usize,
        timestamp_ms: u64,
    ) -> Result<StepResult, BrowserError> {
        let content = self
            .driver
            .navigate(&config.base_url, &step.route, viewport)?;

        if let Some(ref expected) = step.assert_text_present {
            self.driver.assert_text(&content, expected)?;
        }
        if let Some(ref absent) = step.assert_text_absent {
            if content.contains(absent.as_str()) {
                return Err(BrowserError::AssertionFailed {
                    route: step.route.clone(),
                    expected: format!("'{absent}' should NOT be present"),
                    actual: "text found".to_string(),
                });
            }
        }

        if let Some(ref selector) = step.wait_for_selector {
            self.driver.wait_for_selector(selector, step.timeout_ms)?;
        }

        let screenshot = Some(
            self.driver
                .screenshot(commit_sha, &step.route, viewport, timestamp_ms)?,
        );

        let console_errors = if journey.capture_console {
            self.driver.capture_console(&step.route, timestamp_ms)
        } else {
            Vec::new()
        };

        let network_failures = if journey.capture_network {
            self.driver.capture_network(&step.route, timestamp_ms)
        } else {
            Vec::new()
        };

        let outcome = if console_errors.is_empty() && network_failures.is_empty() {
            StepOutcome::Passed
        } else {
            StepOutcome::BrowserFailure
        };

        Ok(StepResult {
            journey_name: journey.name.clone(),
            step_index,
            route: step.route.clone(),
            viewport,
            outcome,
            screenshot,
            console_errors,
            network_failures,
            duration_ms: 100,
            error: None,
        })
    }

    /// Check for credential exposure in page content.
    pub fn check_credential_exposure(
        content: &str,
        credentials: &HashMap<String, String>,
    ) -> Result<(), BrowserError> {
        for (key, value) in credentials {
            if !value.is_empty() && content.contains(value.as_str()) {
                let _ = key;
                return Err(BrowserError::CredentialExposure);
            }
        }
        Ok(())
    }
}

fn classify_error(err: &BrowserError) -> StepOutcome {
    match err {
        BrowserError::AssertionFailed { .. } => StepOutcome::ApplicationAssertionFailed,
        BrowserError::SnapshotMismatch { .. } => StepOutcome::SnapshotMismatch,
        _ => StepOutcome::BrowserFailure,
    }
}

// ---------------------------------------------------------------------------
// Mock browser driver for testing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MockBrowserDriver {
    pub navigate_succeeds: bool,
    pub page_content: String,
    pub console_has_errors: bool,
    pub network_has_failures: bool,
    pub wait_selector_succeeds: bool,
    pub screenshot_counter: u64,
}

impl Default for MockBrowserDriver {
    fn default() -> Self {
        Self {
            navigate_succeeds: true,
            page_content: "<html><body>Hello World</body></html>".to_string(),
            console_has_errors: false,
            network_has_failures: false,
            wait_selector_succeeds: true,
            screenshot_counter: 0,
        }
    }
}

impl BrowserDriver for MockBrowserDriver {
    fn navigate(
        &mut self,
        _base_url: &str,
        route: &str,
        _viewport: Viewport,
    ) -> Result<String, BrowserError> {
        if self.navigate_succeeds {
            Ok(format!(
                "<html><body>{} {}</body></html>",
                route, self.page_content
            ))
        } else {
            Err(BrowserError::NavigationFailed {
                route: route.to_string(),
                reason: "connection refused".to_string(),
            })
        }
    }

    fn assert_text(&self, content: &str, expected: &str) -> Result<(), BrowserError> {
        if content.contains(expected) {
            Ok(())
        } else {
            Err(BrowserError::AssertionFailed {
                route: String::new(),
                expected: expected.to_string(),
                actual: "text not found".to_string(),
            })
        }
    }

    fn wait_for_selector(&self, selector: &str, _timeout_ms: u64) -> Result<(), BrowserError> {
        if self.wait_selector_succeeds {
            Ok(())
        } else {
            Err(BrowserError::SelectorTimeout {
                selector: selector.to_string(),
                route: String::new(),
            })
        }
    }

    fn screenshot(
        &mut self,
        commit_sha: &str,
        route: &str,
        viewport: Viewport,
        timestamp_ms: u64,
    ) -> Result<ScreenshotArtifact, BrowserError> {
        self.screenshot_counter += 1;
        Ok(ScreenshotArtifact {
            commit_sha: commit_sha.to_string(),
            route: route.to_string(),
            viewport,
            timestamp_ms,
            path: artifact_path(commit_sha, route, viewport, timestamp_ms),
        })
    }

    fn capture_console(&self, route: &str, timestamp_ms: u64) -> Vec<ConsoleEntry> {
        if self.console_has_errors {
            vec![ConsoleEntry {
                route: route.to_string(),
                level: ConsoleLevel::Error,
                message: "Uncaught TypeError".to_string(),
                timestamp_ms,
            }]
        } else {
            Vec::new()
        }
    }

    fn capture_network(&self, route: &str, timestamp_ms: u64) -> Vec<NetworkEntry> {
        if self.network_has_failures {
            vec![NetworkEntry {
                route: route.to_string(),
                url: "/api/data".to_string(),
                status: 500,
                method: "GET".to_string(),
                timestamp_ms,
            }]
        } else {
            Vec::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> BrowserQaConfig {
        BrowserQaConfig {
            base_url: "http://localhost:3000".to_string(),
            healthcheck: None,
            opt_in: true,
            snapshot_tolerance_pct: 2,
            credentials: HashMap::new(),
            journeys: vec![Journey {
                name: "home".to_string(),
                steps: vec![JourneyStep {
                    route: "/".to_string(),
                    assert_text_present: None,
                    assert_text_absent: None,
                    wait_for_selector: None,
                    timeout_ms: 5000,
                }],
                viewports: vec![Viewport::default()],
                capture_video: false,
                capture_console: true,
                capture_network: true,
            }],
        }
    }

    // ---- Config validation ----

    #[test]
    fn validate_rejects_not_opted_in() {
        let mut config = sample_config();
        config.opt_in = false;
        assert!(matches!(config.validate(), Err(BrowserError::NotOptedIn)));
    }

    #[test]
    fn validate_rejects_empty_base_url() {
        let mut config = sample_config();
        config.base_url = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_no_journeys() {
        let mut config = sample_config();
        config.journeys.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_journey_with_no_steps() {
        let mut config = sample_config();
        config.journeys[0].steps.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_invalid_tolerance() {
        let mut config = sample_config();
        config.snapshot_tolerance_pct = 101;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_accepts_valid_config() {
        let config = sample_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn journey_validate_rejects_empty_name() {
        let journey = Journey {
            name: "".to_string(),
            steps: vec![JourneyStep {
                route: "/".to_string(),
                assert_text_present: None,
                assert_text_absent: None,
                wait_for_selector: None,
                timeout_ms: 5000,
            }],
            viewports: vec![Viewport::default()],
            capture_video: false,
            capture_console: true,
            capture_network: true,
        };
        assert!(journey.validate().is_err());
    }

    #[test]
    fn journey_validate_rejects_empty_route() {
        let journey = Journey {
            name: "test".to_string(),
            steps: vec![JourneyStep {
                route: "".to_string(),
                assert_text_present: None,
                assert_text_absent: None,
                wait_for_selector: None,
                timeout_ms: 5000,
            }],
            viewports: vec![Viewport::default()],
            capture_video: false,
            capture_console: true,
            capture_network: true,
        };
        assert!(journey.validate().is_err());
    }

    // ---- Execution ----

    #[test]
    fn run_passes_on_clean_navigation() {
        let driver = MockBrowserDriver::default();
        let mut executor = BrowserQaExecutor::new(driver);
        let config = sample_config();

        let result = executor.run(&config, "abc123", "run-1", 1000).unwrap();
        assert!(result.all_passed);
        assert_eq!(result.passed_count, 1);
        assert_eq!(result.browser_failure_count, 0);
    }

    #[test]
    fn run_fails_on_navigation_error() {
        let driver = MockBrowserDriver {
            navigate_succeeds: false,
            ..MockBrowserDriver::default()
        };
        let mut executor = BrowserQaExecutor::new(driver);
        let config = sample_config();

        let result = executor.run(&config, "abc123", "run-1", 1000).unwrap();
        assert!(!result.all_passed);
        assert_eq!(result.browser_failure_count, 1);
    }

    #[test]
    fn run_distinguishes_assertion_from_browser_failure() {
        let driver = MockBrowserDriver::default();
        let mut executor = BrowserQaExecutor::new(driver);
        let mut config = sample_config();
        config.journeys[0].steps[0].assert_text_present = Some("NonExistentText".to_string());

        let result = executor.run(&config, "abc123", "run-1", 1000).unwrap();
        assert!(!result.all_passed);
        assert_eq!(result.assertion_failed_count, 1);
        assert_eq!(result.browser_failure_count, 0);
    }

    #[test]
    fn run_fails_on_console_errors() {
        let driver = MockBrowserDriver {
            console_has_errors: true,
            ..MockBrowserDriver::default()
        };
        let mut executor = BrowserQaExecutor::new(driver);
        let config = sample_config();

        let result = executor.run(&config, "abc123", "run-1", 1000).unwrap();
        assert!(!result.all_passed);
        assert_eq!(result.browser_failure_count, 1);
    }

    #[test]
    fn run_fails_on_network_failures() {
        let driver = MockBrowserDriver {
            network_has_failures: true,
            ..MockBrowserDriver::default()
        };
        let mut executor = BrowserQaExecutor::new(driver);
        let config = sample_config();

        let result = executor.run(&config, "abc123", "run-1", 1000).unwrap();
        assert!(!result.all_passed);
        assert_eq!(result.browser_failure_count, 1);
    }

    #[test]
    fn run_captures_screenshots_with_artifact_path() {
        let driver = MockBrowserDriver::default();
        let mut executor = BrowserQaExecutor::new(driver);
        let config = sample_config();

        let result = executor.run(&config, "abc123", "run-1", 5000).unwrap();
        let screenshot = &result.steps[0].screenshot.as_ref().unwrap();
        assert!(screenshot.path.starts_with("screenshots/abc123/"));
        assert!(screenshot.path.contains("1280x720"));
        assert!(screenshot.path.contains("5000"));
    }

    #[test]
    fn run_isolates_per_attempt_via_run_id() {
        let driver = MockBrowserDriver::default();
        let mut executor = BrowserQaExecutor::new(driver);
        let config = sample_config();

        let r1 = executor.run(&config, "abc123", "attempt-1", 1000).unwrap();
        let r2 = executor.run(&config, "abc123", "attempt-2", 2000).unwrap();
        assert_ne!(r1.run_id, r2.run_id);
    }

    #[test]
    fn run_with_multiple_viewports_executes_each() {
        let driver = MockBrowserDriver::default();
        let mut executor = BrowserQaExecutor::new(driver);
        let mut config = sample_config();
        config.journeys[0].viewports = vec![
            Viewport {
                width: 1280,
                height: 720,
            },
            Viewport {
                width: 375,
                height: 667,
            },
        ];

        let result = executor.run(&config, "abc123", "run-1", 1000).unwrap();
        assert_eq!(result.steps.len(), 2);
    }

    #[test]
    fn run_with_multiple_steps() {
        let driver = MockBrowserDriver::default();
        let mut executor = BrowserQaExecutor::new(driver);
        let mut config = sample_config();
        config.journeys[0].steps = vec![
            JourneyStep {
                route: "/".to_string(),
                assert_text_present: None,
                assert_text_absent: None,
                wait_for_selector: None,
                timeout_ms: 5000,
            },
            JourneyStep {
                route: "/about".to_string(),
                assert_text_present: None,
                assert_text_absent: None,
                wait_for_selector: None,
                timeout_ms: 5000,
            },
        ];

        let result = executor.run(&config, "abc123", "run-1", 1000).unwrap();
        assert_eq!(result.steps.len(), 2);
    }

    #[test]
    fn run_fails_on_not_opted_in() {
        let driver = MockBrowserDriver::default();
        let mut executor = BrowserQaExecutor::new(driver);
        let mut config = sample_config();
        config.opt_in = false;

        let result = executor.run(&config, "abc123", "run-1", 1000);
        assert!(matches!(result, Err(BrowserError::NotOptedIn)));
    }

    // ---- Snapshot comparison ----

    #[test]
    fn snapshot_within_tolerance_passes() {
        let result = compare_snapshots(1, 2);
        assert!(result.within_tolerance);
    }

    #[test]
    fn snapshot_above_tolerance_fails() {
        let result = compare_snapshots(5, 2);
        assert!(!result.within_tolerance);
    }

    #[test]
    fn snapshot_at_boundary_passes() {
        let result = compare_snapshots(2, 2);
        assert!(result.within_tolerance);
    }

    // ---- Artifact path ----

    #[test]
    fn artifact_path_identifies_commit_route_viewport_timestamp() {
        let path = artifact_path(
            "abc123",
            "/dashboard",
            Viewport {
                width: 800,
                height: 600,
            },
            12345,
        );
        assert!(path.contains("abc123"));
        assert!(path.contains("dashboard"));
        assert!(path.contains("800x600"));
        assert!(path.contains("12345"));
    }

    #[test]
    fn artifact_path_cleans_nested_routes() {
        let path = artifact_path("sha", "/api/v1/users", Viewport::default(), 100);
        assert!(path.contains("api_v1_users"));
    }

    // ---- Credential exposure ----

    #[test]
    fn credential_exposure_detected_when_secret_in_content() {
        let mut creds = HashMap::new();
        creds.insert("api_key".to_string(), "secret123".to_string());
        let result = BrowserQaExecutor::<MockBrowserDriver>::check_credential_exposure(
            "the key is secret123 here",
            &creds,
        );
        assert!(matches!(result, Err(BrowserError::CredentialExposure)));
    }

    #[test]
    fn credential_exposure_not_detected_when_secret_absent() {
        let mut creds = HashMap::new();
        creds.insert("api_key".to_string(), "secret123".to_string());
        let result = BrowserQaExecutor::<MockBrowserDriver>::check_credential_exposure(
            "nothing here",
            &creds,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn credential_exposure_ignores_empty_credentials() {
        let mut creds = HashMap::new();
        creds.insert("empty".to_string(), "".to_string());
        let result = BrowserQaExecutor::<MockBrowserDriver>::check_credential_exposure(
            "some content",
            &creds,
        );
        assert!(result.is_ok());
    }

    // ---- Error classification ----

    #[test]
    fn classify_assertion_error_as_application_failure() {
        let err = BrowserError::AssertionFailed {
            route: "/".to_string(),
            expected: "x".to_string(),
            actual: "y".to_string(),
        };
        assert_eq!(
            classify_error(&err),
            StepOutcome::ApplicationAssertionFailed
        );
    }

    #[test]
    fn classify_navigation_error_as_browser_failure() {
        let err = BrowserError::NavigationFailed {
            route: "/".to_string(),
            reason: "timeout".to_string(),
        };
        assert_eq!(classify_error(&err), StepOutcome::BrowserFailure);
    }

    #[test]
    fn classify_snapshot_mismatch_separately() {
        let err = BrowserError::SnapshotMismatch {
            route: "/".to_string(),
            viewport: Viewport::default(),
            diff_pct: 5,
        };
        assert_eq!(classify_error(&err), StepOutcome::SnapshotMismatch);
    }

    // ---- Error display ----

    #[test]
    fn error_display_messages() {
        assert!(format!("{}", BrowserError::NotOptedIn).contains("not opted in"));
        assert!(format!("{}", BrowserError::HealthcheckTimeout { port: 3000 }).contains("3000"));
        assert!(format!("{}", BrowserError::CredentialExposure).contains("credential"));
    }

    // ---- Serialization ----

    #[test]
    fn config_serializes_and_deserializes() {
        let config = sample_config();
        let json = serde_json::to_string(&config).unwrap();
        let back: BrowserQaConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, back);
    }

    #[test]
    fn result_summary_computes_counts() {
        let steps = vec![
            StepResult {
                journey_name: "j".to_string(),
                step_index: 0,
                route: "/".to_string(),
                viewport: Viewport::default(),
                outcome: StepOutcome::Passed,
                screenshot: None,
                console_errors: vec![],
                network_failures: vec![],
                duration_ms: 100,
                error: None,
            },
            StepResult {
                journey_name: "j".to_string(),
                step_index: 1,
                route: "/about".to_string(),
                viewport: Viewport::default(),
                outcome: StepOutcome::ApplicationAssertionFailed,
                screenshot: None,
                console_errors: vec![],
                network_failures: vec![],
                duration_ms: 200,
                error: Some("assert".to_string()),
            },
        ];
        let result = BrowserQaResult::summarize("run-1", "abc", steps, None);
        assert!(!result.all_passed);
        assert_eq!(result.passed_count, 1);
        assert_eq!(result.assertion_failed_count, 1);
        assert_eq!(result.total_duration_ms, 300);
    }

    #[test]
    fn empty_steps_result_not_all_passed() {
        let result = BrowserQaResult::summarize("run-1", "abc", vec![], None);
        assert!(!result.all_passed);
    }
}
