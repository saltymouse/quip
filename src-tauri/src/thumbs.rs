use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("thumbs");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn cache_key(path: &Path, tag: &str) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let seed = format!("{}|{}|{}|{}", path.display(), meta.len(), mtime, tag);
    Ok(blake3::hash(seed.as_bytes()).to_hex().to_string())
}

/// Pull the camera-embedded EXIF thumbnail (IFD1 JPEG) out of the first part
/// of the file — avoids decoding a 24 MP image just to draw a 160 px preview.
pub fn exif_embedded_thumb(path: &Path) -> Option<Vec<u8>> {
    let mut head = vec![0u8; 512 * 1024];
    let mut file = fs::File::open(path).ok()?;
    let n = file.read(&mut head).ok()?;
    head.truncate(n);

    // JPEG container: locate the APP1 (Exif) segment.
    if head.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut pos = 2usize;
    let tiff: &[u8] = loop {
        if head.get(pos)? != &0xFF {
            return None;
        }
        let marker = *head.get(pos + 1)?;
        let len = u16::from_be_bytes([*head.get(pos + 2)?, *head.get(pos + 3)?]) as usize;
        let body = head.get(pos + 4..pos + 2 + len)?;
        if marker == 0xE1 && body.starts_with(b"Exif\0\0") {
            break &body[6..];
        }
        // SOS means image data begins; no Exif segment found.
        if marker == 0xDA {
            return None;
        }
        pos += 2 + len;
    };

    let exif = exif::Reader::new().read_raw(tiff.to_vec()).ok()?;
    let offset = exif
        .get_field(exif::Tag::JPEGInterchangeFormat, exif::In::THUMBNAIL)?
        .value
        .get_uint(0)? as usize;
    let length = exif
        .get_field(exif::Tag::JPEGInterchangeFormatLength, exif::In::THUMBNAIL)?
        .value
        .get_uint(0)? as usize;
    tiff.get(offset..offset + length).map(|b| b.to_vec())
}

fn downscale(path: &Path, max_px: u32) -> Result<Vec<u8>, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let img = img.thumbnail(max_px, max_px);
    let mut out = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut out),
        image::ImageFormat::Jpeg,
    )
    .map_err(|e| e.to_string())?;
    Ok(out)
}

fn build(app: &AppHandle, source: &str, tag: &str, max_px: u32, fast: bool) -> Result<String, String> {
    let src = PathBuf::from(source);
    let out = cache_dir(app)?.join(format!("{}.jpg", cache_key(&src, tag)?));
    if out.is_file() {
        return Ok(out.to_string_lossy().into_owned());
    }
    let bytes = if fast {
        exif_embedded_thumb(&src)
            .map(Ok)
            .unwrap_or_else(|| downscale(&src, max_px))?
    } else {
        downscale(&src, max_px)?
    };
    fs::write(&out, bytes).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().into_owned())
}

/// Small grid thumbnail: EXIF-embedded preview when available, else decode+resize.
#[tauri::command]
pub async fn make_thumb(app: AppHandle, path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || build(&app, &path, "thumb", 480, true))
        .await
        .map_err(|e| e.to_string())?
}

/// Larger preview for the lightbox view; always a real decode.
#[tauri::command]
pub async fn make_preview(app: AppHandle, path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || build(&app, &path, "preview", 1600, false))
        .await
        .map_err(|e| e.to_string())?
}
