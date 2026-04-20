use egui::{
    Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, Painter, Pos2, Rect, Response,
    RichText, Sense, Stroke, Ui,
};

use crate::frequency::FrequencyFn;
use crate::lattice::{self, LatticeKind};
use crate::phase::PhaseMode;
use crate::renderer::MAX_SPEC;
use crate::shape::WaveShape;
use crate::spectrum::SpectrumKind;
use crate::state::{ColorMode, DecayMode, SimState};

pub const PANEL_WIDTH: f32 = 320.0;

const PREVIEW_N: usize = 96;
const FREQ_PREVIEW_N: usize = 48;
const FREQ_PREVIEW_BASE_K: f32 = 1.0;
const FREQ_PREVIEW_ALPHA: f32 = 1.0;
const FREQ_PREVIEW_BETA: f32 = 6.0;

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "NotoSerif".to_owned(),
        FontData::from_static(include_bytes!("../assets/NotoSerif.ttf")).into(),
    );
    fonts
        .families
        .entry(FontFamily::Name("serif".into()))
        .or_default()
        .insert(0, "NotoSerif".to_owned());
    ctx.set_fonts(fonts);
}

pub fn install_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::light();
    visuals.window_fill = Color32::WHITE;
    visuals.panel_fill = Color32::WHITE;
    visuals.extreme_bg_color = Color32::from_gray(248);
    visuals.faint_bg_color = Color32::from_gray(252);
    visuals.override_text_color = Some(Color32::BLACK);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_gray(180));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::BLACK);
    visuals.widgets.inactive.bg_fill = Color32::from_gray(245);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(245);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_gray(180));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::BLACK);
    visuals.widgets.hovered.bg_fill = Color32::from_gray(235);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_gray(235);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::BLACK);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::BLACK);
    visuals.widgets.active.bg_fill = Color32::from_gray(220);
    visuals.widgets.active.weak_bg_fill = Color32::from_gray(220);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::BLACK);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::BLACK);
    visuals.selection.bg_fill = Color32::from_gray(210);
    visuals.selection.stroke = Stroke::new(1.0, Color32::BLACK);
    visuals.window_stroke = Stroke::new(1.0, Color32::BLACK);
    visuals.window_shadow = egui::epaint::Shadow::NONE;
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 2.0),
        blur: 8.0,
        spread: 0.0,
        color: Color32::from_black_alpha(24),
    };

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.slider_width = 160.0;
    style.spacing.slider_rail_height = 0.0;
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    ctx.set_style(style);
}

