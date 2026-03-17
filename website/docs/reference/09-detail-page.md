---
title: "Detail Page"
sidebar_position: 9
---

# Detail Page

The detail page is the full-screen view that opens when you click a game. It's composed of **sections** rendered in order. You define exactly which sections appear and how they extract data from the game's HTML detail page.

---

## Structure

```yaml
detail_page:
  sections:
    - type: hero
      order: 1
      config: ...

    - type: text_content
      order: 2
      title: "Description"
      icon: file-text
      config: ...

    - type: download_buttons
      order: 3
      config: ...
```

Sections are rendered in `order` ascending. `title` and `icon` appear as the section header.

---

## Section types

### `hero`

The top banner — background image, title, subtitle, and info badges.

```yaml
- type: hero
  order: 1
  config:
    background_image:
      selector: "meta[property='og:image']"
      attribute: content
    title:
      selector: "h1.game-title"
    subtitle:
      selector: ".release-date"
    badges:
      - label: "Version"
        icon: package
        selector: ".version-info"
        contains: "Version:"
        extract_after: ":"
        style: primary

      - label: "Players"
        icon: users
        selector: ".player-count"
        extract_number: true
        suffix: " players"
        style: success
```

**Badge fields:**

| Field | Notes |
|---|---|
| `label` | Display label in the badge |
| `icon` | Lucide icon |
| `selector` | CSS selector for the element |
| `contains` | Only use this element if its text contains this string |
| `extract_after` | Extract text after this character (e.g. `:`) |
| `extract_number` | Extract only the numeric value |
| `suffix` | Append to extracted value |
| `style` | `primary`, `success`, `warning`, `danger`, `info` |

---

### `video`

Embed a YouTube or other video iframe.

```yaml
- type: video
  order: 2
  title: "Trailer"
  icon: play-circle
  config:
    selector: "iframe[src*='youtube']"
    attribute: src
    transform: youtube_embed    # Converts watch URL to embed URL
```

`transform: youtube_embed` converts `https://www.youtube.com/watch?v=ID` → `https://www.youtube.com/embed/ID`.

---

### `text_content`

A block of text — description, notes, changelog, etc.

```yaml
- type: text_content
  order: 3
  title: "Description"
  icon: file-text
  config:
    selector: ".game-description"
    extract: text             # "text" or "html"
    max_length: 1200
```

| Field | Notes |
|---|---|
| `selector` | CSS selector |
| `extract` | `text` (plain text) or `html` (rendered HTML) |
| `max_length` | Truncate at this many characters |

---

### `metadata_grid`

A two-column grid of labeled data points.

```yaml
- type: metadata_grid
  order: 4
  title: "Game Info"
  icon: info
  config:
    items:
      - label: "Release Date"
        icon: calendar
        selector: ".release"

      - label: "Developer"
        icon: code
        selector: ".dev-name"

      - label: "Archive Password"
        icon: lock
        selector: ".password-box"
        render_as: code          # renders in monospace code box
        style: highlighted       # highlighted background
```

**Item fields:**

| Field | Notes |
|---|---|
| `label` | Left-column label |
| `icon` | Lucide icon |
| `selector` | CSS selector |
| `attribute` | HTML attribute to extract (default: text content) |
| `contains` | Only match element if it contains this text |
| `extract_after` | Extract text after this separator |
| `render_as` | `text` (default) or `code` |
| `style` | `normal` or `highlighted` |

---

### `numbered_steps`

An ordered list of installation instructions.

```yaml
- type: numbered_steps
  order: 5
  title: "Installation"
  icon: list-ordered
  config:
    selector: ".install-guide"
    extract_method: numbered_list     # or "list_items" or "paragraphs"
    start_after: "Installation steps:"
    end_before: "Notes:"
```

| `extract_method` | Description |
|---|---|
| `numbered_list` | Parses "1. Step one\n2. Step two" format |
| `list_items` | Extracts `<li>` elements |
| `paragraphs` | Splits on `<p>` or double newlines |

---

### `alert_box`

A warning / info callout box — for important notes, warnings, or requirements.

```yaml
- type: alert_box
  order: 6
  title: "Important Notes"
  icon: alert-triangle
  style: warning          # "info", "warning", "danger", "success"
  config:
    selector: ".notes-box ul"
    items_selector: li    # extract each <li> as a separate note
```

---

### `download_buttons`

Action buttons for downloading the game. Each button targets a link on the page.

```yaml
- type: download_buttons
  order: 7
  title: "Download"
  icon: download
  config:
    buttons:
      - label: "Download"
        selector: "a.download-btn"
        icon: download
        style: primary
        action: open_link
        smart_download: true

      - label: "Mirror 1"
        selector: "a[href*='/redirect/']"
        icon: link
        style: secondary
        action: open_link
        resolve_link: true
        use_text_as_label: true

      - label: "Hosters Page"
        selector: "a[href*='/hosters/']"
        icon: server
        style: primary
        action: open_link
        resolver: my_hosters_path
```

**Button fields:**

| Field | Notes |
|---|---|
| `label` | Button text (overridden by `use_text_as_label`) |
| `selector` | CSS selector for the link element |
| `contains_text` | Only match elements whose visible text contains this string |
| `icon` | Lucide icon |
| `style` | `primary`, `secondary`, `danger`, `success` |
| `action` | `open_link` |
| `smart_download` | Run the link through host detection and smart download |
| `resolve_link` | Resolve redirect chain via `link_resolution` first |
| `use_text_as_label` | Use the element's visible text as the button label |
| `resolver` | Name of a path in `paths:` to use for resolution |
| `supported` | Set to `false` to show the button locked ("Not yet supported") |
| `warning` | Amber confirmation modal shown before the download proceeds |

**`contains_text` example** — when one CSS selector matches multiple hosts, show only the one you want:

```yaml
- label: "GoFile"
  selector: "a.download-btn[href*='/ext/']"
  contains_text: "GoFile"    # only match the anchor whose text is "GoFile"
  icon: cloud-download
  style: secondary
  smart_download: true
```

**`supported: false` example** — lock buttons for hosts you haven't configured yet:

```yaml
- label: "RapidGator"
  selector: "a[href*='rapidgator.net']"
  icon: hard-drive
  style: secondary
  action: open_link
  supported: false           # shown but unclickable
```

**`warning` example** — show a confirmation before an unreliable download:

```yaml
- label: "Mirror B"
  selector: "a[href*='mirror-b']"
  icon: cloud-download
  style: primary
  smart_download: true
  warning: "This mirror is sometimes slow. If it fails, try again."
```

---

### `dynamic`

A flexible section that renders arbitrary scraped HTML or text.

```yaml
- type: dynamic
  order: 8
  title: "Additional Info"
  icon: layers
  config:
    selector: ".extra-content"
    render_as: html
```
