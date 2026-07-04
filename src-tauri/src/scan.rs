use chrono::{DateTime, Duration, NaiveDateTime, TimeZone};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Photo,
    Video,
}

#[derive(Serialize, Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub file_name: String,
    pub size: u64,
}

/// One logical shot: an ARW+JPG pair, a lone photo, or a video clip.
#[derive(Serialize, Clone, Debug)]
pub struct MediaItem {
    pub id: String,
    pub kind: ItemKind,
    pub files: Vec<FileEntry>,
    pub capture_time: NaiveDateTime,
    pub total_bytes: u64,
    pub thumb_source: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Session {
    pub date: String,
    pub items: Vec<MediaItem>,
    pub photo_count: usize,
    pub video_count: usize,
    pub total_bytes: u64,
}

fn is_junk(name: &str) -> bool {
    name.starts_with("._") || name.starts_with('.')
}

fn exif_capture_time(path: &Path) -> Option<NaiveDateTime> {
    let file = fs::File::open(path).ok()?;
    let exif = exif::Reader::new()
        .read_from_container(&mut BufReader::new(file))
        .ok()?;
    let field = exif
        .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        .or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))?;
    NaiveDateTime::parse_from_str(&field.display_value().to_string(), "%Y-%m-%d %H:%M:%S").ok()
}

/// Sony XAVC sidecar: <CreationDate value="2026-05-29T23:40:51-07:00"/>
fn sidecar_capture_time(xml_path: &Path) -> Option<NaiveDateTime> {
    let text = fs::read_to_string(xml_path).ok()?;
    let doc = roxmltree::Document::parse(&text).ok()?;
    let value = doc
        .descendants()
        .find(|n| n.has_tag_name("CreationDate"))?
        .attribute("value")?;
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.naive_local())
}

fn mtime(path: &Path) -> Option<NaiveDateTime> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    Some(chrono::Local.timestamp_opt(
        modified
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs() as i64,
        0,
    )
    .single()?
    .naive_local())
}

fn file_entry(path: &Path) -> Option<FileEntry> {
    let size = fs::metadata(path).ok()?.len();
    if size == 0 {
        return None;
    }
    Some(FileEntry {
        path: path.to_string_lossy().into_owned(),
        file_name: path.file_name()?.to_string_lossy().into_owned(),
        size,
    })
}

// Photo formats across vendors: Sony ARW, Canon CR2/CR3, Nikon NEF/NRW,
// Fuji RAF, Olympus ORF, Panasonic RW2, Pentax PEF, Samsung SRW, plus the
// universal JPEG/HEIF/TIFF/PNG/DNG.
const PHOTO_EXTS: &[&str] = &[
    "ARW", "CR2", "CR3", "NEF", "NRW", "RAF", "ORF", "RW2", "DNG", "PEF", "SRW", "JPG", "JPEG",
    "HIF", "HEIF", "HEIC", "TIF", "TIFF", "PNG",
];
// Video formats that live inside DCIM on most non-Sony cameras.
const VIDEO_EXTS: &[&str] = &["MP4", "MOV", "M4V", "MTS", "M2TS", "AVI", "MPG", "3GP"];

/// Walk any folder of media: photos pair by stem (RAW+JPG = one item),
/// videos become individual items. Works for a DCIM tree or an arbitrary
/// folder picked by hand.
fn scan_tree(root: &Path, items: &mut Vec<MediaItem>) {
    let mut by_stem: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    let mut videos: Vec<PathBuf> = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.')
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_junk(&name) {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .map(|e| e.to_string_lossy().to_uppercase())
            .unwrap_or_default();
        if VIDEO_EXTS.contains(&ext.as_str()) {
            videos.push(entry.into_path());
            continue;
        }
        if !PHOTO_EXTS.contains(&ext.as_str()) {
            continue;
        }
        let stem = entry
            .path()
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.clone());
        by_stem.entry(stem).or_default().push(entry.into_path());
    }

    for path in videos {
        let (Some(file), Some(capture_time)) = (file_entry(&path), mtime(&path)) else {
            continue;
        };
        items.push(MediaItem {
            id: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            kind: ItemKind::Video,
            total_bytes: file.size,
            files: vec![file],
            capture_time,
            thumb_source: None,
        });
    }

    for (stem, mut paths) in by_stem {
        paths.sort();
        let files: Vec<FileEntry> = paths.iter().filter_map(|p| file_entry(p)).collect();
        if files.is_empty() {
            continue;
        }
        let jpg = paths.iter().find(|p| {
            matches!(
                p.extension().map(|e| e.to_string_lossy().to_uppercase()),
                Some(ref x) if x == "JPG" || x == "JPEG"
            )
        });
        let capture_time = jpg
            .or(paths.first())
            .and_then(|p| exif_capture_time(p))
            .or_else(|| paths.first().and_then(|p| mtime(p)));
        let Some(capture_time) = capture_time else {
            continue;
        };
        let total_bytes = files.iter().map(|f| f.size).sum();
        items.push(MediaItem {
            id: stem,
            kind: ItemKind::Photo,
            thumb_source: jpg
                .or(paths.first())
                .map(|p| p.to_string_lossy().into_owned()),
            files,
            capture_time,
            total_bytes,
        });
    }
}

