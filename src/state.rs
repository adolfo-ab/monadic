use crate::frequency::FrequencyFn;
use crate::lattice::{self, LatticeKind};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ColorMode {
    Real,
    Intensity,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DecayMode {
    None,
    InvSqrtR,
    InvR,
}

pub struct SimState {
    pub canvas_size: f32,
    /// Requested canvas dimension in physical pixels. The window is resized
    /// so the wave canvas matches this on each axis.
    pub requested_canvas_px: u32,
    pub lattice_kind: LatticeKind,
    pub num_nodes: usize,
    pub freq_fn: FrequencyFn,
    pub base_k: f32,
    pub alpha: f32,
    pub beta: f32,
    pub wave_speed: f32,
    pub amp_scale: f32,
    pub color_mode: ColorMode,
    pub decay_mode: DecayMode,
    pub paused: bool,
    pub time: f32,
    pub dirty: bool,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            canvas_size: 1024.0,
            requested_canvas_px: 1024,
            lattice_kind: LatticeKind::Sunflower,
            num_nodes: 64,
            freq_fn: FrequencyFn::Constant,
            base_k: 0.20,
            alpha: 0.5,
            beta: 6.0,
            wave_speed: 80.0,
            amp_scale: 0.10,
            color_mode: ColorMode::Real,
            decay_mode: DecayMode::InvSqrtR,
            paused: false,
            time: 0.0,
            dirty: true,
        }
    }
}

impl SimState {
    /// Build the emitter list (positions + per-emitter wavenumber).
    /// Layout per emitter: [x, y, k, phase].
    pub fn build_emitters(&self) -> Vec<[f32; 4]> {
        let positions = lattice::generate(self.lattice_kind, self.num_nodes, self.canvas_size);
        let center = self.canvas_size * 0.5;
        let max_r = self.canvas_size * 0.5;
        positions
            .into_iter()
            .map(|[x, y]| {
                let dx = x - center;
                let dy = y - center;
                let r = (dx * dx + dy * dy).sqrt();
                let r_norm = (r / max_r).min(1.0);
                let k = self
                    .freq_fn
                    .eval(r_norm, self.base_k, self.alpha, self.beta);
                [x, y, k, 0.0]
            })
            .collect()
    }

    pub fn color_mode_u32(&self) -> u32 {
        match self.color_mode {
            ColorMode::Real => 0,
            ColorMode::Intensity => 1,
        }
    }

    pub fn decay_mode_u32(&self) -> u32 {
        match self.decay_mode {
            DecayMode::None => 0,
            DecayMode::InvSqrtR => 1,
            DecayMode::InvR => 2,
        }
    }
}