pub fn draw(ctx: &egui::Context, sim: &mut SimState) {
    egui::SidePanel::left("controls")
        .resizable(false)
        .exact_width(PANEL_WIDTH)
        .frame(egui::Frame {
            inner_margin: Margin {
                left: 28.0,
                right: 28.0,
                top: 14.0,
                bottom: 14.0,
            },
            fill: Color32::WHITE,
            stroke: Stroke::new(1.0, Color32::from_gray(220)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            egui::TopBottomPanel::bottom("globals")
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame {
                    inner_margin: Margin::symmetric(0.0, 10.0),
                    fill: Color32::WHITE,
                    ..Default::default()
                })
                .show_inside(ui, |ui| {
                    section(ui, "RESOLUTION", |ui| {
                        if slider(
                            ui,
                            egui::Slider::new(&mut sim.sim_resolution, 64..=4096)
                                .step_by(32.0)
                                .logarithmic(true)
                                .text("N"),
                        )
                        .changed()
                        {
                            sim.emitters_dirty = true;
                        }
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let label = if sim.paused { "▶  resume" } else { "❚❚  pause" };
                        if ui.button(label).clicked() {
                            sim.paused = !sim.paused;
                        }
                        if ui.button("↻  reset t").clicked() {
                            sim.time = 0.0;
                        }
                    });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("t = {:>7.3}  s", sim.time))
                            .monospace()
                            .color(Color32::from_gray(80)),
                    );
                    ui.label(
                        RichText::new(format!(
                            "N = {:>4}    M = {:>2}    res = {}",
                            sim.num_nodes, sim.spec_count, sim.sim_resolution
                        ))
                        .monospace()
                        .color(Color32::from_gray(80)),
                    );
                });

            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(Color32::WHITE))
                .show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("monadic")
                            .font(FontId::new(22.0, FontFamily::Name("serif".into())))
                            .color(Color32::BLACK),
                    );

                    section(ui, "LATTICE", |ui| {
                        lattice_picker(ui, sim);
                        ui.add_space(6.0);
                        if slider(
                            ui,
                            egui::Slider::new(&mut sim.num_nodes, 1..=1024)
                                .integer()
                                .text("nodes"),
                        )
                        .changed()
                        {
                            sim.emitters_dirty = true;
                        }
                    });

                    section(ui, "PROPAGATION", |ui| {
                        slider(
                            ui,
                            egui::Slider::new(&mut sim.wave_speed, 5.0..=400.0)
                                .text("c (px/s)"),
                        );
                        slider(
                            ui,
                            egui::Slider::new(&mut sim.amp_scale, 0.005..=2.0)
                                .text("amp")
                                .logarithmic(true),
                        );
                    });

                    section(ui, "FREQUENCY  k(r)", |ui| {
                        frequency_picker(ui, sim);
                        config_popup_button(ui, "freq-config", |ui| {
                            if slider(
                                ui,
                                egui::Slider::new(&mut sim.base_k, 0.005..=2.0)
                                    .text("k₀")
                                    .logarithmic(true),
                            )
                            .changed()
                            {
                                sim.emitters_dirty = true;
                            }
                            if sim.freq_fn.uses_alpha()
                                && slider(
                                    ui,
                                    egui::Slider::new(&mut sim.alpha, -2.0..=4.0).text("α"),
                                )
                                .changed()
                            {
                                sim.emitters_dirty = true;
                            }
                            if sim.freq_fn.uses_beta()
                                && slider(
                                    ui,
                                    egui::Slider::new(&mut sim.beta, 0.1..=20.0).text("β"),
                                )
                                .changed()
                            {
                                sim.emitters_dirty = true;
                            }
                        });
                    });

                    section(ui, "SPECTRUM  per-node M", |ui| {
                        spectrum_picker(ui, sim);
                        let has_params = sim.spectrum_kind.uses_count()
                            || sim.spectrum_kind.uses_spread();
                        if has_params {
                            config_popup_button(ui, "spec-config", |ui| {
                                if sim.spectrum_kind.uses_count()
                                    && slider(
                                        ui,
                                        egui::Slider::new(
                                            &mut sim.spec_count,
                                            1..=MAX_SPEC as usize,
                                        )
                                        .integer()
                                        .text("M"),
                                    )
                                    .changed()
                                {
                                    sim.spectrum_dirty = true;
                                }
                                if sim.spectrum_kind.uses_spread()
                                    && slider(
                                        ui,
                                        egui::Slider::new(&mut sim.spec_spread, 0.005..=1.0)
                                            .text("Δ")
                                            .logarithmic(true),
                                    )
                                    .changed()
                                {
                                    sim.spectrum_dirty = true;
                                }
                            });
                        }
                    });

                    section(ui, "PHASE  φ", |ui| {
                        phase_picker(ui, sim);
                        if sim.phase_mode.uses_param_a() {
                            config_popup_button(ui, "phase-config", |ui| {
                                let (lo, hi) = sim.phase_mode.param_a_range();
                                slider(
                                    ui,
                                    egui::Slider::new(&mut sim.phase_param_a, lo..=hi)
                                        .text(sim.phase_mode.param_a_label()),
                                );
                            });
                        }
                    });

                    section(ui, "WAVEFRONT", |ui| {
                        wave_shape_picker(ui, sim);
                        let has_params = sim.wave_shape.uses_param_a()
                            || sim.wave_shape.uses_param_b();
                        if has_params {
                            config_popup_button(ui, "shape-config", |ui| {
                                if sim.wave_shape.uses_param_a() {
                                    let (lo, hi) = sim.wave_shape.param_a_range();
                                    slider(
                                        ui,
                                        egui::Slider::new(&mut sim.shape_param_a, lo..=hi)
                                            .text(sim.wave_shape.param_a_label()),
                                    );
                                }
                                if sim.wave_shape.uses_param_b() {
                                    let (lo, hi) = sim.wave_shape.param_b_range();
                                    slider(
                                        ui,
                                        egui::Slider::new(&mut sim.shape_param_b, lo..=hi)
                                            .text(sim.wave_shape.param_b_label()),
                                    );
                                }
                            });
                        }
                    });

                    section(ui, "VIEW", |ui| {
                        egui::ComboBox::from_id_salt("view-combo")
                            .width(ui.available_width() - 28.0)
                            .selected_text(color_mode_label(sim.color_mode))
                            .show_ui(ui, |ui| {
                                for m in [
                                    ColorMode::Real,
                                    ColorMode::Intensity,
                                    ColorMode::Domain,
                                    ColorMode::Spectral,
                                ] {
                                    ui.selectable_value(
                                        &mut sim.color_mode,
                                        m,
                                        color_mode_label(m),
                                    );
                                }
                            });
                    });

                    section(ui, "DECAY", |ui| {
                        egui::ComboBox::from_id_salt("decay-combo")
                            .width(ui.available_width() - 28.0)
                            .selected_text(decay_mode_label(sim.decay_mode))
                            .show_ui(ui, |ui| {
                                for m in [DecayMode::None, DecayMode::InvSqrtR, DecayMode::InvR] {
                                    ui.selectable_value(
                                        &mut sim.decay_mode,
                                        m,
                                        decay_mode_label(m),
                                    );
                                }
                            });
                    });

                });
            });
                });
        });
}

