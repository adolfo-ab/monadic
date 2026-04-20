use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq)]
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
        }
    }

    pub fn uses_alpha(&self) -> bool {
        !matches!(self, Self::Constant)
    }

    pub fn uses_beta(&self) -> bool {
        matches!(
            self,
            Self::Cosine | Self::Gaussian | Self::Sigmoid | Self::Sawtooth | Self::ExpDecay
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
        };
        k.max(0.0001)
    }
}

impl fmt::Display for FrequencyFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
