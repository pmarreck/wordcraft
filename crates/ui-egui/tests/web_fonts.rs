use std::sync::Arc;
use wordcraft_doc::{CharProps, Document, Paragraph, para_block};
use wordcraft_engine::Session;
use wordcraft_fonts::font_offer::{self, Offered};
use wordcraft_ui_egui::{Services, WordApp};

#[test]
fn requested_fonts_include_direct_runs_defaults_and_theme_without_duplicates() {
    let mut doc = Document::new();
    doc.settings.minor_font = "Carlito".into();
    doc.settings.major_font = "Merriweather".into();
    doc.styles.default_chr.font = Some("Lato".into());
    doc.body = vec![para_block(Paragraph::with_text("Direct", CharProps { font: Some("Gelasio".into()), ..Default::default() }))];
    let app = WordApp::new(Session::new(doc), Services::default());
    let used = app.used_fonts();
    for name in ["Carlito", "Merriweather", "Lato", "Gelasio"] {
        assert!(used.iter().any(|f| f == name), "{name} is needed");
    }
    let unique: std::collections::HashSet<_> = used.iter().collect();
    assert_eq!(unique.len(), used.len());
}

#[test]
fn fetched_fonts_refresh_open_layout_and_the_cached_picker_without_an_edit() {
    let mut doc = Document::new();
    doc.body = vec![para_block(Paragraph::with_text("Font arrival", CharProps::default()))];
    let mut app = WordApp::new(Session::new(doc), Services::default());
    let ctx = egui::Context::default();
    app.logic(&ctx);
    let before = app.session.layout();
    let rev = app.session.rev();
    assert!(!app.previews.families().contains(&"WordCraft Offered Family".to_string()));
    font_offer::offer(vec![Offered {
        family: "WordCraft Offered Family".into(),
        aliases: vec![],
        license: "Example/LICENSE.txt".into(),
        faces: vec![],
    }]);
    app.logic(&ctx);
    assert!(app.previews.families().contains(&"WordCraft Offered Family".to_string()));
    let after = app.session.layout();
    assert!(!Arc::ptr_eq(&before, &after), "the open document must be laid out again");
    assert_eq!(app.session.rev(), rev, "loading fonts must not edit the document");
    assert!(!app.session.dirty);
}
