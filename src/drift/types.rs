use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DriftReport {
    pub terraform_version: Option<String>,
    pub drifted_resources: Vec<DriftedResource>,
    pub summary: DriftSummary,
}

#[derive(Debug, Serialize, Clone)]
pub struct DriftedResource {
    pub address: String,
    pub resource_type: String,
    pub name: String,
    pub module_address: Option<String>,
    pub provider: String,
    pub action: DriftAction,
    pub attribute_changes: Vec<AttributeChange>,
    pub source: DriftSource,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DriftAction {
    Update,
    Delete,
    Create,
    Replace,
}

impl std::fmt::Display for DriftAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriftAction::Update => write!(f, "update"),
            DriftAction::Delete => write!(f, "delete"),
            DriftAction::Create => write!(f, "create"),
            DriftAction::Replace => write!(f, "replace"),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct AttributeChange {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub sensitive: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct DriftSummary {
    pub total_drifted: usize,
    pub updates: usize,
    pub deletes: usize,
    pub creates: usize,
    pub replaces: usize,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DriftSource {
    /// From the explicit `resource_drift` field (Terraform 1.4+)
    ResourceDriftKey,
    /// Inferred from `resource_changes` (older Terraform or fallback)
    InferredFromChanges,
}

impl std::fmt::Display for DriftSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriftSource::ResourceDriftKey => write!(f, "resource_drift"),
            DriftSource::InferredFromChanges => write!(f, "inferred"),
        }
    }
}

impl DriftReport {
    pub fn recalculate_summary(&mut self) {
        let mut summary = DriftSummary {
            total_drifted: self.drifted_resources.len(),
            updates: 0,
            deletes: 0,
            creates: 0,
            replaces: 0,
        };
        for r in &self.drifted_resources {
            match r.action {
                DriftAction::Update => summary.updates += 1,
                DriftAction::Delete => summary.deletes += 1,
                DriftAction::Create => summary.creates += 1,
                DriftAction::Replace => summary.replaces += 1,
            }
        }
        self.summary = summary;
    }
}
