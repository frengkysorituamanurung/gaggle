use anyhow::{Context, Result, bail};
use chrono::{DateTime, FixedOffset, Utc};
use croner::Cron;
use gaggle_db::models::RecipeParameter;
use minijinja::Environment;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Validates recipe parameters against user-supplied values:
/// - Required parameters (`required == 1`) must not have a default and must be provided.
/// - Optional parameters (`required == 0`) must have a `default_value` and fall back to it.
pub fn resolve_recipe_parameters(
    specs: &[RecipeParameter],
    provided: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    let mut resolved = HashMap::new();

    for spec in specs {
        if spec.required == 1 {
            if spec.default_value.is_some() {
                bail!(
                    "Recipe parameter '{}' is marked required but has a default_value",
                    spec.name
                );
            }
            let val = provided
                .get(&spec.name)
                .with_context(|| format!("Missing required recipe parameter '{}'", spec.name))?;
            resolved.insert(spec.name.clone(), val.clone());
        } else {
            let default_val = spec.default_value.as_ref().with_context(|| {
                format!(
                    "Optional recipe parameter '{}' must define a default_value",
                    spec.name
                )
            })?;
            let val = provided.get(&spec.name).unwrap_or(default_val);
            resolved.insert(spec.name.clone(), val.clone());
        }
    }

    Ok(resolved)
}

/// Validates only the parameter specifications themselves (without user values).
pub fn validate_recipe_parameters(specs: &[RecipeParameter]) -> Result<()> {
    for spec in specs {
        if spec.required == 1 && spec.default_value.is_some() {
            bail!(
                "Recipe parameter '{}' is marked required but has a default_value",
                spec.name
            );
        }
        if spec.required == 0 && spec.default_value.is_none() {
            bail!(
                "Optional recipe parameter '{}' must define a default_value",
                spec.name
            );
        }
    }
    Ok(())
}

/// Alias used by some callers.
pub fn validate_parameters(specs: &[RecipeParameter]) -> Result<()> {
    validate_recipe_parameters(specs)
}

/// Renders a recipe instructions/prompt template using `{{ parameter_name }}` syntax.
pub fn render_recipe_template(
    template_str: &str,
    values: &HashMap<String, String>,
) -> Result<String> {
    let mut env = Environment::new();
    env.add_template("recipe", template_str)?;
    let tmpl = env.get_template("recipe")?;
    Ok(tmpl.render(values)?)
}

// ============================================================================
// Recipe parser (inline JSON / YAML / files / URLs)
// ============================================================================

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum BoolOrInt {
    Bool(bool),
    Int(i64),
}

fn de_bool_or_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    match BoolOrInt::deserialize(deserializer)? {
        BoolOrInt::Bool(b) => Ok(if b { 1 } else { 0 }),
        BoolOrInt::Int(i) => Ok(i),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecipeParameterDef {
    pub name: String,
    #[serde(default = "default_param_type")]
    pub param_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "de_bool_or_i64")]
    pub required: i64,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub idx: i64,
}