fn color_mode_label(m: ColorMode) -> &'static str {
    match m {
        ColorMode::Real => "ψ real (mono)",
        ColorMode::Intensity => "|ψ|² intensity (mono)",
        ColorMode::Domain => "domain (arg → hue)",
        ColorMode::Spectral => "spectral (per-freq hue)",
    }
}

fn decay_mode_label(m: DecayMode) -> &'static str {
    match m {
        DecayMode::None => "none",
        DecayMode::InvSqrtR => "1 / √r",
        DecayMode::InvR => "1 / r",
    }
}

fn config_popup_button(ui: &mut Ui, id_salt: &str, body: impl FnOnce(&mut Ui)) {
    ui.add_space(8.0);
    let popup_id = ui.make_persistent_id(id_salt);
    let response = ui.button("⚙  config");
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(260.0);
            egui::Frame::none()
                .inner_margin(Margin {
                    left: 8.0,
                    right: 18.0,
                    top: 6.0,
                    bottom: 6.0,
                })
                .show(ui, |ui| {
                    body(ui);
                });
        },
    );
}

// ─── pickers ─────────────────────────────────────────────────────────────

fn lattice_picker(ui: &mut Ui, sim: &mut SimState) {
    let popup_id = ui.make_persistent_id("lattice-picker");
    let response = picker_button(
        ui,
        |painter, rect| draw_lattice_thumb(painter, rect, sim.lattice_kind),
        sim.lattice_kind.label(),
        sim.lattice_kind.description(),
    );
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(280.0);
            let cell_size = egui::vec2(82.0, 96.0);
            let mut chosen: Option<LatticeKind> = None;
            egui::Grid::new("lattice-grid")
                .num_columns(3)
                .spacing(egui::vec2(4.0, 4.0))
                .show(ui, |ui| {
                    for (i, &kind) in LatticeKind::ALL.iter().enumerate() {
                        if thumb_cell(
                            ui,
                            cell_size,
                            kind == sim.lattice_kind,
                            kind.label(),
                            |painter, rect| draw_lattice_thumb(painter, rect, kind),
                        )
                        .clicked()
                        {
                            chosen = Some(kind);
                        }
                        if (i + 1) % 3 == 0 {
                            ui.end_row();
                        }
                    }
                });
            if let Some(kind) = chosen {
                sim.lattice_kind = kind;
                sim.emitters_dirty = true;
                ui.memory_mut(|m| m.close_popup());
            }
        },
    );
}

fn frequency_picker(ui: &mut Ui, sim: &mut SimState) {
    let popup_id = ui.make_persistent_id("freq-picker");
    let response = picker_button(
        ui,
        |painter, rect| draw_freq_thumb(painter, rect, sim.freq_fn),
        sim.freq_fn.label(),
        sim.freq_fn.formula(),
    );
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(280.0);
            let cell_size = egui::vec2(82.0, 96.0);
            let mut chosen: Option<FrequencyFn> = None;
            egui::Grid::new("freq-grid")
                .num_columns(3)
                .spacing(egui::vec2(4.0, 4.0))
                .show(ui, |ui| {
                    for (i, &f) in FrequencyFn::ALL.iter().enumerate() {
                        if thumb_cell(
                            ui,
                            cell_size,
                            f == sim.freq_fn,
                            f.label(),
                            |painter, rect| draw_freq_thumb(painter, rect, f),
                        )
                        .clicked()
                        {
                            chosen = Some(f);
                        }
                        if (i + 1) % 3 == 0 {
                            ui.end_row();
                        }
                    }
                });
            if let Some(f) = chosen {
                sim.freq_fn = f;
                sim.emitters_dirty = true;
                ui.memory_mut(|m| m.close_popup());
            }
        },
    );
}

fn spectrum_picker(ui: &mut Ui, sim: &mut SimState) {
    let popup_id = ui.make_persistent_id("spec-picker");
    let count = sim.spec_count;
    let spread = sim.spec_spread;
    let response = picker_button(
        ui,
        |painter, rect| draw_spectrum_thumb(painter, rect, sim.spectrum_kind, count, spread),
        sim.spectrum_kind.label(),
        sim.spectrum_kind.description(),
    );
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(280.0);
            let cell_size = egui::vec2(82.0, 96.0);
            let mut chosen: Option<SpectrumKind> = None;
            egui::Grid::new("spec-grid")
                .num_columns(3)
                .spacing(egui::vec2(4.0, 4.0))
                .show(ui, |ui| {
                    for (i, &k) in SpectrumKind::ALL.iter().enumerate() {
                        if thumb_cell(
                            ui,
                            cell_size,
                            k == sim.spectrum_kind,
                            k.label(),
                            |painter, rect| draw_spectrum_thumb(painter, rect, k, count, spread),
                        )
                        .clicked()
                        {
                            chosen = Some(k);
                        }
                        if (i + 1) % 3 == 0 {
                            ui.end_row();
                        }
                    }
                });
            if let Some(k) = chosen {
                sim.spectrum_kind = k;
                sim.spectrum_dirty = true;
                ui.memory_mut(|m| m.close_popup());
            }
        },
    );
}

