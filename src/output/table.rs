use crate::drift::types::{DriftAction, DriftReport};
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use std::io::Write;

pub fn render(report: &DriftReport, no_color: bool, writer: &mut dyn Write) -> anyhow::Result<()> {
    if report.drifted_resources.is_empty() {
        writeln!(writer, "No drift detected.")?;
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        "Address",
        "Type",
        "Action",
        "Changed Attributes",
        "Source",
    ]);

    for resource in &report.drifted_resources {
        let attrs: Vec<String> = resource
            .attribute_changes
            .iter()
            .map(|a| a.path.clone())
            .collect();
        let attrs_str = if attrs.is_empty() {
            "-".to_string()
        } else {
            attrs.join(", ")
        };

        let action_color = if no_color {
            None
        } else {
            Some(match resource.action {
                DriftAction::Update => Color::Yellow,
                DriftAction::Delete => Color::Red,
                DriftAction::Create => Color::Green,
                DriftAction::Replace => Color::Magenta,
            })
        };

        let action_cell = if let Some(color) = action_color {
            Cell::new(resource.action.to_string()).fg(color)
        } else {
            Cell::new(resource.action.to_string())
        };

        table.add_row(vec![
            Cell::new(&resource.address),
            Cell::new(&resource.resource_type),
            action_cell,
            Cell::new(&attrs_str),
            Cell::new(resource.source.to_string()),
        ]);
    }

    writeln!(writer, "{}", table)?;

    writeln!(
        writer,
        "\nTotal: {} drifted resource(s)",
        report.summary.total_drifted
    )?;

    Ok(())
}
