//! End-to-end import flow against a fake Sony card layout on disk.

use quip_lib::import::{delete_files_impl, run_import, ImportPlan, SessionPlan};
use std::fs;
use std::path::Path;

fn fake_card(root: &Path) -> (Vec<String>, String) {
    let dcim = root.join("DCIM/100MSDCF");
    let clip = root.join("PRIVATE/M4ROOT/CLIP");
    let thmb = root.join("PRIVATE/M4ROOT/THMBNL");
    fs::create_dir_all(&dcim).unwrap();
    fs::create_dir_all(&clip).unwrap();
    fs::create_dir_all(&thmb).unwrap();

    let jpg = dcim.join("DSC00001.JPG");
    let arw = dcim.join("DSC00001.ARW");
    let mp4 = clip.join("C0001.MP4");
    fs::write(&jpg, vec![0xAAu8; 300_000]).unwrap();
    fs::write(&arw, vec![0xBBu8; 900_000]).unwrap();
    fs::write(&mp4, vec![0xCCu8; 2_000_000]).unwrap();
    // Companions that must survive import but vanish with cleanup:
    fs::write(dcim.join("._DSC00001.JPG"), b"junk").unwrap();
    fs::write(clip.join("C0001M01.XML"), b"<xml/>").unwrap();
    fs::write(thmb.join("C0001T01.JPG"), b"thumbjpeg").unwrap();

    let files = vec![
        jpg.display().to_string(),
        arw.display().to_string(),
        mp4.display().to_string(),
    ];
    (files, root.display().to_string())
}

#[test]
fn import_verify_reimport_and_cleanup() {
    let card = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let (files, _) = fake_card(card.path());

    let app = tauri::test::mock_app();
    let plan = |files: Vec<String>| ImportPlan {
        destination: dest.path().display().to_string(),
        folder_pattern: "{year}/{month}/{date} {name}".into(),
        sessions: vec![SessionPlan {
            date: "2026-05-29".into(),
            name: "テスト/セッション".into(), // slash must be sanitized
            files,
        }],
    };

    // First import: everything copies and verifies.
    let result = run_import(app.handle(), plan(files.clone())).unwrap();
    assert!(!result.cancelled);
    assert_eq!(result.files.len(), 3);
    assert!(result.files.iter().all(|f| f.status == "copied" && f.verified));

    let session_dir = dest
        .path()
        .join("2026/05/2026-05-29 テスト-セッション");
    assert!(session_dir.join("DSC00001.JPG").is_file());
    assert!(session_dir.join("DSC00001.ARW").is_file());
    assert!(session_dir.join("C0001.MP4").is_file());
    assert_eq!(
        fs::metadata(session_dir.join("C0001.MP4")).unwrap().len(),
        2_000_000
    );
    // AppleDouble junk never crosses over.
    assert!(!session_dir.join("._DSC00001.JPG").exists());

    // Second import of the same card: all duplicates, nothing rewritten.
    let again = run_import(app.handle(), plan(files.clone())).unwrap();
    assert!(again
        .files
        .iter()
        .all(|f| f.status == "skipped_duplicate" && f.verified));
    assert_eq!(again.bytes_copied, 0);

    // Cleanup removes media plus sidecar, thumbnail, and AppleDouble files…
    let deleted = delete_files_impl(&files);
    assert_eq!(deleted.deleted, 3);
    assert!(deleted.errors.is_empty());
    let card_root = card.path();
    assert!(!card_root.join("DCIM/100MSDCF/DSC00001.JPG").exists());
    assert!(!card_root.join("DCIM/100MSDCF/._DSC00001.JPG").exists());
    assert!(!card_root.join("PRIVATE/M4ROOT/CLIP/C0001M01.XML").exists());
    assert!(!card_root.join("PRIVATE/M4ROOT/THMBNL/C0001T01.JPG").exists());
    // …but the imported archive is untouched.
    assert!(session_dir.join("C0001.MP4").is_file());
}

#[test]
fn missing_destination_fails_preflight() {
    let card = tempfile::tempdir().unwrap();
    let (files, _) = fake_card(card.path());
    let app = tauri::test::mock_app();
    let err = run_import(
        app.handle(),
        ImportPlan {
            destination: "/nonexistent/nfs/share".into(),
            folder_pattern: String::new(),
            sessions: vec![SessionPlan {
                date: "2026-05-29".into(),
                name: String::new(),
                files,
            }],
        },
    )
    .unwrap_err();
    assert!(err.contains("NFS"), "error should hint at the mount: {err}");
}
