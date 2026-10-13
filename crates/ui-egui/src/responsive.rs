//! A compact shell over the same commands and canvas as the desktop ribbon.
use egui::{Ui, Vec2, vec2};
use serde_json::json;
use wordcraft_layout::ViewMode;

use crate::{WordApp, theme::Tokens};

pub(crate) fn compact(size: Vec2) -> bool {
    size.x < 900.0
}

/// Choose a readable first view on compact screens, then preserve explicit view choices.
pub(crate) fn initialize(app: &mut WordApp, size: Vec2) {
    if app.responsive_initialized {
        return;
    }
    app.responsive_initialized = true;
    if size.x <= 600.0 || (size.x < 900.0 && size.y < 500.0) {
        let _ = app.run("view.webLayout", json!({}));
        let _ = app.run("view.zoom", json!({"value": 100}));
    } else if compact(size) {
        let _ = app.run("view.pageWidth", json!({}));
    }
}

fn command(ui: &mut Ui, app: &mut WordApp, id: &str) {
    let Some(label) = app.session.registry.get(id).map(|s| tl!(s.label).to_owned()) else { return };
    if ui.add_enabled(crate::widgets::enabled(app, id), egui::Button::new(label).min_size(vec2(0.0, 44.0))).clicked() {
        let _ = app.run(id, json!({}));
        ui.close();
    }
}

fn icon(ui: &mut Ui, app: &mut WordApp, id: &str, selected: bool) {
    let t = Tokens::get(ui.ctx());
    let label = app.session.registry.get(id).map(|s| tl!(s.label).to_owned()).unwrap_or_default();
    let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 44.0), egui::Sense::click());
    let enabled = crate::widgets::enabled(app, id);
    if selected || (resp.hovered() && enabled) {
        ui.painter().rect_filled(rect.shrink(2.0), 4.0, t.hover);
    }
    crate::icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), vec2(20.0, 20.0)),
        crate::widgets::command_icon(id),
        if enabled { t.icon } else { t.text_disabled },
        t.accent,
    );
    if resp.on_hover_text(label).clicked() && enabled {
        let _ = app.run(id, json!({}));
    }
}

pub(crate) fn show(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::top("mobile_title").exact_size(50.0).frame(egui::Frame::NONE.fill(t.title_bar).inner_margin(3)).show(ui, |ui| {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            ui.spacing_mut().interact_size.y = 44.0;
            ui.menu_button(tl!("File"), |ui| {
                for id in ["file.new", "file.open", "file.save", "file.saveAs"] {
                    command(ui, app, id);
                }
                ui.separator();
                if ui.button(tl!("Options")).clicked() {
                    let _ = app.run("ui.backstage", json!({"value": true, "page":"options"}));
                    ui.close();
                }
                if ui.button(tl!("About")).clicked() {
                    let _ = app.run("ui.dialog", json!({"name":"about"}));
                    ui.close();
                }
                if ui.button(tl!("Search Commands")).clicked() {
                    let _ = app.run("ui.dialog", json!({"name":"commands"}));
                    ui.close();
                }
            });
            let title = format!("{}{}", app.title_stem(), if app.session.dirty { " •" } else { "" });
            let width = (ui.available_width() - 135.0).max(0.0);
            ui.add_sized(vec2(width, 44.0), egui::Label::new(title).truncate());
            for id in ["file.save", "edit.undo", "edit.redo"] {
                icon(ui, app, id, false);
            }
        });
    });
    // Keep the same browsable command groups as desktop; tabs and groups scroll separately.
    crate::ribbon::show(app, ui);
    egui::Panel::bottom("mobile_status").exact_size(44.0).frame(egui::Frame::NONE.fill(t.panel).inner_margin(3)).show(ui, |ui| {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().interact_size.y = 38.0;
            let web = app.session.view.mode != ViewMode::Print;
            if ui.button(if web { tl!("Print Layout") } else { tl!("Web Layout") }).clicked() {
                let _ = app.run(if web { "view.printLayout" } else { "view.webLayout" }, json!({}));
                let _ = app.run(if web { "view.pageWidth" } else { "view.zoom" }, json!({"value":100}));
            }
            ui.label(format!("{:.0}%", app.session.view.zoom * 100.0));
            ui.label(if app.session.dirty {
                tl!("Edited")
            } else if app.session.path.is_some() || app.browser_binding().is_some() {
                tl!("Saved")
            } else {
                tl!("Not saved")
            });
            if let Some((message, _)) = &app.status_msg {
                ui.add(egui::Label::new(message).truncate());
            }
        });
    });
    crate::panes::show(app, ui);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| crate::canvas::show(app, ui));
}
