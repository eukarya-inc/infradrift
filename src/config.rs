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

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
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
}
