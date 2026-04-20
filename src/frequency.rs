use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrequencyFn {
    Constant,
    Linear,
    Inverse,
    Quadratic,
    Cosine,
    Gaussian,
    Step,
    Sigmoid,
    Sawtooth,
    ExpDecay,
    Log,
    Sine,
    Power,
    Tanh,
    MexicanHat,
    WobblyHat,
    Morlet,
    DampedSine,
    Chirp,
    Fresnel,
    BouncingSine,
    Triangle,
    Heartbeat,
    FractalSines,
}

impl FrequencyFn {
    pub const ALL: &'static [FrequencyFn] = &[
        FrequencyFn::Constant,
        FrequencyFn::Linear,
        FrequencyFn::Inverse,
        FrequencyFn::Quadratic,
        FrequencyFn::Cosine,
        FrequencyFn::Gaussian,
        FrequencyFn::Step,
        FrequencyFn::Sigmoid,
        FrequencyFn::Sawtooth,
        FrequencyFn::ExpDecay,
        FrequencyFn::Log,
        FrequencyFn::Sine,
        FrequencyFn::Power,
        FrequencyFn::Tanh,
        FrequencyFn::MexicanHat,
        FrequencyFn::WobblyHat,
        FrequencyFn::Morlet,
        FrequencyFn::DampedSine,
        FrequencyFn::Chirp,
        FrequencyFn::Fresnel,
        FrequencyFn::BouncingSine,
        FrequencyFn::Triangle,
        FrequencyFn::Heartbeat,
        FrequencyFn::FractalSines,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Constant => "Constant",
            Self::Linear => "Linear",
            Self::Inverse => "Inverse",
            Self::Quadratic => "Quadratic",
            Self::Cosine => "Cosine",
            Self::Gaussian => "Gaussian",
            Self::Step => "Step",
            Self::Sigmoid => "Sigmoid",
            Self::Sawtooth => "Sawtooth",
            Self::ExpDecay => "Exp. decay",
            Self::Log => "Log",
            Self::Sine => "Sine",
            Self::Power => "Power",
            Self::Tanh => "Tanh",
            Self::MexicanHat => "Mexican hat",
            Self::WobblyHat => "Wobbly hat",
            Self::Morlet => "Morlet",
            Self::DampedSine => "Damped sine",
            Self::Chirp => "Chirp",
            Self::Fresnel => "Fresnel",
            Self::BouncingSine => "Bouncing sine",
            Self::Triangle => "Triangle",
            Self::Heartbeat => "Heartbeat",
            Self::FractalSines => "Fractal sines",
        }
    }

    pub fn formula(&self) -> &'static str {
        match self {
            Self::Constant => "k₀",
            Self::Linear => "k₀·(1 + α·r)",
            Self::Inverse => "k₀ / (1 + α·r)",
            Self::Quadratic => "k₀·(1 + α·r²)",
            Self::Cosine => "k₀·(1 + α·cos β·r)",
            Self::Gaussian => "k₀·(1 + α·e^(−β·r²))",
            Self::Step => "k₀ if r<½ else (1+α)·k₀",
            Self::Sigmoid => "k₀·(1 + α·σ(β(r−½)))",
            Self::Sawtooth => "k₀·(1 + α·frac(β·r))",
            Self::ExpDecay => "k₀·(1 + α·e^(−β·r))",
            Self::Log => "k₀·(1 + α·ln(1+β·r))",
            Self::Sine => "k₀·(1 + α·sin(β·r))",
            Self::Power => "k₀·(1 + α·r^β)",
            Self::Tanh => "k₀·(1 + α·tanh(β(r−½)))",
            Self::MexicanHat => "k₀·(1 + α·(1−(β·r)²)·e^(−(β·r)²/2))",
            Self::WobblyHat => "k₀·(1 + α·(1−(β·r)²)·e^(−(β·r)²/2)·cos(3β·r))",
            Self::Morlet => "k₀·(1 + α·cos(β·r)·e^(−r²))",
            Self::DampedSine => "k₀·(1 + α·sin(β·r)·e^(−r))",
            Self::Chirp => "k₀·(1 + α·sin(β·r²))",
            Self::Fresnel => "k₀·(1 + α·cos(β·r²))",
            Self::BouncingSine => "k₀·(1 + α·|sin(β·r)|)",
            Self::Triangle => "k₀·(1 + α·(2/π)·asin(sin(β·r)))",
            Self::Heartbeat => "k₀·(1 + α·(e^(−β(r−⅓)²) − ½·e^(−β(r−⅔)²)))",
            Self::FractalSines => "k₀·(1 + α·Σₙ₌₀³ sin(2ⁿβ·r)/2ⁿ)",
        }
    }

    pub fn uses_alpha(&self) -> bool {
        !matches!(self, Self::Constant)
    }

    pub fn uses_beta(&self) -> bool {
        matches!(
            self,
            Self::Cosine
                | Self::Gaussian
                | Self::Sigmoid
                | Self::Sawtooth
                | Self::ExpDecay
                | Self::Log
                | Self::Sine
                | Self::Power
                | Self::Tanh
                | Self::MexicanHat
                | Self::WobblyHat
                | Self::Morlet
                | Self::DampedSine
                | Self::Chirp
                | Self::Fresnel
                | Self::BouncingSine
                | Self::Triangle
                | Self::Heartbeat
                | Self::FractalSines
        )
    }

    pub fn eval(&self, r_norm: f32, base_k: f32, alpha: f32, beta: f32) -> f32 {
        let r = r_norm.max(0.0);
        let k = match self {
            Self::Constant => base_k,
            Self::Linear => base_k * (1.0 + alpha * r),
            Self::Inverse => base_k / (1.0 + alpha * r).max(0.01),
            Self::Quadratic => base_k * (1.0 + alpha * r * r),
            Self::Cosine => base_k * (1.0 + alpha * (beta * r).cos()),
            Self::Gaussian => base_k * (1.0 + alpha * (-beta * r * r).exp()),
            Self::Step => {
                if r < 0.5 {
                    base_k
                } else {
                    base_k * (1.0 + alpha)
                }
            }
            Self::Sigmoid => {
                let s = 1.0 / (1.0 + (-beta * (r - 0.5)).exp());
                base_k * (1.0 + alpha * s)
            }
            Self::Sawtooth => base_k * (1.0 + alpha * (beta * r - (beta * r).floor())),
            Self::ExpDecay => base_k * (1.0 + alpha * (-beta * r).exp()),
            Self::Log => base_k * (1.0 + alpha * (1.0 + beta * r).ln()),
            Self::Sine => base_k * (1.0 + alpha * (beta * r).sin()),
            Self::Power => base_k * (1.0 + alpha * r.powf(beta.max(0.001))),
            Self::Tanh => base_k * (1.0 + alpha * (beta * (r - 0.5)).tanh()),
            Self::MexicanHat => {
                let x = beta * r;
                let x2 = x * x;
                base_k * (1.0 + alpha * (1.0 - x2) * (-0.5 * x2).exp())
            }
            Self::WobblyHat => {
                let x = beta * r;
                let x2 = x * x;
                let hat = (1.0 - x2) * (-0.5 * x2).exp();
                let wobble = (3.0 * x).cos();
                base_k * (1.0 + alpha * hat * wobble)
            }
            Self::Morlet => base_k * (1.0 + alpha * (beta * r).cos() * (-r * r).exp()),
            Self::DampedSine => base_k * (1.0 + alpha * (beta * r).sin() * (-r).exp()),
            Self::Chirp => base_k * (1.0 + alpha * (beta * r * r).sin()),
            Self::Fresnel => base_k * (1.0 + alpha * (beta * r * r).cos()),
            Self::BouncingSine => base_k * (1.0 + alpha * (beta * r).sin().abs()),
            Self::Triangle => {
                let t = (2.0 / std::f32::consts::PI) * (beta * r).sin().asin();
                base_k * (1.0 + alpha * t)
            }
            Self::Heartbeat => {
                let d1 = r - 1.0 / 3.0;
                let d2 = r - 2.0 / 3.0;
                let pulse = (-beta * d1 * d1).exp() - 0.5 * (-beta * d2 * d2).exp();
                base_k * (1.0 + alpha * pulse)
            }
            Self::FractalSines => {
                let mut sum = 0.0f32;
                let mut amp = 1.0f32;
                let mut freq = beta;
                let mut norm = 0.0f32;
                for _ in 0..4 {
                    sum += amp * (freq * r).sin();
                    norm += amp;
                    amp *= 0.5;
                    freq *= 2.0;
                }
                base_k * (1.0 + alpha * sum / norm.max(0.0001))
            }
        };
        k.max(0.0001)
    }
}

impl fmt::Display for FrequencyFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
