use crate::drift::types::{DriftAction, DriftReport, DriftSource};
use colored::Colorize;
use std::io::Write;

pub fn render(report: &DriftReport, no_color: bool, writer: &mut dyn Write) -> anyhow::Result<()> {
    if no_color {
        colored::control::set_override(false);
    }

    if report.drifted_resources.is_empty() {
        writeln!(writer, "{}", "No drift detected.".green())?;
        return Ok(());
    }

    writeln!(
        writer,
        "{}",
        format!(
            "Drift detected: {} resource(s)",
            report.summary.total_drifted
        )
        .red()
        .bold()
    )?;

    if let Some(ref ver) = report.terraform_version {
        writeln!(writer, "Terraform version: {}", ver)?;
    }
    writeln!(writer)?;

    for resource in &report.drifted_resources {
        let action_str = match resource.action {
            DriftAction::Update => "~ update".yellow(),
            DriftAction::Delete => "- delete".red(),
            DriftAction::Create => "+ create".green(),
            DriftAction::Replace => "-/+ replace".magenta(),
        };

        let source_badge = match resource.source {
            DriftSource::ResourceDriftKey => "[drift]".cyan(),
            DriftSource::InferredFromChanges => "[inferred]".dimmed(),
        };

        writeln!(
            writer,
            "  {} {} {} ({})",
            action_str,
            resource.address.bold(),
            source_badge,
            resource.resource_type.dimmed()
        )?;

        for attr in &resource.attribute_changes {
            let before = attr.before.as_deref().unwrap_or("(none)");
            let after = attr.after.as_deref().unwrap_or("(none)");

            if attr.sensitive {
                writeln!(
                    writer,
                    "      {} = {} -> {}",
                    attr.path,
                    "(sensitive)".yellow(),
                    "(sensitive)".yellow()
                )?;
            } else {
                writeln!(
                    writer,
                    "      {} = {} -> {}",
                    attr.path,
                    before.red(),
                    after.green()
                )?;
            }
        }
        writeln!(writer)?;
    }

    // Summary
    writeln!(writer, "{}", "Summary:".bold())?;
    if report.summary.updates > 0 {
        writeln!(writer, "  ~ {} updated", report.summary.updates)?;
    }
    if report.summary.deletes > 0 {
        writeln!(writer, "  - {} deleted", report.summary.deletes)?;
    }
    if report.summary.creates > 0 {
        writeln!(writer, "  + {} created", report.summary.creates)?;
    }
    if report.summary.replaces > 0 {
        writeln!(writer, "  -/+ {} replaced", report.summary.replaces)?;
    }

    Ok(())
}
