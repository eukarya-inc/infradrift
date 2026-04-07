use crate::plan::parser::parse_plan_json;
use crate::plan::schema::TerraformPlan;
use anyhow::{bail, Context};
use std::path::Path;
use std::process::Command;

/// Run terraform/tofu plan and return the parsed plan.
pub fn execute_plan(
    dir: &Path,
    use_tofu: bool,
    extra_args: &[String],
) -> anyhow::Result<TerraformPlan> {
    let binary = if use_tofu { "tofu" } else { "terraform" };

    // Create a temp file for the plan output
    let plan_file = dir.join(".infradrift-tfplan");

    // Build the plan command
    let mut cmd = Command::new(binary);
    cmd.current_dir(dir);
    cmd.arg("plan");
    cmd.args(["-out", plan_file.to_str().unwrap()]);
    cmd.args(["-detailed-exitcode"]);
    cmd.args(extra_args);

    eprintln!("Running {} plan in {}...", binary, dir.display());

    let output = cmd
        .output()
        .with_context(|| format!("{} binary not found. Is it installed and on PATH?", binary))?;

    let exit_code = output.status.code().unwrap_or(-1);

    // Clean up plan file on exit
    let _cleanup = CleanupGuard(plan_file.clone());

    match exit_code {
        0 => {
            // No changes - return empty plan
            eprintln!("No changes detected.");
            Ok(TerraformPlan {
                format_version: None,
                terraform_version: None,
                resource_changes: vec![],
                resource_drift: vec![],
            })
        }
        2 => {
            // Changes detected - convert plan to JSON
            eprintln!("Changes detected, analyzing drift...");
            let show_output = Command::new(binary)
                .current_dir(dir)
                .args(["show", "-json"])
                .arg(&plan_file)
                .output()
                .with_context(|| format!("{} show -json failed", binary))?;

            if !show_output.status.success() {
                let stderr = String::from_utf8_lossy(&show_output.stderr);
                bail!("{} show -json failed: {}", binary, stderr.trim());
            }

            let json = String::from_utf8(show_output.stdout)
                .context("terraform show output is not valid UTF-8")?;
            parse_plan_json(&json)
        }
        1 => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("{} plan failed:\n{}", binary, stderr.trim());
        }
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "{} plan exited with unexpected code {}:\n{}",
                binary,
                exit_code,
                stderr.trim()
            );
        }
    }
}

/// RAII guard to clean up the temporary plan file.
struct CleanupGuard(std::path::PathBuf);

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
