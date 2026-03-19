<div align="center">

<img src="src-tauri/icons/128x128@2x.png" alt="ScrapStation" width="120" />

# ScrapStation

**A desktop game browser and downloader — driven entirely by community YAML configs.**

[![Latest Release](https://img.shields.io/github/v/release/rayzhed/ScrapStation?style=for-the-badge&logo=github&color=00e5ff&logoColor=black&label=latest)](https://github.com/rayzhed/ScrapStation/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/rayzhed/ScrapStation/total?style=for-the-badge&logo=github&color=00e5ff&logoColor=black&label=downloads)](https://github.com/rayzhed/ScrapStation/releases)
[![Stars](https://img.shields.io/github/stars/rayzhed/ScrapStation?style=for-the-badge&logo=github&color=00e5ff&logoColor=black)](https://github.com/rayzhed/ScrapStation/stargazers)

[![Platform](https://img.shields.io/badge/Windows-0078D4?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/rayzhed/ScrapStation/releases/latest)
[![License](https://img.shields.io/badge/License-ScrapStation%20NC-00e5ff?style=for-the-badge&logoColor=black)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/rayzhed/ScrapStation/ci.yml?style=for-the-badge&label=build&logo=githubactions&logoColor=white)](https://github.com/rayzhed/ScrapStation/actions)
[![Discord](https://img.shields.io/badge/Discord-Join%20Server-5865f2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/tr32SsezDA)

[**Download**](https://github.com/rayzhed/ScrapStation/releases/latest) · [**Documentation**](https://rayzhed.github.io/ScrapStation/) · [**Discord**](https://discord.gg/tr32SsezDA)

</div>

---

## What is ScrapStation?

ScrapStation lets you browse and download games from any website — without writing a single line of code. You describe the site in a YAML file, and the engine handles everything: scraping listings, rendering detail pages, resolving download links across file hosters, handling logins, extracting archives, and tracking your library.

One config file = one source. The community shares configs. You just drop them in.

---

## Features

| | |
|---|---|
| **Source system** | Plug in any site via a `.yaml` config file — drag-and-drop to install |
| **Smart downloader** | Auto-detects the file host and applies the correct download strategy |
| **WebView downloads** | Handles JavaScript-gated hosters with countdown timers and dynamic buttons |
| **Link resolution** | Follows redirect chains, extracts IDs, builds API URLs — all in YAML |
| **Archive extraction** | `.rar`, `.7z`, `.zip`, multi-part archives, password support |
| **Library tracker** | Tracks installed games, links downloads, auto-detects executables |
| **Authentication** | Per-source login via embedded WebView — sessions persist across restarts |
| **Live reload** | Drop or remove YAML files while the app is running — no restart needed |
| **Settings UI** | Each source can define its own settings panel, entirely in YAML |
| **Detail pages** | Rich game pages: hero banner, trailer, metadata, install steps, download buttons |

---

## Download

Grab the latest installer from the [**Releases**](https://github.com/rayzhed/ScrapStation/releases/latest) page.

| File | Description |
|---|---|
| `ScrapStation_x.x.x_x64-setup.exe` | NSIS installer — recommended |
| `ScrapStation_x.x.x_x64_en-US.msi` | MSI installer |

> Windows may show a SmartScreen warning on first launch. Click **More info → Run anyway** — this is expected for unsigned apps.

---

## Adding Sources

Drop any `.yaml` source config onto the app window, or place it in:

```
%APPDATA%\ScrapStation\sources\
```

Sources are **hot-reloaded** — no restart required. The sidebar updates instantly.

Want to write your own? The full guide is at [**rayzhed.github.io/ScrapStation**](https://rayzhed.github.io/ScrapStation/).

---

## Community

<div align="center">

### Join the Discord to get source configs, share your own, and follow the project

[![Join Discord](https://img.shields.io/badge/Join%20the%20Discord-%235865F2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/tr32SsezDA)

The Discord is where configs are shared, bugs are discussed, and new versions are announced.

</div>

---

## Building from source

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) v20+
- [pnpm](https://pnpm.io/)
- [Tauri v2 prerequisites](https://tauri.app/start/prerequisites/)

### Setup

```bash
git clone https://github.com/rayzhed/ScrapStation.git
cd ScrapStation
pnpm install
pnpm tauri dev
```

### Build installer

```bash
pnpm tauri build
# Output: src-tauri/target/release/bundle/
```

---

## Tech stack

- **Frontend** — SvelteKit, TypeScript, Tailwind CSS, Motion
- **Backend** — Rust, Tauri v2
- **Archive support** — `sevenz-rust`, `unrar`, `zip`

---

## Contributing

See [**CONTRIBUTING.md**](CONTRIBUTING.md) for the branch model, commit style, and how to open a PR.

`main` → stable releases only
`dev` → active development
`feat/*`, `fix/*`, `chore/*` → branch from `dev`, PR back to `dev`

---

## License

ScrapStation License v1.0 (custom non-commercial) — see [LICENSE](LICENSE)