fn default_param_type() -> String {
    "string".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecipeSubRecipeDef {
    pub child_recipe_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub values_json: Option<String>,
    #[serde(default, deserialize_with = "de_bool_or_i64")]
    pub sequential_when_repeated: i64,
    #[serde(default)]
    pub idx: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecipeDefinition {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_ref: Option<String>,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub parameters: Vec<RecipeParameterDef>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub sub_recipes: Vec<RecipeSubRecipeDef>,
    #[serde(default)]
    pub default_model_id: Option<String>,
    #[serde(default)]
    pub definition: Option<String>,
}

impl RecipeDefinition {
    pub fn source_kind_or(&self) -> String {
        self.source_kind
            .clone()
            .unwrap_or_else(|| "inline".to_string())
    }
}

pub fn parse_recipe_json(content: &str) -> Result<RecipeDefinition> {
    let def: RecipeDefinition =
        serde_json::from_str(content).context("Failed to parse recipe JSON")?;
    Ok(def)
}

pub fn parse_recipe_yaml(content: &str) -> Result<RecipeDefinition> {
    let def: RecipeDefinition =
        serde_yaml_ng::from_str(content).context("Failed to parse recipe YAML")?;
    Ok(def)
}

/// Parse a recipe from a string, trying JSON first then YAML.
pub fn parse_recipe_str(content: &str) -> Result<RecipeDefinition> {
    if let Ok(def) = parse_recipe_json(content) {
        return Ok(def);
    }
    parse_recipe_yaml(content)
}

/// Alias.
pub fn parse_recipe(content: &str) -> Result<RecipeDefinition> {
    parse_recipe_str(content)
}

pub fn parse_recipe_from_str(content: &str) -> Result<RecipeDefinition> {
    parse_recipe_str(content)
}

/// Load a recipe definition from a file (JSON or YAML based on extension/content).
pub fn load_recipe_from_file(path: impl AsRef<Path>) -> Result<RecipeDefinition> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read recipe file {}", path.display()))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "json" => parse_recipe_json(&content),
        "yaml" | "yml" => parse_recipe_yaml(&content),
        _ => parse_recipe_str(&content),
    }
}

/// Alias.
pub fn load_recipe_file(path: impl AsRef<Path>) -> Result<RecipeDefinition> {
    load_recipe_from_file(path)
}

/// Load a recipe definition from a URL (JSON or YAML).
pub async fn load_recipe_from_url(url: &str) -> Result<RecipeDefinition> {
    let client = Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch recipe URL {url}"))?;
    let status = resp.status();
    if !status.is_success() {
        bail!("Failed to fetch recipe URL {url}: {status}");
    }
    let content = resp
        .text()
        .await
        .context("Failed to read recipe response body")?;
    // Try extension from URL path.
    let lower = url.to_lowercase();
    if lower.ends_with(".json") {
        return parse_recipe_json(&content);
    }
    if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        return parse_recipe_yaml(&content);
    }
    parse_recipe_str(&content)
}

/// Alias.
pub async fn load_recipe_url(url: &str) -> Result<RecipeDefinition> {
    load_recipe_from_url(url).await
}

/// Validate a parsed recipe definition's parameters.
pub fn validate_recipe_definition(def: &RecipeDefinition) -> Result<()> {
    for p in &def.parameters {
        if p.required == 1 && p.default_value.is_some() {
            bail!(
                "Recipe parameter '{}' is marked required but has a default_value",
                p.name
            );
        }
        if p.required == 0 && p.default_value.is_none() {
            bail!(
                "Optional recipe parameter '{}' must define a default_value",
                p.name
            );
        }
    }
    Ok(())
}

// Convert parsed params to DB-friendly (RecipeParameter) is done by callers.
// ============================================================================
// Cron scheduling helpers
// ============================================================================

pub fn parse_cron_expr(expr: &str) -> Result<Cron> {
    Cron::from_str(expr).map_err(|e| anyhow::anyhow!("Invalid cron expression '{expr}': {e}"))
}

