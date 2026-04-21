use crate::frequency::FrequencyFn;
use crate::lattice::{self, LatticeKind};
use crate::phase::PhaseMode;
use crate::renderer::MAX_SPEC;
use crate::shape::WaveShape;
use crate::spectrum::SpectrumKind;

#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ColorMode {
    Real,
    Intensity,
    Domain,
    Spectral,
    Fft,
}

#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DecayMode {
    None,
    InvSqrtR,
    InvR,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SimState {
    /// Internal simulation render resolution (N×N). Higher = sharper.
    pub sim_resolution: u32,
    pub lattice_kind: LatticeKind,
    pub num_nodes: usize,
    pub freq_fn: FrequencyFn,
    pub base_k: f32,
    pub alpha: f32,
    pub beta: f32,
    pub spectrum_kind: SpectrumKind,
    pub spec_count: usize,
    pub spec_spread: f32,
    pub phase_mode: PhaseMode,
    pub phase_param_a: f32,
    pub phase_param_b: f32,
    pub wave_shape: WaveShape,
    pub shape_param_a: f32,
    pub shape_param_b: f32,
    pub wave_speed: f32,
    pub amp_scale: f32,
    pub color_mode: ColorMode,
    pub decay_mode: DecayMode,
    pub paused: bool,
    pub time: f32,
    /// Marks emitter buffer needs rebuild (lattice / freq / count changed).
    #[serde(skip, default = "default_true")]
    pub emitters_dirty: bool,
    /// Marks spectrum buffer needs rebuild.
    #[serde(skip, default = "default_true")]
    pub spectrum_dirty: bool,
    #[serde(skip)]
    pub preset_io: Option<PresetIo>,
}

pub enum PresetIo {
    Save(std::sync::mpsc::Receiver<Option<std::path::PathBuf>>),
    Load(std::sync::mpsc::Receiver<Option<std::path::PathBuf>>),
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            sim_resolution: 1024,
            lattice_kind: LatticeKind::Sunflower,
            num_nodes: 64,
            freq_fn: FrequencyFn::Constant,
            base_k: 0.20,
            alpha: 0.5,
            beta: 6.0,
            spectrum_kind: SpectrumKind::Single,
            spec_count: 4,
            spec_spread: 0.05,
            phase_mode: PhaseMode::Zero,
            phase_param_a: 0.0,
            phase_param_b: 0.0,
            wave_shape: WaveShape::Circular,
            shape_param_a: 0.0,
            shape_param_b: 0.0,
            wave_speed: 80.0,
            amp_scale: 0.10,
            color_mode: ColorMode::Real,
            decay_mode: DecayMode::InvSqrtR,
            paused: false,
            time: 0.0,
            emitters_dirty: true,
            spectrum_dirty: true,
            preset_io: None,
        }
    }
}

impl SimState {
    /// Per-emitter `[x, y, base_k, phase_seed]`.
    pub fn build_emitters(&self) -> Vec<[f32; 4]> {
        let size = self.sim_resolution as f32;
        let positions = lattice::generate(self.lattice_kind, self.num_nodes, size);
        let center = size * 0.5;
        let max_r = size * 0.5;
        positions
            .into_iter()
            .enumerate()
            .map(|(i, [x, y])| {
                let dx = x - center;
                let dy = y - center;
                let r = (dx * dx + dy * dy).sqrt();
                let r_norm = (r / max_r).min(1.0);
                let k = self
                    .freq_fn
                    .eval(r_norm, self.base_k, self.alpha, self.beta);
                [x, y, k, node_phase_seed(i as u32)]
            })
            .collect()
    }

    pub fn build_spectrum(&self) -> Vec<[f32; 4]> {
        self.spectrum_kind
            .build(self.spec_count, MAX_SPEC as usize, self.spec_spread)
    }

    pub fn color_mode_u32(&self) -> u32 {
        match self.color_mode {
            ColorMode::Real => 0,
            ColorMode::Intensity => 1,
            ColorMode::Domain => 2,
            ColorMode::Spectral => 3,
            // FFT mode feeds the wave shader with the real-field path;
            // post-processing handles the transform + colouring.
            ColorMode::Fft => 0,
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

fn default_true() -> bool { true }

impl SimState {
    pub fn save_to_path(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    pub fn load_from_path(path: &std::path::Path) -> std::io::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

fn node_phase_seed(i: u32) -> f32 {
    // xorshift-mix → uniform in [0, 2π).
    let mut s = i.wrapping_mul(2_654_435_761).wrapping_add(0xdead_beef);
    s ^= s >> 13;
    s = s.wrapping_mul(0x85eb_ca6b);
    s ^= s >> 16;
    (s as f32 / u32::MAX as f32) * std::f32::consts::TAU
}