fn phase_picker(ui: &mut Ui, sim: &mut SimState) {
    let popup_id = ui.make_persistent_id("phase-picker");
    let param_a = sim.phase_param_a;
    let response = picker_button(
        ui,
        |painter, rect| draw_phase_thumb(painter, rect, sim.phase_mode, param_a),
        sim.phase_mode.label(),
        sim.phase_mode.description(),
    );
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(280.0);
            let cell_size = egui::vec2(82.0, 96.0);
            let mut chosen: Option<PhaseMode> = None;
            egui::Grid::new("phase-grid")
                .num_columns(3)
                .spacing(egui::vec2(4.0, 4.0))
                .show(ui, |ui| {
                    for (i, &m) in PhaseMode::ALL.iter().enumerate() {
                        let preview_a = if m == sim.phase_mode {
                            param_a
                        } else {
                            m.default_param_a()
                        };
                        if thumb_cell(
                            ui,
                            cell_size,
                            m == sim.phase_mode,
                            m.label(),
                            |painter, rect| draw_phase_thumb(painter, rect, m, preview_a),
                        )
                        .clicked()
                        {
                            chosen = Some(m);
                        }
                        if (i + 1) % 3 == 0 {
                            ui.end_row();
                        }
                    }
                });
            if let Some(m) = chosen {
                sim.phase_mode = m;
                sim.phase_param_a = m.default_param_a();
                ui.memory_mut(|mem| mem.close_popup());
            }
        },
    );
}

fn wave_shape_picker(ui: &mut Ui, sim: &mut SimState) {
    let popup_id = ui.make_persistent_id("shape-picker");
    let param_a = sim.shape_param_a;
    let param_b = sim.shape_param_b;
    let response = picker_button(
        ui,
        |painter, rect| draw_shape_thumb(painter, rect, sim.wave_shape, param_a, param_b),
        sim.wave_shape.label(),
        sim.wave_shape.description(),
    );
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(280.0);
            let cell_size = egui::vec2(82.0, 96.0);
            let mut chosen: Option<WaveShape> = None;
            egui::Grid::new("shape-grid")
                .num_columns(3)
                .spacing(egui::vec2(4.0, 4.0))
                .show(ui, |ui| {
                    for (i, &sh) in WaveShape::ALL.iter().enumerate() {
                        let (pa, pb) = if sh == sim.wave_shape {
                            (param_a, param_b)
                        } else {
                            (sh.default_param_a(), sh.default_param_b())
                        };
                        if thumb_cell(
                            ui,
                            cell_size,
                            sh == sim.wave_shape,
                            sh.label(),
                            |painter, rect| draw_shape_thumb(painter, rect, sh, pa, pb),
                        )
                        .clicked()
                        {
                            chosen = Some(sh);
                        }
                        if (i + 1) % 3 == 0 {
                            ui.end_row();
                        }
                    }
                });
            if let Some(sh) = chosen {
                sim.wave_shape = sh;
                sim.shape_param_a = sh.default_param_a();
                sim.shape_param_b = sh.default_param_b();
                ui.memory_mut(|m| m.close_popup());
            }
        },
    );
}

fn picker_button(
    ui: &mut Ui,
    draw_thumb: impl FnOnce(&Painter, Rect),
    name: &str,
    sub: &str,
) -> Response {
    let row_h = 44.0;
    let response = ui.allocate_response(
        egui::vec2(ui.available_width(), row_h),
        Sense::click(),
    );
    let rect = response.rect;
    let painter = ui.painter_at(rect);

    let bg = if response.hovered() {
        Color32::from_gray(248)
    } else {
        Color32::WHITE
    };
    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(180)));

    let thumb_rect = Rect::from_min_size(
        rect.min + egui::vec2(4.0, 4.0),
        egui::vec2(36.0, 36.0),
    );
    draw_thumb(&painter, thumb_rect);

    let text_x = thumb_rect.right() + 8.0;
    painter.text(
        Pos2::new(text_x, rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        name,
        FontId::proportional(13.0),
        Color32::BLACK,
    );
    painter.text(
        Pos2::new(text_x, rect.top() + 24.0),
        egui::Align2::LEFT_TOP,
        sub,
        FontId::proportional(10.0),
        Color32::from_gray(110),
    );

    let caret_x = rect.right() - 12.0;
    let caret_y = rect.center().y;
    painter.text(
        Pos2::new(caret_x, caret_y),
        egui::Align2::CENTER_CENTER,
        "▾",
        FontId::proportional(12.0),
        Color32::from_gray(120),
    );

    response
}

