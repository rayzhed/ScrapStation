---
title: "Link Resolution"
sidebar_position: 13
---

# Link Resolution

`link_resolution` defines how the engine follows and unpacks redirect/obfuscated links before downloading. It's a sequential step system for resolving a single URL through a chain of fetches and extractions.

> For complex multi-step navigation (branching, loops, conditional logic), use [Navigation Paths](14-navigation-paths) instead.

---

## Structure

```yaml
link_resolution:
  enabled: true
  resolvers:
    resolver_name:
      source_field: link
      steps:
        - action: fetch
          ...
        - action: extract
          ...
```

Steps use `action:` as a tag — this is different from the `- fetch: {...}` format used in navigation paths.

---

## Steps

### `fetch`

```yaml
- action: fetch
  follow_redirects: true
  headers:
    Referer: "https://example.com"
  timeout_ms: 10000
```

---

### `extract`

```yaml
- action: extract
  method: regex
  pattern: "window\\.location = ['\"]([^'\"]+)['\"]"
  group: 1
```

```yaml
- action: extract
  method: selector
  selector: "a#download-link"
  attribute: href
```

```yaml
- action: extract
  method: json_path
  pattern: "data.download_url"
```

| Field | Notes |
|---|---|
| `method` | `regex`, `selector`, `json_path`, `xpath`, `text` |
| `pattern` | For `regex` / `json_path` |
| `group` | Regex capture group (default: `0`) |
| `selector` | CSS selector for `selector` method |
| `attribute` | HTML attribute to extract |
| `fallback` | Value to use if extraction fails |

---

### `transform`

```yaml
- action: transform
  transformations:
    - type: trim
    - type: url_normalize
      base_url: https://example.com
```

---

### `wait`

```yaml
- action: wait
  duration_ms: 2000
```

---

## Examples

### JavaScript redirect page

```yaml
link_resolution:
  resolvers:
    redirect_page:
      source_field: link
      steps:
        - action: fetch
          follow_redirects: true
        - action: extract
          method: regex
          pattern: "location\\.href=['\"]([^'\"]+)['\"]"
          group: 1
```

### HTML page with a download button

```yaml
link_resolution:
  resolvers:
    download_page:
      source_field: link
      steps:
        - action: fetch
          follow_redirects: true
        - action: extract
          method: selector
          selector: "a.download-button"
          attribute: href
        - action: transform
          transformations:
            - type: url_normalize
              base_url: https://example.com
```

### Build URL from extracted ID

```yaml
link_resolution:
  resolvers:
    id_to_url:
      source_field: link
      steps:
        - action: extract
          method: regex
          pattern: "/file/([A-Za-z0-9]+)"
          group: 1
        - action: transform
          transformations:
            - type: template
              template: "https://cdn.example.com/download/{value}"
```

---

## When to use Link Resolution vs Navigation Paths

| Use case | Tool |
|---|---|
| Follow a redirect or extract a URL from a page | `link_resolution` |
| Multiple steps, branching, loops, conditions | `paths` |
| Hosters page with many mirrors | `paths` |
| Per-host programmatic download | `hosts.resolver` |
