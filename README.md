# Radish (bootstrap)

Radish is a Rust-first bootstrap for a high-throughput music library importer inspired by the beets workflow.

## Current scope

This initial scaffold focuses on a concurrent import pipeline skeleton:

- Stage A: walk a directory and detect supported audio files
- Stage B: generate deterministic placeholder "acoustid" fingerprints concurrently
- Stage C: enrich metadata concurrently (placeholder MusicBrainz matching)
- Stage D: batch records for storage (batch write simulation)

## Usage

```bash
cargo run -- import <DIR>
cargo run -- watch <DIR>
```

`watch` now runs an initial import and then polls the directory every 2 seconds, triggering a new import when supported audio files are added, removed, or modified.

## Test

```bash
cargo test
```
