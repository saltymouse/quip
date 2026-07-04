// Smoke test: cargo run --example scan_real_card -- /Volumes/Untitled
fn main() {
    let card = std::env::args().nth(1).unwrap_or("/Volumes/Untitled".into());
    let items = quip_lib::scan::scan(std::path::Path::new(&card));
    println!("{} items", items.len());
    for session in quip_lib::scan::group_sessions(items, 4) {
        println!(
            "{}  {:>3} photos {:>2} videos  {:.2} GB",
            session.date,
            session.photo_count,
            session.video_count,
            session.total_bytes as f64 / 1e9
        );
        let first = session.items.first().unwrap();
        let last = session.items.last().unwrap();
        println!(
            "    {} ({}) .. {} ({})  thumb: {:?}",
            first.id, first.capture_time, last.id, last.capture_time, first.thumb_source
        );
        if let Some(src) = &first.thumb_source {
            let start = std::time::Instant::now();
            match quip_lib::thumbs::exif_embedded_thumb(std::path::Path::new(src)) {
                Some(bytes) => println!(
                    "    exif thumb: {} bytes in {:?}",
                    bytes.len(),
                    start.elapsed()
                ),
                None => println!("    exif thumb: none (would fall back to full decode)"),
            }
        }
    }
}
