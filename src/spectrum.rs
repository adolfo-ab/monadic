use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SpectrumKind {
    Single,
    HarmonicFlat,
    HarmonicSaw,
    HarmonicTriangle,
    HarmonicSquare,
    Octaves,
    Beat,
    Comb,
}

impl SpectrumKind {
    pub const ALL: &'static [SpectrumKind] = &[
        Self::Single,
        Self::HarmonicFlat,
        Self::HarmonicSaw,
        Self::HarmonicTriangle,
        Self::HarmonicSquare,
        Self::Octaves,
        Self::Beat,
        Self::Comb,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Single => "Single",
            Self::HarmonicFlat => "Harmonics flat",
            Self::HarmonicSaw => "Sawtooth",
            Self::HarmonicTriangle => "Triangle",
            Self::HarmonicSquare => "Square",
            Self::Octaves => "Octaves",
            Self::Beat => "Beat pair",
            Self::Comb => "Comb",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Single => "one frequency",
            Self::HarmonicFlat => "k₀, 2k₀, 3k₀ … (flat)",
            Self::HarmonicSaw => "j·k₀, amp 1/j",
            Self::HarmonicTriangle => "odd j, amp 1/j²",
            Self::HarmonicSquare => "odd j, amp 1/j",
            Self::Octaves => "k₀, 2k₀, 4k₀ …",
            Self::Beat => "two close frequencies",
            Self::Comb => "k₀ + j·Δk evenly",
        }
    }

    /// Generate spectrum components: each entry is `[k_mult, amp, phase_off, _pad]`.
    /// `count` clamped to `[1, max]`. `spread` interpreted per kind.
    pub fn build(&self, count: usize, max: usize, spread: f32) -> Vec<[f32; 4]> {
        let m = count.clamp(1, max);
        let s = spread.max(0.005);
        match self {
            Self::Single => vec![[1.0, 1.0, 0.0, 0.0]],
            Self::HarmonicFlat => (1..=m).map(|j| [j as f32, 1.0, 0.0, 0.0]).collect(),
            Self::HarmonicSaw => (1..=m)
                .map(|j| [j as f32, 1.0 / j as f32, 0.0, 0.0])
                .collect(),
            Self::HarmonicTriangle => (0..m)
                .map(|i| {
                    let j = (2 * i + 1) as f32;
                    [j, 1.0 / (j * j), 0.0, 0.0]
                })
                .collect(),
            Self::HarmonicSquare => (0..m)
                .map(|i| {
                    let j = (2 * i + 1) as f32;
                    [j, 1.0 / j, 0.0, 0.0]
                })
                .collect(),
            Self::Octaves => (0..m)
                .map(|i| [(1u32 << i.min(15)) as f32, 1.0, 0.0, 0.0])
                .collect(),
            Self::Beat => vec![[1.0, 1.0, 0.0, 0.0], [1.0 + s, 1.0, 0.0, 0.0]],
            Self::Comb => (0..m)
                .map(|i| [1.0 + i as f32 * s, 1.0, 0.0, 0.0])
                .collect(),
        }
    }

    pub fn uses_count(&self) -> bool {
        !matches!(self, Self::Single | Self::Beat)
    }

    pub fn uses_spread(&self) -> bool {
        matches!(self, Self::Beat | Self::Comb)
    }
}

impl fmt::Display for SpectrumKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
