---
name: infradrift
description: "Use this skill to detect and analyze Terraform/OpenTofu infrastructure drift. Run drift scans against live state, parse existing plan files, configure ignore rules for expected drift, and triage findings by resource type, address, or attribute. Prefer this skill for any IaC drift detection, filtering, or reporting task."
metadata:
  openclaw:
    requires:
      bins:
        - infradrift
      anyBins:
        - terraform
        - tofu
    homepage: https://github.com/eukarya-inc/infradrift
    install:
      - kind: brew
        formula: eukarya-inc/tap/infradrift
        bins: [infradrift]
      - kind: cargo
        package: infradrift
        bins: [infradrift]
---

# Infrastructure Drift Detection with infradrift

infradrift detects infrastructure drift from Terraform/OpenTofu plans. It replaces complex shell/jq-based drift detection scripts with a single, fast binary that supports configurable ignore rules, multiple output formats, and flexible filtering.

## Safety Defaults

- All operations are **read-only**. infradrift never modifies infrastructure state.
- `infradrift scan` runs `terraform plan` (or `tofu plan`) under the hood — this is a non-destructive, read-only operation.
- `infradrift parse` only reads a plan file from disk.
- No credentials, state files, or secrets are transmitted externally.

## Core Workflow

Every drift detection session follows this pattern:

1. **Scan or parse** — either run a live plan (`scan`) or analyze an existing plan file (`parse`).
2. **Review findings** — inspect the drift report for unexpected changes.
3. **Filter noise** — use CLI flags or `infradrift.toml` ignore rules to suppress expected drift.
4. **Triage** — decide whether drifted resources need remediation (re-apply) or config updates.

## Commands

### `scan` — Run terraform/tofu plan and detect drift

```bash
infradrift scan                              # Scan current directory
infradrift scan --dir ./terraform/prod       # Scan a specific directory
infradrift scan --tofu                       # Use OpenTofu instead of Terraform
infradrift scan -- -var-file=prod.tfvars     # Pass extra args to terraform plan
```

### `parse` — Parse an existing plan file

```bash
infradrift parse --file plan.json            # Parse JSON plan output
infradrift parse --file tfplan               # Parse binary planfile (auto-detected)
infradrift parse --file tfplan --binary      # Force binary planfile mode
infradrift parse --file tfplan --tofu        # Use OpenTofu for binary conversion
```

### `validate` — Validate an infradrift.toml configuration file

```bash
infradrift validate                                  # Validate default infradrift.toml
infradrift validate --config ./prod/infradrift.toml  # Validate a specific config file
```

Checks that the config file exists, is valid TOML, and reports warnings for:
- Unknown action values (valid: `update`, `delete`, `create`, `replace`)
- Malformed attribute patterns (empty, leading/trailing dots, empty segments)
- `mode = "any"` without attributes (no effect)
- Overly broad catch-all rules (no resource_types, actions, or attributes)

### `completions` — Generate shell completions

```bash
infradrift completions bash                  # Bash completions
infradrift completions zsh                   # Zsh completions
```

## Shared Options (scan & parse)

| Option | Description |
|--------|-------------|
| `-o, --format <FORMAT>` | Output format: `human`, `json`, `csv`, `table`, `hcl` (default: `human`) |
| `-t, --type <TYPE>` | Filter by resource type. Repeatable. |
| `-n, --name <NAME>` | Filter by resource address (supports glob patterns). Repeatable. |
| `-a, --attr <ATTR>` | Filter by changed attribute name. Repeatable. |
| `--no-color` | Suppress colored output |
| `-c, --config <CONFIG>` | Path to config file (default: `infradrift.toml`) |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | No drift detected |
| `1` | Drift detected |
| `2` | Error (parse failure, binary not found, etc.) |

Use exit codes for CI gating:

```bash
infradrift scan --dir ./terraform/prod || echo "Drift detected!"
```

## Output Formats

- **`human`** (default) — Colored, human-readable output grouped by resource with change details.
- **`json`** — Structured JSON with `drifted_resources`, `attribute_changes`, and `summary`.
- **`csv`** — Flat rows: `address`, `resource_type`, `action`, `attribute_path`, `before`, `after`, `sensitive`, `source`.
- **`table`** — ASCII table with resource address, type, action, changed attributes, and source.
- **`hcl`** — Terraform plan diff-style output.