fn thumb_cell(
    ui: &mut Ui,
    size: egui::Vec2,
    selected: bool,
    label: &str,
    draw: impl FnOnce(&Painter, Rect),
) -> Response {
    let response = ui.allocate_response(size, Sense::click());
    let r = response.rect;
    let painter = ui.painter_at(r);
    let bg = if selected {
        Color32::from_gray(232)
    } else if response.hovered() {
        Color32::from_gray(245)
    } else {
        Color32::WHITE
    };
    painter.rect_filled(r, 4.0, bg);

    let thumb_rect = Rect::from_min_size(
        r.min + egui::vec2(4.0, 4.0),
        egui::vec2(r.width() - 8.0, r.height() - 24.0),
    );
    draw(&painter, thumb_rect);

    painter.text(
        Pos2::new(r.center().x, r.bottom() - 10.0),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(10.0),
        Color32::BLACK,
    );

    let stroke = if selected {
        Stroke::new(1.5, Color32::BLACK)
    } else {
        Stroke::new(1.0, Color32::from_gray(190))
    };
    painter.rect_stroke(r, 4.0, stroke);
    response
}

// ─── thumbnails ──────────────────────────────────────────────────────────

fn draw_lattice_thumb(painter: &Painter, rect: Rect, kind: LatticeKind) {
    let s = rect.width().min(rect.height());
    let center = rect.center();
    let half = s * 0.5;
    let frame = Rect::from_center_size(center, egui::vec2(s, s));

    painter.circle_stroke(
        center,
        half - 1.0,
        Stroke::new(0.6, Color32::from_gray(220)),
    );

    let pts = lattice::generate(kind, PREVIEW_N, 1.0);
    let dot_r = (s / 60.0).clamp(0.9, 2.0);
    for [x, y] in pts {
        let p = Pos2::new(
            frame.left() + x * frame.width(),
            frame.top() + y * frame.height(),
        );
        painter.circle_filled(p, dot_r, Color32::BLACK);
    }
}

fn draw_freq_thumb(painter: &Painter, rect: Rect, f: FrequencyFn) {
    let plot = rect.shrink(3.0);

    let mut samples = [0.0_f32; FREQ_PREVIEW_N];
    let mut min_v = f32::INFINITY;
    let mut max_v = f32::NEG_INFINITY;
    for i in 0..FREQ_PREVIEW_N {
        let r = i as f32 / (FREQ_PREVIEW_N as f32 - 1.0);
        let v = f.eval(
            r,
            FREQ_PREVIEW_BASE_K,
            FREQ_PREVIEW_ALPHA,
            FREQ_PREVIEW_BETA,
        );
        samples[i] = v;
        if v < min_v {
            min_v = v;
        }
        if v > max_v {
            max_v = v;
        }
    }
    let span = (max_v - min_v).max(1e-3);

    painter.line_segment(
        [
            Pos2::new(plot.left(), plot.bottom()),
            Pos2::new(plot.right(), plot.bottom()),
        ],
        Stroke::new(0.6, Color32::from_gray(220)),
    );

    let mut prev: Option<Pos2> = None;
    for (i, &v) in samples.iter().enumerate() {
        let x = plot.left() + (i as f32 / (FREQ_PREVIEW_N as f32 - 1.0)) * plot.width();
        let yn = (v - min_v) / span;
        let y = plot.bottom() - yn * plot.height();
        let p = Pos2::new(x, y);
        if let Some(q) = prev {
            painter.line_segment([q, p], Stroke::new(1.4, Color32::BLACK));
        }
        prev = Some(p);
    }
}

fn draw_spectrum_thumb(
    painter: &Painter,
    rect: Rect,
    kind: SpectrumKind,
    count: usize,
    spread: f32,
) {
    let plot = rect.shrink(3.0);
    let spec = kind.build(count, MAX_SPEC as usize, spread);
    if spec.is_empty() {
        return;
    }
    let k_max = spec
        .iter()
        .map(|s| s[0])
        .fold(0.0_f32, f32::max)
        .max(1.0);
    let amp_max = spec
        .iter()
        .map(|s| s[1])
        .fold(0.0_f32, f32::max)
        .max(1e-3);
    let bar_w = (plot.width() / (spec.len().max(4)) as f32 * 0.55).clamp(1.5, 6.0);

    painter.line_segment(
        [
            Pos2::new(plot.left(), plot.bottom()),
            Pos2::new(plot.right(), plot.bottom()),
        ],
        Stroke::new(0.6, Color32::from_gray(220)),
    );

    for s in spec {
        let nx = (s[0] / k_max).clamp(0.0, 1.0);
        let h = (s[1] / amp_max).clamp(0.0, 1.0) * plot.height();
        let cx = plot.left() + nx * plot.width();
        let bar = Rect::from_min_size(
            Pos2::new(cx - bar_w * 0.5, plot.bottom() - h),
            egui::vec2(bar_w, h),
        );
        painter.rect_filled(bar, 0.0, Color32::BLACK);
    }
}