fn scan_clips(card: &Path, items: &mut Vec<MediaItem>) {
    let clip_dir = card.join("PRIVATE/M4ROOT/CLIP");
    let thumb_dir = card.join("PRIVATE/M4ROOT/THMBNL");
    let Ok(entries) = fs::read_dir(&clip_dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_junk(&name) {
            continue;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_uppercase())
            .unwrap_or_default();
        if ext != "MP4" {
            continue;
        }
        let Some(file) = file_entry(&path) else {
            continue;
        };
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let sidecar = clip_dir.join(format!("{stem}M01.XML"));
        let capture_time = sidecar_capture_time(&sidecar).or_else(|| mtime(&path));
        let Some(capture_time) = capture_time else {
            continue;
        };
        let thumb = thumb_dir.join(format!("{stem}T01.JPG"));
        items.push(MediaItem {
            id: stem.into_owned(),
            kind: ItemKind::Video,
            total_bytes: file.size,
            files: vec![file],
            capture_time,
            thumb_source: thumb.is_file().then(|| thumb.to_string_lossy().into_owned()),
        });
    }
}

fn scan_avchd(card: &Path, items: &mut Vec<MediaItem>) {
    let stream_dir = card.join("PRIVATE/AVCHD/BDMV/STREAM");
    let Ok(entries) = fs::read_dir(&stream_dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_junk(&name) {
            continue;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_uppercase())
            .unwrap_or_default();
        if ext != "MTS" && ext != "M2TS" {
            continue;
        }
        let (Some(file), Some(capture_time)) = (file_entry(&path), mtime(&path)) else {
            continue;
        };
        items.push(MediaItem {
            id: path.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
            kind: ItemKind::Video,
            total_bytes: file.size,
            files: vec![file],
            capture_time,
            thumb_source: None,
        });
    }
}

pub fn scan(card: &Path) -> Vec<MediaItem> {
    let mut items = Vec::new();
    let dcim = card.join("DCIM");
    if dcim.is_dir() {
        // Camera card: standard DCIM tree plus Sony's video locations
        // (no-ops on non-Sony cards).
        scan_tree(&dcim, &mut items);
        scan_clips(card, &mut items);
        scan_avchd(card, &mut items);
    } else {
        // Hand-picked folder with no card structure: scan it directly.
        scan_tree(card, &mut items);
    }
    items.sort_by(|a, b| a.capture_time.cmp(&b.capture_time));
    items
}

/// Group items into date sessions. Times before `boundary_hour` (e.g. 4 AM)
/// count as the previous day, so shoots spanning midnight stay together.
pub fn group_sessions(items: Vec<MediaItem>, boundary_hour: u32) -> Vec<Session> {
    let mut by_date: BTreeMap<String, Vec<MediaItem>> = BTreeMap::new();
    for item in items {
        let shifted = item.capture_time - Duration::hours(boundary_hour as i64);
        by_date
            .entry(shifted.format("%Y-%m-%d").to_string())
            .or_default()
            .push(item);
    }
    by_date
        .into_iter()
        .map(|(date, items)| Session {
            photo_count: items.iter().filter(|i| i.kind == ItemKind::Photo).count(),
            video_count: items.iter().filter(|i| i.kind == ItemKind::Video).count(),
            total_bytes: items.iter().map(|i| i.total_bytes).sum(),
            date,
            items,
        })
        .collect()
}

#[tauri::command]
pub async fn scan_card(path: String, boundary_hour: u32) -> Result<Vec<Session>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let card = PathBuf::from(&path);
        if !card.is_dir() {
            return Err(format!("{path} is not a readable folder"));
        }
        Ok(group_sessions(scan(&card), boundary_hour))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, time: &str) -> MediaItem {
        MediaItem {
            id: id.to_string(),
            kind: ItemKind::Photo,
            files: vec![],
            capture_time: NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S").unwrap(),
            total_bytes: 1,
            thumb_source: None,
        }
    }

    #[test]
    fn midnight_spanning_shoot_stays_in_one_session() {
        let sessions = group_sessions(
            vec![
                item("C0008", "2026-05-29 23:41:00"),
                item("C0009", "2026-05-30 00:12:00"),
                item("C0010", "2026-05-30 03:59:59"),
            ],
            4,
        );
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].date, "2026-05-29");
        assert_eq!(sessions[0].items.len(), 3);
    }

    #[test]
    fn after_boundary_starts_new_session() {
        let sessions = group_sessions(
            vec![
                item("A", "2026-05-29 23:41:00"),
                item("B", "2026-05-30 04:00:00"),
            ],
            4,
        );
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].date, "2026-05-29");
        assert_eq!(sessions[1].date, "2026-05-30");
    }

    #[test]
    fn plain_folder_without_dcim_scans_photos_and_videos() {
        let dir = tempfile::tempdir().unwrap();
        // No EXIF in these dummy files, so capture time falls back to mtime.
        std::fs::write(dir.path().join("IMG_0001.JPG"), vec![0u8; 1000]).unwrap();
        std::fs::write(dir.path().join("IMG_0001.CR3"), vec![0u8; 2000]).unwrap();
        std::fs::write(dir.path().join("MVI_0002.MOV"), vec![0u8; 3000]).unwrap();
        std::fs::write(dir.path().join("._IMG_0001.JPG"), b"junk").unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"skip me").unwrap();

        let items = scan(dir.path());
        assert_eq!(items.len(), 2, "one CR3+JPG pair, one MOV: {items:#?}");
        let photo = items.iter().find(|i| i.kind == ItemKind::Photo).unwrap();
        assert_eq!(photo.files.len(), 2);
        assert_eq!(photo.id, "IMG_0001");
        let video = items.iter().find(|i| i.kind == ItemKind::Video).unwrap();
        assert_eq!(video.id, "MVI_0002");
        assert_eq!(video.total_bytes, 3000);
    }

    #[test]
    fn midnight_boundary_splits_by_calendar_date() {
        let sessions = group_sessions(
            vec![
                item("A", "2026-05-29 23:41:00"),
                item("B", "2026-05-30 00:12:00"),
            ],
            0,
        );
        assert_eq!(sessions.len(), 2);
    }
}
