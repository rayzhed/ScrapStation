---
title: Getting Started
sidebar_position: 2
---

# Getting Started

This page walks you through creating a fully working source config from scratch in about 5 minutes.

---

## Step 1 — Create the file

Open your Sources Folder from **Settings → Open** in the app. Create a new file called `mysite.yaml`.

The filename (without `.yaml`) must match the `id` field you write inside.

---

## Step 2 — Write the minimum required fields

```yaml
name: My Site
id: mysite
type: html_scraper
base_url: https://example.com
```

These four fields are required in every config. See [Identity & UI](reference/01-identity) for all options.

---

## Step 3 — Add browse and search URLs

```yaml
urls:
  main: /games/
  page: /games/page/{page}/
  search: /search/?q={query}
```

`{page}` and `{query}` are template variables the engine fills in automatically.

---

## Step 4 — Tell it what to scrape

Inspect the site's HTML and find the repeating element that wraps each game. Then map the fields you want.

```yaml
selectors:
  main_page:
    container: ".game-card"
    fields:
      title:
        selector: "h3.game-title"
      cover:
        selector: "img.thumbnail"
        attribute: src
      link:
        selector: "a.game-link"
        attribute: href
```

---

## Step 5 — Map fields to standard names

The engine needs to know which scraped key is the title, which is the cover image URL, etc.

```yaml
field_mapping:
  title: title
  cover_url: cover
  game_url: link
```

---

## Step 6 — Clean up URLs

Cover images and links are often relative paths. Normalize them:

```yaml
transformations:
  cover:
    - type: url_normalize
      base_url: https://example.com
  link:
    - type: url_normalize
      base_url: https://example.com
```

---

## Full minimal config

```yaml
name: My Site
id: mysite
type: html_scraper
base_url: https://example.com

ui:
  icon: gamepad-2
  color: "#6366f1"

urls:
  main: /games/
  page: /games/page/{page}/
  search: /search/?q={query}
  search_debounce_ms: 400
  search_min_chars: 2

rate_limit:
  enabled: true
  min_delay_ms: 600
  window_seconds: 10
  burst_threshold: 4

selectors:
  main_page:
    container: ".game-card"
    fields:
      title:
        selector: "h3.game-title"
      cover:
        selector: "img.thumbnail"
        attribute: src
      link:
        selector: "a.game-link"
        attribute: href

  search_page:
    container: ".search-result"
    fields:
      title:
        selector: "h3.result-title"
      cover:
        selector: "img"
        attribute: src
      link:
        selector: "a"
        attribute: href

field_mapping:
  title: title
  cover_url: cover
  game_url: link

transformations:
  cover:
    - type: url_normalize
      base_url: https://example.com
  link:
    - type: url_normalize
      base_url: https://example.com
```

Save the file, click **Reload** in the app (or restart), and your source appears in the sidebar.

---

## Next steps

- Add a [detail page layout](reference/09-detail-page) so clicking a game shows rich information
- Add [host configs](reference/12-hosts) so the app knows how to download from specific file hosters
- Add a [settings panel](reference/10-settings-ui) if the site requires a login
