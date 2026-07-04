use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Runtime};

static CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize, Debug)]
pub struct ImportPlan {
    pub destination: String,
    pub year_subfolders: bool,
    pub sessions: Vec<SessionPlan>,
}

#[derive(Deserialize, Debug)]
pub struct SessionPlan {
    pub date: String,
    pub name: String,
    pub files: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct FileResult {
    pub source: String,
    pub dest: String,
    pub status: String, // copied | skipped_duplicate | renamed | failed | cancelled
    pub verified: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ImportResult {
    pub files: Vec<FileResult>,
    pub session_dirs: Vec<String>,
    pub bytes_copied: u64,
    pub cancelled: bool,
}

#[derive(Serialize, Clone)]
struct Progress {
    file: String,
    phase: String, // copy | verify
    file_done: u64,
    file_total: u64,
    overall_done: u64,
    overall_total: u64,
}

/// Folder names come from user input; keep them safe for macOS + NFS.
fn sanitize_name(name: &str) -> String {
    name.trim()
        .replace(['/', ':'], "-")
        .trim_matches('.')
        .to_string()
}

fn session_dir(plan: &ImportPlan, session: &SessionPlan) -> PathBuf {
    let name = sanitize_name(&session.name);
    let folder = if name.is_empty() {
        session.date.clone()
    } else {
        format!("{} {}", session.date, name)
    };
    let mut dir = PathBuf::from(&plan.destination);
    if plan.year_subfolders {
        dir = dir.join(&session.date[..4]);
    }
    dir.join(folder)
}

#[tauri::command]
pub fn check_destination(path: String) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(format!(
            "{path} does not exist or is not a folder (is the NFS share mounted?)"
        ));
    }
    let probe = dir.join(".quip-write-test");
    fs::write(&probe, b"quip").map_err(|e| format!("{path} is not writable: {e}"))?;
    let _ = fs::remove_file(&probe);
    Ok(())
}

#[tauri::command]
pub fn cancel_import() {
    CANCELLED.store(true, Ordering::SeqCst);
}

const CHUNK: usize = 1024 * 1024;
const PROGRESS_EVERY: u64 = 8 * 1024 * 1024;

fn hash_file(path: &Path) -> Result<blake3::Hash, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize())
}

struct CopyOutcome {
    hash: blake3::Hash,
}

fn copy_with_hash<R: Runtime>(
    app: &AppHandle<R>,
    src: &Path,
    dst: &Path,
    overall_done: &mut u64,
    overall_total: u64,
) -> Result<CopyOutcome, String> {
    let total = fs::metadata(src).map_err(|e| e.to_string())?.len();
    let mut reader = fs::File::open(src).map_err(|e| e.to_string())?;
    let mut writer = fs::File::create(dst).map_err(|e| e.to_string())?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; CHUNK];
    let mut done: u64 = 0;
    let mut last_emit: u64 = 0;
    loop {
        if CANCELLED.load(Ordering::SeqCst) {
            drop(writer);
            let _ = fs::remove_file(dst);
            return Err("cancelled".into());
        }
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        writer.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        *overall_done += n as u64;
        if done - last_emit >= PROGRESS_EVERY || done == total {
            last_emit = done;
            let _ = app.emit(
                "import-progress",
                Progress {
                    file: src.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                    phase: "copy".into(),
                    file_done: done,
                    file_total: total,
                    overall_done: *overall_done,
                    overall_total,
                },
            );
        }
    }
    writer.sync_all().map_err(|e| e.to_string())?;
    Ok(CopyOutcome {
        hash: hasher.finalize(),
    })
}

#[tauri::command]
pub async fn import_sessions<R: Runtime>(
    app: AppHandle<R>,
    plan: ImportPlan,
) -> Result<ImportResult, String> {
    tauri::async_runtime::spawn_blocking(move || run_import(&app, plan))
        .await
        .map_err(|e| e.to_string())?
}

