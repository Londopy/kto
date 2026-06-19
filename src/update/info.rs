//! Update metadata and version-skip logic.

use semver::Version;

/// Information about an available update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// The new version, e.g. `"3.1.0"`.
    pub version: String,
    /// Release page URL.
    pub url: String,
    /// First section of the changelog, if fetched.
    pub changelog_snippet: Option<String>,
}

impl UpdateInfo {
    /// Bottom-bar / CLI one-liner.
    pub fn short_line(&self) -> String {
        format!("Update available: v{}  → {}", self.version, self.url)
    }
}

/// Decide whether `latest` should be offered given the running `current`
/// version and a possibly-empty `skip` version from config.
///
/// Returns `true` only when `latest > current` and `latest != skip`.
pub fn should_offer(current: &str, latest: &str, skip: &str) -> bool {
    let (Ok(cur), Ok(new)) = (Version::parse(current), Version::parse(latest)) else {
        return false;
    };
    if new <= cur {
        return false;
    }
    if let Ok(skip_v) = Version::parse(skip) {
        if new == skip_v {
            return false;
        }
    }
    true
}

/// Normalize a git tag like `v3.1.0` to a semver string `3.1.0`.
pub fn strip_v(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_newer() {
        assert!(should_offer("3.0.0", "3.1.0", ""));
    }

    #[test]
    fn ignores_equal_or_older() {
        assert!(!should_offer("3.0.0", "3.0.0", ""));
        assert!(!should_offer("3.2.0", "3.1.0", ""));
    }

    #[test]
    fn respects_skip() {
        assert!(!should_offer("3.0.0", "3.1.0", "3.1.0"));
        assert!(should_offer("3.0.0", "3.2.0", "3.1.0"));
    }

    #[test]
    fn strips_v_prefix() {
        assert_eq!(strip_v("v3.1.0"), "3.1.0");
        assert_eq!(strip_v("3.1.0"), "3.1.0");
    }
}
