//! JSON session export.

use std::path::Path;

use super::SessionReport;

/// Serialize the report as pretty JSON to a string.
pub fn to_string(report: &SessionReport) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}

/// Write the report to `path` as pretty JSON.
pub fn write(report: &SessionReport, path: &Path) -> anyhow::Result<()> {
    std::fs::write(path, to_string(report)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::config::Config;
    use crate::app::state::AppState;

    #[test]
    fn produces_valid_json() {
        let st = AppState::new(Config::default());
        let report = SessionReport::from_state(&st);
        let json = to_string(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["kto_version"], env!("CARGO_PKG_VERSION"));
        assert!(parsed["summary"]["total_kicks"].is_number());
    }
}
