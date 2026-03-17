---
title: "JSON API"
sidebar_position: 4
---

# JSON API

Used when `type: json_api`. Extracts game data from a JSON response using dot-notation paths.

---

## Structure

```yaml
json_paths:
  items: "data.games"          # Path to the array of game objects
  fields:
    title: "name"              # JSONPath within each item
    cover: "assets.cover_url"
    link: "url"
    size: "file.size_mb"
    date: "published_at"
```

| Field | Description |
|---|---|
| `items` | Dot-notation path to the array of items in the response |
| `fields` | Map of output field name → dot-notation path within each item |

---

## Path syntax

Use dots to navigate nested objects:

```yaml
# Response: { "result": { "list": [ { "game": { "title": "..." } } ] } }
items: "result.list"
fields:
  title: "game.title"
```

Array indices work too:

```yaml
fields:
  cover: "images.0.url"    # first element of images array
```

---

## Full example

```yaml
name: Example API Source
id: exampleapi
type: json_api
base_url: https://api.example.com

urls:
  main: /v1/games?sort=latest
  page: /v1/games?sort=latest&page={page}
  search: /v1/games?search={query}

json_paths:
  items: "data"
  fields:
    title: "title"
    cover: "cover_image"
    link: "page_url"
    description: "short_description"
    size: "download_size"
    genre: "category"
    date: "created_at"

field_mapping:
  title: title
  cover_url: cover
  game_url: link
  description: description
```

---

## Combining with transformations

JSON fields often need cleanup just like HTML ones:

```yaml
transformations:
  title:
    - type: trim
    - type: capitalize

  size:
    - type: append
      text: " GB"

  date:
    - type: replace
      pattern: "T.*$"
      replacement: ""
      regex: true
```

See [Transformations](06-transformations) for the full list.
