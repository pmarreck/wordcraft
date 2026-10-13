//! Finger gestures, independent of the synthetic mouse events emitted by browser/native hosts.
//! All coordinates and time are supplied by input; navigation never dispatches document edits.
use std::collections::BTreeMap;

use egui::{Event, Pos2, Rect, TouchDeviceId, TouchId, TouchPhase, Vec2};

const SLOP: f32 = 8.0;
const TAP_SECONDS: f64 = 0.35;
const TAP_RADIUS: f32 = 28.0;
const MAX_CONTACTS: usize = 10;

#[derive(Clone, Copy)]
enum Gesture {
    Pending { start: Pos2, at: f64, anchor: Option<Pos2> },
    Scroll { last: Pos2 },
    Select { anchor: Pos2 },
    Multi,
}

#[derive(Default)]
pub(crate) struct Touch {
    contacts: BTreeMap<(TouchDeviceId, TouchId), Pos2>,
    gesture: Option<Gesture>,
    tap: Option<(Pos2, f64)>,
    momentum: crate::scroll::Momentum,
    time: Option<f64>,
}

#[derive(Default)]
pub(crate) struct Motion {
    pub handled: bool,
    /// Content displacement in screen points.
    pub pan: Vec2,
    /// Previous and current midpoint and proportional scale, for anchored zoom.
    pub pinch: Option<(Pos2, Pos2, f32)>,
    /// None = caret placement; Some = selection anchored at the first tap.
    pub edit: Option<(Option<Pos2>, Pos2)>,
    pub selecting: bool,
}

impl Touch {
    pub fn active(&self) -> bool {
        self.gesture.is_some()
    }
    pub fn selecting(&self) -> bool {
        matches!(self.gesture, Some(Gesture::Select { .. }))
    }
    pub fn animating(&self) -> bool {
        self.momentum.is_active()
    }
    pub fn stop(&mut self) {
        self.momentum.cancel();
    }
    pub fn clear_tap(&mut self) {
        self.tap = None;
    }
    pub fn cancel(&mut self) {
        self.contacts.clear();
        self.gesture = None;
        self.tap = None;
        self.momentum.cancel();
    }

    /// Classify this frame's contacts into navigation or an explicit caret/selection gesture;
    /// synthetic mouse events cannot turn a finger scroll into an edit.
    pub fn frame(&mut self, events: &[Event], now: f64, area: Rect, accept_start: bool, focused: bool) -> Motion {
        let mut out = Motion { handled: self.active(), ..Default::default() };
        let dt = self.time.map_or(0.0, |last| (now - last).clamp(0.0, 0.1) as f32);
        self.time = Some(now);
        if !events.iter().any(|e| matches!(e, Event::Touch { .. }))
            && events
                .iter()
                .any(|e| matches!(e, Event::Key { pressed: true, .. } | Event::PointerButton { pressed: true, .. } | Event::MouseWheel { .. }))
        {
            self.momentum.cancel();
            self.tap = None;
        }
        if !focused || !now.is_finite() {
            self.cancel();
            return out;
        }
        let previous = pair(&self.contacts);
        let mut changed = false;
        let mut membership_changed = false;
        for event in events {
            let Event::Touch { device_id, id, phase, pos, .. } = event else { continue };
            let key = (*device_id, *id);
            if *phase == TouchPhase::Cancel {
                out.handled |= self.active();
                self.cancel();
                out.edit = None;
                continue;
            }
            if !pos.is_finite() {
                continue;
            }
            match phase {
                TouchPhase::Start => {
                    if self.contacts.contains_key(&key) {
                        continue;
                    }
                    if self.contacts.is_empty() {
                        if !accept_start || !area.contains(*pos) {
                            self.tap = None;
                            continue;
                        }
                        self.momentum.cancel();
                        let anchor =
                            self.tap.take().filter(|(p, at)| now >= *at && now - at <= TAP_SECONDS && p.distance(*pos) <= TAP_RADIUS).map(|(p, _)| p);
                        self.gesture = Some(Gesture::Pending { start: *pos, at: now, anchor });
                    } else {
                        self.gesture = Some(Gesture::Multi);
                        self.tap = None;
                        self.momentum.cancel();
                        out.edit = None;
                    }
                    if self.contacts.len() < MAX_CONTACTS {
                        self.contacts.insert(key, *pos);
                    }
                    membership_changed = true;
                    out.handled = true;
                }
                TouchPhase::Move | TouchPhase::End => {
                    let Some(last) = self.contacts.get_mut(&key) else { continue };
                    *last = *pos;
                    out.handled = true;
                    changed = true;
                    match self.gesture {
                        Some(Gesture::Pending { start, at, anchor }) => {
                            if start.distance(*pos) > SLOP {
                                if let Some(anchor) = anchor {
                                    self.gesture = Some(Gesture::Select { anchor });
                                    out.edit = Some((Some(anchor), *pos));
                                } else {
                                    self.gesture = Some(Gesture::Scroll { last: *pos });
                                    out.pan += *pos - start;
                                }
                                self.tap = None;
                            } else if *phase == TouchPhase::End && now - at <= TAP_SECONDS {
                                out.edit = Some((None, *pos));
                                self.tap = Some((*pos, now));
                            }
                        }
                        Some(Gesture::Scroll { last }) => {
                            out.pan += *pos - last;
                            self.gesture = Some(Gesture::Scroll { last: *pos });
                        }
                        Some(Gesture::Select { anchor }) => out.edit = Some((Some(anchor), *pos)),
                        _ => {}
                    }
                    if *phase == TouchPhase::End {
                        self.contacts.remove(&key);
                        membership_changed = true;
                        if self.contacts.is_empty() {
                            if matches!(self.gesture, Some(Gesture::Scroll { .. })) {
                                // Include the final displacement before choosing the release velocity.
                                if out.pan != Vec2::ZERO {
                                    self.momentum.feed(out.pan, dt);
                                }
                                self.momentum.release();
                            }
                            self.gesture = None;
                        }
                    }
                }
                TouchPhase::Cancel => {}
            }
        }
        if matches!(self.gesture, Some(Gesture::Multi))
            && !membership_changed
            && changed
            && let (Some((old, old_span)), Some((center, span))) = (previous, pair(&self.contacts))
        {
            let zoom = if old_span > 1.0 { (span / old_span).clamp(0.25, 4.0) } else { 1.0 };
            out.pinch = Some((old, center, zoom));
        }
        out.selecting = matches!(self.gesture, Some(Gesture::Select { .. }));
        if matches!(self.gesture, Some(Gesture::Scroll { .. })) && out.pan != Vec2::ZERO {
            self.momentum.feed(out.pan, dt);
        } else if !changed && !self.active() {
            out.pan += self.momentum.tick(dt);
        } else if out.pan == Vec2::ZERO {
            let _ = self.momentum.tick(dt);
        }
        out
    }
}

