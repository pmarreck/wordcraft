use serde_json::json;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::{Services, WordApp};

#[derive(Clone, Default)]
struct Memory(Rc<RefCell<(BTreeMap<String, String>, bool)>>);

impl crate::browser_documents::Storage for Memory {
    fn keys(&self) -> Result<Vec<String>, String> {
        Ok(self.0.borrow().0.keys().cloned().collect())
    }
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.0.borrow().0.get(key).cloned())
    }
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        if state.1 {
            return Err("QuotaExceededError".into());
        }
        state.0.insert(key.into(), value.into());
        Ok(())
    }
    fn lock(&self, _: &str) -> crate::browser_documents::LockFuture {
        Box::pin(async { Ok(Box::new(()) as Box<dyn crate::browser_documents::SaveLock>) })
    }
}

type Downloads = Rc<RefCell<Vec<(String, Vec<u8>)>>>;

fn app(memory: &Memory) -> (WordApp, Downloads) {
    with_storage(Box::new(memory.clone()))
}

fn with_storage(storage: Box<dyn crate::browser_documents::Storage>) -> (WordApp, Downloads) {
    let downloads: Downloads = Rc::default();
    let capture = downloads.clone();
    let services = Services {
        browser_storage: Some(storage),
        download: Some(Box::new(move |name, bytes| {
            capture.borrow_mut().push((name.into(), bytes.to_vec()));
            Ok(())
        })),
        ..Default::default()
    };
    (WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), services), downloads)
}

struct Delayed {
    memory: Memory,
    ready: Rc<std::cell::Cell<bool>>,
    releases: Rc<std::cell::Cell<u32>>,
}
struct Released(Rc<std::cell::Cell<u32>>);
impl crate::browser_documents::SaveLock for Released {}
impl Drop for Released {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
impl crate::browser_documents::Storage for Delayed {
    fn keys(&self) -> Result<Vec<String>, String> {
        self.memory.keys()
    }
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        self.memory.get(key)
    }
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.memory.set(key, value)
    }
    fn lock(&self, _: &str) -> crate::browser_documents::LockFuture {
        let ready = self.ready.clone();
        let releases = self.releases.clone();
        Box::pin(std::future::poll_fn(move |_| {
            if ready.get() {
                std::task::Poll::Ready(Ok(Box::new(Released(releases.clone())) as Box<dyn crate::browser_documents::SaveLock>))
            } else {
                std::task::Poll::Pending
            }
        }))
    }
}

#[test]
fn pending_saves_stay_unsaved_and_include_edits_made_before_the_write() {
    let memory = Memory::default();
    let ready = Rc::new(std::cell::Cell::new(false));
    let releases = Rc::new(std::cell::Cell::new(0));
    let (mut a, _) = with_storage(Box::new(Delayed { memory: memory.clone(), ready: ready.clone(), releases: releases.clone() }));
    type_text(&mut a, "Before waiting");
    assert_eq!(a.execute("ui.browserSave", json!({"name":"Pending"})).unwrap()["pending"], "save");
    assert!(a.session.dirty);
    assert!(memory.0.borrow().0.is_empty());
    type_text(&mut a, "During waiting");
    ready.set(true);
    a.poll_browser_save().unwrap();
    assert!(!a.session.dirty);
    assert_eq!(releases.get(), 1);
    let (mut b, _) = app(&memory);
    b.execute("ui.browserOpen", json!({"name":"Pending"})).unwrap();
    assert!(text(&b).contains("Before waiting"));
    assert!(text(&b).contains("During waiting"));
    ready.set(false);
    type_text(&mut a, "Do not save the replacement");
    a.run("file.save", json!({})).unwrap();
    let stored = memory.0.borrow().0.clone();
    a.execute("file.new", json!({})).unwrap();
    ready.set(true);
    assert!(a.poll_browser_save().is_err());
    assert_eq!(memory.0.borrow().0, stored);
    assert_eq!(releases.get(), 2);
}