/// Compute the next run time (epoch ms) for a cron expression in a timezone,
/// starting after `from_ms` (exclusive).
pub fn compute_next_run(cron_expr: &str, timezone: &str, from_ms: i64) -> Result<i64> {
    let cron = parse_cron_expr(cron_expr)?;
    // UTC
    if timezone.eq_ignore_ascii_case("utc")
        || timezone.eq_ignore_ascii_case("z")
        || timezone.eq_ignore_ascii_case("etc/utc")
        || timezone.eq_ignore_ascii_case("gmt")
    {
        let dt = DateTime::from_timestamp_millis(from_ms)
            .context("Invalid from_ms")?
            .with_timezone(&Utc);
        let next = cron
            .find_next_occurrence(&dt, false)
            .map_err(|e| anyhow::anyhow!("Failed to compute next run: {e}"))?;
        return Ok(next.timestamp_millis());
    }

    // IANA timezone
    if let Ok(tz) = timezone.parse::<chrono_tz::Tz>() {
        let dt = DateTime::from_timestamp_millis(from_ms)
            .context("Invalid from_ms")?
            .with_timezone(&tz);
        let next = cron
            .find_next_occurrence(&dt, false)
            .map_err(|e| anyhow::anyhow!("Failed to compute next run: {e}"))?;
        return Ok(next.timestamp_millis());
    }

    // Fixed offset like +02:00
    if let Ok(off) = timezone.parse::<FixedOffset>() {
        let dt = DateTime::from_timestamp_millis(from_ms)
            .context("Invalid from_ms")?
            .with_timezone(&off);
        let next = cron
            .find_next_occurrence(&dt, false)
            .map_err(|e| anyhow::anyhow!("Failed to compute next run: {e}"))?;
        return Ok(next.timestamp_millis());
    }

    // Fallback: treat as UTC.
    let dt = DateTime::from_timestamp_millis(from_ms)
        .context("Invalid from_ms")?
        .with_timezone(&Utc);
    let next = cron
        .find_next_occurrence(&dt, false)
        .map_err(|e| anyhow::anyhow!("Failed to compute next run: {e}"))?;
    Ok(next.timestamp_millis())
}

// Aliases for compatibility with different caller expectations.
pub fn next_run_at(cron_expr: &str, timezone: &str, from_ms: i64) -> Result<i64> {
    compute_next_run(cron_expr, timezone, from_ms)
}

pub fn compute_next_run_at(cron_expr: &str, timezone: &str, from_ms: i64) -> Result<i64> {
    compute_next_run(cron_expr, timezone, from_ms)
}

pub fn next_run_from_cron(cron_expr: &str, timezone: &str, from_ms: i64) -> Result<i64> {
    compute_next_run(cron_expr, timezone, from_ms)
}

pub fn cron_next_run(cron_expr: &str, timezone: &str, from_ms: i64) -> Result<i64> {
    compute_next_run(cron_expr, timezone, from_ms)
}

/// Compute the next future run from now (missed-run handling).
pub fn compute_next_future_run(cron_expr: &str, timezone: &str, now_ms: i64) -> Result<i64> {
    compute_next_run(cron_expr, timezone, now_ms)
}

pub fn handle_missed_run(cron_expr: &str, timezone: &str, now_ms: i64) -> Result<i64> {
    compute_next_run(cron_expr, timezone, now_ms)
}

/// Render helper for both instructions and prompt.
pub fn render_recipe(
    instructions: Option<&str>,
    prompt: Option<&str>,
    values: &HashMap<String, String>,
) -> Result<(Option<String>, Option<String>)> {
    let instr = instructions
        .map(|t| render_recipe_template(t, values))
        .transpose()?;
    let prmpt = prompt
        .map(|t| render_recipe_template(t, values))
        .transpose()?;
    Ok((instr, prmpt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_required_optional() {
        let specs = vec![
            RecipeParameter {
                id: "1".into(),
                recipe_id: "r".into(),
                name: "name".into(),
                param_type: "string".into(),
                description: None,
                required: 1,
                default_value: None,
                idx: 0,
            },
            RecipeParameter {
                id: "2".into(),
                recipe_id: "r".into(),
                name: "count".into(),
                param_type: "number".into(),
                description: None,
                required: 0,
                default_value: Some("5".into()),
                idx: 1,
            },
        ];
        let mut provided = HashMap::new();
        provided.insert("name".into(), "foo".into());
        let resolved = resolve_recipe_parameters(&specs, &provided).unwrap();
        assert_eq!(resolved.get("name").unwrap(), "foo");
        assert_eq!(resolved.get("count").unwrap(), "5");
    }

    #[test]
    fn render_template() {
        let mut v = HashMap::new();
        v.insert("name".into(), "bar".into());
        let out = render_recipe_template("Hello {{ name }}", &v).unwrap();
        assert_eq!(out, "Hello bar");
    }

    #[test]
    fn cron_next() {
        let from = 1700000000000i64; // arbitrary
        let next = compute_next_run("0 * * * *", "UTC", from).unwrap();
        assert!(next > from);
    }
}
