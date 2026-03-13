use std::path::{Path, PathBuf};

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
        "import" => {
            let dir = args
                .next()
                .ok_or_else(|| "missing directory argument for import command".to_string())?;
            let dir = parse_directory_arg(&dir)?;

            let summary = radish::run_import(&dir, 1000).await?;
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
            println!(
                "radish watch is a placeholder in this bootstrap build. Monitoring is not yet active for: {}",
                dir.display()
            );
            Ok(())
        }
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    println!("Radish (bootstrap)\n\nUSAGE:\n  radish import <DIR>\n  radish watch <DIR>");
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::parse_directory_arg;

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
}