pub fn run_import<R: Runtime>(app: &AppHandle<R>, plan: ImportPlan) -> Result<ImportResult, String> {
    CANCELLED.store(false, Ordering::SeqCst);
    check_destination(plan.destination.clone())?;

    let overall_total: u64 = plan
        .sessions
        .iter()
        .flat_map(|s| &s.files)
        .filter_map(|f| fs::metadata(f).ok())
        .map(|m| m.len())
        .sum();
    let mut overall_done: u64 = 0;

    let mut results = Vec::new();
    let mut session_dirs = Vec::new();
    let mut bytes_copied: u64 = 0;
    let mut cancelled = false;

    'sessions: for session in &plan.sessions {
        let dir = session_dir(&plan, session);
        fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        session_dirs.push(dir.to_string_lossy().into_owned());

        for source in &session.files {
            let src = PathBuf::from(source);
            let file_name = src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if cancelled || CANCELLED.load(Ordering::SeqCst) {
                cancelled = true;
                results.push(FileResult {
                    source: source.clone(),
                    dest: String::new(),
                    status: "cancelled".into(),
                    verified: false,
                    error: None,
                });
                continue;
            }

            let src_size = match fs::metadata(&src) {
                Ok(m) => m.len(),
                Err(e) => {
                    results.push(FileResult {
                        source: source.clone(),
                        dest: String::new(),
                        status: "failed".into(),
                        verified: false,
                        error: Some(e.to_string()),
                    });
                    continue;
                }
            };

            // Collision policy: identical size → already imported; different
            // size → keep both, import under "name (2).ext".
            let mut dst = dir.join(&file_name);
            let mut status = "copied";
            if let Ok(existing) = fs::metadata(&dst) {
                if existing.len() == src_size {
                    overall_done += src_size;
                    results.push(FileResult {
                        source: source.clone(),
                        dest: dst.to_string_lossy().into_owned(),
                        status: "skipped_duplicate".into(),
                        verified: true,
                        error: None,
                    });
                    continue;
                }
                let stem = src.file_stem().unwrap_or_default().to_string_lossy();
                let ext = src.extension().unwrap_or_default().to_string_lossy();
                let mut n = 2;
                loop {
                    let candidate = dir.join(format!("{stem} ({n}).{ext}"));
                    if !candidate.exists() {
                        dst = candidate;
                        break;
                    }
                    n += 1;
                }
                status = "renamed";
            }

            match copy_with_hash(app, &src, &dst, &mut overall_done, overall_total) {
                Ok(outcome) => {
                    let _ = app.emit(
                        "import-progress",
                        Progress {
                            file: file_name.clone(),
                            phase: "verify".into(),
                            file_done: 0,
                            file_total: src_size,
                            overall_done,
                            overall_total,
                        },
                    );
                    match hash_file(&dst) {
                        Ok(dest_hash) if dest_hash == outcome.hash => {
                            bytes_copied += src_size;
                            results.push(FileResult {
                                source: source.clone(),
                                dest: dst.to_string_lossy().into_owned(),
                                status: status.into(),
                                verified: true,
                                error: None,
                            });
                        }
                        Ok(_) => {
                            let _ = fs::remove_file(&dst);
                            results.push(FileResult {
                                source: source.clone(),
                                dest: dst.to_string_lossy().into_owned(),
                                status: "failed".into(),
                                verified: false,
                                error: Some("checksum mismatch after copy".into()),
                            });
                        }
                        Err(e) => {
                            results.push(FileResult {
                                source: source.clone(),
                                dest: dst.to_string_lossy().into_owned(),
                                status: "failed".into(),
                                verified: false,
                                error: Some(format!("verification read failed: {e}")),
                            });
                        }
                    }
                }
                Err(e) if e == "cancelled" => {
                    cancelled = true;
                    results.push(FileResult {
                        source: source.clone(),
                        dest: String::new(),
                        status: "cancelled".into(),
                        verified: false,
                        error: None,
                    });
                    continue 'sessions;
                }
                Err(e) => {
                    let _ = fs::remove_file(&dst);
                    results.push(FileResult {
                        source: source.clone(),
                        dest: String::new(),
                        status: "failed".into(),
                        verified: false,
                        error: Some(e),
                    });
                }
            }
        }
    }

    Ok(ImportResult {
        files: results,
        session_dirs,
        bytes_copied,
        cancelled,
    })
}

