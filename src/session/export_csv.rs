//! CSV session export - RFC 4180, one row per client.

use std::path::Path;

use super::SessionReport;

/// Render the per-client table as a CSV string.
pub fn to_string(report: &SessionReport) -> anyhow::Result<String> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record([
        "mac",
        "vendor",
        "os_guess",
        "nickname",
        "first_seen",
        "last_seen",
        "rssi_avg",
        "kick_count",
        "probe_ssids",
    ])?;
    for c in &report.clients {
        wtr.write_record([
            c.mac.to_string(),
            c.vendor.clone().unwrap_or_default(),
            c.os_guess.clone().unwrap_or_default(),
            c.nickname.clone().unwrap_or_default(),
            c.first_seen.to_rfc3339(),
            c.last_seen.to_rfc3339(),
            c.rssi_avg.to_string(),
            c.kick_count.to_string(),
            // probe SSIDs joined with ';' inside one CSV field
            c.probe_ssids.join(";"),
        ])?;
    }
    let bytes = wtr.into_inner()?;
    Ok(String::from_utf8(bytes)?)
}

/// Write the CSV to `path`.
pub fn write(report: &SessionReport, path: &Path) -> anyhow::Result<()> {
    std::fs::write(path, to_string(report)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::config::Config;
    use crate::app::state::{AppState, Client, Target};
    use crate::util::MacAddr;

    #[test]
    fn header_and_rows() {
        let mut st = AppState::new(Config::default());
        let mut t = Target::new("CorpNet");
        let mac: MacAddr = "11:22:33:44:55:66".parse().unwrap();
        let mut c = Client::new(mac, -60);
        c.vendor = Some("Apple".into());
        c.record_probe("CorpNet");
        c.record_probe("Starbucks");
        t.clients.insert(mac, c);
        st.targets.push(t);

        let report = SessionReport::from_state(&st);
        let csv = to_string(&report).unwrap();
        assert!(csv.starts_with("mac,vendor,os_guess"));
        assert!(csv.contains("11:22:33:44:55:66"));
        assert!(csv.contains("CorpNet;Starbucks"));
    }
}
