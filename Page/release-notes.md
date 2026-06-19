# Release Notes

Notable changes to WinIsland across releases.

## 1.1.0 - Jun 19, 2026

Added:

- Plugin Media Source (`MediaSourceC`) — plugins can inject full media data (title, artist, album, progress, cover art)
- GitHub Releases API integration for reliable update downloads
- Compile-time build timestamp embedded in binary — prevents false update prompts after manual exe replacement
- HTTP retry mechanism (3 attempts, exponential backoff) for update downloads
- Release notes page on website

Changed:

- Version bumped from 1.0.0 to 1.1.0 (minor release with significant new features)
- Update system uses GitHub API to resolve latest nightly download URL instead of hardcoded URL

Fixed:

- "Cannot download new version" error now retries automatically before showing failure dialog
- Manual replacement of exe no longer causes repeated update prompts
- Update download more resilient to network failures

## 1.0.0 - Initial Release

Added:

- Dynamic Island for Windows — replicates iPhone Dynamic Island experience
- System Media Transport Controls (SMTC) integration for now-playing media
- Real-time lyrics display synced with media playback
- Audio spectrum visualization
- Spring physics animations for smooth island expansion and collapse
- Theme color system with customizable colors
- Custom font support
- Glass effect (acrylic/blur background)
- Multi-language i18n support (English, Chinese)
- Settings UI with hot-reload configuration
- Plugin system foundation (C ABI, PluginVTable, content rendering)
- ContextManager for unified content scheduling
- Nightly auto-update system
