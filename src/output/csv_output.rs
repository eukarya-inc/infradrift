use crate::drift::types::DriftReport;
use std::io::Write;

pub fn render(report: &DriftReport, writer: &mut dyn Write) -> anyhow::Result<()> {
    let mut csv_writer = csv::Writer::from_writer(writer);

    csv_writer.write_record([
        "address",
        "resource_type",
        "action",
        "attribute_path",
        "before",
        "after",
        "sensitive",
        "source",
    ])?;

    for resource in &report.drifted_resources {
        if resource.attribute_changes.is_empty() {
            // Resource-level entry with no attribute details (e.g., delete)
            csv_writer.write_record([
                &resource.address,
                &resource.resource_type,
                &resource.action.to_string(),
                "",
                "",
                "",
                "false",
                &resource.source.to_string(),
            ])?;
        } else {
            for attr in &resource.attribute_changes {
                csv_writer.write_record([
                    &resource.address,
                    &resource.resource_type,
                    &resource.action.to_string(),
                    &attr.path,
                    attr.before.as_deref().unwrap_or(""),
                    attr.after.as_deref().unwrap_or(""),
                    &attr.sensitive.to_string(),
                    &resource.source.to_string(),
                ])?;
            }
        }
    }

    csv_writer.flush()?;
    Ok(())
}
