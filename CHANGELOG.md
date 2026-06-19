# Changelog

All notable changes to WinIsland will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-06-19

### Added

#### Core Island
- Dynamic Island for Windows, replicating iPhone Dynamic Island experience
- Collapse/expand animation with frame-rate-independent animation
- Custom screen selection for multi-monitor setups
- Island offset configuration (X, Y position)
- Dock position support (top, bottom, left, center, right alignment)
- Popup auto-flip near screen edges
- Drag-up-to-hide gesture
- Auto-hide with configurable delay
- Window auto-recenter after display resolution change
- Auto-start with Windows

#### Music Controls (SMTC)
- SMTC integration for media playback detection
- Play/pause, next/previous track controls
- Progress bar with drag-to-seek support
- Progress smoothing (disabled while dragging)
- Coalesced seek requests to prevent flooding
- Auto-allow first SMTC app for seamless experience
- SMTC timeline throttling to prevent lyric jumping during seek
- Effective duration helper for accurate progress display
- Video playback session filtering

#### Lyrics
- Real-time lyrics display synced with playback
- Multi-source lyrics: 163, lrclib, local .lrc files
- Lyrics scroll mode with smooth animation
- Configurable lyrics display delay
- Improved lyrics search accuracy
- Offline lyrics support via local .lrc files
- Local lyrics folder picker dialog
- Artist name whitespace trimming before fetcher dispatch

#### Audio Spectrum
- Real-time audio frequency visualization
- Frequency graph display in both normal and expanded states
- Auto-gate with automatic audio gating (enabled by default)
- Spectrum muting when audio gate is closed

#### Visual & Themes
- Multiple island styles: default, glass, Mica, liquid glass, transparent
- System/light/dark theme support
- Dynamic color extraction from desktop background (HSV histogram + smoothstep transition)
- Motion blur effect
- Adaptive text color based on background brightness
- Adaptive icon colors to match background
- Border consistency across themes
- Mica transparency via real Win11 DWM parameters
- Liquid glass with lens-shift algorithm, 5-tap in-shader blur, enhanced shadows and borders
- Background capture with caching by position
- Palette result caching per frame
- SKSL clamp protection for shaders
- Transparent glass theme variant
- Custom rounded corners for settings window
- macOS-style sidebar icons with solid accent selection highlights

#### Custom Font
- FontManager with custom font support
- Font picker with live preview
- Per-element font size configuration
- Font cache refresh when custom font path changes

#### Settings UI
- Declarative settings framework
- Real-time sliders for stepper controls
- Live preview during configuration changes
- Sub-tab navigation with underline alignment
- Settings scroll performance optimization with text caching
- Hover detection with shared hover key
- Multipage settings with navigation
- Custom borderless window decorations with macOS-style window controls
- Transparent settings window support
- RowFolderPicker for directory selection
- Settings window singleton (prevents multiple settings windows)

#### Tray & System
- System tray icon with context menu
- Tray menu: show/hide, settings, restart, exit
- Restart support with proper process termination and relaunch

#### Auto-Update
- Nightly update channel
- GitHub Releases API integration for reliable downloads
- Compile-time build timestamp to prevent false update prompts
- HTTP retry mechanism (3 attempts, exponential backoff)
- Dual-channel update support
- Embedded version information via CARGO_PKG_VERSION

#### Plugin System
- DLL-based native plugin support via C ABI
- Plugin manager for lifecycle management (load, init, update, shutdown)
- ZIP drag-and-drop installation with UI feedback
- Manifest validation during plugin build
- ContentProvider integration for plugin data polling
- Windows Toast notifications for plugin install status
- Packager CLI for plugin distribution
- Plugin security hardening (C2-C5, C7-C8 fixes)
- ZIP extraction on background thread to prevent main thread blocking

#### ContextManager
- Unified content scheduling with priority-based dispatch
- Mini/expanded content routing
- Context types for Music, Plugin, and system states
- Content Plugin output rendering on Dynamic Island

#### Internationalization
- Multi-language support: English, Chinese
- Language auto-detection with fallback
- Decoupled language system

#### Touch & Input
- Touchscreen support
- Custom hit-test logic for interactive elements

#### Diagnostics
- File logging with Minecraft-style crash reports
- Diagnostic logging across all major modules
- GPU retry on startup for resilience

#### Toast Notifications
- Windows toast integration for plugin and system notifications

#### Website
- Project website
- Release notes page

### Changed

- License changed from MIT to GNU GPL v3
- Audio gating logic refactored to auto_gate with auto-enable by default
- Mini player controls simplified for cleaner UI
- Settings file split into `settings/mod.rs` + `settings/input.rs`
- Win32 helpers extracted to `utils/win32.rs`
- Render pipeline adapted to param-struct pattern
- Settings renderer adapted to param-struct pattern
- Plugin polling replaced with ContextManager scheduler
- Update system redesigned with retry, embedded timestamp, GitHub API, and dual-channel
- Backdrop dynamic color rewritten as HSV histogram + smoothstep transition
- Wallpaper blur rewritten with real Win11 Mica parameters + background thread precomputation
- Liquid glass shader rewritten with lens shift algorithm, full-res BitBlt capture, 5-tap blur
- `main_view` renamed to `music_view`
- Various code quality improvements: unwrap safety, SAFETY comments, disk IO optimization
- Stable slider bindings for settings controls

