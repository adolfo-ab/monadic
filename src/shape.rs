use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum WaveShape {
    Circular,
    Petal,
    Wobbly,
    Elliptical,
    Diamond,
    Square,
    Plane,
    Spiral,
    Breathing,
}

impl WaveShape {
    pub const ALL: &'static [WaveShape] = &[
        Self::Circular,
        Self::Petal,
        Self::Wobbly,
        Self::Elliptical,
        Self::Diamond,
        Self::Square,
        Self::Plane,
        Self::Spiral,
        Self::Breathing,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Circular => "Circular",
            Self::Petal => "Petal",
            Self::Wobbly => "Wobbly",
            Self::Elliptical => "Elliptical",
            Self::Diamond => "Diamond",
            Self::Square => "Square",
            Self::Plane => "Plane",
            Self::Spiral => "Spiral",
            Self::Breathing => "Breathing",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Circular => "k·d − ω·t",
            Self::Petal => "k·d·(1+a·cos(n·φ))",
            Self::Wobbly => "k·(d+a·sin(n·φ+ω_w·t))",
            Self::Elliptical => "anisotropic metric",
            Self::Diamond => "L1 metric |x|+|y|",
            Self::Square => "L∞ metric max(|x|,|y|)",
            Self::Plane => "k·(x·cos α + y·sin α)",
            Self::Spiral => "k·d + m·φ",
            Self::Breathing => "k·d + a·sin(b·t)",
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            Self::Circular => 0,
            Self::Petal => 1,
            Self::Wobbly => 2,
            Self::Elliptical => 3,
            Self::Diamond => 4,
            Self::Square => 5,
            Self::Plane => 6,
            Self::Spiral => 7,
            Self::Breathing => 8,
        }
    }

    pub fn uses_param_a(&self) -> bool {
        !matches!(self, Self::Circular | Self::Diamond | Self::Square)
    }

    pub fn uses_param_b(&self) -> bool {
        matches!(
            self,
            Self::Petal | Self::Wobbly | Self::Elliptical | Self::Breathing
        )
    }

    pub fn param_a_label(&self) -> &'static str {
        match self {
            Self::Petal => "a (depth)",
            Self::Wobbly => "a (depth)",
            Self::Elliptical => "e (ecc.)",
            Self::Plane => "α (rad)",
            Self::Spiral => "m (charge)",
            Self::Breathing => "a (depth)",
            _ => "",
        }
    }

    pub fn param_b_label(&self) -> &'static str {
        match self {
            Self::Petal => "n (lobes)",
            Self::Wobbly => "n (lobes)",
            Self::Elliptical => "θ (rot)",
            Self::Breathing => "ω_b (rad/s)",
            _ => "",
        }
    }

    pub fn param_a_range(&self) -> (f32, f32) {
        match self {
            Self::Petal => (-0.8, 0.8),
            Self::Wobbly => (0.0, 60.0),
            Self::Elliptical => (0.0, 0.95),
            Self::Plane => (0.0, std::f32::consts::TAU),
            Self::Spiral => (-6.0, 6.0),
            Self::Breathing => (0.0, 6.0),
            _ => (0.0, 1.0),
        }
    }

    pub fn param_b_range(&self) -> (f32, f32) {
        match self {
            Self::Petal => (1.0, 12.0),
            Self::Wobbly => (1.0, 12.0),
            Self::Elliptical => (0.0, std::f32::consts::PI),
            Self::Breathing => (0.1, 20.0),
            _ => (0.0, 1.0),
        }
    }

    pub fn default_param_a(&self) -> f32 {
        match self {
            Self::Petal => 0.3,
            Self::Wobbly => 12.0,
            Self::Elliptical => 0.5,
            Self::Plane => 0.0,
            Self::Spiral => 1.0,
            Self::Breathing => 2.0,
            _ => 0.0,
        }
    }

    pub fn default_param_b(&self) -> f32 {
        match self {
            Self::Petal => 4.0,
            Self::Wobbly => 5.0,
            Self::Elliptical => 0.0,
            Self::Breathing => 3.0,
            _ => 0.0,
        }
    }
}

impl fmt::Display for WaveShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
