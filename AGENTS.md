# WinIsland AI Agent Guide

## Architecture Overview
WinIsland is a Rust-based Dynamic Island implementation for Windows, integrating with System Media Transport Controls (SMTC) to display media information, lyrics, and audio visualizations. The app uses winit for window management, Skia for hardware-accelerated rendering on softbuffer surfaces, and Windows APIs for media integration.

Key components:
- **Core modules** (`src/core/`): Handle audio processing (cpal + realfft for spectrum), SMTC media listening, lyrics fetching (163 Music API / lrclib), config persistence (TOML in `~/.winisland/config.toml`), and rendering.
- **UI modules** (`src/ui/expanded/`): Draw expanded views (main media controls, widget mode) with Skia.
- **Window management** (`src/window/`): Main winit event loop, tray icon (tray-icon crate), settings window.
- **Utilities** (`src/utils/`): Physics-based animations (springs), glass effects, blur, font loading, auto-startup, updater (nightly releases from GitHub).

Data flows: SMTC listener → MediaInfo struct → Render pipeline (Skia on softbuffer) → Window display. Audio captured via cpal/Windows APIs for real-time spectrum visualization.

## Critical Workflows
- **Build**: `cargo build --release` (uses winres for Windows resources, LTO for optimization).
- **Run**: Single-instance enforced via Windows mutex. Loads config from `~/.winisland/config.toml`, starts tokio runtime, updater task, and winit event loop.
- **Debug rendering**: Inspect `src/core/render.rs` `draw_island()` for Skia drawing calls; use `println!` for media state in `src/window/app.rs`.
- **Config changes**: Edit `src/core/config.rs` AppConfig struct; defaults in `impl Default`. Persisted via `src/core/persistence.rs`.
- **Media integration**: SMTC sessions monitored in `src/core/smtc.rs`; audio spectrum from `src/core/audio.rs` (6-band FFT).

## Project-Specific Patterns
- **Async handling**: Tokio runtime entered in `main.rs`; use `tokio::spawn` for background tasks (updater, audio capture).
- **Windows APIs**: Extensive use of `windows` crate for Win32 (SMTC, audio, registry). Always wrap in `unsafe {}` blocks.
- **Rendering**: Thread-local Skia surface in `src/core/render.rs`; draw to softbuffer buffer. Use `skia_safe` for 2D graphics.
- **State management**: App state in `src/window/app.rs` struct with springs for smooth animations (physics.rs).
- **Icons**: Custom SVG icons in `src/icons/` modules; rendered via Skia paths.
- **Error handling**: Minimal; use `unwrap()` in trusted contexts, `?` for fallibles. No custom error types.
- **Dependencies**: Heavy on Windows-specific crates (windows, tray-icon, winit); graphics via skia-safe/softbuffer; async via tokio.

## Integration Points
- **SMTC**: Listens to global media sessions; filters by config `smtc_apps`. Triggers expansion on media changes.
- **Audio capture**: cpal for cross-platform, but Windows-specific meter via Win32 APIs for gate detection.
- **Lyrics**: Fetched async from APIs; cached in MediaInfo. Sources: "163" (NetEase), "lrclib".
- **Tray icon**: Context menu for show/hide/settings; uses tray-icon crate.
- **Settings window**: Separate winit window in `src/window/settings.rs`; UI drawn with Skia.
- **Updater**: Checks GitHub releases nightly; downloads to `~/.winisland/`, prompts via MessageBoxW.

Reference key files: `src/main.rs` (entry), `src/window/app.rs` (main loop), `src/core/render.rs` (drawing), `src/core/smtc.rs` (media), `src/core/config.rs` (settings).</content>
