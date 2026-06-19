//! Append-mode session log file.

use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;

use chrono::Local;

use crate::util::MacAddr;

/// A line-oriented session logger that appends across runs.
pub struct SessionLogger {
    writer: BufWriter<std::fs::File>,
}

impl SessionLogger {
    /// Open (creating/appending) and write the session header.
    pub fn open(
        path: &Path,
        iface: &str,
        target: &str,
        bssid: Option<MacAddr>,
    ) -> std::io::Result<SessionLogger> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let mut writer = BufWriter::new(file);
        let bssid = bssid.map(|b| b.to_string()).unwrap_or_else(|| "?".into());
        writeln!(
            writer,
            "# session started {ts}  iface={iface}  target={target}  bssid={bssid}",
            ts = Local::now().format("%Y-%m-%d %H:%M:%S"),
        )?;
        writer.flush()?;
        Ok(SessionLogger { writer })
    }

    fn clock() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    /// `13:38:09 KICK 11:22:33:44:55:66 Apple burst #1`
    pub fn kick(&mut self, mac: MacAddr, vendor: &str, burst: u64) -> std::io::Result<()> {
        writeln!(
            self.writer,
            "{}  KICK    {mac}  {vendor:<18} burst #{burst}",
            Self::clock()
        )?;
        self.writer.flush()
    }

    /// `13:38:17 NEW 22:33:44:55:66:77 Samsung -74 dBm`
    pub fn new_client(&mut self, mac: MacAddr, vendor: &str, rssi: i8) -> std::io::Result<()> {
        writeln!(
            self.writer,
            "{}  NEW     {mac}  {vendor:<18} {rssi} dBm",
            Self::clock()
        )?;
        self.writer.flush()
    }

    /// Free-form note line.
    pub fn note(&mut self, msg: &str) -> std::io::Result<()> {
        writeln!(self.writer, "{}  NOTE    {msg}", Self::clock())?;
        self.writer.flush()
    }

    /// Write the session footer.
    pub fn close(mut self, kicks: u64, clients: usize) -> std::io::Result<()> {
        writeln!(
            self.writer,
            "# session ended   {ts}  kicks={kicks}  clients={clients}",
            ts = Local::now().format("%Y-%m-%d %H:%M:%S"),
        )?;
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_header_and_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("k.log");
        let mac: MacAddr = "11:22:33:44:55:66".parse().unwrap();
        {
            let mut log = SessionLogger::open(&path, "wlan0mon", "CorpNet", Some(mac)).unwrap();
            log.new_client(mac, "Apple", -62).unwrap();
            log.kick(mac, "Apple", 1).unwrap();
            log.close(1, 1).unwrap();
        }
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("# session started"));
        assert!(contents.contains("KICK"));
        assert!(contents.contains("# session ended"));
        assert!(contents.contains("kicks=1"));
    }
}
