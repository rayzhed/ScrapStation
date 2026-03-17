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

The file hoster requires JavaScript (countdown timers, CAPTCHA, dynamic buttons).

```yaml
download_method: webview
webview_config:
  wait_for: "#download-btn"
  wait_timeout_ms: 30000
  click: "#download-btn"
  intercept_download: true
  download_url_pattern: "\\.zip|\\.rar|\\.7z"
  pre_script: "document.querySelector('.popup').remove();"
```

| Field | Default | Notes |
|---|---|---|
| `wait_for` | — | CSS selector to wait for before acting |
| `wait_timeout_ms` | `15000` | Timeout in ms |
| `click` | — | CSS selector of element to click |
| `intercept_download` | `false` | Capture the download URL instead of opening the file |
| `download_url_pattern` | — | Regex: only intercept URLs matching this pattern |
| `pre_script` | — | JavaScript to execute before interaction |

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

    # WebView with countdown
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
        intercept_download: true
        download_url_pattern: "\\.zip|\\.rar|\\.7z"

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
