//! Prompt template rendering, secret resolution, and the per-attempt
//! effective-config snapshot (plan §6).
//!
//! A v2 prompt body may reference task/attempt/workspace context and secrets via
//! `{{namespace.var}}` placeholders. Resolution is **strict**: unknown variables
//! and unresolved secrets are errors, never silently left verbatim. Secret
//! *values* are resolved at run time only and are never persisted — the
//! [`EffectiveConfig`] snapshot records the config + the unrendered template, so
//! references appear as names, not values.

use serde::Serialize;

use super::workflow_v2::WorkflowConfigV2;

/// Variables available when rendering a v2 prompt template.
#[derive(Clone, Debug)]
pub struct PromptContext<'a> {
    pub task_id: &'a str,
    pub task_title: &'a str,
    pub task_source: &'a str,
    pub attempt_id: &'a str,
    pub attempt_no: u32,
    pub workspace_key: &'a str,
}

/// Resolves `{{secrets.NAME}}` references at run time. Implementations read from
/// the environment, a vault, or a test map. Resolved values are used to launch
/// the run but are **never** written into the effective-config snapshot.
pub trait SecretResolver {
    fn resolve(&self, name: &str) -> Option<String>;
}

/// A [`SecretResolver`] backed by process environment variables.
pub struct EnvSecretResolver;

impl SecretResolver for EnvSecretResolver {
    fn resolve(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

/// A [`SecretResolver`] backed by an in-memory map (tests / programmatic use).
#[derive(Default)]
pub struct MapSecretResolver(std::collections::HashMap<String, String>);

impl MapSecretResolver {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.insert(name.into(), value.into());
        self
    }
}

impl SecretResolver for MapSecretResolver {
    fn resolve(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

/// Validate a prompt template without a concrete context: every `{{expr}}` must
/// be a known context variable or a syntactically-valid `{{secrets.NAME}}`
/// reference. Returns the list of secret names referenced (for the snapshot).
/// Surfaces typos at config-validation time rather than at attempt launch.
pub fn validate_prompt_template(template: &str) -> Result<Vec<String>, String> {
    let mut secrets = Vec::new();
    for expr in iter_template_exprs(template)? {
        if let Some(name) = expr.strip_prefix("secrets.") {
            if name.is_empty() || !is_valid_secret_name(name) {
                return Err(format!(
                    "Invalid secret reference `{{{{secrets.{name}}}}}`: names must be non-empty alphanumeric/underscore."
                ));
            }
            if !secrets.contains(&name.to_string()) {
                secrets.push(name.to_string());
            }
        } else if !is_known_variable(&expr) {
            return Err(format!(
                "Unknown prompt template variable `{{{{{expr}}}}}`. Known: task.id, task.title, task.source, attempt.id, attempt.no, workspace.key, secrets.*."
            ));
        }
    }
    Ok(secrets)
}

/// Render a prompt template against a concrete [`PromptContext`], resolving
/// secrets via `secrets`. Strict: unknown variables and unresolved secrets
/// error. The returned string contains secret *values* — do not persist it.
pub fn render_prompt(
    template: &str,
    ctx: &PromptContext,
    secrets: &dyn SecretResolver,
) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| "Prompt template has an unclosed `{{` — expected `}}`.".to_string())?;
        let expr = after[..end].trim();
        out.push_str(&resolve_expr(expr, ctx, secrets)?);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn resolve_expr(
    expr: &str,
    ctx: &PromptContext,
    secrets: &dyn SecretResolver,
) -> Result<String, String> {
    if let Some(name) = expr.strip_prefix("secrets.") {
        return secrets
            .resolve(name)
            .ok_or_else(|| format!("Unresolved secret reference `{{{{secrets.{name}}}}}`."));
    }
    match expr {
        "task.id" => Ok(ctx.task_id.to_string()),
        "task.title" => Ok(ctx.task_title.to_string()),
        "task.source" => Ok(ctx.task_source.to_string()),
        "attempt.id" => Ok(ctx.attempt_id.to_string()),
        "attempt.no" => Ok(ctx.attempt_no.to_string()),
        "workspace.key" => Ok(ctx.workspace_key.to_string()),
        other => Err(format!(
            "Unknown prompt template variable `{{{{{other}}}}}`."
        )),
    }
}

/// Yield every `{{ ... }}` expression (trimmed) in the template, erroring on an
/// unclosed `{{`.
fn iter_template_exprs(template: &str) -> Result<Vec<String>, String> {
    let mut exprs = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| "Prompt template has an unclosed `{{` — expected `}}`.".to_string())?;
        exprs.push(after[..end].trim().to_string());
        rest = &after[end + 2..];
    }
    Ok(exprs)
}

fn is_known_variable(expr: &str) -> bool {
    matches!(
        expr,
        "task.id" | "task.title" | "task.source" | "attempt.id" | "attempt.no" | "workspace.key"
    )
}

