---
title: "Metadata Extraction"
sidebar_position: 7
---

# Metadata Extraction

`metadata_extraction` lets you derive structured, typed fields from raw scraped text. This is especially useful for pulling version numbers, dates, file sizes, or boolean flags out of description paragraphs.

---

## Structure

```yaml
metadata_extraction:
  field_name:
    type: string | boolean | date | list | number
    extraction:
      method: ...
    transformations: [...]
    fallback: "default if empty"
```

---

## Types

### `string`

Extract a text value.

```yaml
metadata_extraction:
  version:
    type: string
    extraction:
      method: pattern
      pattern: "Version:"
      in_field: description
    fallback: "Unknown"
```

---

### `boolean`

Returns `true` or `false` based on whether a condition is met.

```yaml
metadata_extraction:
  is_multiplayer:
    type: boolean
    condition:
      field: description
      contains: "multiplayer"

  has_crack:
    type: boolean
    condition:
      field: title
      pattern: "\\[Cracked\\]"
```

---

### `date`

Parse a date string and reformat it.

```yaml
metadata_extraction:
  release_date:
    type: date
    extraction:
      method: pattern
      pattern: "Released:"
      in_field: description
    input_format: "%d.%m.%Y"
    output_format: "%B %d, %Y"
    fallback: "TBA"
```

---

### `number`

Extract a numeric value.

```yaml
metadata_extraction:
  file_size_gb:
    type: number
    extraction:
      method: regex
      pattern: "([0-9.]+)\\s*GB"
      in_field: description
      group: 1
    fallback: 0.0
```

---

### `list`

Extract multiple values.

```yaml
metadata_extraction:
  languages:
    type: list
    extraction:
      method: keywords
      in_field: description
      keywords:
        - pattern: "English"
          value: "English"
        - pattern: "Français|French"
          value: "French"
          regex: true
        - pattern: "Deutsch|German"
          value: "German"
          regex: true
    join_with: ", "
```

---

## Extraction methods

### `pattern` — extract the word/value that comes after a label

```yaml
extraction:
  method: pattern
  pattern: "Version:"        # Find this text, return what comes after
  in_field: description      # Which scraped field to search in
```

Input: `Version: 1.2.3 Build 456`
Output: `1.2.3`

---

### `regex` — extract via regex capture group

```yaml
extraction:
  method: regex
  pattern: "v(\\d+\\.\\d+)"
  group: 1
  in_field: title
```

---

### `next_word` — extract the single word following a trigger phrase

```yaml
extraction:
  method: next_word
  after_pattern: "Genre:"
  in_field: description
```

---

### `between` — extract text between two delimiters

```yaml
extraction:
  method: between
  start: "["
  end: "]"
  in_field: title
```

Input: `My Game [v1.0] [EN]`
Output: `v1.0`

---

### `from_field` — copy value directly from another scraped field

```yaml
extraction:
  method: from_field
  field: size_raw
```

---

### `json_path` — extract from a JSON string value

```yaml
extraction:
  method: json_path
  path: "data.version"
```

---

## List keywords

For `list` type with `keywords` method:

```yaml
keywords:
  - pattern: "Online"        # Exact match
    value: "Online"

  - pattern: "Co-?op"        # Regex match
    value: "Co-op"
    regex: true

  - pattern: "PvP|PvE|Multiplayer"
    value: "Multiplayer"
    regex: true
```

Each keyword is checked against the source field. When matched, the corresponding `value` is added to the list.
