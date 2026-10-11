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
}

type Downloads = Rc<RefCell<Vec<(String, Vec<u8>)>>>;

fn app(memory: &Memory) -> (WordApp, Downloads) {
    let downloads: Downloads = Rc::default();
    let capture = downloads.clone();
    let services = Services {
        browser_storage: Some(Box::new(memory.clone())),
        download: Some(Box::new(move |name, bytes| {
            capture.borrow_mut().push((name.into(), bytes.to_vec()));
            Ok(())
        })),
        ..Default::default()
    };
    (WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), services), downloads)
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