fn is_valid_secret_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The frozen, per-attempt view of the resolved v2 config (§6: "immutable
/// effective-config snapshot on every attempt"). Captured at attempt start for
/// audit and reproducibility.
///
/// Secrets appear **only as references**: the snapshot stores the unrendered
/// `prompt_template` (with `{{secrets.NAME}}` intact) and the list of secret
/// names it references. Secret *values* are never serialized.
#[derive(Clone, Debug, Serialize)]
pub struct EffectiveConfig {
    pub config: WorkflowConfigV2,
    /// The unrendered prompt template (secret references intact, not values).
    pub prompt_template: String,
    /// Secret names referenced by the template, in first-occurrence order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secret_refs: Vec<String>,
}

impl EffectiveConfig {
    /// Capture an immutable snapshot from a parsed v2 config + prompt body.
    /// Validates the template (strict) and records the secret references.
    pub fn capture(config: WorkflowConfigV2, prompt_body: &str) -> Result<Self, String> {
        let secret_refs = validate_prompt_template(prompt_body)?;
        Ok(Self {
            config,
            prompt_template: prompt_body.to_string(),
            secret_refs,
        })
    }

    /// Render the prompt for this attempt, resolving secrets via `secrets`.
    /// The returned string contains secret values — use it to launch the run,
    /// never persist it.
    pub fn render(
        &self,
        ctx: &PromptContext,
        secrets: &dyn SecretResolver,
    ) -> Result<String, String> {
        render_prompt(&self.prompt_template, ctx, secrets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>() -> PromptContext<'a> {
        PromptContext {
            task_id: "t-1",
            task_title: "Fix login",
            task_source: "local",
            attempt_id: "t-1-att-1",
            attempt_no: 1,
            workspace_key: "ws-1",
        }
    }

    #[test]
    fn renders_known_variables() {
        let template = "Task {{task.id}} ({{task.title}}) from {{task.source}} — attempt {{attempt.no}} of {{attempt.id}} in {{workspace.key}}.";
        let rendered = render_prompt(template, &ctx(), &MapSecretResolver::new()).unwrap();
        assert_eq!(
            rendered,
            "Task t-1 (Fix login) from local — attempt 1 of t-1-att-1 in ws-1."
        );
    }

    #[test]
    fn renders_secrets_from_resolver() {
        let template = "Key: {{secrets.API_KEY}}";
        let secrets = MapSecretResolver::new().set("API_KEY", "sk-123");
        let rendered = render_prompt(template, &ctx(), &secrets).unwrap();
        assert_eq!(rendered, "Key: sk-123");
    }

    #[test]
    fn unknown_variable_is_an_error() {
        let template = "Hello {{user.name}}";
        assert!(render_prompt(template, &ctx(), &MapSecretResolver::new()).is_err());
    }

    #[test]
    fn unresolved_secret_is_an_error() {
        let template = "{{secrets.MISSING}}";
        assert!(render_prompt(template, &ctx(), &MapSecretResolver::new()).is_err());
    }

    #[test]
    fn unclosed_brace_is_an_error() {
        assert!(render_prompt("Hello {{task.id", &ctx(), &MapSecretResolver::new()).is_err());
    }

    #[test]
    fn no_placeholders_passes_through() {
        let rendered = render_prompt("plain text", &ctx(), &MapSecretResolver::new()).unwrap();
        assert_eq!(rendered, "plain text");
    }

    #[test]
    fn validate_template_collects_secret_refs() {
        let template = "{{task.id}} needs {{secrets.API_KEY}} and {{secrets.DB_URL}} and again {{secrets.API_KEY}}";
        let refs = validate_prompt_template(template).unwrap();
        // Deduplicated, first-occurrence order.
        assert_eq!(refs, vec!["API_KEY", "DB_URL"]);
    }

    #[test]
    fn validate_template_rejects_unknown_var() {
        assert!(validate_prompt_template("{{bogus.var}}").is_err());
    }

    #[test]
    fn validate_template_rejects_bad_secret_name() {
        assert!(validate_prompt_template("{{secrets.bad-name}}").is_err());
        assert!(validate_prompt_template("{{secrets.}}").is_err());
    }

    #[test]
    fn effective_config_captures_and_renders() {
        // Build a minimal valid v2 config for the snapshot.
        let yaml =
            "version: 2\norchestration:\n  active_states: [todo]\n  terminal_states: [done]\n";
        let config = super::super::workflow_v2::parse(yaml).unwrap();
        let template = "Do {{task.title}} with key {{secrets.API_KEY}}.";

        let snapshot = EffectiveConfig::capture(config, template).unwrap();
        assert_eq!(snapshot.secret_refs, vec!["API_KEY".to_string()]);
        // The stored template is unrendered (secret reference intact).
        assert!(snapshot.prompt_template.contains("{{secrets.API_KEY}}"));

        let secrets = MapSecretResolver::new().set("API_KEY", "sk-xyz");
        let rendered = snapshot.render(&ctx(), &secrets).unwrap();
        assert_eq!(rendered, "Do Fix login with key sk-xyz.");
    }

    #[test]
    fn effective_config_rejects_bad_template() {
        let yaml =
            "version: 2\norchestration:\n  active_states: [todo]\n  terminal_states: [done]\n";
        let config = super::super::workflow_v2::parse(yaml).unwrap();
        assert!(EffectiveConfig::capture(config, "{{unknown.var}}").is_err());
    }

    #[test]
    fn effective_config_serializes_without_secret_values() {
        let yaml =
            "version: 2\norchestration:\n  active_states: [todo]\n  terminal_states: [done]\n";
        let config = super::super::workflow_v2::parse(yaml).unwrap();
        let snapshot = EffectiveConfig::capture(config, "{{secrets.API_KEY}}").unwrap();
        let json = serde_json::to_string(&snapshot).unwrap();
        // The reference name is present, but no value could leak (none was ever stored).
        assert!(json.contains("API_KEY"));
        assert!(!json.contains("sk-"));
    }
}
