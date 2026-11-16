# Building Graphite

Apple Silicon Mac, macOS 14.4 or newer.

## Prerequisites
- Xcode command line tools: `xcode-select --install`
- Rust: https://rustup.rs
- Node 20 or newer, and `pnpm` (the scripts use `npx pnpm@10`)
- cmake: `brew install cmake`

## Build and install
```bash
bash scripts/install-mac.sh
```
It builds the local-model sidecar and the app, signs it (with a local "Graphite Local" identity if you made one with `scripts/make-signing-cert.sh`, so macOS keeps your permissions between builds), and installs to `/Applications/Graphite.app`. The first build takes 10 to 20 minutes. The final "A public key has been found, but no private key" message is expected.

## Release build (maintainers)
```bash
bash scripts/release-build.sh
```
Builds with personal paths removed, checks the app for leaks and the right version, and writes `dist/Graphite-macos-arm64.zip`.

## Tests
```bash
cd frontend && npx tsc --noEmit
cd frontend/src-tauri && cargo test --lib
python3 -m unittest tools/test_live_speakers.py
```

## Run the interface in a browser (no Mac features)
`cd frontend && NEXT_PUBLIC_MOCK_TAURI=1 npx pnpm@10 run dev` serves the interface at http://localhost:3118 with the app's backend stubbed, for visual work.
