//! Lossless browser documents. The host supplies storage; the model and commands are tested natively.
//! One atomic storage write per document, with no separate index to get out of sync.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};
use wordcraft_doc::Document;

use crate::{WordApp, file_dialogs::AfterSave};

pub const PREFIX: &str = "wordcraft.documents.v1:";
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_KEYS: usize = 4096;

/// Origin-local key/value storage. A failed `set` must leave the previous value intact.
pub trait Storage {
    fn keys(&self) -> Result<Vec<String>, String>;
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    /// Serialize the complete read/check/write sequence across tabs. Drop releases the lock.
    fn lock(&self, key: &str) -> LockFuture;
}

pub trait SaveLock {}
impl SaveLock for () {}
pub type LockFuture = Pin<Box<dyn Future<Output = Result<Box<dyn SaveLock>, String>>>>;

pub(crate) struct PendingSave {
    lock: LockFuture,
    params: Value,
    document: u64,
}

struct Repaint(egui::Context);
impl Wake for Repaint {
    fn wake(self: Arc<Self>) {
        self.0.request_repaint();
    }
}

#[derive(Serialize, Deserialize)]
struct Entry {
    schema: u32,
    revision: u64,
    /// Native document JSON keeps all model fields, unlike an interchange-format round trip.
    document: String,
}

pub(crate) struct Binding {
    pub name: String,
    revision: u64,
    document: u64,
}

pub(crate) enum Dialog {
    Open,
    Save { name: String, after: AfterSave, document: u64, overwrite: Option<u64>, message: String },
}

fn error(key: &'static str) -> String {
    let localized = tl!(key);
    if localized == key { localized.into() } else { format!("{localized} (search for: \"{key}\")") }
}

fn name(value: &str) -> Result<&str, String> {
    if value.chars().any(|c| c.is_control() || matches!(c, '/' | '\\')) {
        return Err(error("Use a document name without slashes or control characters."));
    }
    let value = value.trim();
    if value.is_empty() || matches!(value, "." | "..") || value.chars().count() > 160 {
        return Err(error("Use a document name between 1 and 160 characters."));
    }
    Ok(value)
}

fn read(storage: &dyn Storage, name: &str) -> Result<Option<(Entry, Document)>, String> {
    let Some(raw) = storage.get(&format!("{PREFIX}{}", self::name(name)?))? else { return Ok(None) };
    if raw.len() > MAX_BYTES {
        return Err(error("This browser document exceeds the 4 MiB storage limit. Export a file instead."));
    }
    let entry: Entry = serde_json::from_str(&raw).map_err(|_| error("This browser document is damaged or uses an unsupported format."))?;
    if entry.schema != 1 || entry.revision == 0 {
        return Err(error("This browser document is damaged or uses an unsupported format."));
    }
    let document = wordcraft_engine::io::open_bytes("browser.json", entry.document.as_bytes())
        .map_err(|_| error("This browser document is damaged or uses an unsupported format."))?;
    Ok(Some((entry, document)))
}

