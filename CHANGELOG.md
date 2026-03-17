# Changelog

All notable changes to ScrapStation are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
Versioning: [Semantic Versioning](https://semver.org/)

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
