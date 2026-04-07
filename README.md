# infradrift

Simple drift detection for IaC: run plans or parse plan output, identify unmanaged changes, and filter expected drift.

`infradrift` is a Rust CLI tool that detects infrastructure drift from Terraform/OpenTofu plans. It replaces complex shell/jq-based drift detection scripts with a single, fast binary that supports configurable ignore rules, multiple output formats, and flexible filtering.

## Installation

### Homebrew

```bash
brew tap eukarya-inc/tap
brew install infradrift
```

### From source

```bash
cargo install --path .
```

### From releases

Download the binary for your platform from the [Releases](https://github.com/eukarya-inc/infradrift/releases) page.

## Quick start

```bash
# Run terraform plan and detect drift in current directory
infradrift scan

# Parse an existing plan JSON file
infradrift parse --file plan.json

# Parse a binary planfile (auto-detected, or force with --binary)
infradrift parse --file tfplan

# Use OpenTofu instead of Terraform
infradrift scan --tofu
```

## Usage

### Subcommands

#### `scan` - Run terraform/tofu plan and detect drift

```bash
infradrift scan [OPTIONS] [-- <PLAN_ARGS>...]
```

| Option | Description |
|--------|-------------|
| `-d, --dir <DIR>` | Path to the Terraform working directory (default: `.`) |
| `--tofu` | Use OpenTofu instead of Terraform |
| `-- <PLAN_ARGS>` | Additional arguments passed to `terraform plan` |

```bash
# Scan a specific directory
infradrift scan --dir ./terraform/prod

# Pass extra args to terraform plan
infradrift scan -- -var-file=prod.tfvars -lock=false
```

#### `parse` - Parse an existing plan file and detect drift

```bash
infradrift parse [OPTIONS] --file <FILE>
```

| Option | Description |
|--------|-------------|
| `-f, --file <FILE>` | Path to the plan file (JSON or binary planfile) |
| `--binary` | Force treating the file as a binary planfile |
| `--tofu` | Use OpenTofu for converting binary planfiles |

```bash
# Parse JSON output from terraform show
terraform show -json tfplan > plan.json
infradrift parse --file plan.json

# Parse a binary planfile directly (auto-detected)
infradrift parse --file tfplan
```

### Shared options

These options work with both `scan` and `parse`:

| Option | Description |
|--------|-------------|
| `-o, --format <FORMAT>` | Output format: `human`, `json`, `csv`, `table`, `hcl` (default: `human`) |
| `-t, --type <TYPE>` | Filter by resource type. Repeatable. |
| `-n, --name <NAME>` | Filter by resource address (supports glob patterns). Repeatable. |
| `-a, --attr <ATTR>` | Filter by changed attribute name. Repeatable. |
| `--no-color` | Suppress colored output |
| `-c, --config <CONFIG>` | Path to config file (default: `infradrift.toml`) |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | No drift detected |
| `1` | Drift detected |
| `2` | Error (parse failure, binary not found, etc.) |

This makes `infradrift` usable as a CI gate:

```bash
infradrift scan --dir ./terraform/prod || echo "Drift detected!"
```

## Output formats

### Human (default)

Colored, human-readable output grouped by resource:

```
Drift detected: 2 resource(s)
Terraform version: 1.5.7

  ~ update aws_instance.web [drift] (aws_instance)
      instance_type = t3.micro -> t3.small
      tags.Name = web-server -> web-server-modified

  ~ update module.vpc.aws_subnet.private [drift] (aws_subnet)
      cidr_block = 10.0.1.0/24 -> 10.0.2.0/24

Summary:
  ~ 2 updated
```

### JSON

```bash
infradrift parse --file plan.json --format json
```

Outputs structured JSON with full drift report including `drifted_resources`, `attribute_changes`, and `summary`.

### CSV

```bash
infradrift parse --file plan.json --format csv
```

Flat rows with columns: `address`, `resource_type`, `action`, `attribute_path`, `before`, `after`, `sensitive`, `source`. One row per attribute change.

### Table

```bash
infradrift parse --file plan.json --format table
```

```
┌───────────────────────────────┬──────────────┬────────┬──────────────────────┬────────────────┐
│ Address                       ┆ Type         ┆ Action ┆ Changed Attributes   ┆ Source         │
╞═══════════════════════════════╪══════════════╪════════╪══════════════════════╪════════════════╡
│ aws_instance.web              ┆ aws_instance ┆ update ┆ instance_type, tags  ┆ resource_drift │
└───────────────────────────────┴──────────────┴────────┴──────────────────────┴────────────────┘
```

### HCL

```bash
infradrift parse --file plan.json --format hcl
```

Terraform plan diff-style output:

```hcl
# ~ resource "aws_instance" "web"
resource "aws_instance" "web" {
  # address: aws_instance.web
  # action:  update
  ~ instance_type = "t3.micro" -> "t3.small"
  ~ tags.Name = "web-server" -> "web-server-modified"
}
```

## Filtering

### By resource type

```bash
# Only show drift for aws_instance resources
infradrift parse --file plan.json -t aws_instance

# Multiple types
infradrift parse --file plan.json -t aws_instance -t aws_s3_bucket
```

### By resource address (glob patterns)

```bash
# All resources in a module
infradrift parse --file plan.json -n "module.vpc.*"

# Specific resource
infradrift parse --file plan.json -n "aws_instance.web"
```

### By attribute

```bash
# Only show tag-related drift
infradrift parse --file plan.json -a tags

# Multiple attributes
infradrift parse --file plan.json -a tags -a instance_type
```

### Combining filters

```bash
# AWS instances with tag drift in the vpc module
infradrift parse --file plan.json -t aws_instance -n "module.vpc.*" -a tags
```

## Configuration

Create an `infradrift.toml` file to define ignore rules for expected drift. This replaces complex jq/bash filtering scripts.

### Ignore rules

```toml
# Ignore Cloud Run deployment-related attribute changes
[[ignore]]
resource_types = ["google_cloud_run_v2_service", "google_cloud_run_v2_job"]
actions = ["update"]
attributes = [
  "client",
  "client_version",
  "template.*.containers.*.image",
  "template.*.containers.*.name",
  "template.*.revision",
  "traffic.*.revision",
  "traffic.*.type",
]
mode = "all"

# Ignore tag-only drift on all resources
# [[ignore]]
# attributes = ["tags", "tags_all"]
# mode = "all"
```

### Ignore rule fields

| Field | Description |
|-------|-------------|
| `resource_types` | Resource types to match (empty = all types) |
| `actions` | Actions to match: `update`, `delete`, `create`, `replace` (empty = all) |
| `attributes` | Attribute patterns to match (supports `*` wildcard for path segments) |
| `mode` | `all` = ignore resource only if ALL changes are in listed attributes; `any` = ignore individual matching attributes but keep the resource |

### Attribute patterns

Patterns use dot-separated paths with `*` as a single-segment wildcard:

| Pattern | Matches |
|---------|---------|
| `tags` | `tags.Name`, `tags.Environment`, etc. |
| `tags.Name` | Exactly `tags.Name` |
| `template.*.containers.*.image` | `template.0.containers.0.image`, etc. |
| `traffic.*.revision` | `traffic.0.revision`, `traffic.1.revision`, etc. |

## Drift detection

`infradrift` uses two strategies for detecting drift:

1. **`resource_drift` key** (Terraform 1.4+): The plan JSON includes an explicit `resource_drift` field listing resources that changed outside of Terraform. This is the most reliable signal.

2. **`resource_changes` fallback**: For older Terraform versions without `resource_drift`, infradrift scans `resource_changes` for non-no-op, non-create actions. These are flagged as "inferred" drift since they may include config-driven changes.

Sensitive attribute values are automatically masked as `(sensitive)`.

## Shell completions

Generate shell completions for tab-completion support:

### Bash

```bash
# Add to ~/.bashrc
eval "$(infradrift completions bash)"
```

### Zsh

```bash
# Add to ~/.zshrc (ensure the completions directory is in your fpath)
infradrift completions zsh > "${fpath[1]}/_infradrift"
```

After adding, restart your shell or source the config file.

## CI/CD integration

### GitHub Actions example

```yaml
- name: Detect drift
  run: |
    infradrift parse --file tfplan --format json --no-color > drift-report.json
    infradrift parse --file tfplan --format human --no-color
```

### Using exit codes for gating

```yaml
- name: Check for drift
  id: drift
  run: infradrift parse --file tfplan --no-color
  continue-on-error: true

- name: Alert on drift
  if: steps.drift.outcome == 'failure'
  run: echo "Drift detected!"
```

## License

MIT