Choose the format based on context:

- Use `human` for interactive review.
- Use `json` for programmatic processing or piping to other tools.
- Use `csv` for spreadsheet export or further analysis.
- Use `table` for concise summaries.
- Use `hcl` for Terraform-native diff representation.

## Filtering

### By resource type

```bash
infradrift parse --file plan.json -t aws_instance
infradrift parse --file plan.json -t aws_instance -t aws_s3_bucket
```

### By resource address (glob patterns)

```bash
infradrift parse --file plan.json -n "module.vpc.*"
infradrift parse --file plan.json -n "aws_instance.web"
```

### By attribute

```bash
infradrift parse --file plan.json -a tags
infradrift parse --file plan.json -a tags -a instance_type
```

### Combining filters

```bash
infradrift parse --file plan.json -t aws_instance -n "module.vpc.*" -a tags
```

## Configuration — Ignore Rules

Create an `infradrift.toml` to suppress expected drift. This is the primary mechanism for reducing noise in drift reports.

### Rule structure

```toml
[[ignore]]
resource_types = ["google_cloud_run_v2_service", "google_cloud_run_v2_job"]
actions = ["update"]
attributes = [
  "client_version",
  "etag",
  "template.*.containers.*.image",
  "update_time",
]
mode = "all"
```

### Rule fields

| Field | Description |
|-------|-------------|
| `resource_types` | Resource types to match (empty = all types) |
| `actions` | Actions to match: `update`, `delete`, `create`, `replace` (empty = all) |
| `attributes` | Attribute patterns to match (supports `*` wildcard for path segments) |
| `mode` | `all` or `any` (default: `all`) |

### Mode behavior

- **`all`** — Ignore the entire resource only if ALL drifted attributes match the ignore patterns. If any attribute doesn't match, the resource appears with all its changes. Use this when you want to hide a resource only if the drift is entirely expected.

- **`any`** — Ignore individual matching attributes but keep the resource visible if it has other non-matching changes. Use this when you want to surgically strip known-noisy attributes while preserving real drift signals.

### Attribute patterns

Patterns use dot-separated paths with `*` as a single-segment wildcard:

| Pattern | Matches |
|---------|---------|
| `tags` | `tags.Name`, `tags.Environment`, etc. |
| `tags.Name` | Exactly `tags.Name` |
| `template.*.containers.*.image` | `template.0.containers.0.image`, etc. |
| `latest_created_execution.*` | All sub-attributes of `latest_created_execution.0` |

### Common ignore patterns

```toml
# Cloud Run deployment churn
[[ignore]]
resource_types = ["google_cloud_run_v2_service", "google_cloud_run_v2_job"]
actions = ["update"]
attributes = [
  "client_version", "etag", "generation", "observed_generation",
  "conditions.*", "terminal_condition.*",
  "latest_created_execution.*", "latest_created_revision", "latest_ready_revision",
  "template.*.containers.*.image", "template.*.template.*.containers.*.image",
  "update_time",
]
mode = "all"

# IAM etag churn
[[ignore]]
resource_types = ["google_service_account_iam_member", "google_project_iam_member"]
actions = ["update"]
attributes = ["etag"]
mode = "all"

# Cloud SQL computed fields
[[ignore]]
resource_types = ["google_sql_database_instance"]
actions = ["update"]
attributes = ["settings.*.version", "dns_names.*"]
mode = "all"
```

## Drift Detection Strategy

infradrift uses two strategies:

1. **`resource_drift` key** (Terraform 1.4+) — The plan JSON includes an explicit `resource_drift` field. This is the most reliable signal.
2. **`resource_changes` fallback** — For older Terraform versions, infradrift scans `resource_changes` for non-no-op, non-create actions. These are flagged as "inferred" drift.

Sensitive attribute values are automatically masked as `(sensitive)`.

## CI/CD Integration

### GitHub Actions

```yaml
- name: Detect drift
  run: |
    infradrift parse --file tfplan --format json --no-color > drift-report.json
    infradrift parse --file tfplan --format human --no-color

- name: Check for drift
  id: drift
  run: infradrift parse --file tfplan --no-color
  continue-on-error: true

- name: Alert on drift
  if: steps.drift.outcome == 'failure'
  run: echo "Drift detected!"
```
