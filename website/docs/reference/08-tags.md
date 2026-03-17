---
title: "Tags"
sidebar_position: 8
---

# Tags

Tags are colored badges displayed on game cards. They're driven by conditions — a tag appears when its condition matches.

---

## Structure

```yaml
tags:
  tag_id:
    label: "Displayed text"
    condition:
      field: description
      contains: "multiplayer"
    style: badge
    color: "#32d74b"
    background: "#32d74b20"
    icon: users
    priority: 10
```

Tags are a map — each key is a unique internal identifier.

---

## Fields

| Field | Type | Required | Notes |
|---|---|---|---|
| `label` | string | ✅ | Text shown on the badge |
| `condition` | object | — | When to show this tag (see below) |
| `style` | enum | — | Visual style of the badge |
| `color` | string | — | Text/border color (hex) |
| `background` | string | — | Background color (hex, can include alpha) |
| `icon` | string | — | Lucide icon name |
| `priority` | integer | — | Higher = shown first. Default `0` |
| `value_from` | object | — | Dynamic label text (see below) |

---

## Styles

| Value | Appearance |
|---|---|
| `badge` | Filled rounded pill |
| `chip` | Small compact chip |
| `outline` | Border only, transparent background |
| `solid` | Solid color background |
| `glow` | Badge with a color glow effect |

```yaml
tags:
  featured:
    label: "Featured"
    style: glow
    color: "#ff9f0a"
    icon: star
```

---

## Conditions

### `contains` — field contains a substring

```yaml
condition:
  field: description
  contains: "co-op"
```

### `pattern` — field matches a regex

```yaml
condition:
  field: title
  pattern: "\\[v[0-9]"
```

### `equals` — field equals exact value

```yaml
condition:
  field: genre
  equals: "Action"
```

### `not_empty` — field exists and is not empty

```yaml
condition:
  field: version    # tag appears whenever "version" field has a value
```

### `and` — all conditions must match

```yaml
condition:
  conditions:
    - field: genre
      contains: "RPG"
    - field: description
      contains: "online"
```

### `or` — at least one must match

```yaml
condition:
  conditions:
    - field: title
      contains: "Repack"
    - field: title
      contains: "DODI"
```

---

## Dynamic labels with `value_from`

### From a field

```yaml
tags:
  version_tag:
    label: "Version"   # fallback if field is empty
    style: outline
    color: "#0a84ff"
    value_from:
      field: version
      prefix: "v"      # prepended to the field value
      suffix: ""
```

### Template

```yaml
tags:
  size_tag:
    label: "Size"
    style: chip
    value_from:
      template: "{size} GB"
      fields:
        - size
```

---

## Full example

```yaml
tags:
  multiplayer:
    label: "Multiplayer"
    condition:
      field: description
      contains: "multiplayer"
    style: badge
    color: "#32d74b"
    background: "#32d74b18"
    icon: users
    priority: 20

  online:
    label: "Online"
    condition:
      field: description
      contains: "online"
    style: outline
    color: "#0a84ff"
    icon: wifi
    priority: 15

  version:
    label: "Version"
    style: chip
    color: "#ffffff80"
    priority: 5
    condition:
      field: version
    value_from:
      field: version
      prefix: "v"

  large_file:
    label: "Large"
    condition:
      field: description
      pattern: "[5-9][0-9]\\s*GB|[1-9][0-9]{2}\\s*GB"
    style: solid
    color: "#ff453a"
    icon: hard-drive
    priority: 10
```
