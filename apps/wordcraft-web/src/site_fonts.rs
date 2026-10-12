//! Same-origin HTTP adapter for the GridCraft-compatible font catalog.
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::JsFuture;
use wordcraft_fonts::font_offer::{self, Offered};

/// Read a response incrementally, bounding allocation even without Content-Length.
async fn fetch_bytes(url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let js = |e: wasm_bindgen::JsValue| format!("{e:?}");
    let window = web_sys::window().ok_or("no window")?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str(url)).await.map_err(js)?.dyn_into().map_err(js)?;
    if !response.ok() {
        return Err(format!("HTTP {} for {url}", response.status()));
    }
    let reader: web_sys::ReadableStreamDefaultReader =
        response.body().ok_or("empty response")?.get_reader().dyn_into().map_err(|e| format!("{e:?}"))?;
    let mut bytes = Vec::new();
    loop {
        let result = JsFuture::from(reader.read()).await.map_err(js)?;
        if js_sys::Reflect::get(&result, &"done".into()).map_err(js)?.as_bool() == Some(true) {
            break;
        }
        let value = js_sys::Reflect::get(&result, &"value".into()).map_err(js)?;
        let chunk = js_sys::Uint8Array::new(&value);
        let start = bytes.len();
        let end = start.checked_add(chunk.length() as usize).filter(|end| *end <= limit);
        let Some(end) = end else {
            let _ = reader.cancel();
            return Err(format!("font response exceeds {limit} bytes"));
        };
        bytes.resize(end, 0);
        chunk.copy_to(bytes.get_mut(start..end).ok_or("invalid font response bounds")?);
    }
    Ok(bytes)
}

pub fn offer(ctx: egui::Context) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let bytes = fetch_bytes("fonts/manifest.json", font_offer::MAX_MANIFEST_BYTES).await?;
            let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            font_offer::parse_manifest(&text)
        }
        .await;
        match result {
            Ok(families) => {
                font_offer::offer(families);
                ctx.request_repaint();
            }
            Err(e) => log::warn!("font catalog unavailable; using bundled fonts: {e}"),
        }
    });
}

pub fn fetch(family: Offered, ctx: egui::Context) {
    wasm_bindgen_futures::spawn_local(async move {
        let mut files = Vec::new();
        for face in &family.faces {
            match fetch_bytes(&format!("fonts/{}", face.file), font_offer::MAX_FONT_BYTES).await {
                Ok(bytes) => files.push(bytes),
                Err(e) => log::warn!("font {} unavailable: {e}", face.file),
            }
        }
        let kept = font_offer::add_loaded(&family, files);
        if kept > 0 {
            log::info!("wordcraft-web: loaded font {} ({kept} faces)", family.family);
        } else {
            log::warn!("no usable face of {}; using bundled fallback", family.family);
        }
        ctx.request_repaint();
    });
}
