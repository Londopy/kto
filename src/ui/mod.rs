//! UI layer: front-end selection, theme palettes, and shared session
//! finalization (exports on exit).

pub mod cli;

#[cfg(feature = "tui")]
pub mod tui;

use crate::app::state::Shared;
use crate::cli::Args;
use crate::session::{export_csv, export_html, export_json, SessionReport};

/// An RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// A theme palette.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Rgb,
    pub primary: Rgb,
    pub accent: Rgb,
    pub alert: Rgb,
}

impl Palette {
    /// Resolve a palette by theme name (falls back to `dark`).
    pub fn by_name(name: &str) -> Palette {
        match name {
            "matrix" => Palette {
                bg: Rgb(0x00, 0x00, 0x00),
                primary: Rgb(0x00, 0xff, 0x41),
                accent: Rgb(0x00, 0x3b, 0x00),
                alert: Rgb(0x39, 0xff, 0x14),
            },
            "blood" => Palette {
                bg: Rgb(0x1a, 0x00, 0x00),
                primary: Rgb(0xff, 0x00, 0x00),
                accent: Rgb(0x8b, 0x00, 0x00),
                alert: Rgb(0xff, 0x44, 0x44),
            },
            "dracula" => Palette {
                bg: Rgb(0x28, 0x2a, 0x36),
                primary: Rgb(0xff, 0x79, 0xc6),
                accent: Rgb(0x62, 0x72, 0xa4),
                alert: Rgb(0xff, 0xb8, 0x6c),
            },
            "light" => Palette {
                bg: Rgb(0xf8, 0xf8, 0xf2),
                primary: Rgb(0x62, 0x72, 0xa4),
                accent: Rgb(0x28, 0x2a, 0x36),
                alert: Rgb(0xff, 0x55, 0x55),
            },
            _ => Palette {
                bg: Rgb(0x1a, 0x1a, 0x2e),
                primary: Rgb(0xe9, 0x45, 0x60),
                accent: Rgb(0x0f, 0x34, 0x60),
                alert: Rgb(0xff, 0x6b, 0x6b),
            },
        }
    }
}

/// Write any configured exports and print a one-line summary. Called by both
/// the CLI and TUI front-ends on exit.
pub fn finalize(shared: &Shared, args: &Args) {
    let report = {
        let st = shared.read();
        SessionReport::from_state(&st)
    };

    if let Some(path) = &args.export_json {
        report_result("JSON", export_json::write(&report, std::path::Path::new(path)), path);
    }
    if let Some(path) = &args.export_csv {
        report_result("CSV", export_csv::write(&report, std::path::Path::new(path)), path);
    }
    if let Some(path) = &args.export_html {
        report_result("HTML", export_html::write(&report, std::path::Path::new(path)), path);
    }

    eprintln!(
        "\nSession summary: {} kicks · {} clients · {} handshakes · {} PMKIDs",
        report.summary.total_kicks,
        report.summary.unique_clients,
        report.summary.handshakes_captured,
        report.summary.pmkids_captured,
    );
}

fn report_result(kind: &str, res: anyhow::Result<()>, path: &str) {
    match res {
        Ok(()) => eprintln!("Exported {kind} → {path}"),
        Err(e) => eprintln!("Failed to export {kind} to {path}: {e}"),
    }
}