#[test]
fn saves_waiting_for_a_lock_recheck_the_revision_and_release_after_failure() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    type_text(&mut a, "Original");
    a.execute("ui.browserSave", json!({"name":"Shared"})).unwrap();
    let ready = Rc::new(std::cell::Cell::new(false));
    let releases = Rc::new(std::cell::Cell::new(0));
    let (mut b, _) = with_storage(Box::new(Delayed { memory: memory.clone(), ready: ready.clone(), releases: releases.clone() }));
    b.execute("ui.browserOpen", json!({"name":"Shared"})).unwrap();
    type_text(&mut b, "Other tab");
    b.run("file.save", json!({})).unwrap();
    type_text(&mut a, "First tab");
    a.run("file.save", json!({})).unwrap();
    let stored = memory.0.borrow().0.clone();
    ready.set(true);
    assert!(b.poll_browser_save().is_err());
    assert!(b.session.dirty);
    assert!(text(&b).contains("Other tab"));
    assert_eq!(memory.0.borrow().0, stored);
    assert_eq!(releases.get(), 1);
    memory.0.borrow_mut().1 = true;
    assert!(b.execute("ui.browserSave", json!({"name":"Another copy"})).is_err());
    assert!(b.session.dirty);
    assert_eq!(releases.get(), 2);
    assert_eq!(memory.0.borrow().0, stored);
}

#[test]
fn save_changes_waits_for_confirmation_before_replacing_the_document() {
    let memory = Memory::default();
    let ready = Rc::new(std::cell::Cell::new(true));
    let releases = Rc::new(std::cell::Cell::new(0));
    let (mut a, _) = with_storage(Box::new(Delayed { memory: memory.clone(), ready: ready.clone(), releases }));
    type_text(&mut a, "Original");
    a.execute("ui.browserSave", json!({"name":"Keep"})).unwrap();
    type_text(&mut a, "Unsaved");
    let document = a.session.document_id();
    ready.set(false);
    a.run("file.new", json!({})).unwrap();
    a.run("ui.saveChanges", json!({"answer":"save"})).unwrap();
    assert_eq!(a.session.document_id(), document, "pending is not a confirmed save");
    assert!(a.session.dirty);
    ready.set(true);
    a.poll_browser_save().unwrap();
    assert_ne!(a.session.document_id(), document);
    let (mut b, _) = app(&memory);
    b.execute("ui.browserOpen", json!({"name":"Keep"})).unwrap();
    assert!(text(&b).contains("Unsaved"));
}

fn type_text(app: &mut WordApp, text: &str) {
    app.run("text.insert", json!({"text": text})).unwrap();
}
fn text(app: &WordApp) -> String {
    app.session.doc.plain_text(wordcraft_doc::StoryRef::Body)
}

#[test]
fn browser_documents_report_unavailable_storage() {
    let mut app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), Services::default());
    let error = app.execute("ui.browserList", json!({})).unwrap_err();
    assert_eq!(error, "Browser document storage is unavailable.");
}

#[test]
fn scripted_downloads_do_not_confirm_a_browser_document_save() {
    let memory = Memory::default();
    let (mut a, downloads) = app(&memory);
    type_text(&mut a, "Stored original");
    a.execute("ui.browserSave", json!({"name":"Keep"})).unwrap();
    type_text(&mut a, "Unsaved edit");
    let stored = memory.0.borrow().0.clone();
    let snapshot = serde_json::to_value(&a.session.doc).unwrap();
    for command in ["file.save", "file.saveAs"] {
        a.execute(command, json!({"path":"export.docx"})).unwrap();
        assert!(a.session.dirty, "a download request is not persistence confirmation: {command}");
        assert_eq!(memory.0.borrow().0, stored);
        assert_eq!(serde_json::to_value(&a.session.doc).unwrap(), snapshot, "export must not stamp a save into the open model");
        assert_eq!(a.browser_binding().unwrap().name, "Keep");
    }
    assert_eq!(downloads.borrow().len(), 2);
    let (_, bytes) = downloads.borrow().last().unwrap().clone();
    let exported = wordcraft_engine::io::open_bytes("export.docx", &bytes).unwrap();
    assert!(exported.plain_text(wordcraft_doc::StoryRef::Body).contains("Unsaved edit"));
    a.run("file.save", json!({})).unwrap();
    assert!(!a.session.dirty, "an acknowledged browser write still confirms the save");
}

