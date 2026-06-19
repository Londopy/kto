//! RSSI helpers: signal-strength bars and tier classification.

/// Signal quality tier, used for color coding across UIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalTier {
    /// `> -60 dBm` - strong (green).
    Strong,
    /// `-60..=-75 dBm` - moderate (yellow).
    Moderate,
    /// `< -75 dBm` - weak (red).
    Weak,
}

impl SignalTier {
    /// Classify an RSSI value in dBm.
    pub fn from_rssi(rssi: i8) -> SignalTier {
        if rssi > -60 {
            SignalTier::Strong
        } else if rssi >= -75 {
            SignalTier::Moderate
        } else {
            SignalTier::Weak
        }
    }

    /// An ANSI-ish color name for the CLI printer / theme lookups.
    pub fn color_name(&self) -> &'static str {
        match self {
            SignalTier::Strong => "green",
            SignalTier::Moderate => "yellow",
            SignalTier::Weak => "red",
        }
    }
}

/// Map an RSSI to 0..=8 filled tiers. -90 dBm or worse -> 0, -30 dBm or better -> 8.
pub fn rssi_to_level(rssi: i8) -> u8 {
    let clamped = rssi.clamp(-90, -30) as i32;
    // -90 -> 0.0, -30 -> 1.0
    let frac = (clamped + 90) as f32 / 60.0;
    (frac * 8.0).round() as u8
}

/// An 8-cell Unicode block bar like `████████░░` (width fixed at 8 cells but
/// rendered as full/empty blocks).
pub fn bar(rssi: i8) -> String {
    let level = rssi_to_level(rssi) as usize;
    let filled = "█".repeat(level);
    let empty = "░".repeat(8 - level);
    format!("{filled}{empty}")
}

/// A tiny sparkline cell for a single RSSI sample (used in signal-history rows).
pub fn spark_cell(rssi: i8) -> char {
    const CELLS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let level = rssi_to_level(rssi).clamp(1, 8) as usize - 1;
    CELLS[level]
}

/// Render a slice of RSSI samples as a sparkline string.
pub fn sparkline(samples: &[i8]) -> String {
    samples.iter().map(|&r| spark_cell(r)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers() {
        assert_eq!(SignalTier::from_rssi(-50), SignalTier::Strong);
        assert_eq!(SignalTier::from_rssi(-70), SignalTier::Moderate);
        assert_eq!(SignalTier::from_rssi(-80), SignalTier::Weak);
    }

    #[test]
    fn bar_bounds() {
        assert_eq!(bar(-30).chars().filter(|&c| c == '█').count(), 8);
        assert_eq!(bar(-95).chars().filter(|&c| c == '█').count(), 0);
        assert_eq!(bar(-50).chars().count(), 8);
    }

    #[test]
    fn sparkline_len() {
        assert_eq!(sparkline(&[-60, -70, -50]).chars().count(), 3);
    }
}