fn pair(contacts: &BTreeMap<(TouchDeviceId, TouchId), Pos2>) -> Option<(Pos2, f32)> {
    let mut points = contacts.values();
    let a = points.next()?;
    let b = points.next()?;
    Some((a.lerp(*b, 0.5), a.distance(*b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};
    fn event(id: u64, phase: TouchPhase, p: Pos2) -> Event {
        Event::Touch { device_id: TouchDeviceId(1), id: TouchId(id), phase, pos: p, force: None }
    }
    fn frame(t: &mut Touch, now: f64, events: Vec<Event>) -> Motion {
        t.frame(&events, now, Rect::from_min_size(Pos2::ZERO, vec2(400.0, 800.0)), true, true)
    }
    #[test]
    fn drag_is_scroll_and_tap_arms_only_a_near_immediate_selection() {
        for (delay, distance, select) in [(0.1, 0.0, true), (0.5, 0.0, false), (0.1, 50.0, false)] {
            let mut t = Touch::default();
            let p = pos2(100.0, 300.0);
            frame(&mut t, 1.0, vec![event(1, TouchPhase::Start, p)]);
            assert!(frame(&mut t, 1.1, vec![event(1, TouchPhase::End, p)]).edit.is_some());
            let start = p + vec2(distance, 0.0);
            frame(&mut t, 1.1 + delay, vec![event(2, TouchPhase::Start, start)]);
            let moved = frame(&mut t, 1.2 + delay, vec![event(2, TouchPhase::Move, start + vec2(0.0, -80.0))]);
            assert_eq!(moved.selecting, select);
            assert_eq!(moved.pan, if select { Vec2::ZERO } else { vec2(0.0, -80.0) });
            assert_eq!(moved.edit.is_some(), select);
        }
    }
    #[test]
    fn two_finger_translation_has_unit_zoom_and_does_not_promote_a_remaining_finger() {
        let mut t = Touch::default();
        frame(&mut t, 1.0, vec![event(1, TouchPhase::Start, pos2(100.0, 300.0)), event(2, TouchPhase::Start, pos2(200.0, 300.0))]);
        let m = frame(&mut t, 1.1, vec![event(1, TouchPhase::Move, pos2(100.0, 200.0)), event(2, TouchPhase::Move, pos2(200.0, 200.0))]);
        assert_eq!(m.pinch, Some((pos2(150.0, 300.0), pos2(150.0, 200.0), 1.0)));
        assert!(m.edit.is_none());
        frame(&mut t, 1.2, vec![event(1, TouchPhase::End, pos2(100.0, 200.0))]);
        let m = frame(&mut t, 1.3, vec![event(2, TouchPhase::Move, pos2(250.0, 250.0))]);
        assert!(m.handled && m.edit.is_none() && m.pan == Vec2::ZERO && m.pinch.is_none());
        frame(&mut t, 1.4, vec![event(2, TouchPhase::End, pos2(250.0, 250.0))]);
        assert!(!t.active());
    }
    #[test]
    fn cancellation_focus_loss_and_invalid_coordinates_cannot_arm_an_edit() {
        for phase in [TouchPhase::Cancel, TouchPhase::End] {
            let mut t = Touch::default();
            let p = pos2(100.0, 300.0);
            frame(&mut t, 1.0, vec![event(1, TouchPhase::Start, p)]);
            if phase == TouchPhase::End {
                t.frame(&[], 1.1, Rect::EVERYTHING, true, false);
            }
            let m = frame(&mut t, 1.2, vec![event(1, phase, p), event(1, TouchPhase::Move, pos2(f32::NAN, 0.0))]);
            assert!(m.edit.is_none());
            assert!(!t.active());
        }
    }
    #[test]
    fn a_touch_that_starts_outside_the_page_view_never_captures() {
        let mut t = Touch::default();
        frame(&mut t, 1.0, vec![event(1, TouchPhase::Start, pos2(-10.0, 300.0))]);
        let m = frame(&mut t, 1.1, vec![event(1, TouchPhase::Move, pos2(100.0, 300.0)), event(1, TouchPhase::End, pos2(100.0, 300.0))]);
        assert!(!m.handled && m.edit.is_none());
    }
}