/// A bounded serializer: reject oversize documents before growing an unbounded JSON buffer.
struct Limited(Vec<u8>);
impl std::io::Write for Limited {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.len().checked_add(bytes.len()).is_none_or(|len| len > MAX_BYTES) {
            return Err(std::io::Error::other("browser document exceeds 4 MiB"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn encode(value: &impl Serialize) -> Result<String, String> {
    let mut buffer = Limited(Vec::new());
    serde_json::to_writer(&mut buffer, value).map_err(|_| error("This browser document exceeds the 4 MiB storage limit. Export a file instead."))?;
    String::from_utf8(buffer.0).map_err(|_| error("This browser document could not be encoded."))
}

pub(crate) fn list(storage: &dyn Storage) -> Result<Vec<String>, String> {
    let keys = storage.keys()?;
    if keys.len() > MAX_KEYS {
        return Err(error("There are too many browser storage entries to list."));
    }
    let mut names: Vec<String> = keys
        .into_iter()
        .filter_map(|k| k.strip_prefix(PREFIX).filter(|n| name(n).is_ok_and(|normalized| normalized == *n)).map(str::to_owned))
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

impl WordApp {
    pub(crate) fn browser_binding(&self) -> Option<&Binding> {
        self.browser_document.as_ref().filter(|b| b.document == self.session.document_id())
    }

    pub(crate) fn browser_save_dialog(&mut self, after: AfterSave) {
        let name = self.browser_binding().map(|b| b.name.clone()).unwrap_or_else(|| self.title_stem());
        self.browser_dialog = Some(Dialog::Save { name, after, document: self.session.document_id(), overwrite: None, message: String::new() });
    }

    pub(crate) fn browser_save(&mut self, params: &Value) -> Result<Value, String> {
        if self.browser_saving.is_some() {
            return Err(error("A browser save is already in progress."));
        }
        let name = self::name(params.get("name").and_then(Value::as_str).ok_or_else(|| error("A browser document name is required."))?)?;
        let storage = self.services.browser_storage.as_deref().ok_or_else(|| error("Browser document storage is unavailable."))?;
        self.browser_saving =
            Some(PendingSave { lock: storage.lock(&format!("{PREFIX}{name}")), params: params.clone(), document: self.session.document_id() });
        self.poll_browser_save()
    }

    pub(crate) fn poll_browser_save(&mut self) -> Result<Value, String> {
        let Some(pending) = self.browser_saving.as_mut() else { return Ok(Value::Null) };
        let waker = self.ctx.as_ref().map(|ctx| Waker::from(Arc::new(Repaint(ctx.clone())))).unwrap_or_else(|| Waker::noop().clone());
        let lock = match pending.lock.as_mut().poll(&mut Context::from_waker(&waker)) {
            Poll::Pending => return Ok(json!({"pending":"save"})),
            Poll::Ready(result) => result,
        };
        let Some(pending) = self.browser_saving.take() else { return Err(error("The browser save was cancelled.")) };
        let _lock = lock?;
        if pending.document != self.session.document_id() {
            return Err(error("The document changed while waiting to save. Save the current document again."));
        }
        // No await inside this critical section: edits made while waiting are included in the write.
        self.browser_save_locked(&pending.params)
    }

    fn browser_save_locked(&mut self, params: &Value) -> Result<Value, String> {
        let name = self::name(params.get("name").and_then(Value::as_str).ok_or_else(|| error("A browser document name is required."))?)?.to_owned();
        if self.session.password.is_some() {
            return Err(error("Password-protected documents must be exported as encrypted Word files instead of browser storage."));
        }
        if matches!(&self.browser_dialog, Some(Dialog::Save { document, .. }) if *document != self.session.document_id()) {
            return Err(error("The document has changed since the save dialog opened. Open Save As again."));
        }
        let expected = match params.get("revision") {
            Some(v) => Some(v.as_u64().filter(|v| *v > 0).ok_or_else(|| error("The browser document revision must be a positive integer."))?),
            None => self.browser_binding().filter(|b| b.name == name).map(|b| b.revision),
        };
        let storage = self.services.browser_storage.as_deref().ok_or_else(|| error("Browser document storage is unavailable."))?;
        let stored = read(storage, &name)?;
        let previous = stored.as_ref().map(|(entry, _)| entry.revision);
        if previous != expected {
            if !self.session.dirty
                && self.browser_binding().is_some_and(|b| b.name == name && Some(b.revision) == expected)
                && let Some((entry, document)) = stored
                && expected.is_some_and(|revision| entry.revision > revision)
            {
                self.session.set_document(document);
                self.session.path = None;
                self.autosave_path = None;
                self.browser_document = Some(Binding { name: name.clone(), revision: entry.revision, document: self.session.document_id() });
                self.browser_dialog = None;
                self.canvas.want_focus = true;
                self.status(tl!("Reloaded the newer browser document."));
                self.report_dirty();
                return Ok(json!({"reloaded":true, "name":name, "revision":entry.revision}));
            }
            return Err(error("This browser document already exists or changed in another tab. Reopen it or save with another name."));
        }
        let revision =
            previous.unwrap_or(0).checked_add(1).ok_or_else(|| error("This browser document has too many revisions. Save with another name."))?;
        let old_core = self.session.doc.core.clone();
        self.session.stamp_save();
        let result = encode(&self.session.doc)
            .and_then(|document| encode(&Entry { schema: 1, revision, document }))
            .and_then(|encoded| storage.set(&format!("{PREFIX}{name}"), &encoded));
        if let Err(e) = result {
            self.session.doc.core = old_core;
            return Err(e);
        }
        self.session.dirty = false;
        self.session.path = None;
        self.autosave_path = None;
        self.browser_document = Some(Binding { name: name.clone(), revision, document: self.session.document_id() });
        self.ui.backstage = false;
        self.canvas.want_focus = true;
        self.status(tl!("Saved in this browser."));
        let after = self.browser_dialog.take();
        if let Some(Dialog::Save { after: AfterSave::Continue { then, params, document }, .. }) = after
            && document == self.session.document_id()
        {
            self.execute_user(&then, params)?;
        }
        self.report_dirty();
        Ok(json!({"saved":true, "name":name, "revision":revision}))
    }
}

pub(crate) fn command(app: &mut WordApp, id: &str, params: &Value) -> Option<Result<Value, String>> {
    if !matches!(id, "ui.browserList" | "ui.browserOpen" | "ui.browserSave" | "ui.browserExport" | "ui.browserImport") {
        return None;
    }
    let result = (|| {
        if id == "ui.browserImport" {
            app.services.open_async.as_ref().ok_or_else(|| error("File import is unavailable."))?("document");
            app.browser_dialog = None;
            return Ok(json!({"pending":"import"}));
        }
        let storage = app.services.browser_storage.as_deref().ok_or_else(|| error("Browser document storage is unavailable."))?;
        match id {
            "ui.browserList" => Ok(json!({"names":list(storage)?})),
            "ui.browserSave" => app.browser_save(params),
            "ui.browserOpen" => {
                let name = name(params.get("name").and_then(Value::as_str).ok_or_else(|| error("A browser document name is required."))?)?.to_owned();
                let (entry, document) = read(storage, &name)?.ok_or_else(|| error("This browser document no longer exists."))?;
                app.session.set_document(document);
                app.session.path = None;
                app.autosave_path = None;
                app.browser_document = Some(Binding { name: name.clone(), revision: entry.revision, document: app.session.document_id() });
                app.browser_dialog = None;
                app.dialog = None;
                app.ui.backstage = false;
                app.canvas.want_focus = true;
                app.report_dirty();
                Ok(json!({"opened":name, "revision":entry.revision}))
            }
            _ => {
                let name = params.get("name").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| app.default_save_name());
                let bytes = wordcraft_engine::io::save_bytes_with(
                    &name,
                    &app.session.doc,
                    app.session.password.as_ref().map(wordcraft_engine::Password::as_str),
                )?;
                app.services.download.as_ref().ok_or_else(|| error("File export is unavailable."))?(&name, &bytes)?;
                app.status(crate::i18n::fmt(tl!("Exported {path}"), &[("path", &name)]));
                Ok(json!({"downloaded":name, "bytes":bytes.len()}))
            }
        }
    })();
    if let Err(e) = &result {
        app.status(e.clone());
    }
    Some(result)
}

/// File › Open's browser library. Fetch on each display so changes in other tabs appear.
pub(crate) fn library(app: &mut WordApp, ui: &mut egui::Ui) -> bool {
    let mut picked = false;
    ui.heading(tl!("This browser"));
    ui.label(tl!("Documents saved here stay in this browser profile. Export a file to keep a separate copy."));
    let names = app.services.browser_storage.as_deref().ok_or_else(|| error("Browser document storage is unavailable.")).and_then(list);
    match names {
        Ok(names) => {
            if names.is_empty() {
                ui.label(tl!("No documents saved in this browser yet."));
            }
            for name in names {
                if ui.button(&name).clicked() {
                    app.browser_dialog = None;
                    let _ = app.run("ui.browserOpen", json!({"name":name}));
                    picked = true;
                }
            }
        }
        Err(e) => {
            ui.colored_label(crate::theme::Tokens::get(ui.ctx()).text, e);
        }
    }
    picked
}

pub(crate) fn show(app: &mut WordApp, ctx: &egui::Context) {
    let Some(dialog) = app.browser_dialog.take() else { return };
    let mut open = true;
    let mut keep = true;
    let mut dialog = dialog;
    let mut save = false;
    egui::Window::new(tl!("Browser documents"))
        .id(egui::Id::new("browser_documents"))
        .collapsible(false)
        .resizable(false)
        .default_width(460.0)
        .open(&mut open)
        .show(ctx, |ui| {
            match &mut dialog {
                Dialog::Open => {
                    if egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| library(app, ui)).inner {
                        keep = false;
                    }
                    ui.separator();
                    if ui.button(tl!("Import from your computer…")).clicked() {
                        let _ = app.run("ui.browserImport", json!({}));
                        keep = false;
                    }
                }
                Dialog::Save { name, overwrite, message, .. } => {
                    ui.label(tl!("Document name"));
                    if ui.text_edit_singleline(name).changed() {
                        *overwrite = None;
                    }
                    ui.label(tl!("Save in this browser, or export a file to your computer."));
                    if !message.is_empty() {
                        ui.label(message.as_str());
                    }
                    if overwrite.is_some() {
                        ui.label(tl!("A document with this name already exists. Replace the saved copy?"));
                    }
                    let label = if overwrite.is_some() { "Replace saved document" } else { "Save in this browser" };
                    if ui.button(tl!(label)).clicked() {
                        let existing = app.services.browser_storage.as_deref().and_then(|s| read(s, name).ok().flatten()).map(|(e, _)| e.revision);
                        if existing.is_some() && overwrite.is_none() && app.browser_binding().is_none_or(|b| b.name != name.trim()) {
                            *overwrite = existing;
                        } else {
                            save = true;
                        }
                    }
                    if ui.button(tl!("Export to your computer…")).clicked() {
                        let ext = std::path::Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or_default();
                        let filename = if wordcraft_engine::io::SAVE_EXTS.contains(&ext) { name.clone() } else { format!("{name}.docx") };
                        match app.run("ui.browserExport", json!({"name":filename})) {
                            Ok(_) => {
                                keep = false;
                                app.ui.backstage = false;
                            }
                            Err(e) => *message = e,
                        }
                    }
                }
            }
            if ui.button(tl!("Cancel")).clicked() {
                keep = false;
            }
        });
    // Saving may need the continuation which belongs to this dialog, so execute after layout.
    if save && keep && open {
        let mut params = json!({});
        if let Dialog::Save { name, overwrite, .. } = &dialog {
            params["name"] = json!(name);
            if let Some(revision) = overwrite {
                params["revision"] = json!(revision);
            }
        }
        app.browser_dialog = Some(dialog);
        if let Err(e) = app.run("ui.browserSave", params)
            && let Some(Dialog::Save { message, .. }) = &mut app.browser_dialog
        {
            *message = e;
        }
    } else if keep && open {
        app.browser_dialog = Some(dialog);
    }
}
