use anyhow::{bail, Context};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub ignore: Vec<IgnoreRule>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IgnoreRule {
    /// Resource types to match (e.g., ["google_cloud_run_v2_service"])
    #[serde(default)]
    pub resource_types: Vec<String>,

    /// Actions to match (e.g., ["update"]). Empty means match all actions.
    #[serde(default)]
    pub actions: Vec<String>,

    /// Attribute patterns to match (supports glob with *)
    #[serde(default)]
    pub attributes: Vec<String>,

    /// "all" = ignore resource only if ALL changes are in listed attributes
    /// "any" = ignore individual matching attribute changes but keep the resource
    #[serde(default = "default_mode")]
    pub mode: IgnoreMode,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IgnoreMode {
    All,
    Any,
}

fn default_mode() -> IgnoreMode {
    IgnoreMode::All
}

const VALID_ACTIONS: &[&str] = &["update", "delete", "create", "replace"];

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load and validate a config file, returning all validation errors found.
    pub fn load_and_validate(path: &Path) -> anyhow::Result<Vec<String>> {
        if !path.exists() {
            bail!("config file not found: {}", path.display());
        }

        let content = std::fs::read_to_string(path).context("failed to read config file")?;

        let config: Config = toml::from_str(&content).context("failed to parse TOML")?;

        Ok(config.validate())
    }

    /// Validate the config and return a list of warnings/errors.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.ignore.is_empty() {
            errors.push("no ignore rules defined (file is valid but has no effect)".to_string());
            return errors;
        }

        for (i, rule) in self.ignore.iter().enumerate() {
            let idx = i + 1;

            // Check for empty attributes with non-default mode
            if rule.attributes.is_empty() && rule.mode == IgnoreMode::Any {
                errors.push(format!(
                    "ignore rule #{}: mode is \"any\" but no attributes are defined (mode has no effect without attributes)",
                    idx
                ));
            }

            // Validate action values
            for action in &rule.actions {
                let lower = action.to_lowercase();
                if !VALID_ACTIONS.contains(&lower.as_str()) {
                    errors.push(format!(
                        "ignore rule #{}: unknown action \"{}\" (valid: {})",
                        idx,
                        action,
                        VALID_ACTIONS.join(", ")
                    ));
                }
            }

            // Validate attribute patterns
            for attr in &rule.attributes {
                if attr.is_empty() {
                    errors.push(format!("ignore rule #{}: empty attribute pattern", idx));
                } else if attr.starts_with('.') || attr.ends_with('.') {
                    errors.push(format!(
                        "ignore rule #{}: attribute pattern \"{}\" should not start or end with \".\"",
                        idx, attr
                    ));
                } else if attr.contains("..") {
                    errors.push(format!(
                        "ignore rule #{}: attribute pattern \"{}\" has empty segment (\"..\")",
                        idx, attr
                    ));
                }
            }

            // Warn if rule has no attributes and mode is "all" (catches everything)
            if rule.attributes.is_empty()
                && rule.resource_types.is_empty()
                && rule.actions.is_empty()
            {
                errors.push(format!(
                    "ignore rule #{}: no resource_types, actions, or attributes specified (matches all drift — this will hide everything)",
                    idx
                ));
            }
        }

        errors
    }
}

/// Check if an attribute path matches a glob pattern.
/// Supports `*` as a wildcard for a single path segment.
pub fn attribute_matches_pattern(attr_path: &str, pattern: &str) -> bool {
    let attr_parts: Vec<&str> = attr_path.split('.').collect();
    let pattern_parts: Vec<&str> = pattern.split('.').collect();

    if attr_parts.len() < pattern_parts.len() {
        // Allow prefix matching: pattern "tags" should match "tags.Name"
        if pattern_parts.len() == 1 {
            return attr_parts.first() == pattern_parts.first();
        }
        return false;
    }

    // Match pattern parts against attribute parts
    for (i, pat) in pattern_parts.iter().enumerate() {
        if *pat == "*" {
            continue;
        }
        if i >= attr_parts.len() {
            return false;
        }
        if *pat != attr_parts[i] {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_matches_exact() {
        assert!(attribute_matches_pattern("tags.Name", "tags.Name"));
        assert!(!attribute_matches_pattern("tags.Name", "tags.Env"));
    }

    #[test]
    fn test_attribute_matches_prefix() {
        assert!(attribute_matches_pattern("tags.Name", "tags"));
        assert!(attribute_matches_pattern("tags.Environment", "tags"));
    }

    #[test]
    fn test_attribute_matches_wildcard() {
        assert!(attribute_matches_pattern(
            "template.0.containers.0.image",
            "template.*.containers.*.image"
        ));
        assert!(!attribute_matches_pattern(
            "template.0.containers.0.name",
            "template.*.containers.*.image"
        ));
    }

    #[test]
    fn test_attribute_matches_simple_wildcard() {
        assert!(attribute_matches_pattern(
            "traffic.0.revision",
            "traffic.*.revision"
        ));
        assert!(attribute_matches_pattern(
            "traffic.1.type",
            "traffic.*.type"
        ));
    }

    #[test]
    fn test_load_missing_config() {
        let config = Config::load(Path::new("nonexistent.toml")).unwrap();
        assert!(config.ignore.is_empty());
    }

    #[test]
    fn test_validate_empty_ignore() {
        let config = Config { ignore: vec![] };
        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("no ignore rules defined"));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = Config {
            ignore: vec![IgnoreRule {
                resource_types: vec!["aws_instance".to_string()],
                actions: vec!["update".to_string()],
                attributes: vec!["tags".to_string()],
                mode: IgnoreMode::All,
            }],
        };
        let warnings = config.validate();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_unknown_action() {
        let config = Config {
            ignore: vec![IgnoreRule {
                resource_types: vec![],
                actions: vec!["destroy".to_string()],
                attributes: vec!["tags".to_string()],
                mode: IgnoreMode::All,
            }],
        };
        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unknown action \"destroy\""));
    }

    #[test]
    fn test_validate_any_mode_without_attributes() {
        let config = Config {
            ignore: vec![IgnoreRule {
                resource_types: vec!["aws_instance".to_string()],
                actions: vec!["update".to_string()],
                attributes: vec![],
                mode: IgnoreMode::Any,
            }],
        };
        let warnings = config.validate();
        assert!(warnings
            .iter()
            .any(|w| w.contains("mode is \"any\" but no attributes")));
    }

    #[test]
    fn test_validate_bad_attribute_patterns() {
        let config = Config {
            ignore: vec![IgnoreRule {
                resource_types: vec!["aws_instance".to_string()],
                actions: vec![],
                attributes: vec![
                    ".tags".to_string(),
                    "tags.".to_string(),
                    "tags..name".to_string(),
                    "".to_string(),
                ],
                mode: IgnoreMode::All,
            }],
        };
        let warnings = config.validate();
        assert_eq!(warnings.len(), 4);
    }

    #[test]
    fn test_validate_catch_all_rule() {
        let config = Config {
            ignore: vec![IgnoreRule {
                resource_types: vec![],
                actions: vec![],
                attributes: vec![],
                mode: IgnoreMode::All,
            }],
        };
        let warnings = config.validate();
        assert!(warnings.iter().any(|w| w.contains("matches all drift")));
    }
}
