//! GitHub release update checker (,).
//!
//! A single blocking GET to the GitHub releases API; the caller decides whether
//! to run it on a background thread (startup check) or synchronously
//! (`--check-update`).

use std::time::Duration;

use serde::Deserialize;

use super::info::{should_offer, strip_v, UpdateInfo};

const RELEASES_API: &str = "https://api.github.com/repos/Londopy/kto/releases/latest";
const CHANGELOG_RAW: &str = "https://raw.githubusercontent.com/Londopy/kto/main/CHANGELOG.md";
const USER_AGENT: &str = concat!("kto/", env!("CARGO_PKG_VERSION"));

/// The current crate version (compile-time).
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    html_url: String,
}

/// Perform the release check. Returns `Ok(Some(info))` if an update should be
/// offered (newer than current and not skipped), `Ok(None)` if up to date.
pub fn check(current: &str, skip_version: &str) -> anyhow::Result<Option<UpdateInfo>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(8))
        .build()?;

    let release: Release = client
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()?
        .error_for_status()?
        .json()?;

    let latest = strip_v(&release.tag_name).to_string();
    if !should_offer(current, &latest, skip_version) {
        return Ok(None);
    }

    let url = if release.html_url.is_empty() {
        "https://github.com/Londopy/kto/releases".to_string()
    } else {
        release.html_url
    };

    let changelog_snippet = fetch_changelog_snippet(&client).ok().flatten();

    Ok(Some(UpdateInfo { version: latest, url, changelog_snippet }))
}

/// Fetch CHANGELOG.md and return the top section (up to the second `## `
/// heading) - used for the changelog preview popup.
fn fetch_changelog_snippet(client: &reqwest::blocking::Client) -> anyhow::Result<Option<String>> {
    let body = client.get(CHANGELOG_RAW).send()?.error_for_status()?.text()?;
    Ok(top_section(&body))
}

/// Extract everything from the first `## ` heading up to (but not including)
/// the second `## ` heading.
pub fn top_section(markdown: &str) -> Option<String> {
    let mut lines = markdown.lines();
    let mut out = String::new();
    let mut started = false;
    for line in lines.by_ref() {
        if line.starts_with("## ") {
            if started {
                break; // second section heading
            }
            started = true;
        }
        if started {
            out.push_str(line);
            out.push('\n');
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out.trim_end().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_top_section() {
        let md = "# Title\n\n## 3.0.0\n- a\n- b\n\n## 2.0.0\n- old\n";
        let top = top_section(md).unwrap();
        assert!(top.contains("3.0.0"));
        assert!(top.contains("- a"));
        assert!(!top.contains("2.0.0"));
    }

    #[test]
    fn none_when_no_section() {
        assert!(top_section("just text, no headings").is_none());
    }
}
