---
title: "Navigation Paths"
sidebar_position: 14
---

# Navigation Paths

`paths` is the most powerful part of the config system. It lets you write multi-step programs — fetch pages, extract data, branch on conditions, loop, aggregate results from multiple mirrors — all declaratively in YAML.

Paths are referenced by name from `download_buttons` (via `resolver:`) or from `hosts` configs.

---

## Structure

```yaml
paths:
  my_path:
    timeout_ms: 30000
    max_depth: 10
    steps:
      - fetch:
          follow_redirects: true
      - extract:
          method: regex
          pattern: "..."
      - return:
          current: true
    on_error:
      - return_original
```

Steps use the **map key as the step type** (`- fetch: {...}`). This is different from the `link_resolution` step format.

---

## Step types

### `fetch`

```yaml
- fetch:
    url: "https://api.example.com/file/{file_id}"
    follow_redirects: true
    method: GET
    headers:
      Referer: "https://example.com"
    timeout_ms: 10000
    store_as: api_response
```

---

### `extract`

```yaml
- extract:
    method: regex
    pattern: "download_url: ['\"]([^'\"]+)['\"]"
    group: 1
    as: download_url
    fallback: ""

- extract:
    method: selector
    selector: "a#download"
    attribute: href

- extract:
    method: json_path
    pattern: "response.data.url"

- extract:
    method: header
    pattern: "Location"
```

**Extraction methods:**

| Method | Description |
|---|---|
| `regex` | Regex pattern with optional capture group |
| `selector` | CSS selector with optional `attribute` |
| `json_path` | Dot-notation path into JSON response |
| `xpath` | XPath expression |
| `header` | HTTP response header by name |
| `text` | Entire response body |

---

### `extract_all`

Extract multiple links from the page and process each one.

```yaml
- extract_all:
    selector: "div.mirror-option"
    attribute: "data-url"
    limit: 10
    extract_meta:
      name:
        selector: ".host-name"
    foreach:
      path:
        - fetch:
            follow_redirects: true
        - return:
            current: true
    aggregate:
      mode: all           # all | first_success | priority | fastest | parallel
```

**`foreach` with pattern matching:**

```yaml
foreach:
  match:
    - when:
        url_contains: "fasthost.com"
      path:
        - fetch:
            follow_redirects: true
        - return:
            current: true
    - when:
        url_contains: "slowhost.net"
      path:
        - webview:
            wait_for: "#download"
            click: "#download"
            intercept_download: true
  default:
    - return:
        current: true
```

---

### `transform`

```yaml
- transform:
    transformations:
      - type: trim
      - type: url_normalize
        base_url: https://example.com
```

---

### `branch`

```yaml
- branch:
    if: not_empty
    then:
      - return:
          current: true
    else:
      - return:
          error: "Could not extract download URL"
```

---

### `loop`

```yaml
- loop:
    while:
      response_contains: "processing"
    max: 10
    do:
      - wait:
          ms: 2000
      - fetch:
          follow_redirects: true
```

---

### `wait`

```yaml
- wait:
    ms: 2000

- wait:
    seconds: 5
```

---

### `set_var`

```yaml
- set_var:
    name: file_id
    value: "abc123"
```

Variables are referenced as `{variable_name}` in URL templates.

---

### `detect_host`

```yaml
- detect_host:
    hosts:
      "fasthost.com":
        - fetch:
            follow_redirects: true
        - return:
            current: true
      "slowhost.net":
        - webview:
            wait_for: "#countdown-done"
            click: "#download"
            intercept_download: true
    default:
      - return:
          current: true
```

---

### `webview`

```yaml
- webview:
    wait_for: "a#download-ready"
    wait_timeout_ms: 60000
    click: "a#download-ready"
    intercept_download: true
    download_url_pattern: "\\.zip|\\.rar|\\.7z"
    execute_js: "document.querySelector('.overlay').remove();"
```

---

### `return`

```yaml
- return:
    current: true        # return the current extracted value

- return:
    value: "{download_url}"

- return:
    error: "File is no longer available"

- return:
    browser_only: true
    reason: "This file requires manual download"
```

---

### `use`

```yaml
- use:
    use: paths.common_resolver
```

---

## Conditions

Used in `branch.if`, `loop.while/until`, and `foreach.match`:

### Value conditions

```yaml
not_empty
is_empty
{ contains: "keyword" }
{ starts_with: "https" }
{ ends_with: ".zip" }
{ matches: "\\d{6}" }
{ equals: "ok" }
```

### HTTP response conditions

```yaml
status_success
{ status: 200 }
{ response_contains: "error" }
{ selector_exists: "#download-btn" }
```

### URL conditions

```yaml
{ url_contains: "premium" }
{ host_equals: "files.example.com" }
```

### Logic operators

```yaml
{ and: [not_empty, { url_contains: "file" }] }
{ or: [{ url_contains: "zip" }, { url_contains: "rar" }] }
{ not: { url_contains: "error" } }
```

---

## Error handling

```yaml
on_error:
  - return_original      # return the original unresolved URL

on_error:
  - retry:
      max_attempts: 3
      delay_ms: 1000
```

---

## Full example — hosters aggregation page

```yaml
paths:
  hosters_page:
    timeout_ms: 30000
    steps:
      - fetch:
          follow_redirects: true
          headers:
            Referer: "https://example.com/"

      - extract_all:
          selector: "div.mirror-tab"
          attribute: "data-link"
          extract_meta:
            label:
              selector: ".mirror-name"
          foreach:
            match:
              - when:
                  url_contains: "fasthost.com"
                path:
                  - extract:
                      method: regex
                      pattern: "/f/([A-Za-z0-9]+)"
                      group: 1
                  - transform:
                      transformations:
                        - type: template
                          template: "https://fasthost.com/api/dl/{value}"
                  - return:
                      current: true
            default:
              - return:
                  current: true
          aggregate:
            mode: all

    on_error:
      - return_original
```