#[test]
fn programmatic_save_uses_browser_binding_and_never_opens_a_dialog() {
    let memory = Memory::default();
    let (mut a, downloads) = app(&memory);
    type_text(&mut a, "Unbound");
    assert!(a.execute("file.save", json!({})).is_err());
    assert!(a.browser_dialog.is_none());
    a.execute("ui.browserSave", json!({"name":"Programmatic"})).unwrap();
    type_text(&mut a, " saved edit");
    a.execute("file.save", json!({})).unwrap();
    assert!(downloads.borrow().is_empty());
    let (mut b, _) = app(&memory);
    b.execute("ui.browserOpen", json!({"name":"Programmatic"})).unwrap();
    assert!(text(&b).contains("saved edit"));
    type_text(&mut a, " private edit");
    let before = memory.0.borrow().0.clone();
    assert!(a.run("file.save", json!({"password":"secret"})).is_err());
    assert!(a.execute("file.save", json!({"password":"secret"})).is_err());
    assert_eq!(memory.0.borrow().0, before);
    assert!(a.session.dirty);
}

#[test]
fn hostile_programmatic_save_parameters_leave_the_browser_copy_unchanged() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    a.execute("ui.browserSave", json!({"name":"Keep"})).unwrap();
    type_text(&mut a, "Unsaved");
    let before = memory.0.borrow().0.clone();
    for params in [json!([]), json!(false), json!("bad"), json!(null)] {
        assert!(a.execute("file.save", params).is_err());
        assert_eq!(memory.0.borrow().0, before);
        assert!(a.session.dirty);
    }
}

#[test]
fn browser_documents_survive_a_new_session_without_losing_model_data() {
    let memory = Memory::default();
    let (mut a, downloads) = app(&memory);
    a.session.set_document(wordcraft_engine::sample::sample_document());
    type_text(&mut a, "Unicode café 世界 \u{1F331}");
    let saved = a.run("ui.browserSave", json!({"name": "Chapter one"})).unwrap();
    assert_eq!(saved["saved"], true);
    assert!(!a.session.dirty);
    assert!(downloads.borrow().is_empty());
    let snapshot = serde_json::to_value(&a.session.doc).unwrap();
    let (mut b, _) = app(&memory);
    assert_eq!(b.execute("ui.browserList", json!({})).unwrap()["names"], json!(["Chapter one"]));
    b.run("ui.browserOpen", json!({"name": "Chapter one"})).unwrap();
    assert_eq!(serde_json::to_value(&b.session.doc).unwrap(), snapshot);
    assert_eq!(b.title_stem(), "Chapter one");
    assert!(!b.session.dirty);
    type_text(&mut b, "Second revision");
    b.run("file.save", json!({})).unwrap();
    let (mut c, _) = app(&memory);
    c.execute("ui.browserOpen", json!({"name": "Chapter one"})).unwrap();
    assert!(text(&c).contains("Second revision"));
}

#[test]
fn a_failed_browser_save_keeps_the_old_copy_and_unsaved_document() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    type_text(&mut a, "Original");
    a.run("ui.browserSave", json!({"name": "Novel"})).unwrap();
    let stored = memory.0.borrow().0.clone();
    type_text(&mut a, "Unsaved edit");
    let before = serde_json::to_value(&a.session.doc).unwrap();
    memory.0.borrow_mut().1 = true;
    assert!(a.run("file.save", json!({})).is_err());
    assert_eq!(memory.0.borrow().0, stored);
    assert_eq!(serde_json::to_value(&a.session.doc).unwrap(), before, "failed saves also restore save metadata");
    assert!(a.session.dirty);
    memory.0.borrow_mut().1 = false;
    a.run("file.save", json!({})).unwrap();
    assert!(!a.session.dirty);
}

