//! Browser adapter for the document store. Never substitute temporary memory when persistence fails.

use std::{
    cell::Cell,
    future::{Future, poll_fn},
    pin::Pin,
    rc::Rc,
    task::Poll,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use wordcraft_ui_egui::browser_documents::{LockFuture, SaveLock, Storage};

pub struct LocalStorage;

/// Returning from the Web Locks callback releases the lock. Keep its promise pending until
/// the UI completes the synchronous read/check/write critical section and drops this guard.
struct BrowserLock(js_sys::Function);
impl SaveLock for BrowserLock {}
impl Drop for BrowserLock {
    fn drop(&mut self) {
        let _ = self.0.call0(&JsValue::UNDEFINED);
    }
}

async fn exclusive(key: String) -> Result<Box<dyn SaveLock>, String> {
    let failure = |e: JsValue| format!("Browser save locking is unavailable: {e:?}. Export a file to keep your changes.");
    let window = web_sys::window().ok_or("Browser save locking is unavailable.")?;
    // Web Locks' generated web-sys types are unstable. Reflect calls this standard browser
    // API without enabling unstable APIs throughout the dependency graph or evaluating JS.
    let navigator = js_sys::Reflect::get(&window, &"navigator".into()).map_err(failure)?;
    let locks = js_sys::Reflect::get(&navigator, &"locks".into()).map_err(failure)?;
    let request: js_sys::Function = js_sys::Reflect::get(&locks, &"request".into()).map_err(failure)?.dyn_into().map_err(failure)?;
    let mut grant = None;
    let granted = js_sys::Promise::new(&mut |resolve, _| grant = Some(resolve));
    let grant = grant.ok_or("Browser save locking is unavailable.")?;
    let release = Rc::new(Cell::new(None));
    let release_slot = release.clone();
    let callback = Closure::wrap(Box::new(move |_lock: JsValue| -> js_sys::Promise {
        let held = js_sys::Promise::new(&mut |resolve, _| release_slot.set(Some(resolve)));
        let _ = grant.call0(&JsValue::UNDEFINED);
        held
    }) as Box<dyn FnMut(JsValue) -> js_sys::Promise>);
    let requested: js_sys::Promise = request.call2(&locks, &key.into(), callback.as_ref()).map_err(failure)?.dyn_into().map_err(failure)?;
    let mut requested = JsFuture::from(requested);
    let mut granted = JsFuture::from(granted);
    poll_fn(|cx| {
        if let Poll::Ready(result) = Pin::new(&mut granted).poll(cx) {
            return Poll::Ready(result.map(|_| ()).map_err(failure));
        }
        if let Poll::Ready(result) = Pin::new(&mut requested).poll(cx) {
            return Poll::Ready(Err(result.err().map(failure).unwrap_or_else(|| "The browser did not grant the save lock.".into())));
        }
        Poll::Pending
    })
    .await?;
    let release = release.take().ok_or("The browser did not grant the save lock.")?;
    Ok(Box::new(BrowserLock(release)))
}

fn storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .ok_or("Browser document storage is unavailable.")?
        .local_storage()
        .map_err(|e| format!("Browser document storage is unavailable: {e:?}"))?
        .ok_or_else(|| "Browser document storage is unavailable.".into())
}

impl Storage for LocalStorage {
    fn lock(&self, key: &str) -> LockFuture {
        Box::pin(exclusive(key.to_owned()))
    }
    fn keys(&self) -> Result<Vec<String>, String> {
        let storage = storage()?;
        let length = storage.length().map_err(|e| format!("Could not list browser documents: {e:?}"))?;
        if length > 4096 {
            return Err("There are too many browser storage entries to list.".into());
        }
        let mut keys = Vec::new();
        for i in 0..length {
            if let Some(key) = storage.key(i).map_err(|e| format!("Could not list browser documents: {e:?}"))? {
                keys.push(key);
            }
        }
        Ok(keys)
    }
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        storage()?.get_item(key).map_err(|e| format!("Could not read this browser document: {e:?}"))
    }
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        storage()?.set_item(key, value).map_err(|e| format!("Could not save in this browser: {e:?}. Export a file to keep your changes."))
    }
}