fn draw_phase_thumb(painter: &Painter, rect: Rect, mode: PhaseMode, param_a: f32) {
    let s = rect.width().min(rect.height());
    let center = rect.center();
    let radius = s * 0.45;
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(0.6, Color32::from_gray(220)),
    );

    let stroke = Stroke::new(1.2, Color32::BLACK);
    match mode {
        PhaseMode::Zero => {
            painter.circle_filled(center, 1.8, Color32::BLACK);
        }
        PhaseMode::Random => {
            // Deterministic "random" dot scatter inside disc.
            let mut state: u32 = 0x1234_5678;
            for _ in 0..18 {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let u = ((state >> 8) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0;
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let v = ((state >> 8) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0;
                if u * u + v * v <= 1.0 {
                    painter.circle_filled(
                        Pos2::new(center.x + u * radius, center.y + v * radius),
                        1.4,
                        Color32::BLACK,
                    );
                }
            }
        }
        PhaseMode::Focus => {
            for i in 1..=4 {
                let r = radius * (i as f32 / 4.0);
                painter.circle_stroke(center, r, stroke);
            }
        }
        PhaseMode::Vortex => {
            let m = (param_a.abs().round() as i32).max(1);
            let segs = (m * 2).max(2) as usize;
            for k in 0..segs {
                if k & 1 == 1 {
                    continue;
                }
                let theta0 = (k as f32 / segs as f32) * std::f32::consts::TAU;
                let theta1 = ((k + 1) as f32 / segs as f32) * std::f32::consts::TAU;
                let steps = 8;
                let mut pts = Vec::with_capacity(steps + 2);
                pts.push(center);
                for j in 0..=steps {
                    let t = theta0 + (theta1 - theta0) * (j as f32 / steps as f32);
                    pts.push(Pos2::new(center.x + radius * t.cos(), center.y + radius * t.sin()));
                }
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::BLACK,
                    Stroke::NONE,
                ));
            }
        }
        PhaseMode::Gradient => {
            let angle = param_a;
            let nx = angle.cos();
            let ny = angle.sin();
            let tx = -ny;
            let ty = nx;
            let count = 6;
            for i in -count..=count {
                let f = i as f32 / count as f32;
                let mid = Pos2::new(center.x + f * radius * nx, center.y + f * radius * ny);
                let len = (radius * radius - (f * radius).powi(2)).max(0.0).sqrt();
                let a = Pos2::new(mid.x - len * tx, mid.y - len * ty);
                let b = Pos2::new(mid.x + len * tx, mid.y + len * ty);
                painter.line_segment([a, b], Stroke::new(1.0, Color32::BLACK));
            }
        }
        PhaseMode::Chirp => {
            // Spiral.
            let turns = 2.0;
            let steps = 60;
            let mut prev: Option<Pos2> = None;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let theta = t * turns * std::f32::consts::TAU;
                let r = radius * t;
                let p = Pos2::new(center.x + r * theta.cos(), center.y + r * theta.sin());
                if let Some(q) = prev {
                    painter.line_segment([q, p], stroke);
                }
                prev = Some(p);
            }
        }
        PhaseMode::Spiral => {
            let m = (param_a.abs().round() as i32).max(1);
            let turns = 2.0;
            let steps = 60;
            for arm in 0..m {
                let arm_off = arm as f32 * std::f32::consts::TAU / m as f32;
                let mut prev: Option<Pos2> = None;
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let theta = t * turns * std::f32::consts::TAU + arm_off;
                    let r = radius * t;
                    let p = Pos2::new(center.x + r * theta.cos(), center.y + r * theta.sin());
                    if let Some(q) = prev {
                        painter.line_segment([q, p], stroke);
                    }
                    prev = Some(p);
                }
            }
        }
        PhaseMode::Hyperbolic => {
            // Two orthogonal hyperbolas x²−y² = ±c.
            let sign = if param_a >= 0.0 { 1.0 } else { -1.0 };
            for lvl in 1..=3 {
                let c = radius * (lvl as f32 / 3.0) * 0.8;
                let steps = 40;
                let mut prev_a: Option<Pos2> = None;
                let mut prev_b: Option<Pos2> = None;
                for i in 0..=steps {
                    let t = (i as f32 / steps as f32) * 2.0 - 1.0;
                    let y = t * radius;
                    let under = c * c + sign * y * y;
                    if under <= 0.0 {
                        prev_a = None;
                        prev_b = None;
                        continue;
                    }
                    let x = under.sqrt();
                    let p1 = Pos2::new(center.x + x, center.y + y);
                    let p2 = Pos2::new(center.x - x, center.y + y);
                    if let Some(q) = prev_a {
                        painter.line_segment([q, p1], stroke);
                    }
                    if let Some(q) = prev_b {
                        painter.line_segment([q, p2], stroke);
                    }
                    prev_a = Some(p1);
                    prev_b = Some(p2);
                }
            }
        }
        PhaseMode::Bands => {
            let bands = (param_a.max(1.0).round() as i32).min(8);
            for b in 1..=bands {
                let r = radius * (b as f32 / bands as f32);
                let filled = b & 1 == 1;
                if filled {
                    painter.circle_stroke(center, r, Stroke::new(1.4, Color32::BLACK));
                } else {
                    painter.circle_stroke(center, r, Stroke::new(0.6, Color32::from_gray(160)));
                }
            }
        }
        PhaseMode::Checker => {
            let cols = (param_a.max(1.0).round() as i32).min(8).max(2) as usize;
            let cell = (radius * 2.0) / cols as f32;
            let origin = Pos2::new(center.x - radius, center.y - radius);
            for j in 0..cols {
                for i in 0..cols {
                    let cx = origin.x + (i as f32 + 0.5) * cell;
                    let cy = origin.y + (j as f32 + 0.5) * cell;
                    let dx = cx - center.x;
                    let dy = cy - center.y;
                    if dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                    if (i + j) & 1 == 0 {
                        let r = Rect::from_center_size(
                            Pos2::new(cx, cy),
                            egui::vec2(cell * 0.9, cell * 0.9),
                        );
                        painter.rect_filled(r, 0.0, Color32::BLACK);
                    }
                }
            }
        }
        PhaseMode::Antiphase => {
            let cols = 4;
            let cell = (radius * 2.0) / cols as f32;
            let origin = Pos2::new(center.x - radius, center.y - radius);
            for j in 0..cols {
                for i in 0..cols {
                    let cx = origin.x + (i as f32 + 0.5) * cell;
                    let cy = origin.y + (j as f32 + 0.5) * cell;
                    let dx = cx - center.x;
                    let dy = cy - center.y;
                    if dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                    let parity = (i + j) & 1;
                    if parity == 0 {
                        painter.circle_filled(Pos2::new(cx, cy), cell * 0.18, Color32::BLACK);
                    } else {
                        painter.circle_stroke(
                            Pos2::new(cx, cy),
                            cell * 0.18,
                            Stroke::new(1.0, Color32::BLACK),
                        );
                    }
                }
            }
        }
    }
}

