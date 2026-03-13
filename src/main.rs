use std::path::Path;

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

            let summary = radish::run_import(Path::new(&dir), 1000).await?;
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
            println!(
                "radish watch is a placeholder in this bootstrap build. Monitoring is not yet active for: {dir}"
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
