---
title: "Identity & UI"
sidebar_position: 1
---

# Identity & UI

Every source config starts with four required fields plus optional UI metadata.

---

## Required fields

```yaml
name: My Awesome Site      # Display name shown in the sidebar
id: myawesomesite          # Unique identifier — MUST match the filename
type: html_scraper         # Source type (see below)
base_url: https://example.com  # Root URL, used for relative URL normalization
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `name` | string | ✅ | Displayed in sidebar and header |
| `id` | string | ✅ | Must match the `.yaml` filename exactly |
| `type` | enum | ✅ | `html_scraper`, `json_api`, `graphql_api`, `xml_api`, `custom` |
| `base_url` | string | ✅ | No trailing slash |

### Source types

| Value | Description |
|---|---|
| `html_scraper` | Parse HTML pages with CSS selectors |
| `json_api` | Parse JSON responses with JSONPath |
| `graphql_api` | Query a GraphQL endpoint |
| `xml_api` | Parse XML/RSS feeds |
| `custom` | Custom logic (advanced) |

---

## UI metadata

Controls the icon and accent color shown in the sidebar and settings panel.

```yaml
ui:
  icon: gamepad-2       # Lucide icon name
  color: "#6366f1"      # Hex color for accent
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `icon` | string | `gamepad` | Any [Lucide icon](https://lucide.dev/icons/) name in kebab-case |
| `color` | string | `#00e5ff` | Hex color string |

### Icon examples

```yaml
ui:
  icon: gamepad-2       # controller
  icon: monitor         # screen / desktop
  icon: download        # arrow down into tray
  icon: globe           # web / internet
  icon: hard-drive      # storage
  icon: zap             # lightning / fast
  icon: shield          # security / protected
  icon: star            # featured
  icon: wifi            # online / connected
  icon: package         # software / bundle
```

---

## Description

Optional subtitle shown in the settings panel.

```yaml
description: "Browse and download games with online multiplayer support"
```

---

## Authentication flag

If the site uses JavaScript-heavy bot protection (Cloudflare, DDoS-Guard) and all requests must go through a WebView session:

```yaml
auth:
  requires_webview_fetch: true
```

When `true`, the app will refuse to load the source until an active WebView session exists. See [Authentication](11-authentication).
