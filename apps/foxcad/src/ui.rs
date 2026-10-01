use crate::{
    command::{Commands, parse_point},
    viewport::Viewport,
};
use egui::{Color32, Id, Key, Modifiers, PointerButton, Rect, Sense, TextEdit};
use fox_graphics::Stroke;

pub struct DraftUi {
    pub commands: Commands,
    pub camera: Viewport,
    pub rect: Rect,
    pub strokes: Vec<Stroke>,
    name: String,
    focus: Option<usize>,
    pub grid: bool,
    pub pointer: Option<[f64; 2]>,
}
impl Default for DraftUi {
    fn default() -> Self {
        Self {
            commands: Commands::default(),
            camera: Viewport::default(),
            rect: Rect::NOTHING,
            strokes: vec![],
            name: String::new(),
            focus: Some(0),
            grid: true,
            pointer: None,
        }
    }
}
impl DraftUi {
    fn invoke_line(&mut self) {
        self.commands.invoke("LINE");
        self.name.clear();
        self.focus = Some(0);
    }
    pub fn show(&mut self, root: &mut egui::Ui, adapter: &str) {
        let ctx = root.ctx().clone();
        let mut requested_focus = self.focus.take();
        let ids = [Id::new("command-x"), Id::new("command-y")];
        let name_id = Id::new("command-name");
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.commands.escape();
            self.name.clear();
            self.focus = Some(0);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F6)) {
            if ctx.memory(|m| m.focused().is_some()) {
                ctx.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id);
                    }
                });
            } else {
                self.focus = Some(0);
            }
        }
        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(23, 29, 38))
                    .inner_margin(12),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("FOXCAD")
                            .size(20.0)
                            .strong()
                            .color(Color32::from_rgb(239, 166, 104)),
                    );
                    ui.separator();
                    ui.label("Drafting workspace");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("PHASE 0  /  INTEGRATION SPIKE")
                                .small()
                                .weak(),
                        );
                    });
                });
            });
        egui::Panel::bottom("status").show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label("MODEL");
                ui.separator();
                ui.label("mm");
                ui.separator();
                if let Some(p) = self.pointer {
                    ui.monospace(format!("X {:>12.3}   Y {:>12.3}", p[0], p[1]));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small(format!(
                        "{:.1}× DPI  ·  {:.3} px/mm",
                        ctx.pixels_per_point(),
                        self.camera.scale
                    ));
                });
            });
        });
        egui::Panel::bottom("command-toolbar").resizable(false).frame(egui::Frame::new().fill(Color32::from_rgb(26,33,43)).inner_margin(12)).show(root, |ui| {
            ui.horizontal(|ui| { ui.strong("COMMAND"); ui.label(self.commands.prompt()); });
            let active = self.commands.line.is_some();
            let focused = ctx.memory(|m| m.focused());
            let is_field = focused == Some(ids[0]) || focused == Some(ids[1]);
            if active && is_field {
                let tab = ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab) || i.consume_key(Modifiers::NONE, Key::Tab));
                if tab { requested_focus=Some(if focused==Some(ids[0]) {1} else {0}); }
            }
            let mut enter = false;
            ui.horizontal(|ui| {
                if active {
                    ui.strong("LINE");
                    for (index,id) in ids.iter().enumerate() {
                        ui.label(if index==0 {"X"} else {"Y"});
                        let response=ui.add(TextEdit::singleline(&mut self.commands.fields[index]).id(*id).desired_width(160.0).hint_text(if index==0 {"10  or  @10,20"} else {"10"}));
                        if requested_focus==Some(index) { response.request_focus(); }
                        if response.lost_focus() && ctx.input(|i| i.key_pressed(Key::Enter)) { enter=true; }
                    }
                    if ui.button("Accept").clicked() { enter=true; }
                    if ui.button("Finish").clicked() { self.commands.finish(); self.focus=Some(0); }
                } else {
                    let response=ui.add(TextEdit::singleline(&mut self.name).id(name_id).desired_width(350.0).hint_text("Type LINE or L…"));
                    if requested_focus.is_some() { response.request_focus(); }
                    let completion = !self.name.is_empty() && "LINE".starts_with(&self.name.to_ascii_uppercase());
                    if response.has_focus() && completion && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) { self.name="LINE".into(); }
                    if completion && ui.button("LINE  ·  Draw connected segments").clicked() { self.invoke_line(); }
                    if response.lost_focus() && ctx.input(|i| i.key_pressed(Key::Enter)) {
                        if self.commands.invoke(&self.name) { self.name.clear(); } self.focus=Some(0);
                    }
                }
            });
            if enter {
                self.commands.submit(); self.focus=Some(if self.commands.error.is_some() && self.commands.fields[1].is_empty() && !self.commands.fields[0].contains(',') {1} else {0});
                ctx.request_repaint();
            }
            if let Some(error)=&self.commands.error { ui.colored_label(Color32::from_rgb(255,158,136),error); }
            else { ui.small("Tab / Shift+Tab: fields   ·   Enter: accept   ·   Esc: clear draft, then finish   ·   F6: leave / focus command bar"); }
            egui::CollapsingHeader::new("Command history").default_open(true).show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(66.0).min_scrolled_height(66.0).stick_to_bottom(true).show(ui, |ui| {
                    if self.commands.history.is_empty() { ui.weak("Ready. Start LINE from the toolbar or command field."); }
                    for entry in &self.commands.history { ui.monospace(entry); }
                });
            });
        });
        egui::Panel::left("tools").resizable(false).default_size(198.0).frame(egui::Frame::new().fill(Color32::from_rgb(23,29,38)).inner_margin(14)).show(root, |ui| {
            ui.small("DRAW"); ui.add_space(8.0);
            if ui.add_sized([164.0,34.0],egui::Button::new("/  Line       L")).clicked() { self.invoke_line(); }
            ui.add_space(18.0); ui.small("VIEW"); ui.add_space(8.0);
            if ui.button("Reset view").clicked() { self.camera=Viewport::default(); }
            ui.checkbox(&mut self.grid,"Show grid");
            ui.add_space(18.0); ui.separator(); ui.add_space(8.0);
            ui.label("Navigate"); ui.small("Middle / right drag to pan. Scroll or pinch to zoom at the pointer. Shift+scroll to pan.");
            ui.add_space(18.0); ui.label("Temporary drawing");
            ui.small(format!("{} accepted segments",self.commands.segments.len()));
            ui.small("This spike has no save or undo. Geometry is discarded on exit.");
            ui.add_space(18.0); ui.small("SHARED GPU"); ui.small(adapter);
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(root, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
                self.rect = rect;
                let size = [rect.width() as f64, rect.height() as f64];
                if let Some(pos) = response.hover_pos().or_else(|| {
                    ctx.input(|i| i.pointer.interact_pos())
                        .filter(|_| response.dragged())
                }) {
                    let local = [(pos.x - rect.min.x) as f64, (pos.y - rect.min.y) as f64];
                    if response.dragged_by(PointerButton::Middle)
                        || response.dragged_by(PointerButton::Secondary)
                    {
                        let delta = ctx.input(|i| i.pointer.delta());
                        self.camera.pan([delta.x as f64, delta.y as f64]);
                    }
                    if response.hovered() {
                        let (scroll, shift, pinch) = ctx
                            .input(|i| (i.smooth_scroll_delta, i.modifiers.shift, i.zoom_delta()));
                        if shift {
                            self.camera.pan([scroll.x as f64, scroll.y as f64]);
                        } else {
                            self.camera.zoom(
                                (scroll.y as f64 * 0.003).exp() * pinch as f64,
                                local,
                                size,
                            );
                        }
                    }
                    self.pointer = Some(self.camera.world(local, size));
                    if response.clicked_by(PointerButton::Primary) {
                        if self.commands.has_draft() {
                            self.commands.error = Some(
                                "Submit or clear typed coordinates before picking a point.".into(),
                            );
                        } else if let Some(point) = self.pointer {
                            self.commands.accept(point);
                        }
                        self.focus = Some(0);
                        ctx.request_repaint();
                    }
                    response.on_hover_cursor(egui::CursorIcon::Crosshair);
                } else {
                    self.pointer = None;
                }
                let painter = ui.painter();
                painter.text(
                    rect.min + egui::vec2(18.0, 16.0),
                    egui::Align2::LEFT_TOP,
                    "TOP  /  2D",
                    egui::FontId::monospace(11.0),
                    Color32::from_gray(120),
                );
            });
        self.build_strokes();
    }
    fn build_strokes(&mut self) {
        self.strokes.clear();
        let size = [self.rect.width() as f64, self.rect.height() as f64];
        if size[0] <= 0.0 || size[1] <= 0.0 {
            return;
        }
        let lo = self.camera.world([0.0, size[1]], size);
        let hi = self.camera.world([size[0], 0.0], size);
        if self.grid {
            let step = 10.0_f64.powf((60.0 / self.camera.scale).log10().ceil());
            let mut x = (lo[0] / step).ceil() * step;
            for _ in 0..200 {
                if x > hi[0] {
                    break;
                }
                self.stroke([x, lo[1]], [x, hi[1]], [0.15, 0.19, 0.24, 1.0], 0.6);
                x += step;
            }
            let mut y = (lo[1] / step).ceil() * step;
            for _ in 0..200 {
                if y > hi[1] {
                    break;
                }
                self.stroke([lo[0], y], [hi[0], y], [0.15, 0.19, 0.24, 1.0], 0.6);
                y += step;
            }
        }
        self.stroke([lo[0], 0.0], [hi[0], 0.0], [0.48, 0.27, 0.28, 1.0], 1.0);
        self.stroke([0.0, lo[1]], [0.0, hi[1]], [0.24, 0.42, 0.36, 1.0], 1.0);
        for segment in self.commands.segments.clone() {
            self.stroke(segment[0], segment[1], [0.9, 0.92, 0.95, 1.0], 1.5);
        }
        if let Some(anchor) = self.commands.line.as_ref().and_then(|l| l.anchor) {
            let candidate = if self.commands.has_draft() {
                parse_point(&self.commands.fields, Some(anchor)).ok()
            } else {
                self.pointer
            };
            if let Some(point) = candidate {
                self.stroke(anchor, point, [0.96, 0.65, 0.36, 1.0], 1.5);
            }
        }
        if let Some(point) = self.pointer {
            let d = 7.0 / self.camera.scale;
            self.stroke(
                [point[0] - d, point[1]],
                [point[0] + d, point[1]],
                [0.85, 0.86, 0.9, 0.8],
                1.0,
            );
            self.stroke(
                [point[0], point[1] - d],
                [point[0], point[1] + d],
                [0.85, 0.86, 0.9, 0.8],
                1.0,
            );
        }
    }
    fn stroke(&mut self, a: [f64; 2], b: [f64; 2], color: [f32; 4], width: f32) {
        let size = [self.rect.width() as f64, self.rect.height() as f64];
        let a = self.camera.screen(a, size);
        let b = self.camera.screen(b, size);
        // Clip in f64 before casting huge world spans into GPU f32 coordinates.
        if let Some((a, b)) = clip(a, b, size) {
            self.strokes.push(Stroke {
                start: [a[0] as f32, a[1] as f32],
                end: [b[0] as f32, b[1] as f32],
                color,
                width,
            });
        }
    }
}
fn clip(a: [f64; 2], b: [f64; 2], size: [f64; 2]) -> Option<([f64; 2], [f64; 2])> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let (mut low, mut high) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-d[0], a[0] + 2.0),
        (d[0], size[0] + 2.0 - a[0]),
        (-d[1], a[1] + 2.0),
        (d[1], size[1] + 2.0 - a[1]),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                low = low.max(r);
            } else {
                high = high.min(r);
            }
            if low > high {
                return None;
            }
        }
    }
    Some((
        [a[0] + low * d[0], a[1] + low * d[1]],
        [a[0] + high * d[0], a[1] + high * d[1]],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(ctx: &egui::Context, ui: &mut DraftUi, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events,
            focused: true,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |root| ui.show(root, "test adapter"));
        output.textures_delta.clear();
    }
    fn key(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }
    #[test]
    fn tab_fields_and_enter_advance_only_after_a_complete_point() {
        let ctx = egui::Context::default();
        let mut ui = DraftUi::default();
        ui.invoke_line();
        frame(&ctx, &mut ui, vec![]);
        frame(&ctx, &mut ui, vec![egui::Event::Text("10".into())]);
        frame(&ctx, &mut ui, vec![key(Key::Tab, Modifiers::NONE)]);
        assert!(ui.commands.line.as_ref().unwrap().anchor.is_none());
        assert_eq!(ctx.memory(|m| m.focused()), Some(Id::new("command-y")));
        frame(&ctx, &mut ui, vec![egui::Event::Text("10".into())]);
        frame(&ctx, &mut ui, vec![key(Key::Enter, Modifiers::NONE)]);
        assert_eq!(
            ui.commands.line.as_ref().unwrap().anchor,
            Some([10.0, 10.0])
        );
        assert_eq!(ui.commands.prompt(), "Specify next point");
    }
    #[test]
    fn reverse_tab_returns_to_x_without_losing_y() {
        let ctx = egui::Context::default();
        let mut ui = DraftUi::default();
        ui.invoke_line();
        frame(&ctx, &mut ui, vec![]);
        frame(&ctx, &mut ui, vec![key(Key::Tab, Modifiers::NONE)]);
        frame(&ctx, &mut ui, vec![egui::Event::Text("25".into())]);
        frame(&ctx, &mut ui, vec![key(Key::Tab, Modifiers::SHIFT)]);
        assert_eq!(ctx.memory(|m| m.focused()), Some(Id::new("command-x")));
        assert_eq!(ui.commands.fields[1], "25");
    }
    #[test]
    fn typed_and_button_invocation_have_identical_presentation() {
        let ctx = egui::Context::default();
        let mut typed = DraftUi::default();
        frame(&ctx, &mut typed, vec![]);
        frame(&ctx, &mut typed, vec![egui::Event::Text("LINE".into())]);
        frame(&ctx, &mut typed, vec![key(Key::Enter, Modifiers::NONE)]);
        let mut button = DraftUi::default();
        button.invoke_line();
        assert_eq!(typed.commands.prompt(), button.commands.prompt());
        assert_eq!(typed.commands.history, button.commands.history);
        assert_eq!(typed.commands.fields, button.commands.fields);
    }
}
