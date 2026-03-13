use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub path: PathBuf,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub acoustid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    pub files_seen: usize,
    pub tracks_imported: usize,
    pub batches_written: usize,
}

pub async fn run_import(dir: &Path, batch_size: usize) -> Result<ImportSummary, String> {
    if batch_size == 0 {
        return Err("batch size must be greater than zero".to_string());
    }

    let (scan_tx, scan_rx) = mpsc::channel(256);
    let (fp_tx, fp_rx) = mpsc::channel(256);
    let (meta_tx, meta_rx) = mpsc::channel(256);

    let scan_dir = dir.to_path_buf();
    let scanner = tokio::spawn(async move { scan_and_read_tags(&scan_dir, scan_tx).await });

    let fp_workers = spawn_worker_pool(scan_rx, fp_tx, 4, fingerprint_track);
    let meta_workers = spawn_worker_pool(fp_rx, meta_tx, 8, match_musicbrainz_async);

    let db_worker = tokio::spawn(async move { process_batches(meta_rx, batch_size).await });

    let files_seen = scanner
        .await
        .map_err(|e| format!("scanner task failed: {e}"))?;

    wait_for_workers(fp_workers, "fingerprint").await?;
    wait_for_workers(meta_workers, "metadata").await?;

    let (tracks_imported, batches_written) = db_worker
        .await
        .map_err(|e| format!("database task failed: {e}"))?;

    Ok(ImportSummary {
        files_seen,
        tracks_imported,
        batches_written,
    })
}

async fn wait_for_workers(
    workers: Vec<tokio::task::JoinHandle<()>>,
    stage_name: &str,
) -> Result<(), String> {
    for worker in workers {
        worker
            .await
            .map_err(|e| format!("{stage_name} worker task failed: {e}"))?;
    }

    Ok(())
}

fn spawn_worker_pool<F, Fut>(
    rx: mpsc::Receiver<Track>,
    tx: mpsc::Sender<Track>,
    workers: usize,
    handler: F,
) -> Vec<tokio::task::JoinHandle<()>>
where
    F: Fn(Track) -> Fut + Copy + Send + Sync + 'static,
    Fut: std::future::Future<Output = Track> + Send + 'static,
{
    let shared_rx = Arc::new(Mutex::new(rx));
    let mut tasks = Vec::with_capacity(workers);

    for _ in 0..workers {
        let rx = Arc::clone(&shared_rx);
        let tx = tx.clone();

        tasks.push(tokio::spawn(async move {
            loop {
                let message = {
                    let mut guard = rx.lock().await;
                    guard.recv().await
                };

                let Some(track) = message else {
                    break;
                };

                let processed = handler(track).await;
                if tx.send(processed).await.is_err() {
                    break;
                }
            }
        }));
    }

    drop(tx);
    tasks
}

pub async fn scan_and_read_tags(dir: &Path, tx: mpsc::Sender<Track>) -> usize {
    let mut files_seen = 0usize;

    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        files_seen += 1;

        if !is_supported_audio_file(entry.path()) {
            continue;
        }

        let path = entry.path().to_path_buf();
        let mut title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        if title.as_deref() == Some("") {
            title = None;
        }

        let track = Track {
            path,
            title,
            artist: None,
            acoustid: None,
        };

        if tx.send(track).await.is_err() {
            break;
        }
    }

    drop(tx);
    files_seen
}

pub fn is_supported_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ["mp3", "flac", "ogg", "m4a", "wav", "aac"]
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
        .unwrap_or(false)
}

pub async fn fingerprint_track(mut track: Track) -> Track {
    let mut hasher = DefaultHasher::new();
    track.path.hash(&mut hasher);
    track.acoustid = Some(format!("acoustid:{:016x}", hasher.finish()));
    track
}

pub async fn match_musicbrainz_async(mut track: Track) -> Track {
    if track.artist.is_none() {
        track.artist = Some("Unknown Artist".to_string());
    }

    track
}

pub async fn process_batches(mut rx: mpsc::Receiver<Track>, batch_size: usize) -> (usize, usize) {
    let mut batch = Vec::with_capacity(batch_size);
    let mut tracks_imported = 0usize;
    let mut batches_written = 0usize;

    while let Some(track) = rx.recv().await {
        batch.push(track);

        if batch.len() >= batch_size {
            tracks_imported += batch.len();
            batches_written += 1;
            batch.clear();
        }
    }

    if !batch.is_empty() {
        tracks_imported += batch.len();
        batches_written += 1;
    }

    (tracks_imported, batches_written)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{is_supported_audio_file, run_import};

    #[test]
    fn supported_audio_extensions_are_detected_case_insensitively() {
        assert!(is_supported_audio_file(Path::new("track.flac")));
        assert!(is_supported_audio_file(Path::new("track.MP3")));
        assert!(!is_supported_audio_file(Path::new("cover.jpg")));
    }

    #[tokio::test]
    async fn import_pipeline_counts_tracks_and_batches() {
        let dir = tempdir().expect("temp dir");
        fs::write(dir.path().join("one.mp3"), b"dummy").expect("write one");
        fs::write(dir.path().join("two.flac"), b"dummy").expect("write two");
        fs::write(dir.path().join("note.txt"), b"not audio").expect("write note");

        let summary = run_import(dir.path(), 1).await.expect("import should work");

        assert_eq!(summary.files_seen, 3);
        assert_eq!(summary.tracks_imported, 2);
        assert_eq!(summary.batches_written, 2);
    }

    #[tokio::test]
    async fn import_pipeline_rejects_zero_batch_size() {
        let dir = tempdir().expect("temp dir");
        fs::write(dir.path().join("one.mp3"), b"dummy").expect("write one");

        let result = run_import(dir.path(), 0).await;
        assert!(result.is_err());
    }
}
