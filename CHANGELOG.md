# Changelog

## 1.0.0
- Rewrite of the Python script in Rust as a library + CLI.
- Own ICO encoder (PNG/BMP entries, AND mask, correct 256px handling).
- Alpha-correct Lanczos3 resampling (no dark halos).
- `--fit`, `--padding`, `--background`, `--encoding`, `--no-upscale`.
- Batch input: files, directories (`-r`), glob patterns; parallel processing.
- `inspect`, `sizes`, `completions` subcommands; `--dry-run`, atomic writes.
- Fixed: 512px was requested but ICO cannot store >256px; now skipped with a warning.
