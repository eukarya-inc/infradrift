use serde::Deserialize;

/// Root structure of `terraform show -json <planfile>` output.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TerraformPlan {
    pub format_version: Option<String>,
    pub terraform_version: Option<String>,

    /// Resource changes planned by Terraform (includes config-driven + drift).
    #[serde(default)]
    pub resource_changes: Vec<ResourceChange>,

    /// Explicit drift detected before planning (Terraform 1.4+).
    /// Each entry has the same shape as resource_changes.
    #[serde(default)]
    pub resource_drift: Vec<ResourceChange>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResourceChange {
    /// Full address, e.g. "module.vpc.aws_subnet.private[0]"
    pub address: String,

    /// Module address, e.g. "module.vpc"
    pub module_address: Option<String>,

    /// Resource type, e.g. "aws_instance"
    #[serde(rename = "type")]
    pub resource_type: String,

    /// Resource name, e.g. "web"
    pub name: String,

    /// Provider name, e.g. "registry.terraform.io/hashicorp/aws"
    pub provider_name: String,

    /// The change details
    pub change: Change,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Change {
    /// Actions: ["no-op"], ["create"], ["update"], ["delete"], ["create", "delete"], etc.
    pub actions: Vec<String>,

    /// Resource state before the change (null for create)
    pub before: Option<serde_json::Value>,

    /// Resource state after the change (null for delete)
    pub after: Option<serde_json::Value>,

    /// Marks which fields in `before` are sensitive
    #[serde(default)]
    pub before_sensitive: serde_json::Value,

    /// Marks which fields in `after` are sensitive
    #[serde(default)]
    pub after_sensitive: serde_json::Value,

    /// Fields whose values are not yet known (computed after apply)
    #[serde(default)]
    pub after_unknown: serde_json::Value,
}
