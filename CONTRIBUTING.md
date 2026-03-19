# Contributing to ScrapStation

Thanks for your interest in contributing. Here's everything you need to know.

---

## Branch model

```
main        ← stable releases only — never commit directly
dev         ← integration branch — merge features here
feat/*      ← new features        (branch from dev)
fix/*       ← bug fixes           (branch from dev)
chore/*     ← maintenance, deps, CI, config (branch from dev)
docs/*      ← documentation only  (branch from dev)
```

**The flow:**

1. Branch off `dev`
2. Work on your branch
3. Open a PR → `dev`
4. Once `dev` is tested and stable → PR `dev` → `main` → tag a release

---

## Versioning conventions

| Branch / context | Version format      | Example        |
|-----------------|---------------------|----------------|
| `main`          | `x.y.z`             | `0.1.3`        |
| `dev`           | `x.y.z-dev`         | `0.1.3-dev`    |
| `feat/*`        | inherits from `dev` | `0.1.3-dev`    |
| `fix/*`         | inherits from `dev` | `0.1.3-dev`    |
| `chore/*`       | inherits from `dev` | `0.1.3-dev`    |

**Rules:**
- Immediately after merging a release to `main`, bump `dev` to the next patch version with `-dev` suffix
- Never change the version on feature/fix/chore branches — they inherit from `dev`
- Strip `-dev` only when preparing a release PR from `dev` → `main`
- Version must be kept in sync across `tauri.conf.json`, `Cargo.toml`, and `package.json`

---

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
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

---

## Commit style

Use short, imperative commit messages:

```
Add download retry on failure
Fix cover image not loading for lazy-loaded sources
Update GoFile host config for new URL format
```

Prefix with a scope when helpful:

```
engine: fix link resolver looping on empty response
ui: improve empty state color tinting
docs: add navigation paths examples
```

---

## Pull requests

- Keep PRs focused — one thing per PR
- Fill in the PR template
- Make sure CI passes before requesting review
- No private source configs, credentials, or personal settings

---

## Source configs

Source config files (`.yaml`) are **personal** — they contain site-specific selectors that belong to individual users. Do **not** submit source configs as PRs. The `src-tauri/sources/example.yaml` is the only source file in this repo.

---

## Reporting bugs

Use the [Bug Report](.github/ISSUE_TEMPLATE/bug_report.yml) issue template. Include your OS, app version, and steps to reproduce.

---

## Questions

Open a [Discussion](https://github.com/rayzhed/ScrapStation/discussions) rather than an issue for general questions.
