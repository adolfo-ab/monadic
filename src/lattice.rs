use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LatticeKind {
    Cartesian,
    Triangular,
    Sunflower,
    LogSpiral,
    Rings,
    Polygon,
    Cross,
    Random,
    Halton,
}

impl LatticeKind {
    pub const ALL: &'static [LatticeKind] = &[
        LatticeKind::Cartesian,
        LatticeKind::Triangular,
        LatticeKind::Sunflower,
        LatticeKind::LogSpiral,
        LatticeKind::Rings,
        LatticeKind::Polygon,
        LatticeKind::Cross,
        LatticeKind::Random,
        LatticeKind::Halton,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Cartesian => "Cartesian",
            Self::Triangular => "Triangular",
            Self::Sunflower => "Sunflower",
            Self::LogSpiral => "Log spiral",
            Self::Rings => "Rings",
            Self::Polygon => "Polygon",
            Self::Cross => "Cross",
            Self::Random => "Random",
            Self::Halton => "Halton",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Cartesian => "square grid",
            Self::Triangular => "offset hexagonal packing",
            Self::Sunflower => "Vogel phyllotaxis",
            Self::LogSpiral => "Bernoulli spiral",
            Self::Rings => "concentric circles",
            Self::Polygon => "regular polygon ring",
            Self::Cross => "axial cross",
            Self::Random => "uniform random disc",
            Self::Halton => "quasi-random (2,3)",
        }
    }
}

impl fmt::Display for LatticeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Generate `n` emitter positions inside a square canvas of `canvas_size` units.
/// Output coords lie in `[0, canvas_size]`. With `canvas_size = 1.0` the result
/// is normalized for thumbnails.
pub fn generate(kind: LatticeKind, n: usize, canvas_size: f32) -> Vec<[f32; 2]> {
    let n = n.max(1);
    let center = canvas_size * 0.5;
    let radius = canvas_size * 0.48;

    match kind {
        LatticeKind::Cartesian => cartesian(n, center, radius),
        LatticeKind::Triangular => triangular(n, center, radius),
        LatticeKind::Sunflower => sunflower(n, center, radius),
        LatticeKind::LogSpiral => log_spiral(n, center, radius),
        LatticeKind::Rings => rings(n, center, radius),
        LatticeKind::Polygon => polygon(n, center, radius),
        LatticeKind::Cross => cross(n, center, radius),
        LatticeKind::Random => pseudo_random(n, center, radius),
        LatticeKind::Halton => halton(n, center, radius),
    }
}

fn cartesian(n: usize, center: f32, radius: f32) -> Vec<[f32; 2]> {
    let side = (n as f32).sqrt().ceil() as usize;
    let step = (2.0 * radius) / side as f32;
    let origin = center - radius + step * 0.5;
    let mut pts = Vec::with_capacity(side * side);
    for j in 0..side {
        for i in 0..side {
            pts.push([origin + i as f32 * step, origin + j as f32 * step]);
        }
    }
    sort_central(&mut pts, center);
    pts.truncate(n);
    pts
}

fn triangular(n: usize, center: f32, radius: f32) -> Vec<[f32; 2]> {
    let area = std::f32::consts::PI * radius * radius;
    let s = (area / (n as f32 * 0.866_025_4)).sqrt();
    let dx = s;
    let dy = s * 0.866_025_4;
    let cols = ((2.0 * radius) / dx).ceil() as i32 + 2;
    let rows = ((2.0 * radius) / dy).ceil() as i32 + 2;
    let mut pts = Vec::new();
    for j in -rows..=rows {
        for i in -cols..=cols {
            let offset = if j & 1 == 0 { 0.0 } else { dx * 0.5 };
            let x = center + i as f32 * dx + offset;
            let y = center + j as f32 * dy;
            let dxc = x - center;
            let dyc = y - center;
            if dxc * dxc + dyc * dyc <= radius * radius {
                pts.push([x, y]);
            }
        }
    }
    sort_central(&mut pts, center);
    pts.truncate(n);
    pts
}

fn sunflower(n: usize, center: f32, radius: f32) -> Vec<[f32; 2]> {
    let golden = std::f32::consts::PI * (3.0 - (5.0_f32).sqrt());
    let denom = (n as f32 - 1.0).max(1.0);
    (0..n)
        .map(|i| {
            let r = radius * ((i as f32) / denom).sqrt();
            let theta = i as f32 * golden;
            [center + r * theta.cos(), center + r * theta.sin()]
        })
        .collect()
}

