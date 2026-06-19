# Changelog

## 1.0.0 - 2026-06-19

### Added

- **Plugin Media Source** (`MediaSourceC`) — plugins can inject full media data (title, artist, album, progress, cover art)
- **GitHub Releases API** integration for reliable update downloads
- **Compile-time build timestamp** embedded in binary — prevents false update prompts after manual exe replacement
- **HTTP retry mechanism** (3 attempts, exponential backoff) for update downloads
- **Release notes page** on website

### Fixed

- "Cannot download new version" error now retries automatically before showing failure dialog
- Manual replacement of exe no longer causes repeated update prompts
- Update download more resilient to network failures

---

## 1.0.0 - Initial Release

### Features

- **Dynamic Island** for Windows — replicates iPhone Dynamic Island experience
- **Music Controls** — SMTC integration with play/pause, next/prev, progress bar, seek
- **Custom Font** — font picker with preview, manager, and per-element font size config
- **Lyrics** — real-time display synced with playback, multi-source (163, lrclib, local .lrc), scroll mode, delay config
- **Spectrum** — audio frequency visualization with configurable gate
- **Themes** — system/light/dark, dynamic color, default/glass/mica/liquid-glass styles
- **Multi-language** — English, Chinese, with fallback and auto-detect
- **Plugin System** — DLL-based plugins via C ABI (`PluginVTable`, `PluginInstanceC`, etc.), ZIP drag-and-drop install, manifest validation, digital signing, packager CLI
- **ContextManager** — unified content scheduling with priority-based mini/expanded dispatch
- **Settings UI** — declarative settings with live preview, sliders, steppers, popups, folder pickers
- **Transparency Effects** — acrylic blur, Mica (Win11), liquid glass with background capture and shader-based blur
- **Tray** — system tray with show/hide/settings/restart/exit
- **Auto-update** — nightly channel with version info comparison
- **Touchscreen** — touch support
- **Toast Notifications** — Windows toast integration
- **Auto-hide** — configurable delay, drag-up-to-hide
- **Auto-start** — boot with Windows

### Changes

- Version `1.0.0` maintained throughout development

### Known Issues

- Update download may fail on first attempt (mitigated by retry mechanism in latest build)
