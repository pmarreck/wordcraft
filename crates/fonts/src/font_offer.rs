//! Catalog of open fonts served by the web host, compatible with GridCraft's manifest.
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OfferedFace {
    pub file: String,
    #[serde(default = "regular_weight")]
    pub weight: u16,
    #[serde(default)]
    pub italic: bool,
}
fn regular_weight() -> u16 {
    400
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Offered {
    pub family: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub license: String,
    pub faces: Vec<OfferedFace>,
}

pub const MAX_MANIFEST_BYTES: usize = 128 * 1024;
pub const MAX_FONT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
struct Manifest {
    families: Vec<Offered>,
}

/// Parse a bounded catalog. Unsafe entries are omitted; an invalid envelope is an error.
pub fn parse_manifest(text: &str) -> Result<Vec<Offered>, String> {
    if text.len() > MAX_MANIFEST_BYTES {
        return Err("font manifest exceeds 128 KiB".into());
    }
    let manifest: Manifest = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if manifest.families.len() > 64 {
        return Err("too many offered font families".into());
    }
    Ok(manifest
        .families
        .into_iter()
        .filter_map(|mut family| {
            if !safe_name(&family.family) || !safe_file(&family.license) || family.faces.len() > 16 || family.aliases.len() > 16 {
                return None;
            }
            family.aliases.retain(|alias| safe_name(alias));
            family.faces.retain(|face| {
                safe_file(&face.file) && (face.file.ends_with(".ttf") || face.file.ends_with(".otf")) && (1..=1000).contains(&face.weight)
            });
            (!family.faces.is_empty()).then_some(family)
        })
        .collect())
}

fn safe_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 160 && !name.chars().any(char::is_control)
}

// Restrict URL syntax as well as path traversal: encoded dots/slashes, queries, fragments,
// backslashes and schemes must never turn a same-origin font request into another URL.
fn safe_file(file: &str) -> bool {
    !file.is_empty()
        && file.len() <= 240
        && file.bytes().all(|c| c.is_ascii_alphanumeric() || b"/-_.".contains(&c))
        && file.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

fn resolve<'a>(offered: &'a [Offered], name: &str) -> Option<&'a Offered> {
    offered
        .iter()
        .find(|f| f.family.eq_ignore_ascii_case(name))
        .or_else(|| offered.iter().find(|f| f.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(name))))
}

/// Select needed families once, in first-use order, excluding requested families.
pub fn wanted(offered: &[Offered], used: &[String], requested: &HashSet<String>) -> Vec<Offered> {
    let mut seen = requested.clone();
    used.iter().filter_map(|name| resolve(offered, name)).filter(|family| seen.insert(family.family.to_ascii_lowercase())).cloned().collect()
}

#[derive(Default)]
struct Registry {
    offered: Vec<Offered>,
    requested: HashSet<String>,
    loaded: HashSet<String>,
    generation: u64,
}
fn registry() -> &'static RwLock<Registry> {
    static REGISTRY: OnceLock<RwLock<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Registry::default()))
}

