# infradrift Security & Trust

**TL;DR**: infradrift is a local, read-only CLI tool. It never modifies infrastructure state, sends telemetry, or makes external network calls.

## What infradrift Does

- Runs `terraform plan` or `tofu plan` (read-only) and parses the JSON output
- Reads existing plan files from disk
- Detects resource drift by comparing planned vs actual state
- Outputs drift reports in various formats (human, JSON, CSV, table, HCL)
- Applies configurable ignore rules to filter expected drift

**All operations are local and read-only.** No state is modified. No data leaves your machine.

## What infradrift Does NOT Do

- Does not apply, destroy, or modify any infrastructure
- Does not read or transmit secrets, credentials, or state files
- Does not make network calls (Terraform/OpenTofu handles provider communication)
- Does not send telemetry, analytics, or crash reports
- Does not write to any files outside of stdout/stderr

## Builds & Verification

Every release includes checksums alongside binaries:

```bash
# After downloading, verify:
sha256sum -c checksums.txt
```

Binaries are built automatically from tagged commits via GitHub Actions (publicly visible at https://github.com/eukarya-inc/infradrift/actions).

## Open Source

- **Source**: https://github.com/eukarya-inc/infradrift (MIT)
- **Releases**: https://github.com/eukarya-inc/infradrift/releases
- **Single static binary** — fully auditable Rust codebase with no runtime dependencies beyond Terraform/OpenTofu.

## Dependency on Terraform/OpenTofu

infradrift shells out to `terraform` or `tofu` for two operations:

1. `terraform plan -out=<tmpfile>` — when using `infradrift scan`
2. `terraform show -json <planfile>` — when converting binary planfiles

These are standard, read-only Terraform commands. infradrift never runs `apply`, `destroy`, `import`, or any state-modifying command.

## Questions?

- Source code: https://github.com/eukarya-inc/infradrift
- Issues: https://github.com/eukarya-inc/infradrift/issues
