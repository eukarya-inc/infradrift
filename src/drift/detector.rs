use crate::config::{attribute_matches_pattern, Config, IgnoreMode, IgnoreRule};
use crate::drift::types::*;
use crate::plan::schema::{Change, ResourceChange, TerraformPlan};

/// Detect drift from a parsed Terraform plan.
pub fn detect_drift(plan: &TerraformPlan, config: &Config) -> DriftReport {
    let mut drifted_resources = Vec::new();

    // Phase 1: Use resource_drift key if available (Terraform 1.4+)
    if !plan.resource_drift.is_empty() {
        for rc in &plan.resource_drift {
            if let Some(drifted) = process_resource_change(rc, DriftSource::ResourceDriftKey) {
                drifted_resources.push(drifted);
            }
        }
    }

    // Phase 2: Fallback to resource_changes if resource_drift was empty
    if drifted_resources.is_empty() {
        for rc in &plan.resource_changes {
            // Skip no-op changes
            if rc.change.actions == ["no-op"] {
                continue;
            }
            // Skip pure creates (new resources in config, not drift)
            if rc.change.actions == ["create"] {
                continue;
            }
            if let Some(drifted) = process_resource_change(rc, DriftSource::InferredFromChanges) {
                drifted_resources.push(drifted);
            }
        }
    }

    // Apply ignore rules from config
    apply_ignore_rules(&mut drifted_resources, &config.ignore);

    let mut report = DriftReport {
        terraform_version: plan.terraform_version.clone(),
        drifted_resources,
        summary: DriftSummary {
            total_drifted: 0,
            updates: 0,
            deletes: 0,
            creates: 0,
            replaces: 0,
        },
    };
    report.recalculate_summary();
    report
}

fn process_resource_change(rc: &ResourceChange, source: DriftSource) -> Option<DriftedResource> {
    let action = classify_action(&rc.change.actions)?;
    let attribute_changes = compute_attribute_changes(&rc.change);

    Some(DriftedResource {
        address: rc.address.clone(),
        resource_type: rc.resource_type.clone(),
        name: rc.name.clone(),
        module_address: rc.module_address.clone(),
        provider: rc.provider_name.clone(),
        action,
        attribute_changes,
        source,
    })
}

fn classify_action(actions: &[String]) -> Option<DriftAction> {
    match actions {
        [a] if a == "update" => Some(DriftAction::Update),
        [a] if a == "delete" => Some(DriftAction::Delete),
        [a] if a == "create" => Some(DriftAction::Create),
        [a, b] if (a == "create" && b == "delete") || (a == "delete" && b == "create") => {
            Some(DriftAction::Replace)
        }
        [a] if a == "read" => None, // data sources, not drift
        [a] if a == "no-op" => None,
        _ => {
            // Unknown action combination, treat as update
            if !actions.is_empty() {
                Some(DriftAction::Update)
            } else {
                None
            }
        }
    }
}

/// Compute attribute-level changes by diffing before/after JSON values.
fn compute_attribute_changes(change: &Change) -> Vec<AttributeChange> {
    let mut changes = Vec::new();

    let before = change.before.as_ref();
    let after = change.after.as_ref();

    match (before, after) {
        (Some(b), Some(a)) => {
            let mut before_flat = std::collections::BTreeMap::new();
            let mut after_flat = std::collections::BTreeMap::new();
            flatten_json(b, "", &mut before_flat);
            flatten_json(a, "", &mut after_flat);

            // Find all keys in either map
            let mut all_keys: Vec<String> = before_flat.keys().cloned().collect();
            for k in after_flat.keys() {
                if !before_flat.contains_key(k) {
                    all_keys.push(k.clone());
                }
            }
            all_keys.sort();

            for key in all_keys {
                let bv = before_flat.get(&key);
                let av = after_flat.get(&key);
                if bv != av {
                    let sensitive = is_sensitive_path(&key, &change.before_sensitive)
                        || is_sensitive_path(&key, &change.after_sensitive);

                    changes.push(AttributeChange {
                        path: key,
                        before: if sensitive {
                            Some("(sensitive)".to_string())
                        } else {
                            bv.cloned()
                        },
                        after: if sensitive {
                            Some("(sensitive)".to_string())
                        } else {
                            av.cloned()
                        },
                        sensitive,
                    });
                }
            }
        }
        (Some(b), None) => {
            // Resource deleted
            let mut before_flat = std::collections::BTreeMap::new();
            flatten_json(b, "", &mut before_flat);
            for (key, val) in before_flat {
                changes.push(AttributeChange {
                    path: key,
                    before: Some(val),
                    after: None,
                    sensitive: false,
                });
            }
        }
        (None, Some(a)) => {
            // Resource created
            let mut after_flat = std::collections::BTreeMap::new();
            flatten_json(a, "", &mut after_flat);
            for (key, val) in after_flat {
                changes.push(AttributeChange {
                    path: key,
                    before: None,
                    after: Some(val),
                    sensitive: false,
                });
            }
        }
        (None, None) => {}
    }

    changes
}