fn draw_shape_thumb(painter: &Painter, rect: Rect, shape: WaveShape, a: f32, b: f32) {
    let s = rect.width().min(rect.height());
    let center = rect.center();
    let radius = s * 0.45;
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(0.6, Color32::from_gray(220)),
    );
    let stroke = Stroke::new(1.2, Color32::BLACK);
    let thin = Stroke::new(0.9, Color32::BLACK);

    let poly = |painter: &Painter, f: &dyn Fn(f32) -> (f32, f32), steps: usize, stroke: Stroke| {
        let mut prev: Option<Pos2> = None;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let (x, y) = f(t);
            let p = Pos2::new(center.x + x, center.y + y);
            if let Some(q) = prev {
                painter.line_segment([q, p], stroke);
            }
            prev = Some(p);
        }
    };

    match shape {
        WaveShape::Circular => {
            for i in 1..=4 {
                let r = radius * (i as f32 / 4.0);
                painter.circle_stroke(center, r, thin);
            }
        }
        WaveShape::Petal => {
            let n = b.max(1.0);
            let depth = a.abs().clamp(0.0, 0.7);
            for ring in 1..=3 {
                let base_r = radius * (ring as f32 / 3.0);
                poly(
                    painter,
                    &|t| {
                        let phi = t * std::f32::consts::TAU;
                        let r = base_r * (1.0 + depth * (n * phi).cos());
                        (r * phi.cos(), r * phi.sin())
                    },
                    96,
                    thin,
                );
            }
        }
        WaveShape::Wobbly => {
            let n = b.max(1.0);
            let depth = (a / 60.0).clamp(0.0, 0.5);
            for ring in 1..=3 {
                let base_r = radius * (ring as f32 / 3.0);
                poly(
                    painter,
                    &|t| {
                        let phi = t * std::f32::consts::TAU;
                        let r = base_r + depth * radius * (n * phi).sin();
                        (r * phi.cos(), r * phi.sin())
                    },
                    96,
                    thin,
                );
            }
        }
        WaveShape::Elliptical => {
            let e = a.clamp(0.0, 0.95);
            let sa = 1.0 - e;
            let sb = 1.0 + e;
            let theta = b;
            let ct = theta.cos();
            let st = theta.sin();
            for ring in 1..=3 {
                let base_r = radius * (ring as f32 / 3.0);
                poly(
                    painter,
                    &|t| {
                        let phi = t * std::f32::consts::TAU;
                        let u = base_r * sa * phi.cos();
                        let v = base_r * sb * phi.sin();
                        (u * ct - v * st, u * st + v * ct)
                    },
                    80,
                    thin,
                );
            }
        }
        WaveShape::Diamond => {
            for ring in 1..=4 {
                let r = radius * (ring as f32 / 4.0);
                let pts = vec![
                    Pos2::new(center.x + r, center.y),
                    Pos2::new(center.x, center.y + r),
                    Pos2::new(center.x - r, center.y),
                    Pos2::new(center.x, center.y - r),
                    Pos2::new(center.x + r, center.y),
                ];
                for w in pts.windows(2) {
                    painter.line_segment([w[0], w[1]], thin);
                }
            }
        }
        WaveShape::Square => {
            for ring in 1..=4 {
                let r = radius * (ring as f32 / 4.0);
                let rc = Rect::from_center_size(center, egui::vec2(r * 2.0, r * 2.0));
                painter.rect_stroke(rc, 0.0, thin);
            }
        }
        WaveShape::Plane => {
            let nx = a.cos();
            let ny = a.sin();
            let tx = -ny;
            let ty = nx;
            let count = 6;
            for i in -count..=count {
                let f = i as f32 / count as f32;
                let mid = Pos2::new(center.x + f * radius * nx, center.y + f * radius * ny);
                let len = (radius * radius - (f * radius).powi(2)).max(0.0).sqrt();
                let p0 = Pos2::new(mid.x - len * tx, mid.y - len * ty);
                let p1 = Pos2::new(mid.x + len * tx, mid.y + len * ty);
                painter.line_segment([p0, p1], thin);
            }
        }
        WaveShape::Spiral => {
            let m = (a.abs().round() as i32).max(1);
            let turns = 2.0;
            let steps = 80;
            for arm in 0..m {
                let arm_off = arm as f32 * std::f32::consts::TAU / m as f32;
                let mut prev: Option<Pos2> = None;
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let theta = t * turns * std::f32::consts::TAU + arm_off;
                    let r = radius * t;
                    let p = Pos2::new(center.x + r * theta.cos(), center.y + r * theta.sin());
                    if let Some(q) = prev {
                        painter.line_segment([q, p], stroke);
                    }
                    prev = Some(p);
                }
            }
        }
        WaveShape::Breathing => {
            // Concentric rings with dashed outer "pulse" halo.
            for i in 1..=3 {
                let r = radius * (i as f32 / 4.0);
                painter.circle_stroke(center, r, thin);
            }
            let halo = radius * 0.95;
            let dashes = 24;
            for d in 0..dashes {
                if d & 1 == 1 {
                    continue;
                }
                let t0 = d as f32 / dashes as f32 * std::f32::consts::TAU;
                let t1 = (d as f32 + 1.0) / dashes as f32 * std::f32::consts::TAU;
                let p0 = Pos2::new(center.x + halo * t0.cos(), center.y + halo * t0.sin());
                let p1 = Pos2::new(center.x + halo * t1.cos(), center.y + halo * t1.sin());
                painter.line_segment([p0, p1], stroke);
            }
            let _ = b;
        }
    }
}