#[test]
fn a_clean_stale_tab_reloads_the_newer_copy_without_writing() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    type_text(&mut a, "Original");
    a.execute("ui.browserSave", json!({"name":"Shared"})).unwrap();
    let (mut b, _) = app(&memory);
    b.execute("ui.browserOpen", json!({"name":"Shared"})).unwrap();
    type_text(&mut a, "Newer revision");
    a.run("file.save", json!({})).unwrap();
    let stored = memory.0.borrow().0.clone();
    let expected = serde_json::to_value(&a.session.doc).unwrap();
    let result = b.run("file.save", json!({})).unwrap();
    assert_eq!(result["reloaded"], true);
    assert_eq!(result["revision"], 2);
    assert_eq!(serde_json::to_value(&b.session.doc).unwrap(), expected);
    assert_eq!(memory.0.borrow().0, stored, "refresh must not write another revision");
    assert!(!b.session.dirty);
    type_text(&mut b, "Next edit");
    b.run("file.save", json!({})).unwrap();
    a.execute("ui.browserOpen", json!({"name":"Shared"})).unwrap();
    assert!(text(&a).contains("Next edit"), "refresh updates the save binding");
}

#[test]
fn stale_tabs_and_duplicate_names_cannot_silently_replace_documents() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    type_text(&mut a, "Original");
    a.run("ui.browserSave", json!({"name": "Novel"})).unwrap();
    let (mut b, _) = app(&memory);
    b.execute("ui.browserOpen", json!({"name": "Novel"})).unwrap();
    type_text(&mut a, "First tab");
    a.run("file.save", json!({})).unwrap();
    let before = memory.0.borrow().0.clone();
    type_text(&mut b, "Other tab");
    assert!(b.run("file.save", json!({})).is_err());
    assert!(b.session.dirty);
    assert_eq!(memory.0.borrow().0, before);
    let (mut c, _) = app(&memory);
    assert!(c.run("ui.browserSave", json!({"name": "Novel"})).is_err());
    assert_eq!(memory.0.borrow().0, before);
    b.run("ui.browserSave", json!({"name": "Other copy"})).unwrap();
    assert_eq!(b.execute("ui.browserList", json!({})).unwrap()["names"], json!(["Novel", "Other copy"]));
}

#[test]
fn corrupt_unsupported_or_missing_documents_leave_the_current_document_intact() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    type_text(&mut a, "Keep this");
    let before = serde_json::to_value(&a.session.doc).unwrap();
    for raw in ["not JSON", r#"{"schema":99,"revision":1,"document":"{}"}"#, r#"{"schema":1,"revision":1,"document":"{broken}"}"#] {
        memory.0.borrow_mut().0.insert(format!("{}Broken", crate::browser_documents::PREFIX), raw.into());
        assert!(a.execute("ui.browserOpen", json!({"name": "Broken"})).is_err());
        assert!(a.execute("ui.browserSave", json!({"name": "Broken", "revision":1})).is_err());
        assert_eq!(serde_json::to_value(&a.session.doc).unwrap(), before);
        assert!(a.session.dirty);
    }
    assert!(a.execute("ui.browserOpen", json!({"name": "Missing"})).is_err());
}

#[test]
fn browser_names_are_bounded_and_namespaced() {
    let memory = Memory::default();
    memory.0.borrow_mut().0.insert("eframe.preferences".into(), "unrelated".into());
    let (mut a, _) = app(&memory);
    for name in ["", " ", ".", "..", "../novel", "a/b", "a\\b", "a\n", "a\0"] {
        assert!(a.execute("ui.browserSave", json!({"name": name})).is_err(), "{name:?}");
    }
    assert!(a.execute("ui.browserSave", json!({"name": "a".repeat(161)})).is_err());
    for name in ["a", "Résumé", "世界 \u{1F331}", "Chapter 1.docx"] {
        a.execute("ui.browserSave", json!({"name": name})).unwrap();
    }
    assert_eq!(a.execute("ui.browserList", json!({})).unwrap()["names"].as_array().unwrap().len(), 4);
    assert_eq!(memory.0.borrow().0.get("eframe.preferences").unwrap(), "unrelated");
}

