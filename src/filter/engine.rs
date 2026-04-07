use crate::drift::types::DriftReport;

pub struct Filters {
    pub resource_types: Vec<String>,
    pub resource_names: Vec<String>,
    pub attributes: Vec<String>,
}

impl Filters {
    pub fn is_empty(&self) -> bool {
        self.resource_types.is_empty()
            && self.resource_names.is_empty()
            && self.attributes.is_empty()
    }

    pub fn apply(&self, report: &mut DriftReport) {
        if self.is_empty() {
            return;
        }

        report.drifted_resources.retain(|r| {
            self.matches_type(&r.resource_type)
                && self.matches_name(&r.address)
                && self.matches_attributes(&r.attribute_changes)
        });

        // If attribute filter is active, prune non-matching attribute changes
        if !self.attributes.is_empty() {
            for r in &mut report.drifted_resources {
                r.attribute_changes.retain(|a| {
                    self.attributes
                        .iter()
                        .any(|f| a.path.starts_with(f) || a.path == *f)
                });
            }
            // Remove resources with no remaining attribute changes
            report
                .drifted_resources
                .retain(|r| !r.attribute_changes.is_empty());
        }

        report.recalculate_summary();
    }

    fn matches_type(&self, resource_type: &str) -> bool {
        if self.resource_types.is_empty() {
            return true;
        }
        self.resource_types.iter().any(|t| t == resource_type)
    }

    fn matches_name(&self, address: &str) -> bool {
        if self.resource_names.is_empty() {
            return true;
        }
        self.resource_names
            .iter()
            .any(|pattern| glob_match(pattern, address))
    }

    fn matches_attributes(
        &self,
        attribute_changes: &[crate::drift::types::AttributeChange],
    ) -> bool {
        if self.attributes.is_empty() {
            return true;
        }
        // At least one attribute change must match a filter
        attribute_changes.iter().any(|a| {
            self.attributes
                .iter()
                .any(|f| a.path.starts_with(f) || a.path == *f)
        })
    }
}

/// Simple glob matching with `*` as wildcard for any substring.
fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();

    if parts.len() == 1 {
        // No wildcard, exact match
        return pattern == text;
    }

    let mut pos = 0;

    // First part must match at the start
    if !parts[0].is_empty() {
        if !text.starts_with(parts[0]) {
            return false;
        }
        pos = parts[0].len();
    }

    // Middle parts must appear in order
    for part in &parts[1..parts.len() - 1] {
        if part.is_empty() {
            continue;
        }
        if let Some(found) = text[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }

    // Last part must match at the end
    let last = parts[parts.len() - 1];
    if !last.is_empty() && !text[pos..].ends_with(last) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drift::types::*;

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("aws_instance.web", "aws_instance.web"));
        assert!(!glob_match("aws_instance.web", "aws_instance.api"));
    }

    #[test]
    fn test_glob_match_wildcard() {
        assert!(glob_match("module.vpc.*", "module.vpc.aws_subnet.private"));
        assert!(glob_match(
            "*.aws_instance.*",
            "module.app.aws_instance.web"
        ));
        assert!(!glob_match("module.vpc.*", "module.app.aws_instance.web"));
    }

    #[test]
    fn test_glob_match_trailing_wildcard() {
        assert!(glob_match("module.*", "module.vpc.aws_subnet.private"));
    }

    #[test]
    fn test_filter_by_type() {
        let filters = Filters {
            resource_types: vec!["aws_instance".to_string()],
            resource_names: vec![],
            attributes: vec![],
        };

        let mut report = make_test_report();
        filters.apply(&mut report);

        assert_eq!(report.drifted_resources.len(), 1);
        assert_eq!(report.drifted_resources[0].resource_type, "aws_instance");
    }

    #[test]
    fn test_filter_by_name_glob() {
        let filters = Filters {
            resource_types: vec![],
            resource_names: vec!["module.vpc.*".to_string()],
            attributes: vec![],
        };

        let mut report = make_test_report();
        filters.apply(&mut report);

        assert_eq!(report.drifted_resources.len(), 1);
        assert_eq!(
            report.drifted_resources[0].address,
            "module.vpc.aws_subnet.private"
        );
    }

    #[test]
    fn test_filter_by_attribute() {
        let filters = Filters {
            resource_types: vec![],
            resource_names: vec![],
            attributes: vec!["tags".to_string()],
        };

        let mut report = make_test_report();
        filters.apply(&mut report);

        // Only resources with tag changes should remain
        assert!(report.drifted_resources.iter().all(|r| r
            .attribute_changes
            .iter()
            .any(|a| a.path.starts_with("tags"))));
    }

    fn make_test_report() -> DriftReport {
        let mut report = DriftReport {
            terraform_version: Some("1.5.0".to_string()),
            drifted_resources: vec![
                DriftedResource {
                    address: "aws_instance.web".to_string(),
                    resource_type: "aws_instance".to_string(),
                    name: "web".to_string(),
                    module_address: None,
                    provider: "aws".to_string(),
                    action: DriftAction::Update,
                    attribute_changes: vec![AttributeChange {
                        path: "tags.Name".to_string(),
                        before: Some("old".to_string()),
                        after: Some("new".to_string()),
                        sensitive: false,
                    }],
                    source: DriftSource::ResourceDriftKey,
                },
                DriftedResource {
                    address: "module.vpc.aws_subnet.private".to_string(),
                    resource_type: "aws_subnet".to_string(),
                    name: "private".to_string(),
                    module_address: Some("module.vpc".to_string()),
                    provider: "aws".to_string(),
                    action: DriftAction::Update,
                    attribute_changes: vec![AttributeChange {
                        path: "cidr_block".to_string(),
                        before: Some("10.0.1.0/24".to_string()),
                        after: Some("10.0.2.0/24".to_string()),
                        sensitive: false,
                    }],
                    source: DriftSource::ResourceDriftKey,
                },
            ],
            summary: DriftSummary {
                total_drifted: 2,
                updates: 2,
                deletes: 0,
                creates: 0,
                replaces: 0,
            },
        };
        report.recalculate_summary();
        report
    }
}