fn log_spiral(n: usize, center: f32, radius: f32) -> Vec<[f32; 2]> {
    // r(theta) = a * exp(b * theta) parameterized so r(theta_max) = radius.
    // Plot two-and-a-half turns.
    let turns = 2.5_f32;
    let theta_max = turns * std::f32::consts::TAU;
    let r0 = radius * 0.02;
    let b = (radius / r0).ln() / theta_max;
    (0..n)
        .map(|i| {
            let t = (i as f32) / ((n as f32 - 1.0).max(1.0));
            let theta = t * theta_max;
            let r = r0 * (b * theta).exp();
            [center + r * theta.cos(), center + r * theta.sin()]
        })
        .collect()
}

fn rings(n: usize, center: f32, radius: f32) -> Vec<[f32; 2]> {
    let mut pts = Vec::with_capacity(n);
    if n == 0 {
        return pts;
    }
    pts.push([center, center]);
    let remaining = n - 1;
    if remaining == 0 {
        return pts;
    }
    let base = 6.0_f32;
    let r_rings = ((-1.0 + (1.0 + 8.0 * remaining as f32 / base).sqrt()) / 2.0)
        .ceil()
        .max(1.0) as usize;
    let dr = radius / r_rings as f32;
    let mut placed = 0usize;
    for ring in 1..=r_rings {
        let r = ring as f32 * dr;
        let count = ((base * ring as f32) as usize).min(remaining - placed);
        if count == 0 {
            break;
        }
        let offset = (ring as f32) * 0.5;
        for i in 0..count {
            let theta = offset + (i as f32) * std::f32::consts::TAU / count as f32;
            pts.push([center + r * theta.cos(), center + r * theta.sin()]);
        }
        placed += count;
        if placed >= remaining {
            break;
        }
    }
    while pts.len() < n {
        let i = pts.len();
        let golden = std::f32::consts::PI * (3.0 - (5.0_f32).sqrt());
        let r = radius * ((i as f32) / (n as f32)).sqrt();
        let theta = i as f32 * golden;
        pts.push([center + r * theta.cos(), center + r * theta.sin()]);
    }
    pts.truncate(n);
    pts
}

fn polygon(n: usize, center: f32, radius: f32) -> Vec<[f32; 2]> {
    // Single regular polygon: n vertices on circle of radius `radius`.
    (0..n)
        .map(|i| {
            let theta =
                -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / n as f32;
            [center + radius * theta.cos(), center + radius * theta.sin()]
        })
        .collect()
}

fn cross(n: usize, center: f32, radius: f32) -> Vec<[f32; 2]> {
    // Axial cross: half on x-axis, half on y-axis (excluding double-counted center).
    let mut pts = Vec::with_capacity(n);
    pts.push([center, center]);
    let arm = (n - 1).max(0);
    let per_axis = arm / 2;
    let extra = arm - per_axis * 2;
    let step = radius / per_axis.max(1) as f32;
    for i in 1..=per_axis {
        let d = i as f32 * step;
        pts.push([center + d, center]);
        pts.push([center - d, center]);
    }
    for i in 1..=per_axis {
        let d = i as f32 * step;
        pts.push([center, center + d]);
        pts.push([center, center - d]);
    }
    if extra > 0 {
        let d = (per_axis as f32 + 0.5) * step;
        pts.push([center + d.min(radius), center]);
    }
    pts.truncate(n);
    pts
}

fn pseudo_random(n: usize, center: f32, radius: f32) -> Vec<[f32; 2]> {
    let mut state: u32 = 0x1234_5678;
    let mut next = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 8) as f32 / ((1u32 << 24) as f32)
    };
    let mut pts = Vec::with_capacity(n);
    while pts.len() < n {
        let u = next() * 2.0 - 1.0;
        let v = next() * 2.0 - 1.0;
        if u * u + v * v <= 1.0 {
            pts.push([center + u * radius, center + v * radius]);
        }
    }
    pts
}

fn halton(n: usize, center: f32, radius: f32) -> Vec<[f32; 2]> {
    fn halton_seq(mut i: u32, base: u32) -> f32 {
        let mut f = 1.0_f32;
        let mut r = 0.0_f32;
        while i > 0 {
            f /= base as f32;
            r += f * (i % base) as f32;
            i /= base;
        }
        r
    }
    (0..n)
        .map(|i| {
            let h1 = halton_seq(i as u32 + 1, 2);
            let h2 = halton_seq(i as u32 + 1, 3);
            let r = radius * h1.sqrt();
            let theta = std::f32::consts::TAU * h2;
            [center + r * theta.cos(), center + r * theta.sin()]
        })
        .collect()
}

fn sort_central(pts: &mut [[f32; 2]], center: f32) {
    pts.sort_by(|a, b| {
        let da = (a[0] - center).powi(2) + (a[1] - center).powi(2);
        let db = (b[0] - center).powi(2) + (b[1] - center).powi(2);
        da.partial_cmp(&db).unwrap()
    });
}
