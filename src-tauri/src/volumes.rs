use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone, Debug)]
pub struct CardInfo {
    pub path: String,
    pub name: String,
    pub model: Option<String>,
}

/// Camera model from the first Sony clip sidecar, e.g. "ILCE-7M3".
fn detect_model(volume: &Path) -> Option<String> {
    let clip_dir = volume.join("PRIVATE/M4ROOT/CLIP");
    let entries = fs::read_dir(clip_dir).ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map(|e| e.to_string_lossy().to_uppercase()) != Some("XML".into()) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(doc) = roxmltree::Document::parse(&text) else {
            continue;
        };
        if let Some(device) = doc.descendants().find(|n| n.has_tag_name("Device")) {
            let make = device.attribute("manufacturer").unwrap_or_default();
            let model = device.attribute("modelName").unwrap_or_default();
            if !model.is_empty() {
                return Some(format!("{make} {model}").trim().to_string());
            }
        }
    }
    None
}

pub fn list_cards_impl() -> Vec<CardInfo> {
    let Ok(entries) = fs::read_dir("/Volumes") else {
        return vec![];
    };
    entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.join("DCIM").is_dir() {
                return None;
            }
            Some(CardInfo {
                name: entry.file_name().to_string_lossy().into_owned(),
                model: detect_model(&path),
                path: path.to_string_lossy().into_owned(),
            })
        })
        .collect()
}

#[tauri::command]
pub fn list_cards() -> Vec<CardInfo> {
    list_cards_impl()
}

/// Watch /Volumes for mounts/unmounts and push the fresh card list to the UI.
pub fn start_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(_) => return,
        };
        if watcher
            .watch(Path::new("/Volumes"), RecursiveMode::NonRecursive)
            .is_err()
        {
            return;
        }
        while rx.recv().is_ok() {
            // Debounce the burst of events a mount produces, and give the
            // volume a moment to become readable before scanning for DCIM.
            std::thread::sleep(Duration::from_millis(800));
            while rx.try_recv().is_ok() {}
            let _ = app.emit("cards-changed", list_cards_impl());
        }
    });
}

#[tauri::command]
pub fn eject_volume(path: String) -> Result<(), String> {
    let output = std::process::Command::new("diskutil")
        .args(["unmount", &path])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}
