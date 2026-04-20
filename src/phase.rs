use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PhaseMode {
    Zero,
    Random,
    Focus,
    Vortex,
    Gradient,
    Chirp,
    Antiphase,
    Spiral,
    Hyperbolic,
    Bands,
    Checker,
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
        Self::Spiral,
        Self::Hyperbolic,
        Self::Bands,
        Self::Checker,
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
            Self::Spiral => "Spiral",
            Self::Hyperbolic => "Hyperbolic",
            Self::Bands => "Bands",
            Self::Checker => "Checker",
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
            Self::Spiral => "φ = m·θ − k·r (focusing vortex)",
            Self::Hyperbolic => "φ = α·(x²−y²) (saddle)",
            Self::Bands => "φ = π·⌊β·r⌋ (radial ±1)",
            Self::Checker => "φ = π·(⌊β·x⌋+⌊β·y⌋) mod 2",
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
            Self::Spiral => 7,
            Self::Hyperbolic => 8,
            Self::Bands => 9,
            Self::Checker => 10,
        }
    }

    pub fn uses_param_a(&self) -> bool {
        matches!(
            self,
            Self::Vortex
                | Self::Gradient
                | Self::Chirp
                | Self::Spiral
                | Self::Hyperbolic
                | Self::Bands
                | Self::Checker
        )
    }

    pub fn param_a_label(&self) -> &'static str {
        match self {
            Self::Vortex => "m (charge)",
            Self::Gradient => "θ (rad)",
            Self::Chirp => "α (rad/s)",
            Self::Spiral => "m (charge)",
            Self::Hyperbolic => "α",
            Self::Bands => "β (rings)",
            Self::Checker => "β (cells)",
            _ => "",
        }
    }

    pub fn param_a_range(&self) -> (f32, f32) {
        match self {
            Self::Vortex => (-6.0, 6.0),
            Self::Gradient => (0.0, std::f32::consts::TAU),
            Self::Chirp => (-10.0, 10.0),
            Self::Spiral => (-6.0, 6.0),
            Self::Hyperbolic => (-8.0, 8.0),
            Self::Bands => (1.0, 24.0),
            Self::Checker => (1.0, 24.0),
            _ => (0.0, 1.0),
        }
    }

    pub fn default_param_a(&self) -> f32 {
        match self {
            Self::Vortex => 1.0,
            Self::Gradient => 0.0,
            Self::Chirp => 2.0,
            Self::Spiral => 1.0,
            Self::Hyperbolic => 2.0,
            Self::Bands => 6.0,
            Self::Checker => 4.0,
            _ => 0.0,
        }
    }
}

impl fmt::Display for PhaseMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
