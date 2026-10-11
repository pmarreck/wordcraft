//! Browser adapter for the document store. Never substitute temporary memory when persistence fails.

use wordcraft_ui_egui::browser_documents::Storage;

pub struct LocalStorage;

fn storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .ok_or("Browser document storage is unavailable.")?
        .local_storage()
        .map_err(|e| format!("Browser document storage is unavailable: {e:?}"))?
        .ok_or_else(|| "Browser document storage is unavailable.".into())
}

impl Storage for LocalStorage {
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
