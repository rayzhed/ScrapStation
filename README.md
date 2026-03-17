# ScrapStation

A desktop game browser and downloader built with Tauri v2 + SvelteKit.

Browse games from any community-made source config, view game details, manage downloads, and track your local library — all from one place.

## Features

- **Source system** — plug in any site via a YAML config file; drag-and-drop `.yaml` files to install new sources
- **Smart downloader** — auto-detects the file host and applies the correct download strategy automatically
- **Library tracker** — tracks installed games, links downloads, and auto-detects executables for launching
- **Archive extraction** — supports `.rar`, `.7z`, `.zip`, and multi-part archives with password support
- **Per-source authentication** — login via embedded WebView for sources that require accounts
- **Live source reload** — drop or remove YAML files while the app is running; the source list updates automatically

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) + [pnpm](https://pnpm.io/)
- [Tauri v2 prerequisites](https://tauri.app/start/prerequisites/)

## Development

```bash
pnpm install
pnpm tauri dev
```

## Build

```bash
pnpm tauri build
```

## Adding Sources

Drop any `.yaml` source config onto the app window, or place it manually in:

- **Windows**: `%APPDATA%\ScrapStation\sources\`

Sources are hot-reloaded — no restart required.

## Tech Stack

- **Frontend**: SvelteKit, TypeScript, Tailwind CSS, Motion
- **Backend**: Rust, Tauri v2
- **Archive support**: `sevenz-rust`, `unrar`, `zip`

## License

MIT
