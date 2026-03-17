---
title: "Transformations"
sidebar_position: 6
---

# Transformations

Transformations clean, reshape, and normalize field values after extraction. They run in order — the output of each step is the input of the next.

---

## Where transformations apply

```yaml
transformations:
  fieldname:
    - type: trim
    - type: lowercase
```

You can transform any field by its scraped name. Multiple transformations chain together:

```yaml
transformations:
  title:
    - type: trim
    - type: replace
      pattern: "\\s+"
      replacement: " "
      regex: true
    - type: capitalize
```

---

## All transformation types

### `trim`
Remove leading and trailing whitespace.

```yaml
- type: trim
```

---

### `lowercase`
Convert to lowercase.

```yaml
- type: lowercase
```

---

### `uppercase`
Convert to uppercase.

```yaml
- type: uppercase
```

---

### `capitalize`
Capitalize the first letter of the string.

```yaml
- type: capitalize
```

---

### `collapse_whitespace`
Replace multiple consecutive whitespace characters with a single space.

```yaml
- type: collapse_whitespace
```

---

### `remove_html`
Strip all HTML tags from the string.

```yaml
- type: remove_html
```

Input: `<b>Hello</b> <i>World</i>`
Output: `Hello World`

---

### `replace`
Replace a substring or regex pattern.

```yaml
# Simple string replace
- type: replace
  pattern: " - PC Game"
  replacement: ""

# Regex replace
- type: replace
  pattern: "\\[.*?\\]"     # remove [anything in brackets]
  replacement: ""
  regex: true

# Regex with capture groups
- type: replace
  pattern: "(\\d+)\\s*MB"
  replacement: "$1 MB"
  regex: true
```

| Field | Required | Notes |
|---|---|---|
| `pattern` | ✅ | String to find (or regex pattern if `regex: true`) |
| `replacement` | ✅ | Replacement string (supports `$1`, `$2` for capture groups) |
| `regex` | — | Default `false`. Set `true` to use regex |

---

### `extract`
Extract a substring using a regex capture group.

```yaml
# Extract version number from "Game Title v1.2.3"
- type: extract
  pattern: "v(\\d+\\.\\d+\\.\\d+)"
  group: 1       # which capture group to use (default: 0 = full match)
```

| Field | Required | Notes |
|---|---|---|
| `pattern` | ✅ | Regex pattern |
| `group` | — | Capture group index (default `0`) |

---

### `template`
Build a new string by interpolating field values.

```yaml
- type: template
  template: "https://example.com/covers/{value}.jpg"
```

`{value}` refers to the current field value.

---

### `url_normalize`
Turn relative URLs into absolute URLs.

```yaml
- type: url_normalize
  base_url: https://example.com
```

| Field | Required | Notes |
|---|---|---|
| `base_url` | — | Prepended to relative paths |
| `rules` | — | Additional URL normalization steps |

**URL rules:**
- `prepend_protocol: "https"` — add `https://` if no protocol present
- `prepend_domain: "example.com"` — add domain if path starts with `/`
- `remove_query_params` — strip query string
- `remove_fragment` — strip `#fragment`

---

### `truncate`
Limit string length with a suffix.

```yaml
- type: truncate
  max_length: 100
  suffix: "..."    # default is "..."
```

---

### `strip_prefix`
Remove a fixed prefix if present.

```yaml
- type: strip_prefix
  prefix: "Download: "
```

---

### `strip_suffix`
Remove a fixed suffix if present.

```yaml
- type: strip_suffix
  suffix: " | My Site"
```

---

### `append`
Add text to the end of the value.

```yaml
- type: append
  text: " GB"
```

---

### `prepend`
Add text to the beginning of the value.

```yaml
- type: prepend
  text: "https://cdn.example.com/"
```

---

### `split`
Split a string by a delimiter and keep one or all parts.

```yaml
# Keep the first part (index 0)
- type: split
  delimiter: "|"
  index: 0

# Keep the second part
- type: split
  delimiter: " - "
  index: 1

# Split and rejoin with a different separator
- type: split
  delimiter: ","
  join: " / "
```

| Field | Required | Notes |
|---|---|---|
| `delimiter` | ✅ | String to split on |
| `index` | — | Part to keep (omit to keep all parts) |
| `join` | — | Rejoin parts with this separator |

---

### `default`
Set a fallback value if the current value is empty.

```yaml
- type: default
  value: "Unknown"
```

---

## Chaining examples

### Normalize a messy title

```yaml
transformations:
  title:
    - type: trim
    - type: collapse_whitespace
    - type: replace
      pattern: "\\[.*?\\]"
      replacement: ""
      regex: true
    - type: trim
    - type: capitalize
```

### Build a full image URL from a partial path

```yaml
transformations:
  cover:
    - type: strip_prefix
      prefix: "/thumb"
    - type: prepend
      text: "https://cdn.example.com/images"
    - type: append
      text: "?w=600"
```

### Extract just the year from a date string

```yaml
transformations:
  year:
    - type: extract
      pattern: "(\\d{4})"
      group: 1
```
