//! Route toolbar swipes through egui's wheel path, even when a text field owns mouse drags.
//! Document gestures stay in the canvas; toolbar taps become clicks only after the finger lifts.
use egui::{Event, PointerButton, Pos2, RawInput, Rect, TouchPhase, Vec2};

#[derive(Default)]
pub(crate) struct RibbonTouch {
    pub area: Option<Rect>,
    ctx: Option<egui::Context>,
    gesture: crate::touch::Touch,
    origin: Option<Pos2>,
    release: Option<Pos2>,
    wheel_live: bool,
    time: f64,
}

impl RibbonTouch {
    pub fn register(&mut self, ctx: &egui::Context, area: Option<Rect>) {
        self.area = area;
        self.ctx = Some(ctx.clone());
        if area.is_none() {
            self.gesture.cancel();
        }
    }

    pub fn wake(&self, ctx: &egui::Context) {
        if self.release.is_some() {
            ctx.request_repaint();
        } else if self.gesture.animating() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    /// Preserve ordinary mouse and popup input, but turn a ribbon finger drag into wheel deltas
    /// anchored where it began. A queued click release is delivered in the next UI frame.
    pub fn filter(&mut self, raw: &mut RawInput) {
        let release = self.release.take();
        if self.area.is_none() {
            self.gesture.cancel();
            self.origin = None;
        }
        let now = raw.time.unwrap_or(self.time + f64::from(raw.predicted_dt));
        self.time = now;
        let start = raw.events.iter().find_map(|e| match e {
            Event::Touch { phase: TouchPhase::Start, pos, .. } => Some(*pos),
            _ => None,
        });
        let accept = start.is_some_and(|p| {
            self.area.is_some_and(|r| r.contains(p))
                && self.ctx.as_ref().is_some_and(|ctx| ctx.layer_id_at(p).is_some_and(|l| l.order == egui::Order::Background))
        });
        let beginning = accept && !self.gesture.active();
        if beginning {
            self.origin = start;
        }
        // A second tap on a toolbar can still become a swipe; selection belongs to the document.
        self.gesture.clear_tap();
        let motion = self.gesture.frame(&raw.events, now, self.area.unwrap_or(Rect::NOTHING), accept, raw.focused);
        if motion.handled {
            raw.events.retain(|e| {
                !matches!(
                    e,
                    Event::Touch { .. } | Event::PointerMoved(_) | Event::PointerGone | Event::PointerButton { button: PointerButton::Primary, .. }
                )
            });
            if beginning {
                // Native hosts may have delivered a mouse-down frame before touch-start. Move
                // outside the control before releasing so it cannot become a click.
                let away = egui::pos2(-10_000.0, -10_000.0);
                raw.events.extend([Event::PointerMoved(away), button(away, false), Event::PointerGone]);
                raw.events.push(wheel(Vec2::ZERO, TouchPhase::Start));
                self.wheel_live = true;
            }
        }
        if motion.pan.x != 0.0
            && let Some(p) = self.origin
        {
            raw.events.push(Event::PointerMoved(p));
            raw.events.push(wheel(Vec2::new(motion.pan.x, 0.0), TouchPhase::Move));
        }
        if let Some((_, p)) = motion.edit {
            raw.events.extend([Event::PointerMoved(p), button(p, true)]);
            self.release = Some(p);
        }
        if let Some(p) = release {
            let p = if raw.focused && self.area.is_some() { p } else { egui::pos2(-10_000.0, -10_000.0) };
            raw.events.extend([Event::PointerMoved(p), button(p, false), Event::PointerGone]);
        }
        if self.wheel_live && !self.gesture.active() && !self.gesture.animating() {
            // End before any real wheel input that interrupted momentum, so its delta survives.
            raw.events.insert(0, wheel(Vec2::ZERO, TouchPhase::End));
            self.wheel_live = false;
        }
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Default::default() }
}

fn wheel(delta: Vec2, phase: TouchPhase) -> Event {
    Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta, modifiers: Default::default(), phase }
}
