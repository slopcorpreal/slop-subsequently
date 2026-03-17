# Radish (bootstrap)

Radish is a Rust-first bootstrap for a high-throughput music library importer inspired by the beets workflow.

## Current scope

This initial scaffold focuses on a concurrent import pipeline:

- Stage A: walk a directory and detect supported audio files
- Stage B: generate deterministic content-based "acoustid" fingerprints concurrently
- Stage C: enrich metadata concurrently (derive artist/title from `Artist - Title` filenames)
- Stage D: batch records for storage accounting

## Usage

```bash
cargo run -- import <DIR> [BATCH_SIZE]
cargo run -- watch <DIR>
cargo run -- --help
```

`watch` now runs an initial import and then polls the directory every 2 seconds, triggering a new import when supported audio files are added, removed, or modified.

## Test

```bash
cargo test
```