fn section<R>(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui) -> R) -> R {
    divider(ui);
    ui.label(
        RichText::new(title)
            .size(10.0)
            .color(Color32::from_gray(110))
            .strong(),
    );
    ui.add_space(6.0);
    let r = body(ui);
    ui.add_space(6.0);
    r
}

fn slider(ui: &mut Ui, s: egui::Slider<'_>) -> Response {
    let slider_w = ui.style().spacing.slider_width;
    let avail_w = ui.available_width();
    let h = ui.spacing().interact_size.y;
    let (_id, rect) = ui.allocate_space(egui::vec2(avail_w, h));

    let y = rect.center().y;
    let x0 = rect.left();
    let x1 = (x0 + slider_w).min(rect.right());
    ui.painter().line_segment(
        [Pos2::new(x0, y), Pos2::new(x1, y)],
        Stroke::new(1.0, Color32::BLACK),
    );

    let saved = ui.style().visuals.clone();
    {
        let v = &mut ui.style_mut().visuals;
        v.extreme_bg_color = Color32::TRANSPARENT;
        v.widgets.inactive.bg_fill = Color32::WHITE;
        v.widgets.inactive.weak_bg_fill = Color32::WHITE;
        v.widgets.inactive.bg_stroke = Stroke::NONE;
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::BLACK);
        v.widgets.hovered.bg_fill = Color32::WHITE;
        v.widgets.hovered.weak_bg_fill = Color32::WHITE;
        v.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::BLACK);
        v.widgets.active.bg_fill = Color32::from_gray(230);
        v.widgets.active.weak_bg_fill = Color32::from_gray(230);
        v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::BLACK);
        v.widgets.noninteractive.bg_stroke = Stroke::NONE;
    }
    let r = ui.put(rect, s);
    ui.style_mut().visuals = saved;

    ui.add_space(4.0);
    r
}

fn divider(ui: &mut Ui) {
    ui.add_space(14.0);
    let rect = ui.available_rect_before_wrap();
    let y = ui.cursor().top();
    ui.painter()
        .hline(rect.x_range(), y, Stroke::new(1.0, Color32::from_gray(195)));
    ui.add_space(14.0);
}
