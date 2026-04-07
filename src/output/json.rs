use crate::drift::types::DriftReport;
use std::io::Write;

pub fn render(report: &DriftReport, writer: &mut dyn Write) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    writeln!(writer, "{}", json)?;
    Ok(())
}
