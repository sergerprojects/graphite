# Contributing

Thanks for helping. Graphite is a Mac app: a Next.js interface inside a Tauri shell with a Rust core.

## Set up
Install Xcode command line tools, Rust, Node 20+, and cmake. Then see [docs/BUILDING.md](docs/BUILDING.md).

## Before you open a pull request
- `cd frontend && npx tsc --noEmit`
- `cd frontend/src-tauri && cargo test --lib` (two audio-device tests depend on your hardware and may fail on some Macs)
- `python3 -m unittest tools/test_live_speakers.py`
- Keep changes small and focused. Match the style of the code around them.
- Use fictional names in tests and fixtures (Alex, Sam, Priya). Never commit real meeting content, names, emails or recordings.

## Reporting bugs
In Graphite open Settings, General and press **Copy diagnostics**, then paste it in the issue. It has versions and setup status, no meeting content.
