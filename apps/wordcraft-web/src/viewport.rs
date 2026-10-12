//! The visible browser viewport can shrink for a software keyboard without resizing the page.
//! Keep its adapter separate from document/UI state; browser page zoom remains browser-owned.
fn visible_frame(width: f64, height: f64, top: f64, scale: f64) -> Option<(f64, f64)> {
    if ![width, height, top, scale].iter().all(|v| v.is_finite())
        || !(1.0..=100_000.0).contains(&width)
        || !(1.0..=100_000.0).contains(&height)
        || !(0.0..=100_000.0).contains(&top)
        || (scale - 1.0).abs() > 0.01
    {
        return None;
    }
    Some((height, top))
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct BrowserViewport {
    viewport: web_sys::VisualViewport,
    listener: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>,
}

#[cfg(target_arch = "wasm32")]
impl BrowserViewport {
    /// Follow the keyboard-visible browser area while leaving browser page zoom alone.
    pub fn install() -> Result<Option<Self>, wasm_bindgen::JsValue> {
        use wasm_bindgen::JsCast as _;
        let Some(window) = web_sys::window() else { return Ok(None) };
        let Some(viewport) = window.visual_viewport() else { return Ok(None) };
        let Some(host) =
            window.document().and_then(|d| d.get_element_by_id("wordcraft_host")).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Ok(None);
        };
        let update = move |v: &web_sys::VisualViewport| {
            if let Some((height, top)) = visible_frame(v.width(), v.height(), v.offset_top(), v.scale()) {
                let style = host.style();
                let result = style.set_property("height", &format!("{height}px")).and_then(|()| style.set_property("top", &format!("{top}px")));
                if let Err(e) = result {
                    log::warn!("Could not resize the editor viewport: {e:?}");
                }
            }
        };
        update(&viewport);
        let observed = viewport.clone();
        let listener = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| update(&observed)) as Box<dyn FnMut(web_sys::Event)>);
        viewport.add_event_listener_with_callback("resize", listener.as_ref().unchecked_ref())?;
        if let Err(e) = viewport.add_event_listener_with_callback("scroll", listener.as_ref().unchecked_ref()) {
            let _ = viewport.remove_event_listener_with_callback("resize", listener.as_ref().unchecked_ref());
            return Err(e);
        }
        Ok(Some(Self { viewport, listener }))
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for BrowserViewport {
    fn drop(&mut self) {
        use wasm_bindgen::JsCast as _;
        for event in ["resize", "scroll"] {
            let _ = self.viewport.remove_event_listener_with_callback(event, self.listener.as_ref().unchecked_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::visible_frame;
    #[test]
    fn keyboard_and_rotation_follow_visible_height_without_interfering_with_page_zoom() {
        assert_eq!(visible_frame(390.0, 844.0, 0.0, 1.0), Some((844.0, 0.0)));
        assert_eq!(visible_frame(390.0, 420.0, 24.0, 1.0), Some((420.0, 24.0)));
        assert_eq!(visible_frame(844.0, 390.0, 0.0, 1.0), Some((390.0, 0.0)));
        assert_eq!(visible_frame(195.0, 422.0, 0.0, 2.0), None);
        for v in [f64::NAN, f64::INFINITY, -1.0, 1e20] {
            assert_eq!(visible_frame(v, 420.0, 0.0, 1.0), None);
            assert_eq!(visible_frame(390.0, v, 0.0, 1.0), None);
        }
    }
}
