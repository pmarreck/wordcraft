//! Owner's gesture contract through real egui input, including eframe's synthetic mouse events.
use egui::{Event, Pos2, RawInput, Rect, TouchPhase, Vec2, vec2};
use serde_json::json;
use wordcraft_engine::Session;
use wordcraft_layout::ViewMode;
use wordcraft_ui_egui::{Services, WordApp};

struct Editor {
    ctx: egui::Context,
    app: WordApp,
    size: Vec2,
    time: f64,
    ime: bool,
    interrupts_ime: bool,
    discard: bool,
    mouse_events: bool,
}

impl Editor {
    fn new(size: Vec2) -> Self {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), Services::default());
        app.autosave = false;
        app.run("text.insert", json!({"text": "The quick brown fox jumps over the lazy dog.\n".repeat(100)})).unwrap();
        app.run("caret.docStart", json!({})).unwrap();
        let mut e = Self { ctx: Default::default(), app, size, time: 1.0, ime: false, interrupts_ime: false, discard: false, mouse_events: true };
        for _ in 0..5 {
            e.frame(vec![]);
        }
        e
    }
    fn frame(&mut self, events: Vec<Event>) {
        self.time += 1.0 / 60.0;
        let mut raw = RawInput { events, time: Some(self.time), screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)), ..Default::default() };
        self.app.raw_input_hook(&mut raw);
        let output = self.ctx.run_ui(raw, |ui| {
            self.app.logic(ui.ctx());
            self.app.ui(ui);
            if self.discard && ui.ctx().current_pass_index() == 0 {
                ui.ctx().request_discard("recalculate layout during a completed tap");
            }
        });
        self.ime = output.platform_output.ime.is_some();
        self.interrupts_ime = output.platform_output.ime.is_some_and(|ime| ime.should_interrupt_composition);
        output.drop_without_applying_deltas();
    }
    fn touch(&mut self, id: u64, phase: TouchPhase, p: Pos2) {
        let mut events = vec![Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId(id), phase, pos: p, force: None }];
        if self.mouse_events {
            match phase {
                TouchPhase::Start | TouchPhase::End => events.insert(
                    0,
                    Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed: phase == TouchPhase::Start,
                        modifiers: Default::default(),
                    },
                ),
                TouchPhase::Move => events.insert(0, Event::PointerMoved(p)),
                TouchPhase::Cancel => {}
            }
        }
        self.frame(events);
    }
    fn text_point(&mut self, offset: usize) -> Pos2 {
        let mut at = self.app.session.sel.focus.clone();
        at.off = offset;
        let layout = self.app.session.layout();
        let c = layout.caret_on(&at, 0).unwrap();
        let r = self.app.canvas.page_rects[c.page];
        r.min + vec2(c.x + 0.5, c.top + c.height * 0.5) * self.app.canvas.scale
    }
}

#[test]
fn a_completed_tap_resets_old_composition_but_navigation_retains_text_input() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    // The web adapter suppresses duplicate mouse callbacks and delivers real touches.
    e.mouse_events = false;
    let p = e.text_point(4);
    e.touch(1, TouchPhase::Start, p);
    assert!(!e.interrupts_ime);
    e.touch(1, TouchPhase::End, p);
    assert!(e.interrupts_ime, "a new insertion point must reset the browser's previous composition state");
    e.frame(vec![]);
    assert!(!e.interrupts_ime, "a completed tap must interrupt composition only once");
    let p = e.app.canvas.canvas_rect.unwrap().center();
    for (phase, point) in [(TouchPhase::Start, p), (TouchPhase::Move, p - vec2(0.0, 120.0)), (TouchPhase::End, p - vec2(0.0, 120.0))] {
        e.touch(2, phase, point);
        assert!(e.ime && !e.interrupts_ime, "scrolling leaves existing keyboard/composition focus intact");
    }
}

#[test]
fn completed_tap_requests_ime_in_the_same_frame_after_controls_had_focus() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    e.ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
    e.app.canvas.focused = false;
    let p = e.text_point(4);
    e.touch(1, TouchPhase::Start, p);
    assert!(!e.ime && !e.app.canvas.touch_tapped, "a contact is not yet a tap");
    e.discard = true;
    e.touch(1, TouchPhase::End, p);
    assert!(e.ime, "Safari needs the text input request during the completed touch callback");
    assert!(e.app.canvas.touch_tapped);
    assert_eq!(e.app.session.sel.focus.off, 4);
    e.discard = false;
    e.frame(vec![]);
    assert!(!e.app.canvas.touch_tapped, "keyboard activation is a completed-tap effect, not a persistent focus flag");
}

