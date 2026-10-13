//! Browse the real ribbon on a narrow display without searching for command names.
use egui::accesskit::Role;
use egui::{Event, PointerButton, Pos2, TouchPhase, vec2};
use egui_kittest::kittest::Queryable as _;
use serde_json::json;
use wordcraft_ui_egui::{Services, WordApp};

type Harness = egui_kittest::Harness<'static, WordApp>;
const WIDTH: f32 = 390.0;

fn harness() -> Harness {
    let mut app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), Services::default());
    app.autosave = false;
    app.run("text.insert", json!({"text":"Toolbar swipes must preserve this writing."})).unwrap();
    app.run("select.all", json!({})).unwrap();
    let mut h = Harness::builder().with_size(vec2(WIDTH, 844.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut WordApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

fn frame(h: &mut Harness, events: Vec<Event>) {
    let mut raw = std::mem::take(h.input_mut());
    raw.events.extend(events);
    h.state_mut().raw_input_hook(&mut raw);
    *h.input_mut() = raw;
    h.run_steps(1);
}

fn pointer(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Default::default() }
}

fn touch(phase: TouchPhase, pos: Pos2) -> Event {
    Event::Touch { device_id: egui::TouchDeviceId(0), id: egui::TouchId(1), phase, pos, force: None }
}

/// Include eframe's immediate pointer callbacks before the corresponding touch callbacks.
fn drag(h: &mut Harness, from: Pos2, dx: f32, finger: bool) {
    frame(h, vec![Event::PointerMoved(from), pointer(from, true)]);
    if finger {
        frame(h, vec![touch(TouchPhase::Start, from)]);
    }
    for i in 1..=20 {
        let p = from + vec2(dx * i as f32 / 20.0, 0.0);
        let mut events = vec![Event::PointerMoved(p)];
        if finger {
            events.push(touch(TouchPhase::Move, p));
        }
        frame(h, events);
    }
    let end = from + vec2(dx, 0.0);
    for _ in 0..12 {
        frame(h, vec![Event::PointerMoved(end)]);
    }
    frame(h, vec![pointer(end, false)]);
    if finger {
        frame(h, vec![touch(TouchPhase::End, end)]);
    }
    frame(h, vec![Event::PointerGone]);
}

fn tap(h: &mut Harness, p: Pos2) {
    frame(h, vec![Event::PointerMoved(p), pointer(p, true)]);
    frame(h, vec![touch(TouchPhase::Start, p)]);
    frame(h, vec![pointer(p, false)]);
    frame(h, vec![touch(TouchPhase::End, p)]);
    frame(h, vec![Event::PointerGone]);
}

#[test]
fn every_tab_can_be_swiped_into_view_and_tapped_without_search() {
    let mut h = harness();
    let home = h.get_by_role_and_label(Role::Button, "Home").rect();
    assert!(home.height() >= 44.0);
    for tab in wordcraft_ui_egui::ribbon::TABS.into_iter().filter(|t| *t != "File") {
        for _ in 0..8 {
            let r = h.get_by_role_and_label(Role::Button, tab).rect();
            if r.left() >= 0.0 && r.right() <= WIDTH {
                break;
            }
            drag(&mut h, Pos2::new(WIDTH - 30.0, home.center().y), if r.left() < 0.0 { 280.0 } else { -280.0 }, true);
        }
        let r = h.get_by_role_and_label(Role::Button, tab).rect();
        assert!(r.left() >= 0.0 && r.right() <= WIDTH, "{tab} stays directly reachable: {r:?}");
        tap(&mut h, r.center());
        assert_eq!(h.state().ui.tab, tab);
    }
}

#[test]
fn dragging_command_groups_over_cut_scrolls_without_cutting_and_taps_still_format() {
    for finger in [false, true] {
        let mut h = harness();
        let before = serde_json::to_value(&h.state().session.doc).unwrap();
        let selection = h.state().session.sel.clone();
        let home = h.get_by_role_and_label(Role::Button, "Home").rect();
        let cut = h.get_by_role_and_label(Role::Button, "Cut").rect();
        drag(&mut h, cut.center(), -120.0, finger);
        assert!(h.get_by_role_and_label(Role::Button, "Cut").rect().left() < cut.left() - 100.0, "drag scrolls the groups");
        assert_eq!(h.get_by_role_and_label(Role::Button, "Home").rect(), home, "groups scroll independently of tabs");
        assert_eq!(serde_json::to_value(&h.state().session.doc).unwrap(), before, "swiping a control never executes it");
        assert_eq!(h.state().session.sel, selection);
        let bold = h.get_by_role_and_label(Role::Button, "Bold").rect();
        assert!(bold.left() >= 26.0 && bold.right() <= WIDTH - 26.0, "the button is clear of the overlaid edge arrows");
        tap(&mut h, bold.center());
        assert_eq!(
            h.state_mut().session.run("format.state", &json!({})).unwrap()["bold"],
            true,
            "finger={finger}, bold={bold:?}, toolbar={:?}",
            h.state().toolbar_touch_area()
        );
    }
}

#[test]
fn a_finger_drag_starting_on_the_font_field_scrolls_the_toolbar() {
    let mut h = harness();
    let font = h.query_all_by_role(Role::TextInput).map(|n| n.rect()).find(|r| r.width() > 100.0 && r.top() < 200.0).unwrap();
    let cut = h.get_by_role_and_label(Role::Button, "Cut").rect();
    let before = serde_json::to_value(&h.state().session.doc).unwrap();
    drag(&mut h, font.center(), -150.0, true);
    assert!(h.get_by_role_and_label(Role::Button, "Cut").rect().left() < cut.left() - 100.0, "swiping over a font field must browse the toolbar");
    assert_eq!(serde_json::to_value(&h.state().session.doc).unwrap(), before);
}