/// Companion files the camera/macOS create per clip or photo — removed
/// together with their parent so the card ends up genuinely clean.
fn companions(path: &Path) -> Vec<PathBuf> {
    let mut extra = Vec::new();
    let Some(dir) = path.parent() else {
        return extra;
    };
    let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return extra;
    };
    extra.push(dir.join(format!("._{name}")));
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_uppercase())
        .unwrap_or_default();
    if ext == "MP4" {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let sidecar = dir.join(format!("{stem}M01.XML"));
        extra.push(dir.join(format!("._{stem}M01.XML")));
        extra.push(sidecar);
        if let Some(m4root) = dir.parent() {
            let thumb = m4root.join("THMBNL").join(format!("{stem}T01.JPG"));
            extra.push(m4root.join("THMBNL").join(format!("._{stem}T01.JPG")));
            extra.push(thumb);
        }
    }
    extra
}

#[derive(Serialize, Clone, Debug)]
pub struct DeleteResult {
    pub deleted: usize,
    pub errors: Vec<String>,
}

pub fn delete_files_impl(files: &[String]) -> DeleteResult {
    let mut deleted = 0;
    let mut errors = Vec::new();
    for file in files {
        let path = PathBuf::from(file);
        match fs::remove_file(&path) {
            Ok(()) => {
                deleted += 1;
                for extra in companions(&path) {
                    if extra.is_file() {
                        let _ = fs::remove_file(&extra);
                    }
                }
            }
            Err(e) => errors.push(format!("{file}: {e}")),
        }
    }
    DeleteResult { deleted, errors }
}

#[tauri::command]
pub async fn delete_from_card(files: Vec<String>) -> Result<DeleteResult, String> {
    tauri::async_runtime::spawn_blocking(move || Ok(delete_files_impl(&files)))
        .await
        .map_err(|e| e.to_string())?
}

/// Open the session folder in Finder (or reveal a file within its folder).
#[tauri::command]
pub fn reveal(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let status = if p.is_dir() {
        std::process::Command::new("open").arg(&path).status()
    } else {
        std::process::Command::new("open").args(["-R", &path]).status()
    };
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("open exited with {s}")),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(year_subfolders: bool) -> ImportPlan {
        ImportPlan {
            destination: "/archive".into(),
            year_subfolders,
            sessions: vec![],
        }
    }

    #[test]
    fn session_dir_with_name_and_year() {
        let s = SessionPlan {
            date: "2026-05-29".into(),
            name: "奥津宮神社".into(),
            files: vec![],
        };
        assert_eq!(
            session_dir(&plan(true), &s),
            PathBuf::from("/archive/2026/2026-05-29 奥津宮神社")
        );
    }

    #[test]
    fn session_dir_empty_name_is_just_date() {
        let s = SessionPlan {
            date: "2026-05-29".into(),
            name: "  ".into(),
            files: vec![],
        };
        assert_eq!(
            session_dir(&plan(false), &s),
            PathBuf::from("/archive/2026-05-29")
        );
    }

    #[test]
    fn sanitize_strips_path_separators() {
        assert_eq!(sanitize_name("a/b:c"), "a-b-c");
    }

    #[test]
    fn mp4_companions_include_sidecar_and_thumbnail() {
        let extras = companions(Path::new("/card/PRIVATE/M4ROOT/CLIP/C0001.MP4"));
        let strs: Vec<String> = extras.iter().map(|p| p.display().to_string()).collect();
        assert!(strs.contains(&"/card/PRIVATE/M4ROOT/CLIP/C0001M01.XML".to_string()));
        assert!(strs.contains(&"/card/PRIVATE/M4ROOT/THMBNL/C0001T01.JPG".to_string()));
        assert!(strs.contains(&"/card/PRIVATE/M4ROOT/CLIP/._C0001.MP4".to_string()));
    }
}
