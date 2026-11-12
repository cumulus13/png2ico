# png2ico

Fast, robust image → ICO converter written in Rust. Library + CLI.

- Input: PNG, JPEG, WebP, BMP, GIF, TIFF (format sniffed from content)
- Multi-size ICO (`-S`) or one file per size, up to 256px (the ICO limit)
- Own ICO encoder: PNG and/or 32-bit BMP entries with AND mask
- Alpha-correct Lanczos3 resampling (no halos), `--fit contain|cover|stretch`, `--padding`, `--background`
- Batch: files, directories (`-r`), glob patterns (works on Windows too), parallel
- Safe: atomic writes, no overwrite without `--force`, `--dry-run`
- `favicon` subcommand: apple-touch-icon, favicon 16/32, favicon.ico, android-chrome 192/512, site.webmanifest + HTML snippet
- `inspect` ICO files, shell `completions`

## Install

```sh
cargo install png2ico          # from crates.io
cargo binstall png2ico         # prebuilt binary
```
Or download an archive from the Releases page (Linux gnu/musl x64+arm64, Windows x64+arm64, macOS Intel+Apple Silicon).

## Usage

```sh
png2ico logo.png -S                       # logo.ico with all default sizes
png2ico logo.png -S -p favicon            # 16, 32, 48
png2ico logo.png -s 16,32,256 -S -o app.ico
png2ico "assets/*.png" -d out/ -S -f      # batch
png2ico assets/ -r -S --fit cover --padding 8
png2ico logo.png --background "#ffffff" --encoding png -S
png2ico favicon logo.png -d public --name "My App"   # favicon.io-style package
png2ico inspect logo.ico
png2ico completions bash > /etc/bash_completion.d/png2ico
```

`png2ico file.png ...` is shorthand for `png2ico convert file.png ...`.

### Differences from the Python version
- 512px is no longer generated: ICO cannot store sizes above 256 (Pillow silently dropped it).
- Every requested size is genuinely stored in `-S` mode.

## Library

```rust
use png2ico::{convert_file, Layout, Options, Plan};
let opts = Options { sizes: vec![16, 32, 256], ..Options::default() };
let plan = Plan { layout: Layout::Single, force: true, ..Plan::default() };
convert_file("logo.png".as_ref(), &opts, &plan)?;
```

## Development

```sh
cargo fmt && cargo clippy --all-targets && cargo test
```

Release: bump `version` in `Cargo.toml`, then `git tag vX.Y.Z && git push --tags`.
This triggers `release.yml` (binaries + GitHub Release) and `publish.yml` (crates.io, needs the `CARGO_REGISTRY_TOKEN` secret).

License: MIT

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)
