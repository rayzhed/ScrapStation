---
title: "Hosts"
sidebar_position: 12
---

# Hosts — Smart Download

The `hosts` section teaches the app how to handle download links for specific file hosting services. When a user clicks a download button, the app checks the URL against all host patterns and uses the matching host's download strategy.

---

## Structure

```yaml
hosts:
  hosts:
    hostname:
      patterns:
        - "filehost.com"
      display:
        label: "FileHost"
        icon: "hard-drive"
        color: "#6366f1"
      download_method: direct       # direct | webview | api
```

---

## Pattern matching

Patterns are matched against the download URL:
- Domain names: `"filehost.com"` — matches any URL containing that domain
- Subdomains: `"cdn.filehost.com"` — exact subdomain
- File extensions: `".torrent"` — matches URLs ending in that extension

---

## Download methods

### `direct`

The URL is a direct download link — no extra steps needed.

```yaml
download_method: direct
```

---

### `webview`

The file hoster requires JavaScript (countdown timers, CAPTCHA, dynamic buttons). The engine opens an invisible browser, optionally clicks a button or runs a script to extract the CDN URL, then streams the file directly with full progress tracking.

```yaml
download_method: webview
webview_config:
  wait_for: "#download-btn"
  wait_timeout_ms: 30000
  click: "#download-btn"
  download_url_pattern: "\\.zip|\\.rar|\\.7z"
  pre_script: "document.querySelector('.popup').remove();"
```

| Field | Default | Notes |
|---|---|---|
| `wait_for` | — | CSS selector to wait for before acting |
| `wait_timeout_ms` | `30000` | How long (ms) to wait for `wait_for` / overall download URL timeout |
| `click` | — | CSS selector of the element to click to start the download |
| `intercept_download` | `false` | Skip `click` / `extract_url_script` entirely — wait for the page to initiate the download on its own (auto-start timers, server-side redirects) |
| `download_url_pattern` | — | Regex: only treat URLs matching this pattern as the download URL |
| `pre_script` | — | JavaScript to run before interaction (close pop-ups, dismiss banners, etc.) |
| `extract_url_script` | — | JavaScript that returns the download URL directly; use when clicking isn't enough (e.g. HTMX, multi-step JS flows) |
| `visible` | `false` | Show the browser window — required for Cloudflare or similar JS challenges that only pass in a visible browser |
| `navigate_from` | — | Load this URL first so the browser sends it as `Referer` when navigating to the real download URL — bypasses hotlink protection |
| `navigate_from_wait_ms` | `3000` | How long (ms) to wait after the `navigate_from` page loads before navigating to the download URL |

---

### `api`

The hoster exposes a programmatic API to get the download URL.

```yaml
download_method: api
resolver:
  steps:
    - action: extract
      method: regex
      pattern: "/file/([a-zA-Z0-9]+)"
      group: 1
    - action: transform
      transformations:
        - type: template
          template: "https://api.filehost.com/download/{value}"
```

---

## `browser_only`

Some hosters can't be automated — mark them as browser-only:

```yaml
gdrive:
  patterns:
    - "drive.google.com"
  display:
    label: "Google Drive"
    icon: "hard-drive"
    color: "#4285f4"
  browser_only: true
  browser_only_reason: "Google Drive requires your own Google account"
```

---

## Full example

```yaml
hosts:
  hosts:
    # API-based extraction
    pixeldrain:
      patterns:
        - "pixeldrain.com"
      display:
        label: "PixelDrain"
        icon: "hard-drive"
        color: "#8b5cf6"
      download_method: api
      resolver:
        steps:
          - action: extract
            method: regex
            pattern: "/u/([a-zA-Z0-9]+)"
            group: 1
          - action: transform
            transformations:
              - type: template
                template: "https://pixeldrain.com/api/file/{value}?download"

    # WebView with countdown timer — click the link when it appears
    slowhost:
      patterns:
        - "slowhost.net"
      display:
        label: "SlowHost"
        icon: "clock"
        color: "#f59e0b"
      download_method: webview
      webview_config:
        wait_for: "a#download-link"
        wait_timeout_ms: 90000
        click: "a#download-link"
        download_url_pattern: "\\.zip|\\.rar|\\.7z"

    # WebView that auto-starts the download — no click needed
    autohost:
      patterns:
        - "autohost.net"
      display:
        label: "AutoHost"
        icon: "zap"
        color: "#10b981"
      download_method: webview
      webview_config:
        intercept_download: true
        wait_timeout_ms: 30000

    # WebView behind hotlink protection — navigate from the game page first
    refererhost:
      patterns:
        - "cdn.refererhost.net"
      display:
        label: "RefererHost"
        icon: "link"
        color: "#6366f1"
      download_method: webview
      webview_config:
        navigate_from: "https://refererhost.net"
        navigate_from_wait_ms: 4000
        click: "a.download-button"
        wait_timeout_ms: 60000

    # Browser-only
    accountonly:
      patterns:
        - "premium.example.com"
      display:
        label: "Premium Host"
        icon: "shield"
        color: "#ef4444"
      browser_only: true
      browser_only_reason: "Premium Host requires your own account to download"
```