#[test]
fn phone_reflows_without_changing_print_layout_and_respects_explicit_view() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    assert_eq!(e.app.session.view.mode, ViewMode::Web);
    let before = serde_json::to_value(&e.app.session.doc).unwrap();
    let print = e.app.session.export_layout();
    let r = e.app.canvas.page_rects[0];
    let canvas = e.app.canvas.canvas_rect.unwrap();
    assert!(r.width() <= canvas.width(), "reflow fits phone: {r:?}, {canvas:?}");
    assert!(canvas.height() > 600.0, "controls leave room to write: {canvas:?}");
    assert!((e.app.canvas.scale - 96.0 / 72.0).abs() < 0.01, "readable 100% text");
    e.size = vec2(844.0, 390.0);
    e.frame(vec![]);
    assert!(e.app.canvas.page_rects[0].width() > r.width());
    e.app.run("view.printLayout", json!({})).unwrap();
    e.frame(vec![]);
    e.size = vec2(390.0, 844.0);
    e.frame(vec![]);
    assert_eq!(e.app.session.view.mode, ViewMode::Print, "rotation preserves user's choice");
    assert_eq!(serde_json::to_value(&e.app.session.doc).unwrap(), before);
    assert_eq!(e.app.session.export_layout().pages.len(), print.pages.len());
    assert_eq!(e.app.session.export_layout().pages[0].w, print.pages[0].w);
}

#[test]
fn ordinary_touch_drag_scrolls_without_moving_selection_or_editing() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    e.ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
    e.app.canvas.focused = false;
    let p = e.app.canvas.canvas_rect.unwrap().center();
    let before = serde_json::to_value(&e.app.session.doc).unwrap();
    let sel = e.app.session.sel.clone();
    let y = e.app.canvas.page_rects[0].top();
    e.touch(1, TouchPhase::Start, p);
    assert!(!e.ime && !e.app.canvas.touch_tapped && !e.app.canvas.focused);
    e.touch(1, TouchPhase::Move, p - vec2(0.0, 120.0));
    assert!(!e.ime && !e.app.canvas.touch_tapped && !e.app.canvas.focused);
    let moved = e.app.canvas.page_rects[0].top();
    e.touch(1, TouchPhase::End, p - vec2(0.0, 120.0));
    assert!(!e.ime && !e.app.canvas.touch_tapped && !e.app.canvas.focused, "scrolling never requests text input");
    assert!(e.app.canvas.page_rects[0].top() < y - 80.0, "normal drag scrolls: {y} -> {moved} -> {}", e.app.canvas.page_rects[0].top());
    assert_eq!(e.app.session.sel, sel, "scrolling must preserve selection");
    assert_eq!(serde_json::to_value(&e.app.session.doc).unwrap(), before);
}

#[test]
fn tap_then_immediate_drag_selects_and_replacement_edits_only_that_range() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    let p = e.text_point(4);
    let end = e.text_point(9);
    let y = e.app.canvas.page_rects[0].top();
    e.touch(1, TouchPhase::Start, p);
    e.touch(1, TouchPhase::End, p);
    e.touch(2, TouchPhase::Start, p);
    e.touch(2, TouchPhase::Move, end);
    assert!(!e.app.canvas.touch_tapped, "selection motion must not reopen the keyboard");
    e.touch(2, TouchPhase::End, end);
    assert!(!e.app.canvas.touch_tapped);
    assert_eq!(e.app.session.sel.anchor.off, 4);
    assert_eq!(e.app.session.sel.focus.off, 9);
    assert!((e.app.canvas.page_rects[0].top() - y).abs() < 1.0, "selection does not pan");
    e.app.run("text.insert", json!({"text": "SLOW"})).unwrap();
    let text = wordcraft_engine::io::save_bytes("document.txt", &e.app.session.doc).unwrap();
    assert!(String::from_utf8(text).unwrap().starts_with("The SLOW brown fox"));
}

#[test]
fn two_fingers_pan_and_zoom_without_selecting_even_after_one_lifts() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    let p = e.app.canvas.canvas_rect.unwrap().center();
    let sel = e.app.session.sel.clone();
    let before = serde_json::to_value(&e.app.session.doc).unwrap();
    let scale = e.app.canvas.scale;
    e.touch(1, TouchPhase::Start, p - vec2(40.0, 0.0));
    e.touch(2, TouchPhase::Start, p + vec2(40.0, 0.0));
    e.touch(1, TouchPhase::Move, p - vec2(70.0, 50.0));
    e.touch(2, TouchPhase::Move, p + vec2(70.0, -50.0));
    assert!(e.app.canvas.scale > scale * 1.5, "pinch zooms");
    e.touch(1, TouchPhase::End, p - vec2(70.0, 50.0));
    e.touch(2, TouchPhase::Move, p + vec2(70.0, -100.0));
    e.touch(2, TouchPhase::End, p + vec2(70.0, -100.0));
    assert_eq!(e.app.session.sel, sel);
    assert_eq!(serde_json::to_value(&e.app.session.doc).unwrap(), before);
}

#[test]
fn canceled_touch_never_latches_a_selection_drag() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    let p = e.text_point(4);
    let sel = e.app.session.sel.clone();
    e.touch(1, TouchPhase::Start, p);
    e.touch(1, TouchPhase::Cancel, p);
    e.frame(vec![Event::PointerMoved(p + vec2(90.0, 20.0))]);
    assert_eq!(e.app.session.sel, sel);
    assert!(!e.app.canvas.drag_selecting());
}

