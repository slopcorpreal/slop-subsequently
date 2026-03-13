use std::path::{Path, PathBuf};
use std::time::Duration;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    match command.as_str() {
        "-h" | "--help" => {
            print_usage();
            Ok(())
        }
        "-v" | "--version" => {
            println!("radish {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "import" => {
            let dir = args
                .next()
                .ok_or_else(|| "missing directory argument for import command".to_string())?;
            let dir = parse_directory_arg(&dir)?;
            let batch_size = parse_batch_size(args.next())?;
            if args.next().is_some() {
                return Err("too many arguments for import command".to_string());
            }

            let summary = radish::run_import(&dir, batch_size).await?;
            println!(
                "imported {} tracks from {} files in {} batch(es)",
                summary.tracks_imported, summary.files_seen, summary.batches_written
            );
            Ok(())
        }
        "watch" => {
            let dir = args
                .next()
                .ok_or_else(|| "missing directory argument for watch command".to_string())?;
            let dir = parse_directory_arg(&dir)?;
            if args.next().is_some() {
                return Err("too many arguments for watch command".to_string());
            }

            println!("watching {} for supported audio changes", dir.display());
            println!("press Ctrl+C to stop");

            let initial = radish::run_import(&dir, 1000).await?;
            println!(
                "imported {} tracks from {} files in {} batch(es)",
                initial.tracks_imported, initial.files_seen, initial.batches_written
            );

            let mut previous_signature = radish::audio_library_signature(&dir)?;
            loop {
                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        result.map_err(|e| format!("failed to listen for Ctrl+C: {e}"))?;
                        println!("stopping watch mode");
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {
                        let current_signature = match radish::audio_library_signature(&dir) {
                            Ok(signature) => signature,
                            Err(err) => {
                                eprintln!("warning: failed to read watch directory state: {err}");
                                continue;
                            }
                        };
                        if current_signature != previous_signature {
                            let summary = radish::run_import(&dir, 1000).await?;
                            println!(
                                "imported {} tracks from {} files in {} batch(es)",
                                summary.tracks_imported, summary.files_seen, summary.batches_written
                            );
                            previous_signature = current_signature;
                        }
                    }
                }
            }
            Ok(())
        }
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    println!(
        "Radish (bootstrap)\n\nUSAGE:\n  radish import <DIR> [BATCH_SIZE]\n  radish watch <DIR>\n  radish --help\n  radish --version"
    );
}

fn parse_directory_arg(raw: &str) -> Result<PathBuf, String> {
    let dir = Path::new(raw);
    if !dir.exists() {
        return Err(format!("directory does not exist: {raw}"));
    }
    if !dir.is_dir() {
        return Err(format!("not a directory: {raw}"));
    }

    std::fs::canonicalize(dir).map_err(|e| format!("failed to resolve directory path: {e}"))
}

fn parse_batch_size(raw: Option<String>) -> Result<usize, String> {
    let Some(raw) = raw else {
        return Ok(1000);
    };

    let batch = raw
        .parse::<usize>()
        .map_err(|_| format!("invalid batch size: {raw}"))?;
    if batch == 0 {
        return Err("batch size must be greater than zero".to_string());
    }

    Ok(batch)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{parse_batch_size, parse_directory_arg};

    #[test]
    fn parse_directory_arg_accepts_existing_directory() {
        let dir = tempdir().expect("temp dir");
        let parsed = parse_directory_arg(dir.path().to_str().expect("utf8 path"));
        assert!(parsed.is_ok());
    }

    #[test]
    fn parse_directory_arg_rejects_file_paths() {
        let dir = tempdir().expect("temp dir");
        let file = dir.path().join("track.mp3");
        fs::write(&file, b"dummy").expect("write file");

        let parsed = parse_directory_arg(file.to_str().expect("utf8 path"));
        assert!(parsed.is_err());
    }

    #[test]
    fn parse_batch_size_defaults_to_1000() {
        let batch = parse_batch_size(None).expect("default batch size");
        assert_eq!(batch, 1000);
    }

    #[test]
    fn parse_batch_size_rejects_invalid_or_zero_values() {
        assert!(parse_batch_size(Some("abc".to_string())).is_err());
        assert!(parse_batch_size(Some("0".to_string())).is_err());
    }
}
