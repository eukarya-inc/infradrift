use crate::drift::types::{DriftAction, DriftReport};
use std::io::Write;

pub fn render(report: &DriftReport, writer: &mut dyn Write) -> anyhow::Result<()> {
    if report.drifted_resources.is_empty() {
        writeln!(writer, "# No drift detected.")?;
        return Ok(());
    }

    writeln!(
        writer,
        "# Drift report: {} resource(s) drifted",
        report.summary.total_drifted
    )?;
    if let Some(ref ver) = report.terraform_version {
        writeln!(writer, "# Terraform version: {}", ver)?;
    }
    writeln!(writer)?;

    for resource in &report.drifted_resources {
        let action_prefix = match resource.action {
            DriftAction::Update => "~",
            DriftAction::Delete => "-",
            DriftAction::Create => "+",
            DriftAction::Replace => "-/+",
        };

        writeln!(
            writer,
            "# {} resource \"{}\" \"{}\"",
            action_prefix, resource.resource_type, resource.name
        )?;
        writeln!(
            writer,
            "resource \"{}\" \"{}\" {{",
            resource.resource_type, resource.name
        )?;
        writeln!(writer, "  # address: {}", resource.address)?;
        writeln!(writer, "  # action:  {}", resource.action)?;
        writeln!(writer, "  # source:  {}", resource.source)?;

        for attr in &resource.attribute_changes {
            let before = attr.before.as_deref().unwrap_or("null");
            let after = attr.after.as_deref().unwrap_or("null");

            let change_marker = match (attr.before.is_some(), attr.after.is_some()) {
                (true, true) => "~",
                (false, true) => "+",
                (true, false) => "-",
                (false, false) => "?",
            };

            if attr.sensitive {
                writeln!(
                    writer,
                    "  {} {} = (sensitive) -> (sensitive)",
                    change_marker, attr.path
                )?;
            } else {
                writeln!(
                    writer,
                    "  {} {} = \"{}\" -> \"{}\"",
                    change_marker, attr.path, before, after
                )?;
            }
        }

        writeln!(writer, "}}")?;
        writeln!(writer)?;
    }

    Ok(())
}
