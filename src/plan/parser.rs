use crate::plan::schema::TerraformPlan;
use anyhow::{bail, Context};
use std::path::Path;
use std::process::Command;

/// Parse a plan file. Auto-detects JSON vs binary planfile format.
/// For binary planfiles, runs `terraform show -json` to convert.
pub fn parse_plan_file(
    path: &Path,
    force_binary: bool,
    use_tofu: bool,
) -> anyhow::Result<TerraformPlan> {
    if force_binary {
        return parse_binary_planfile(path, use_tofu);
    }

    // Auto-detect: try reading first bytes to check if JSON
    let content = std::fs::read(path)
        .with_context(|| format!("Failed to read plan file: {}", path.display()))?;

    // Check if it looks like JSON (starts with '{' after trimming whitespace)
    let trimmed = content.iter().position(|&b| !b.is_ascii_whitespace());
    match trimmed {
        Some(pos) if content[pos] == b'{' => {
            // It's JSON, parse directly
            let json_str = std::str::from_utf8(&content).context("Plan file is not valid UTF-8")?;
            parse_plan_json(json_str)
        }
        _ => {
            // Binary planfile, convert via terraform show
            parse_binary_planfile(path, use_tofu)
        }
    }
}

/// Parse a JSON string as a Terraform plan.
pub fn parse_plan_json(json: &str) -> anyhow::Result<TerraformPlan> {
    serde_json::from_str(json).context("Failed to parse Terraform plan JSON")
}

/// Convert a binary planfile to JSON using `terraform show -json`.
fn parse_binary_planfile(path: &Path, use_tofu: bool) -> anyhow::Result<TerraformPlan> {
    let binary = if use_tofu { "tofu" } else { "terraform" };

    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    let output = Command::new(binary)
        .args(["show", "-json"])
        .arg(&abs_path)
        .output()
        .with_context(|| format!("{} binary not found. Is it installed and on PATH?", binary))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "{} show -json failed (exit code {}): {}",
            binary,
            output.status.code().unwrap_or(-1),
            stderr.trim()
        );
    }

    let json =
        String::from_utf8(output.stdout).context("terraform show output is not valid UTF-8")?;
    parse_plan_json(&json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_plan_json() {
        let json = r#"{
            "format_version": "1.2",
            "terraform_version": "1.5.0",
            "resource_changes": [],
            "resource_drift": []
        }"#;

        let plan = parse_plan_json(json).unwrap();
        assert_eq!(plan.terraform_version.unwrap(), "1.5.0");
        assert!(plan.resource_changes.is_empty());
        assert!(plan.resource_drift.is_empty());
    }

    #[test]
    fn test_parse_plan_without_resource_drift() {
        // Older Terraform versions don't have resource_drift
        let json = r#"{
            "format_version": "1.0",
            "terraform_version": "1.2.0",
            "resource_changes": [
                {
                    "address": "aws_instance.web",
                    "type": "aws_instance",
                    "name": "web",
                    "provider_name": "registry.terraform.io/hashicorp/aws",
                    "change": {
                        "actions": ["update"],
                        "before": {"ami": "ami-old"},
                        "after": {"ami": "ami-new"}
                    }
                }
            ]
        }"#;

        let plan = parse_plan_json(json).unwrap();
        assert!(plan.resource_drift.is_empty());
        assert_eq!(plan.resource_changes.len(), 1);
    }

    #[test]
    fn test_parse_empty_json_object() {
        let json = "{}";
        let plan = parse_plan_json(json).unwrap();
        assert!(plan.resource_changes.is_empty());
        assert!(plan.resource_drift.is_empty());
    }

    #[test]
    fn test_parse_malformed_json() {
        let json = "not json at all";
        assert!(parse_plan_json(json).is_err());
    }
}
