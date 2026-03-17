---
title: Home
sidebar_position: 1
slug: /
---

# ScrapStation — Source Configuration Guide

ScrapStation is driven entirely by YAML configuration files. Each file describes one **source** — a website you want to browse and download from. You write the config; the engine does the rest.

There is no code to write. Everything from scraping game listings, rendering detail pages, handling authentication, resolving download links across dozens of file hosters, and presenting a custom settings UI — all of it is expressed in YAML.

---

## How it works

1. Drop a `.yaml` file into your **Sources Folder** (Settings → Open Sources Folder), or drag-and-drop it directly onto the app window
2. The source list reloads automatically — no restart needed
3. Your source appears in the sidebar immediately

One file = one source. The filename must match the `id` field inside it.

```
sources/
  mysite.yaml      ← id: mysite
  anothersite.yaml ← id: anothersite
```

---

## Anatomy of a config

Every config is divided into independent sections. You only include what you need.

```
┌─────────────────────────────────────────────────────┐
│  IDENTITY        name, id, type, base_url, ui        │
│  URLS            browse, paginate, search endpoints  │
│  RATE LIMIT      throttle requests politely          │
│  SELECTORS       CSS selectors for HTML sources      │
│  JSON PATHS      JSONPath for API sources            │
│  FIELD MAPPING   map scraped keys → standard fields  │
│  TRANSFORMATIONS clean / normalize field values      │
│  METADATA        extract structured data from text   │
│  TAGS            badge rules for game cards          │
│  DETAIL PAGE     layout of the game detail view      │
│  SETTINGS UI     per-source settings panel in app    │
│  AUTHENTICATION  login flow via WebView or cookies   │
│  HOSTS           smart download for file hosters     │
│  LINK RESOLUTION resolve redirect / obfuscated URLs  │
│  NAVIGATION PATHS complex multi-step link resolution  │
│  NOTICES         one-time popup modals for users     │
└─────────────────────────────────────────────────────┘
```

---

## Quick links

| Topic | What it covers |
|---|---|
| [Getting Started](getting-started) | Your first working source in 5 minutes |
| [Identity & UI](reference/01-identity) | Required fields, icons, colors |
| [URLs & Pagination](reference/02-urls-pagination) | Browse, paginate, search |
| [HTML Selectors](reference/03-selectors-html) | CSS selectors, field extraction |
| [JSON API](reference/04-json-api) | JSONPath-based API sources |
| [Field Mapping](reference/05-field-mapping) | Map scraped data to standard fields |
| [Transformations](reference/06-transformations) | Clean and reshape field values |
| [Metadata Extraction](reference/07-metadata-extraction) | Parse structured data from text |
| [Tags](reference/08-tags) | Conditional badges on game cards |
| [Detail Page](reference/09-detail-page) | Section-based game detail layout |
| [Settings UI](reference/10-settings-ui) | Build a custom settings panel |
| [Authentication](reference/11-authentication) | Login flows, cookies, WebView sessions |
| [Hosts](reference/12-hosts) | Smart download for file hosters |
| [Link Resolution](reference/13-link-resolution) | Resolve redirect chains |
| [Navigation Paths](reference/14-navigation-paths) | Multi-step programmatic navigation |
| [Notices](reference/15-notices) | One-time popup modals shown to users |