/// Publish a host's catalog. No font bytes are fetched here.
pub fn offer(families: Vec<Offered>) {
    let mut r = registry().write().unwrap_or_else(|e| e.into_inner());
    r.offered = families;
    r.requested.clear();
    r.generation = r.generation.wrapping_add(1);
}
pub fn generation() -> u64 {
    registry().read().unwrap_or_else(|e| e.into_inner()).generation
}
pub fn take_wanted(used: &[String]) -> Vec<Offered> {
    let mut r = registry().write().unwrap_or_else(|e| e.into_inner());
    let families = wanted(&r.offered, used, &r.requested);
    r.requested.extend(families.iter().map(|f| f.family.to_ascii_lowercase()));
    families
}
pub fn offered_families() -> Vec<String> {
    registry().read().unwrap_or_else(|e| e.into_inner()).offered.iter().map(|f| f.family.clone()).collect()
}
/// Only acknowledge an alias after matching font bytes have arrived.
pub fn loaded_family(name: &str) -> Option<String> {
    let r = registry().read().unwrap_or_else(|e| e.into_inner());
    resolve(&r.offered, name).filter(|f| r.loaded.contains(&f.family.to_ascii_lowercase())).map(|f| f.family.clone())
}
/// Atomically add a family's parse-checked files. Duplicate bundled faces still count as usable.
pub fn add_loaded(family: &Offered, files: Vec<Vec<u8>>) -> usize {
    let good: Vec<_> =
        files.into_iter().take(16).filter(|bytes| bytes.len() <= MAX_FONT_BYTES && crate::fontdb::file_has_family(bytes, &family.family)).collect();
    let kept = good.len();
    if kept > 0 {
        crate::FontDb::global().add_font_files(good);
        let mut r = registry().write().unwrap_or_else(|e| e.into_inner());
        r.loaded.insert(family.family.to_ascii_lowercase());
        r.generation = r.generation.wrapping_add(1);
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    const MANIFEST: &str = r#"{"families":[
        {"family":"Carlito","aliases":["Calibri"],"license":"Carlito/LICENSE.txt","faces":[{"file":"Carlito/Regular.ttf"},{"file":"Carlito/Bold.ttf","weight":700}]},
        {"family":"Lato","license":"Lato/LICENSE.txt","faces":[{"file":"Lato/Regular.ttf"}]}
    ]}"#;

    #[test]
    fn manifest_and_demand_match_gridcraft_contract() {
        let offered = parse_manifest(MANIFEST).unwrap();
        assert_eq!(offered[0].faces[0].weight, 400);
        assert_eq!(offered[0].faces[1].weight, 700);
        let used = ["calibri", "Carlito", "Missing", "Lato", "CALIBRI"].map(String::from);
        assert_eq!(wanted(&offered, &used, &HashSet::new()), offered);
        let requested = HashSet::from(["carlito".to_string()]);
        assert_eq!(wanted(&offered, &used, &requested), offered[1..]);
        assert!(wanted(&offered, &[], &requested).is_empty());
    }

    #[test]
    fn network_paths_are_classified_as_a_set() {
        let safe = ["Carlito/a.ttf", "Liberation-Sans/Bold_Italic.otf"];
        let hostile = [
            "../a.ttf",
            "/a.ttf",
            "//evil/a.ttf",
            "https://evil/a.ttf",
            "a\\b.ttf",
            "a//b.ttf",
            "a/./b.ttf",
            "a/%2e%2e/b.ttf",
            "a.ttf?redirect=evil",
            "a.ttf#x",
            "a.woff2",
            "",
            "a/\n.ttf",
        ];
        let accepted: Vec<_> = safe
            .into_iter()
            .chain(hostile)
            .filter(|file| {
                let text =
                    serde_json::json!({"families":[{"family":"Example", "license":"Example/LICENSE.txt", "faces":[{"file":file}]}]}).to_string();
                parse_manifest(&text).is_ok_and(|v| v.len() == 1)
            })
            .collect();
        assert_eq!(accepted, safe);
    }

    #[test]
    fn malformed_or_oversized_catalogs_are_recoverable() {
        for text in ["", "[]", "not json", r#"{"families":7}"#] {
            assert!(parse_manifest(text).is_err());
        }
        assert!(parse_manifest(&" ".repeat(128 * 1024 + 1)).is_err());
    }

    #[test]
    fn arrival_replaces_memoized_fallback_and_rejects_wrong_or_corrupt_bytes() {
        let alias = "WordCraft Async Font Regression";
        let before = crate::word::resolve(alias, false, false);
        let family = Offered { family: "Source Serif 4".into(), aliases: vec![alias.into()], license: "Serif/LICENSE.txt".into(), faces: vec![] };
        offer(vec![family.clone()]);
        assert_eq!(take_wanted(&[alias.into()]).as_slice(), std::slice::from_ref(&family));
        assert!(take_wanted(&[alias.into()]).is_empty(), "an in-flight family is requested once");
        let old_generation = generation();
        assert_eq!(add_loaded(&family, vec![b"corrupt".to_vec(), crate::bundled()[0].to_vec()]), 0);
        assert_eq!(generation(), old_generation, "invalid or mismatched bytes do not publish a font");
        assert_eq!(add_loaded(&family, vec![include_bytes!("../../../assets/fonts/SourceSerif4-Regular.ttf").to_vec()]), 1);
        let after = crate::word::resolve(alias, false, false);
        assert_eq!(after.face.family, "Source Serif 4");
        assert_ne!(before.face.id(), after.face.id(), "the cached substitute must change after arrival");
        assert_eq!(crate::word::resolve(alias, true, true).face.style, "Bold Italic");
    }
}