#[test]
fn imports_can_be_saved_in_browser_and_exports_keep_the_browser_binding_and_dirty_state() {
    let memory = Memory::default();
    let (mut a, downloads) = app(&memory);
    a.run("file.open", json!({"path":"import.txt", "data":wordcraft_engine::cmd::insert::base64_encode(b"Imported from disk")})).unwrap();
    a.run("ui.browserSave", json!({"name": "Imported"})).unwrap();
    type_text(&mut a, "Pending edit");
    let snapshot = serde_json::to_value(&a.session.doc).unwrap();
    a.run("ui.browserExport", json!({"name":"export.docx"})).unwrap();
    assert!(a.session.dirty, "exporting a copy does not claim the browser document was saved");
    assert_eq!(serde_json::to_value(&a.session.doc).unwrap(), snapshot);
    let (name, bytes) = downloads.borrow().last().unwrap().clone();
    assert_eq!(name, "export.docx");
    let exported = wordcraft_engine::io::open_bytes(&name, &bytes).unwrap();
    assert!(exported.plain_text(wordcraft_doc::StoryRef::Body).contains("Imported from disk"));
    assert!(exported.plain_text(wordcraft_doc::StoryRef::Body).contains("Pending edit"));
    a.run("file.save", json!({})).unwrap();
    assert_eq!(downloads.borrow().len(), 1, "Save still goes to the browser document");
    a.run("file.new", json!({})).unwrap();
    type_text(&mut a, "Another document");
    a.run("file.save", json!({})).unwrap();
    assert!(a.browser_dialog.is_some(), "a new document asks where to save");
}

#[test]
fn browser_open_and_save_changes_wait_until_work_is_safely_saved() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    type_text(&mut a, "Saved document");
    a.run("ui.browserSave", json!({"name":"Stored"})).unwrap();
    a.run("file.new", json!({})).unwrap();
    type_text(&mut a, "Unsaved chapter");
    assert_eq!(a.run("ui.browserOpen", json!({"name":"Stored"})).unwrap()["pending"], "saveChanges");
    a.run("ui.saveChanges", json!({"answer":"cancel"})).unwrap();
    assert!(text(&a).contains("Unsaved chapter"));
    a.run("ui.browserOpen", json!({"name":"Stored"})).unwrap();
    a.run("ui.saveChanges", json!({"answer":"save"})).unwrap();
    assert!(a.browser_dialog.is_some());
    assert!(text(&a).contains("Unsaved chapter"));
    a.run("ui.browserSave", json!({"name":"Chapter"})).unwrap();
    assert!(text(&a).contains("Saved document"));
    let (mut b, _) = app(&memory);
    b.execute("ui.browserOpen", json!({"name":"Chapter"})).unwrap();
    assert!(text(&b).contains("Unsaved chapter"));
}

#[test]
fn browser_storage_never_persists_a_password_protected_document_in_plaintext() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    type_text(&mut a, "Secret chapter");
    a.run("file.encrypt", json!({"password":"test-password"})).unwrap();
    assert!(a.run("ui.browserSave", json!({"name":"Private"})).is_err());
    assert!(memory.0.borrow().0.is_empty());
    assert!(a.session.dirty);
}

#[test]
fn oversized_documents_and_exhausted_revisions_fail_without_writing() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    a.session.doc = wordcraft_doc::Document::from_text(&"x".repeat(crate::browser_documents::MAX_BYTES));
    a.session.dirty = true;
    assert!(a.execute("ui.browserSave", json!({"name":"Too large"})).is_err());
    assert!(memory.0.borrow().0.is_empty());
    assert!(a.session.dirty);
    let (mut b, _) = app(&memory);
    b.execute("ui.browserSave", json!({"name":"Full"})).unwrap();
    let key = format!("{}Full", crate::browser_documents::PREFIX);
    let mut entry: serde_json::Value = serde_json::from_str(memory.0.borrow().0.get(&key).unwrap()).unwrap();
    entry["revision"] = json!(u64::MAX);
    memory.0.borrow_mut().0.insert(key, entry.to_string());
    let before = memory.0.borrow().0.clone();
    assert!(b.execute("ui.browserSave", json!({"name":"Full", "revision":u64::MAX})).is_err());
    assert_eq!(memory.0.borrow().0, before);
}

#[test]
fn malformed_storage_command_parameters_never_panic_or_write() {
    let memory = Memory::default();
    let (mut a, _) = app(&memory);
    for params in [
        json!(null),
        json!([]),
        json!({}),
        json!({"name":5}),
        json!({"name":[]}),
        json!({"name":"Test", "revision":0}),
        json!({"name":"Test", "revision":-1}),
        json!({"name":"Test", "revision":"1"}),
    ] {
        assert!(a.execute("ui.browserSave", params).is_err());
    }
    assert!(memory.0.borrow().0.is_empty());
}
