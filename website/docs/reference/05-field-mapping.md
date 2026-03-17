---
title: "Field Mapping"
sidebar_position: 5
---

# Field Mapping, Defaults & Custom Fields

After scraping, the engine needs to know which of your extracted fields corresponds to the title, cover image, page URL, etc. That's what `field_mapping` does.

---

## field_mapping

Maps your scraped field names to the standard names the engine understands.

```yaml
field_mapping:
  title: title             # scraped field "title"   → standard "title"
  cover_url: cover         # scraped field "cover"   → standard "cover_url"
  game_url: link           # scraped field "link"    → standard "game_url"
  description: summary     # scraped field "summary" → standard "description"
  genre: category          # scraped field "category"→ standard "genre"
  author: uploader         # scraped field "uploader"→ standard "author"
```

| Standard field | Purpose |
|---|---|
| `title` | Game name displayed on card and detail page |
| `cover_url` | Thumbnail / poster image URL |
| `game_url` | URL to the game's detail page |
| `description` | Short description shown on card hover |
| `genre` | Genre label |
| `author` | Uploader / publisher name |

Any field you don't map is still available to use in [detail page sections](09-detail-page) or [transformations](06-transformations).

---

## default_values

Hardcode a fallback value for any field when it's missing or empty:

```yaml
default_values:
  genre: "Unknown"
  author: "Anonymous"
  platform: "PC"
```

---

## custom_fields

Derive additional fields from existing ones using templates or transformations.

### Static value

```yaml
custom_fields:
  platform:
    source: "PC"
```

### From another field

```yaml
custom_fields:
  short_title:
    source:
      field: title
    transformations:
      - type: truncate
        max_length: 30
```

### Template combining multiple fields

```yaml
custom_fields:
  display_label:
    source:
      template: "{title} ({genre})"
      fields:
        - title
        - genre
```

### With a condition — only set when a condition is met

```yaml
custom_fields:
  multiplayer_label:
    source:
      field: title
    transformations:
      - type: template
        template: "{title} [MP]"
    condition:
      field: tags
      contains: "multiplayer"
```

---

## Condition types

Conditions can be used on `custom_fields`, `tags`, and `metadata_extraction`:

```yaml
# Field contains a substring
condition:
  field: description
  contains: "multiplayer"

# Field matches a regex pattern
condition:
  field: title
  pattern: "^\\[.*\\]"

# Field equals exact value
condition:
  field: genre
  equals: "Action"

# Field is not empty
condition:
  field: cover_url

# AND — all conditions must be true
condition:
  conditions:
    - field: genre
      contains: "RPG"
    - field: title
      pattern: "\\d{4}"

# OR — at least one must be true
condition:
  conditions:
    - field: author
      equals: "Admin"
    - field: author
      equals: "Moderator"
```
