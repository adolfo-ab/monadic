use egui::{
    Color32, FontId, Margin, Painter, Pos2, Rect, Response, RichText, Sense, Stroke, Ui,
};

use crate::frequency::FrequencyFn;
use crate::lattice::{self, LatticeKind};
use crate::state::{ColorMode, DecayMode, SimState};

pub const PANEL_WIDTH: f32 = 290.0;

const PREVIEW_N: usize = 96;
const FREQ_PREVIEW_N: usize = 48;
const FREQ_PREVIEW_BASE_K: f32 = 1.0;
const FREQ_PREVIEW_ALPHA: f32 = 1.0;
const FREQ_PREVIEW_BETA: f32 = 6.0;

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
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.slider_width = 160.0;
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    ctx.set_style(style);
}

pub fn draw(ctx: &egui::Context, sim: &mut SimState) {
    egui::SidePanel::left("controls")
        .resizable(false)
        .exact_width(PANEL_WIDTH)
        .frame(egui::Frame {
            inner_margin: Margin::symmetric(18.0, 18.0),
            fill: Color32::WHITE,
            stroke: Stroke::new(1.0, Color32::from_gray(220)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("INTERFERENTIA")
                        .font(FontId::proportional(18.0))
                        .strong()
                        .color(Color32::BLACK),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new("a study of wave concurrence")
                        .italics()
                        .color(Color32::from_gray(90))
                        .size(11.0),
                );
                divider(ui);

                section(ui, "CANVAS", |ui| {
                    if ui
                        .add(
                            egui::Slider::new(&mut sim.requested_canvas_px, 256..=4096)
                                .step_by(64.0)
                                .text("N px"),
                        )
                        .changed()
                    {
                        sim.dirty = true;
                    }
                });

                section(ui, "LATTICE", |ui| {
                    lattice_picker(ui, sim);
                    if ui
                        .add(
                            egui::Slider::new(&mut sim.num_nodes, 1..=1024)
                                .integer()
                                .text("nodes"),
                        )
                        .changed()
                    {
                        sim.dirty = true;
                    }
                });

                section(ui, "FREQUENCY  k(r)", |ui| {
                    frequency_picker(ui, sim);
                    if ui
                        .add(
                            egui::Slider::new(&mut sim.base_k, 0.005..=2.0)
                                .text("k₀")
                                .logarithmic(true),
                        )
                        .changed()
                    {
                        sim.dirty = true;
                    }
                    if sim.freq_fn.uses_alpha()
                        && ui
                            .add(egui::Slider::new(&mut sim.alpha, -2.0..=4.0).text("α"))
                            .changed()
                    {
                        sim.dirty = true;
                    }
                    if sim.freq_fn.uses_beta()
                        && ui
                            .add(egui::Slider::new(&mut sim.beta, 0.1..=20.0).text("β"))
                            .changed()
                    {
                        sim.dirty = true;
                    }
                });

                section(ui, "PROPAGATION", |ui| {
                    ui.add(
                        egui::Slider::new(&mut sim.wave_speed, 5.0..=400.0).text("c (px/s)"),
                    );
                    ui.add(
                        egui::Slider::new(&mut sim.amp_scale, 0.005..=2.0)
                            .text("amp")
                            .logarithmic(true),
                    );
                });

                section(ui, "VIEW", |ui| {
                    ui.radio_value(&mut sim.color_mode, ColorMode::Real, "ψ (real part)");
                    ui.radio_value(
                        &mut sim.color_mode,
                        ColorMode::Intensity,
                        "|ψ|² (intensity)",
                    );
                });

                section(ui, "DECAY", |ui| {
                    ui.radio_value(&mut sim.decay_mode, DecayMode::None, "none");
                    ui.radio_value(&mut sim.decay_mode, DecayMode::InvSqrtR, "1 / √r");
                    ui.radio_value(&mut sim.decay_mode, DecayMode::InvR, "1 / r");
                });

                divider(ui);

                ui.horizontal(|ui| {
                    let label = if sim.paused { "▶  resume" } else { "❚❚  pause" };
                    if ui.button(label).clicked() {
                        sim.paused = !sim.paused;
                    }
                    if ui.button("↻  reset t").clicked() {
                        sim.time = 0.0;
                    }
                });

                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("t = {:>7.3}  s", sim.time))
                        .monospace()
                        .color(Color32::from_gray(80)),
                );
                ui.label(
                    RichText::new(format!(
                        "N = {:>4}    canvas = {} px",
                        sim.num_nodes, sim.requested_canvas_px
                    ))
                    .monospace()
                    .color(Color32::from_gray(80)),
                );
            });
        });
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
                sim.dirty = true;
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
                sim.dirty = true;
                ui.memory_mut(|m| m.close_popup());
            }
        },
    );
}

/// Render the row that shows the current selection and opens the popup on click.
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

    // Caret indicator.
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

// ─── thumbnail painters ──────────────────────────────────────────────────

fn draw_lattice_thumb(painter: &Painter, rect: Rect, kind: LatticeKind) {
    // Inscribed circle inside rect.
    let s = rect.width().min(rect.height());
    let center = rect.center();
    let half = s * 0.5;
    let frame = Rect::from_center_size(center, egui::vec2(s, s));

    // Subtle bounding circle for context.
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

    // Sample k(r) over r ∈ [0, 1] with default preview parameters.
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

    // Faint baseline at min value.
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

// ─── small helpers ───────────────────────────────────────────────────────

fn section<R>(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(4.0);
    ui.label(
        RichText::new(title)
            .size(10.0)
            .color(Color32::from_gray(110))
            .strong(),
    );
    ui.add_space(2.0);
    let r = body(ui);
    ui.add_space(2.0);
    r
}

fn divider(ui: &mut Ui) {
    ui.add_space(8.0);
    let rect = ui.available_rect_before_wrap();
    let y = ui.cursor().top();
    ui.painter()
        .hline(rect.x_range(), y, Stroke::new(1.0, Color32::from_gray(210)));
    ui.add_space(10.0);
}