### Deprecated

- None.

### Removed

- Mica and liquid glass style options from settings (simplified to core styles)
- Album art cover shape and rotation configuration options
- Mini controls from compact island mode
- Lyrics display from expanded widget page
- Audio gating settings toggle (auto-gating always enabled)

### Fixed

#### Window & Rendering
- Startup white flash and window drift eliminated
- Island misalignment after system wake / display resolution change
- Island focus stealing when other windows minimize (WS_EX_NOACTIVATE)
- Island interfering with MyDockFinder (WS_EX_TOOLWINDOW)
- Maximize prevention for the island window
- Multiple settings windows opening simultaneously
- Settings child process cleanup on exit
- `monitor_index` off-by-one in `get_target_monitor`
- Mica coordinate mapping (screen pixels not scaled to blur surface size)
- Mica feedback loop causing black background
- Self-capture contamination resolved for real-time blur
- WDA_EXCLUDEFROMCAPTURE used instead of hide/show for self-capture prevention
- Text color adaptation for light theme in About page

#### Music & SMTC
- SMTC progress syncing at startup (progress no longer starts from 0)
- SMTC session management and song change detection
- Song cover fetched asynchronously with retries
- Cover cache key collision prevention
- Premultiplied alpha color correction in cover rendering
- MediaPlaybackType corrected to use Music variant
- Video playback sessions properly ignored
- Unrelated lyrics prevented for browser video sessions

#### Lyrics
- Lyrics scroll state sync at startup
- Problematic lyrics display for some songs fixed
- Font weight inconsistency in lyrics resolved
- Lyrics font display corrected
- `lyrics_delay` now correctly applied to widget view
- Stray closing brace removed in `fetch_lyrics`

#### Audio
- Audio gate disable now bypasses spectrum instead of zeroing
- COM objects explicitly dropped before `CoUninitialize` to prevent use-after-free
- Audio mute cleanup improved

#### Settings & UI
- Sub-tab text underline alignment corrected
- `count_group_rows_from` logic bug causing group background not rendering
- General page title displayed above sub-tab pages
- Audio gate switch animation index missing
- Collapse animation frame rate improved
- Progress bar and lyrics state synchronized at startup
- `draw_text_in_rect_deprecated` x-centering calculation corrected
- Popup menu position corrected (array index vs row count)
- Hit-test index mismatch fixed (`items.get(idx)` vs `get_row_item`)
- Mini control hit-test using wrong Y after auto-hide
- Duplicate `bring_settings_to_front` removed
- Mica DWM properties applied on runtime style change
- `disable_mica` now correctly resets fallback DWM property 1029 for pre-22H2
- RowFolderPicker height raised to 64px when path is shown
- Dynamic row height for hover detection using cached heights

#### Plugin
- Empty plugin ID rejected due to vacuous truth in charset validation
- Trailing slash allowed in ZIP directory path validation
- Plugin ZIP extraction moved to background thread
- `plugin_set_host_api` uses standalone symbol to avoid dangling pointer crash

#### Performance
- Text group cache keys replaced with integer hashes, LRU eviction
- ASCII fast path for text group rendering
- Duplicate `pause_t` computation eliminated
- SMTC `update_media_info` throttled, auto-allow skipped on regular polls
- `format!` cache keys replaced with integer hashes
- Settings UI: reduced allocations, cached text, smoother scroll
- Liquid glass 4x downscale optimization
- Background capture cached by position, captured once

#### Code Quality & Robustness
- All clippy warnings eliminated
- SAFETY comments added for unsafe blocks
- `unwrap` calls made safe
- `is_animating` early-exit optimization restored
- Dead code removed
- `wrap_pixels` safety improved
- `thread_local` const init corrected
- `AppItem` index bug fixed
- Restart mutex timeout forces old process termination
- `set_gate_override` one-frame delay fixed
- Logger, Mica, dead code, group background, `max_scroll`, `adaptive_max` drift issues addressed
- Null-safe Drop implementation
- Nightly download links pointed to upstream
- `Collapsible match` guard in `DroppedFile` handler fixed
- `FontManager` cache refreshed when custom font path changes
- Config edition corrected to 2024
- SMTL thumbnail fetching conditional logic simplified
- `DockPosition` Default trait implementation added
- `effective_duration_ms` helper for progress identification

### Security

- Plugin security issues C2-C5, C7-C8 addressed
- Empty plugin ID validation enforced
- Manifest validation added in plugin build
- ZIP directory path traversal protection
- SAFETY documentation for all unsafe blocks
- Host API dangling pointer crash prevention
- Auto-allow gating for SMTC apps

[1.0.0]: https://github.com/Eatgrapes/WinIsland/releases/tag/v1.0.0