/// Flatten a JSON value into dot-separated key paths.
fn flatten_json(
    value: &serde_json::Value,
    prefix: &str,
    out: &mut std::collections::BTreeMap<String, String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_json(v, &key, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let key = if prefix.is_empty() {
                    i.to_string()
                } else {
                    format!("{}.{}", prefix, i)
                };
                flatten_json(v, &key, out);
            }
        }
        serde_json::Value::Null => {
            if !prefix.is_empty() {
                out.insert(prefix.to_string(), "null".to_string());
            }
        }
        other => {
            if !prefix.is_empty() {
                let s = match other {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => other.to_string(),
                };
                out.insert(prefix.to_string(), s);
            }
        }
    }
}

/// Check if a flattened path is marked as sensitive.
fn is_sensitive_path(path: &str, sensitive: &serde_json::Value) -> bool {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = sensitive;
    for part in &parts {
        match current {
            serde_json::Value::Object(map) => {
                if let Some(v) = map.get(*part) {
                    current = v;
                } else {
                    return false;
                }
            }
            serde_json::Value::Array(arr) => {
                if let Ok(idx) = part.parse::<usize>() {
                    if let Some(v) = arr.get(idx) {
                        current = v;
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            serde_json::Value::Bool(b) => return *b,
            _ => return false,
        }
    }
    matches!(current, serde_json::Value::Bool(true))
}

/// Apply ignore rules to the drifted resources list.
fn apply_ignore_rules(resources: &mut Vec<DriftedResource>, rules: &[IgnoreRule]) {
    resources.retain(|resource| {
        for rule in rules {
            if should_ignore_resource(resource, rule) {
                return false;
            }
        }
        true
    });

    // For "any" mode rules, also prune individual attribute changes
    for resource in resources.iter_mut() {
        for rule in rules {
            if rule.mode == IgnoreMode::Any && rule_matches_resource_type(resource, rule) {
                resource.attribute_changes.retain(|attr| {
                    !rule
                        .attributes
                        .iter()
                        .any(|pat| attribute_matches_pattern(&attr.path, pat))
                });
            }
        }
    }

    // Remove resources with no remaining attribute changes after "any" mode pruning
    resources.retain(|r| !r.attribute_changes.is_empty() || r.action == DriftAction::Delete);
}

fn should_ignore_resource(resource: &DriftedResource, rule: &IgnoreRule) -> bool {
    // Check resource type filter
    if !rule.resource_types.is_empty() && !rule.resource_types.contains(&resource.resource_type) {
        return false;
    }

    // Check action filter
    if !rule.actions.is_empty() {
        let action_str = resource.action.to_string();
        if !rule.actions.contains(&action_str) {
            return false;
        }
    }

    // For "all" mode: ignore only if ALL attribute changes match the ignore patterns
    if rule.mode == IgnoreMode::All && !rule.attributes.is_empty() {
        if resource.attribute_changes.is_empty() {
            return false;
        }
        return resource.attribute_changes.iter().all(|attr| {
            rule.attributes
                .iter()
                .any(|pat| attribute_matches_pattern(&attr.path, pat))
        });
    }

    // If no attribute filter specified and type+action matched, ignore the whole resource
    if rule.attributes.is_empty() {
        return true;
    }

    false
}

fn rule_matches_resource_type(resource: &DriftedResource, rule: &IgnoreRule) -> bool {
    if rule.resource_types.is_empty() {
        return true;
    }
    rule.resource_types.contains(&resource.resource_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plan_json(
        resource_changes: serde_json::Value,
        resource_drift: serde_json::Value,
    ) -> String {
        serde_json::json!({
            "format_version": "1.2",
            "terraform_version": "1.5.0",
            "resource_changes": resource_changes,
            "resource_drift": resource_drift
        })
        .to_string()
    }

    #[test]
    fn test_detect_drift_from_resource_drift_key() {
        let json = make_plan_json(
            serde_json::json!([]),
            serde_json::json!([{
                "address": "aws_instance.web",
                "type": "aws_instance",
                "name": "web",
                "provider_name": "registry.terraform.io/hashicorp/aws",
                "change": {
                    "actions": ["update"],
                    "before": {"tags": {"Name": "old"}},
                    "after": {"tags": {"Name": "new"}}
                }
            }]),
        );

        let plan: TerraformPlan = serde_json::from_str(&json).unwrap();
        let config = Config::default();
        let report = detect_drift(&plan, &config);

        assert_eq!(report.drifted_resources.len(), 1);
        assert_eq!(report.drifted_resources[0].address, "aws_instance.web");
        assert_eq!(
            report.drifted_resources[0].source,
            DriftSource::ResourceDriftKey
        );
    }

    #[test]
    fn test_detect_drift_fallback_to_resource_changes() {
        let json = make_plan_json(
            serde_json::json!([{
                "address": "aws_s3_bucket.data",
                "type": "aws_s3_bucket",
                "name": "data",
                "provider_name": "registry.terraform.io/hashicorp/aws",
                "change": {
                    "actions": ["update"],
                    "before": {"acl": "private"},
                    "after": {"acl": "public-read"}
                }
            }]),
            serde_json::json!([]),
        );

        let plan: TerraformPlan = serde_json::from_str(&json).unwrap();
        let config = Config::default();
        let report = detect_drift(&plan, &config);

        assert_eq!(report.drifted_resources.len(), 1);
        assert_eq!(
            report.drifted_resources[0].source,
            DriftSource::InferredFromChanges
        );
    }

    #[test]
    fn test_no_drift() {
        let json = make_plan_json(
            serde_json::json!([{
                "address": "aws_instance.web",
                "type": "aws_instance",
                "name": "web",
                "provider_name": "registry.terraform.io/hashicorp/aws",
                "change": {
                    "actions": ["no-op"],
                    "before": {},
                    "after": {}
                }
            }]),
            serde_json::json!([]),
        );

        let plan: TerraformPlan = serde_json::from_str(&json).unwrap();
        let config = Config::default();
        let report = detect_drift(&plan, &config);

        assert_eq!(report.drifted_resources.len(), 0);
        assert_eq!(report.summary.total_drifted, 0);
    }

    #[test]
    fn test_ignore_rules_all_mode() {
        let json = make_plan_json(
            serde_json::json!([]),
            serde_json::json!([{
                "address": "google_cloud_run_v2_service.app",
                "type": "google_cloud_run_v2_service",
                "name": "app",
                "provider_name": "registry.terraform.io/hashicorp/google",
                "change": {
                    "actions": ["update"],
                    "before": {"client": "gcloud", "template": {"0": {"containers": {"0": {"image": "old:v1"}}}}},
                    "after": {"client": "terraform", "template": {"0": {"containers": {"0": {"image": "new:v2"}}}}}
                }
            }]),
        );

        let plan: TerraformPlan = serde_json::from_str(&json).unwrap();
        let config = Config {
            ignore: vec![IgnoreRule {
                resource_types: vec!["google_cloud_run_v2_service".to_string()],
                actions: vec!["update".to_string()],
                attributes: vec![
                    "client".to_string(),
                    "template.*.containers.*.image".to_string(),
                ],
                mode: IgnoreMode::All,
            }],
        };
        let report = detect_drift(&plan, &config);

        assert_eq!(report.drifted_resources.len(), 0);
    }

    #[test]
    fn test_flatten_json() {
        let val = serde_json::json!({
            "tags": {"Name": "web", "Env": "prod"},
            "count": 3
        });
        let mut out = std::collections::BTreeMap::new();
        flatten_json(&val, "", &mut out);

        assert_eq!(out.get("tags.Name").unwrap(), "web");
        assert_eq!(out.get("tags.Env").unwrap(), "prod");
        assert_eq!(out.get("count").unwrap(), "3");
    }
}
