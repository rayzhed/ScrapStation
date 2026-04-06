# Changelog

All notable changes to ScrapStation are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
Versioning: [Semantic Versioning](https://semver.org/)

---

## [0.1.4] — 2026-04-05

### Added
- **Settings page** — converted from a popup modal to a full navigation page with a left sidebar (Storage, Sources, Recovery, per-source sections)
- **Multi-library location system** — app now tracks multiple ScrapStation library trees; auto-detects existing `ScrapStation\Library` folders on every connected drive at startup
- **Controlled game transfer** — Move button in the Library panel opens a location picker instead of a free folder browser; lists all known library locations as destinations
- **Library locations management** in Settings → Storage: lists active, auto-detected, and user-added library folders with Active/Auto badges; supports adding extra locations via folder picker
- **Drive badge** on each library card (bottom-right) showing which drive the game is installed on
- **Install path display** in the selected-game panel, responsive — shows full path when space allows, truncates only when needed
- **Custom transfer icon** — Move button now shows two document shapes with an arrow between them for clarity
- **Repair Library scans all known locations** — Recovery → Fix Library now walks every detected library folder (active + default AppData + extras + all drives), recovering games from any previously active root
- **Fix broken paths now multi-root aware** — Recovery → Fix broken paths resolves legacy relative paths against every known library folder, finding the correct one instead of defaulting to the current root
- `get_known_library_locations`, `add_library_location`, `remove_library_location` Tauri commands
- **Download location picker** in the install preflight modal — "Save archive to" Change button shows known app-managed locations as a dropdown; Browse fallback still available for custom paths
- **Install location auto-linked** to download location — changing the download drive automatically targets the matching library folder on that drive; overridable via Advanced
- **Enlarged install preflight modal** — wider (`max-w-2xl`), larger text and padding, full paths shown without truncation, scrollable body with pinned header and action bar

### Changed
- `add_game_to_library` now stores the **absolute install path** at creation time so games remain locatable when the data root is changed later
- `normalize_paths` (Fix broken paths) now probes all known library roots when upgrading a legacy relative path, picking whichever root actually contains the game on disk
- `repair_library` now stores absolute paths for recovered entries (was relative, which broke after root changes)
- Settings left nav active state is now tracked via IntersectionObserver so it follows the scroll position
- Library location entries are marked `removable: false` for auto-detected and system entries — only user-added extras show the remove button
- Settings → Storage data root "Browse" button replaced with a "Change" dropdown listing known locations, with Browse fallback
- `downloadDir` and `installDir` now forwarded through both smart and resolved download paths (previously only smart carried them)

### Fixed
- Games installed on a previously active drive (e.g. F:) no longer disappear from the library after the data root is changed; running Repair Library recovers them automatically
- Move game dropdown no longer filters out "Current (custom)" — games on a different drive can now be moved into the active library
- Settings modal `overflow: hidden` was clipping the Move dropdown; panel now uses explicit border-radius on the accent bar instead
- Drive badge `{@const}` was placed outside an `{#each}` child — moved to the correct scope
- Path comparison in `get_known_library_locations` now normalises case and trailing separators on Windows to avoid duplicates
- Install preflight modal Advanced accordion `overflow: hidden` was clipping the install location dropdown
- Preflight modal scroll container switches to `overflow: visible` while a picker is open so dropdowns are never clipped

[0.1.4]: https://github.com/rayzhed/ScrapStation/compare/v0.1.3...v0.1.4

---

## [0.1.3] — 2026-03-19

### Added
- Markdown rendering for changelogs in the Updates page
- Overflow-aware "Show more / Show less" accordion on long changelogs
- Pulse animation on the update dot in the sidebar (green = ready to install, blue = available)
- Version display in the Updates page header
- `package.json` as the single source of truth for the app version

### Changed
- Removed redundant section headers from About and Updates pages
- Dev builds now publish NSIS only (MSI rejects `-dev` version suffix)
- Release bodies now show the actual changelog instead of installation instructions
- CI skips the Windows build for docs and workflow-only commits

### Fixed
- Signing keys now passed correctly to dev-build CI workflow
- Version and newer/older label in Updates page now read dynamically from the installed app

[0.1.3]: https://github.com/rayzhed/ScrapStation/compare/v0.1.2...v0.1.3

---

## [0.1.2] — 2026-03-19

### Added
- Auto-load `.env` via `dotenv-cli` so local builds pick up signing keys automatically

### Fixed
- Updater artifacts now generated correctly (`createUpdaterArtifacts: true`) so `latest.json` is uploaded to releases
- Release version and newer/older label now read dynamically from the installed app instead of being hardcoded
- New signing key pair — previous key was unrecoverable

[0.1.2]: https://github.com/rayzhed/ScrapStation/compare/v0.1.1...v0.1.2

---

## [0.1.1] — 2026-03-19

### Added
- About page — full-panel view with app identity, tech stack, author credit, license summary, and legal disclosures
- In-app installer — download and run any release version directly without leaving the app, with live progress bar
- Auto-updater — silent update check on launch with dedicated Updates page for upgrade/downgrade to any version
- Custom noncommercial license (ScrapStation License v1.0) replacing MIT

### Fixed
- File downloads now correctly resolve filenames from `Content-Disposition` and `Content-Type` headers
- Stale "downloading" badge no longer lingers after all downloads complete
- Blob URLs now allowed in CSP so game cover images render correctly in production builds
- `process:allow-restart` capability name corrected (was `allow-relaunch`)
- Version alignment between `tauri-plugin-updater` and `@tauri-apps/plugin-updater`

[0.1.1]: https://github.com/rayzhed/ScrapStation/compare/v0.1.0...v0.1.1

---

## [0.1.0] — 2026-03-17

### Added
- Source system — plug in any site via a YAML config file
- HTML scraper engine with CSS selector support
- JSON API engine with dot-notation path support
- Detail page rendering — hero, video, text, metadata grid, numbered steps, alert box, download buttons
- Smart downloader — auto-detects file host and applies the correct download strategy
- WebView-based downloads for JavaScript-gated file hosters
- API-based URL resolution for programmatic file hosts
- Navigation paths — multi-step YAML programs for complex link resolution
- Link resolution — sequential redirect/obfuscation unwrapping
- Library tracker — tracks installed games and links downloads
- Archive extraction — `.rar`, `.7z`, `.zip`, and multi-part archives with password support
- Per-source authentication via embedded WebView
- Live source reload — drop or remove `.yaml` files while the app is running
- Settings UI system — per-source settings panels defined entirely in YAML
- Tags system — conditional badge chips on game cards
- Transformation pipeline — clean, reshape, and normalize scraped field values
- Notices system — one-time popup modals for per-source user guidance
- `contains_text` filter on download buttons for text-level element selection
- `supported: false` flag to lock unsupported download host buttons
- `warning` field for amber confirmation modal before downloads proceed
- Rate limiter — configurable per-source request throttling
- Docusaurus documentation site at https://rayzhed.github.io/ScrapStation/

[0.1.0]: https://github.com/rayzhed/ScrapStation/releases/tag/v0.1.0
