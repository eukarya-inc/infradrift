pub mod csv_output;
pub mod hcl;
pub mod human;
pub mod json;
pub mod table;

use crate::cli::OutputFormat;
use crate::drift::types::DriftReport;
use std::io::Write;

pub fn render(
    report: &DriftReport,
    format: &OutputFormat,
    no_color: bool,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Human => human::render(report, no_color, writer),
        OutputFormat::Json => json::render(report, writer),
        OutputFormat::Csv => csv_output::render(report, writer),
        OutputFormat::Table => table::render(report, no_color, writer),
        OutputFormat::Hcl => hcl::render(report, writer),
    }
}
