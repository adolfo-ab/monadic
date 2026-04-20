use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PhaseMode {
    Zero,
    Random,
    Focus,
    Vortex,
    Gradient,
    Chirp,
    Antiphase,
}

impl PhaseMode {
    pub const ALL: &'static [PhaseMode] = &[
        Self::Zero,
        Self::Random,
        Self::Focus,
        Self::Vortex,
        Self::Gradient,
        Self::Chirp,
        Self::Antiphase,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Zero => "Zero",
            Self::Random => "Random",
            Self::Focus => "Focus",
            Self::Vortex => "Vortex",
            Self::Gradient => "Gradient",
            Self::Chirp => "Chirp",
            Self::Antiphase => "Antiphase",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Zero => "all in phase",
            Self::Random => "decorrelated speckle",
            Self::Focus => "φ = −k·r (focal spot at center)",
            Self::Vortex => "φ = m·θ (topological charge)",
            Self::Gradient => "φ = k·(p·n̂) (tilted plane wave)",
            Self::Chirp => "φ = α·t (frequency sweep)",
            Self::Antiphase => "alternate ±π every other emitter",
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            Self::Zero => 0,
            Self::Random => 1,
            Self::Focus => 2,
            Self::Vortex => 3,
            Self::Gradient => 4,
            Self::Chirp => 5,
            Self::Antiphase => 6,
        }
    }

    pub fn uses_param_a(&self) -> bool {
        matches!(self, Self::Vortex | Self::Gradient | Self::Chirp)
    }

    pub fn param_a_label(&self) -> &'static str {
        match self {
            Self::Vortex => "m (charge)",
            Self::Gradient => "θ (rad)",
            Self::Chirp => "α (rad/s)",
            _ => "",
        }
    }

    pub fn param_a_range(&self) -> (f32, f32) {
        match self {
            Self::Vortex => (-6.0, 6.0),
            Self::Gradient => (0.0, std::f32::consts::TAU),
            Self::Chirp => (-10.0, 10.0),
            _ => (0.0, 1.0),
        }
    }

    pub fn default_param_a(&self) -> f32 {
        match self {
            Self::Vortex => 1.0,
            Self::Gradient => 0.0,
            Self::Chirp => 2.0,
            _ => 0.0,
        }
    }
}

impl fmt::Display for PhaseMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
