use anyhow::{Context, Result, bail};
use gaggle_db::models::RecipeParameter;
use minijinja::Environment;
use std::collections::HashMap;

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
            let val = provided.get(&spec.name).with_context(|| {
                format!("Missing required recipe parameter '{}'", spec.name)
            })?;
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
