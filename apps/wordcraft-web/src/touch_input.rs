//! eframe executes pointer callbacks before the matching DOM touch event. Filter those duplicate
//! document pointer callbacks, while preserving UI callbacks that need browser user activation.
use std::collections::BTreeSet;

#[derive(Default)]
struct PointerFilter {
    contacts: BTreeSet<i32>,
    pending: bool,
}

impl PointerFilter {
    fn pointer(&mut self, id: i32, down: bool, over_document: bool) {
        if down && (over_document || !self.contacts.is_empty()) {
            if self.contacts.len() < 10 {
                self.contacts.insert(id);
            }
            self.pending = true;
        } else if !down && self.contacts.remove(&id) {
            self.pending = true;
        }
    }
    fn filter(&mut self, raw: &mut egui::RawInput) {
        if std::mem::take(&mut self.pending) {
            raw.events.retain(|e| !matches!(e, egui::Event::PointerButton { button: egui::PointerButton::Primary, .. }));
        }
        if !raw.focused {
            self.contacts.clear();
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
pub(crate) struct BrowserTouch {
    filter: PointerFilter,
    document_gesture: bool,
    pending: Vec<egui::Event>,
    pub tapped: bool,
    pub area: Option<egui::Rect>,
    pub toolbar: Option<egui::Rect>,
}

#[cfg(target_arch = "wasm32")]
impl BrowserTouch {
    pub fn filter(&mut self, raw: &mut egui::RawInput) {
        self.filter.filter(raw);
        raw.events.append(&mut self.pending);
        if !raw.focused {
            self.document_gesture = false;
        }
    }

    /// Keep document navigation from invoking host keyboard-focus callbacks, while preserving
    /// real touches and synchronous browser activation for completed taps and toolbar commands.
    pub fn install(runner: &eframe::WebRunner, state: std::rc::Rc<std::cell::RefCell<Self>>) -> Result<(), wasm_bindgen::JsValue> {
        use wasm_bindgen::JsCast as _;
        let Some(document) = web_sys::window().and_then(|w| w.document()) else { return Ok(()) };
        let Some(canvas) = document.get_element_by_id("wordcraft_canvas") else { return Ok(()) };
        let options = web_sys::AddEventListenerOptions::new();
        options.set_capture(true);
        options.set_passive(false);
        for (name, down) in [("pointerdown", true), ("pointerup", false), ("pointercancel", false)] {
            let state = state.clone();
            let target: &web_sys::EventTarget = if down { canvas.as_ref() } else { document.as_ref() };
            runner.add_event_listener_ex(target, name, &options, move |event: web_sys::PointerEvent, runner, _| {
                if event.pointer_type() != "touch" {
                    return;
                }
                let bounds = runner.canvas().get_bounding_client_rect();
                let ctx = runner.egui_ctx();
                let p = egui::pos2(event.client_x() as f32 - bounds.left() as f32, event.client_y() as f32 - bounds.top() as f32) / ctx.zoom_factor();
                let mut state = state.borrow_mut();
                let background = ctx.layer_id_at(p).is_some_and(|l| l.order == egui::Order::Background);
                let over_document = state.area.is_some_and(|r| r.contains(p)) && background;
                let over = over_document || (state.toolbar.is_some_and(|r| r.contains(p)) && background);
                state.document_gesture |= down && over_document;
                state.filter.pointer(event.pointer_id(), down, over);
                if state.document_gesture {
                    // Filtering egui's duplicate mouse input is insufficient: eframe's pointer
                    // callbacks also focus the canvas and text agent before the real touch ends.
                    event.prevent_default();
                    event.stop_immediate_propagation();
                }
            })?;
        }
        for (name, phase) in [("touchend", egui::TouchPhase::End), ("touchcancel", egui::TouchPhase::Cancel)] {
            let state = state.clone();
            runner.add_event_listener_ex(document.unchecked_ref(), name, &options, move |event: web_sys::TouchEvent, runner, _| {
                {
                    let mut state = state.borrow_mut();
                    if !state.document_gesture {
                        return;
                    }
                    // Deliver the real contact endings ourselves, bypassing eframe's unconditional
                    // blur/refocus on touchend. Navigation must leave an already focused input alone.
                    let bounds = runner.canvas().get_bounding_client_rect();
                    let zoom = runner.egui_ctx().zoom_factor();
                    let changed = event.changed_touches();
                    for index in 0..changed.length().min(10) {
                        if let Some(touch) = changed.item(index) {
                            state.pending.push(egui::Event::Touch {
                                device_id: egui::TouchDeviceId(0),
                                id: egui::TouchId::from(touch.identifier()),
                                phase,
                                pos: egui::pos2(touch.client_x() as f32 - bounds.left() as f32, touch.client_y() as f32 - bounds.top() as f32) / zoom,
                                force: Some(touch.force()),
                            });
                        }
                    }
                    state.pending.push(egui::Event::PointerGone);
                    state.document_gesture = event.touches().length() != 0;
                }
                event.prevent_default();
                event.stop_immediate_propagation();
                runner.logic();
                if state.borrow().tapped
                    && let Some(input) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.active_element())
                    && input.tag_name() == "INPUT"
                    && let Ok(input) = input.dyn_into::<web_sys::HtmlElement>()
                {
                    // Safari needs a fresh focus within the completed tap's activation callback,
                    // even when dismissing its keyboard left the hidden input focused.
                    input.blur().ok();
                    let options = web_sys::FocusOptions::new();
                    options.set_prevent_scroll(true);
                    input.focus_with_options(&options).ok();
                }
                runner.logic();
            })?;
        }
        // Run after eframe's document touchend listener has queued the completed gesture. Text
        // focus/IME is then updated while the browser still grants user activation (Safari).
        runner.add_event_listener(document.unchecked_ref(), "touchend", |_: web_sys::TouchEvent, runner| {
            runner.logic();
            // Toolbar taps queue a mouse release for the next UI frame. Process it in the same
            // browser callback so picture/file commands retain user activation.
            runner.logic()
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn button(down: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: egui::pos2(100.0, 200.0),
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: Default::default(),
        }
    }
    #[test]
    fn separate_touch_pointer_frames_are_filtered_but_real_touch_and_ui_input_survive() {
        let mut f = PointerFilter::default();
        f.pointer(1, true, true);
        let mut raw = egui::RawInput { events: vec![button(true)], ..Default::default() };
        f.filter(&mut raw);
        assert!(raw.events.is_empty());
        let touch = egui::Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(1),
            phase: egui::TouchPhase::Start,
            pos: egui::pos2(100.0, 200.0),
            force: None,
        };
        raw.events = vec![button(true), touch.clone()];
        f.filter(&mut raw);
        assert_eq!(raw.events, vec![button(true), touch]);
        f.pointer(1, false, true);
        raw.events = vec![button(false)];
        f.filter(&mut raw);
        assert!(raw.events.is_empty());
        f.pointer(2, true, false);
        raw.events = vec![button(true)];
        f.filter(&mut raw);
        assert_eq!(raw.events, vec![button(true)], "UI pointer callbacks retain user activation");
    }
    #[test]
    fn a_second_finger_cannot_activate_ui_until_the_document_gesture_ends() {
        let mut f = PointerFilter::default();
        f.pointer(1, true, true);
        f.pointer(2, true, false);
        assert_eq!(f.contacts.len(), 2);
        f.pointer(1, false, false);
        f.pointer(2, false, false);
        assert!(f.contacts.is_empty());
    }

    #[test]
    fn tap_then_drag_survives_eframes_separate_pointer_callback_frames() {
        use serde_json::json;
        let ctx = egui::Context::default();
        let mut app = wordcraft_ui_egui::WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), Default::default());
        app.autosave = false;
        app.run("text.insert", json!({"text":"The quick brown fox"})).unwrap();
        app.run("caret.docStart", json!({})).unwrap();
        let mut filter = PointerFilter::default();
        let mut time = 1.0;
        let mut frame = |app: &mut wordcraft_ui_egui::WordApp, filter: &mut PointerFilter, events| {
            time += 0.02;
            let mut raw = egui::RawInput {
                events,
                time: Some(time),
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(390.0, 844.0))),
                ..Default::default()
            };
            filter.filter(&mut raw);
            app.raw_input_hook(&mut raw);
            ctx.run_ui(raw, |ui| {
                app.logic(ui.ctx());
                app.ui(ui);
            })
            .drop_without_applying_deltas();
        };
        for _ in 0..4 {
            frame(&mut app, &mut filter, vec![]);
        }
        let layout = app.session.layout();
        let point = |off| {
            let at = wordcraft_doc::Pos::body(0, off);
            let c = layout.caret_on(&at, 0).unwrap();
            app.canvas.page_rects[c.page].min + egui::vec2(c.x + 0.5, c.top + c.height * 0.5) * app.canvas.scale
        };
        let start = point(4);
        let end = point(9);
        let touch = |id, phase, pos| egui::Event::Touch { device_id: egui::TouchDeviceId(0), id: egui::TouchId(id), phase, pos, force: None };
        for id in [1, 2] {
            filter.pointer(id, true, true);
            frame(&mut app, &mut filter, vec![button(true)]);
            frame(&mut app, &mut filter, vec![touch(id as u64, egui::TouchPhase::Start, start)]);
            if id == 2 {
                frame(&mut app, &mut filter, vec![touch(2, egui::TouchPhase::Move, end)]);
            }
            filter.pointer(id, false, true);
            frame(&mut app, &mut filter, vec![button(false)]);
            frame(&mut app, &mut filter, vec![touch(id as u64, egui::TouchPhase::End, if id == 1 { start } else { end })]);
        }
        app.run("text.insert", json!({"text":"SLOW"})).unwrap();
        assert_eq!(wordcraft_engine::io::save_bytes("proof.txt", &app.session.doc).unwrap(), b"The SLOW brown fox");
    }
}