#[test]
fn pinch_anchors_the_document_under_the_midpoint_and_two_finger_pan_is_one_to_one() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    let center = e.app.canvas.canvas_rect.unwrap().center();
    let point = (center - e.app.canvas.page_rects[0].min) / e.app.canvas.scale;
    let event = |id, pos| Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId(id), phase: TouchPhase::Move, pos, force: None };
    e.touch(1, TouchPhase::Start, center - vec2(40.0, 0.0));
    e.touch(2, TouchPhase::Start, center + vec2(40.0, 0.0));
    e.frame(vec![event(1, center - vec2(80.0, 0.0)), event(2, center + vec2(80.0, 0.0))]);
    let anchored = e.app.canvas.page_rects[0].min + point * e.app.canvas.scale;
    assert!(anchored.distance(center) < 1.5, "pinch midpoint stays on the same document point: {anchored:?} vs {center:?}");
    let y = e.app.canvas.page_rects[0].top();
    e.frame(vec![event(1, center - vec2(80.0, 80.0)), event(2, center + vec2(80.0, -80.0))]);
    assert!((e.app.canvas.page_rects[0].top() - y + 80.0).abs() < 1.5, "two-finger pan tracks the fingers");
}

#[test]
fn pinching_a_later_printed_page_keeps_that_page_under_the_fingers_after_release() {
    let mut e = Editor::new(vec2(768.0, 1024.0));
    e.app.run("view.printLayout", json!({})).unwrap();
    e.app.run("view.zoom", json!({"value":25})).unwrap();
    e.app.run("caret.docEnd", json!({})).unwrap();
    for _ in 0..3 {
        e.frame(vec![]);
    }
    // Leave a later page below the anchor: the bottom boundary must not prevent anchoring.
    assert!(e.app.canvas.page_rects.len() > 2);
    let page = 1;
    let center = e.app.canvas.page_rects[page].intersect(e.app.canvas.canvas_rect.unwrap()).center();
    let point = (center - e.app.canvas.page_rects[page].min) / e.app.canvas.scale;
    e.touch(1, TouchPhase::Start, center - vec2(20.0, 0.0));
    e.touch(2, TouchPhase::Start, center + vec2(20.0, 0.0));
    e.frame(
        [1, 2]
            .map(|id| Event::Touch {
                device_id: egui::TouchDeviceId(1),
                id: egui::TouchId(id),
                phase: TouchPhase::Move,
                pos: center + vec2(if id == 1 { -60.0 } else { 60.0 }, 0.0),
                force: None,
            })
            .to_vec(),
    );
    let anchored = e.app.canvas.page_rects[page].min + point * e.app.canvas.scale;
    assert!((anchored.y - center.y).abs() < 1.5, "later-page midpoint stays anchored: {anchored:?} vs {center:?}");
    e.touch(1, TouchPhase::End, center - vec2(60.0, 0.0));
    e.touch(2, TouchPhase::End, center + vec2(60.0, 0.0));
    e.frame(vec![]);
    assert!(
        (e.app.canvas.page_rects[page].min.y + point.y * e.app.canvas.scale - anchored.y).abs() < 1.5,
        "releasing a pinch must not jump back to the caret when page columns change"
    );
}

#[test]
fn a_keyboard_sized_viewport_keeps_the_insertion_point_visible() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    let p = e.text_point(4);
    e.touch(1, TouchPhase::Start, p);
    e.touch(1, TouchPhase::End, p);
    assert!(e.app.canvas.focused, "a tap focuses text input");
    e.app.run("caret.docEnd", json!({})).unwrap();
    for _ in 0..3 {
        e.frame(vec![]);
    }
    e.size = vec2(390.0, 420.0);
    for _ in 0..3 {
        e.frame(vec![]);
    }
    let layout = e.app.session.layout();
    let c = layout.caret_on(&e.app.session.sel.focus, e.app.session.page_hint).unwrap();
    let screen = e.app.canvas.page_rects[c.page].min + vec2(c.x, c.top + c.height * 0.5) * e.app.canvas.scale;
    assert!(e.app.canvas.canvas_rect.unwrap().contains(screen), "keyboard resizing keeps the caret visible: {screen:?}");
}

#[test]
fn switching_layout_after_navigation_reveals_the_caret() {
    let mut e = Editor::new(vec2(390.0, 844.0));
    let p = e.app.canvas.canvas_rect.unwrap().center();
    e.touch(1, TouchPhase::Start, p);
    e.touch(1, TouchPhase::Move, p - vec2(0.0, 200.0));
    e.touch(1, TouchPhase::End, p - vec2(0.0, 200.0));
    e.app.run("view.printLayout", json!({})).unwrap();
    e.app.run("view.pageWidth", json!({})).unwrap();
    e.frame(vec![]);
    e.app.run("view.webLayout", json!({})).unwrap();
    e.app.run("view.zoom", json!({"value":100})).unwrap();
    for _ in 0..3 {
        e.frame(vec![]);
    }
    let caret = e.text_point(0);
    assert!(e.app.canvas.canvas_rect.unwrap().contains(caret), "switching back to reflow reveals the insertion point");
}
