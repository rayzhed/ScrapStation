---
title: "URLs & Pagination"
sidebar_position: 2
---

# URLs & Pagination

Controls how the engine fetches the game listing, navigates pages, and executes searches.

---

## URL templates

```yaml
urls:
  main: /games/                          # First page / homepage
  page: /games/page/{page}/             # Paginated pages ({page} = page number)
  search: /search/?q={query}            # Search endpoint ({query} = URL-encoded term)
  search_debounce_ms: 400               # Wait this long after last keystroke before firing
  search_min_chars: 2                   # Don't search until user has typed this many chars
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `main` | string | — | Relative path appended to `base_url` |
| `page` | string | — | `{page}` replaced with page number (1-indexed) |
| `search` | string | — | `{query}` replaced with URL-encoded search term |
| `search_debounce_ms` | integer | `300` | Milliseconds to debounce typing |
| `search_min_chars` | integer | `1` | Minimum chars before search fires |

### Path vs full URL

Both relative paths and full URLs work:

```yaml
urls:
  main: /games/           # relative → https://example.com/games/
  main: https://cdn.example.com/games/  # absolute → used as-is
```

### Pagination styles

```yaml
# Page number in path
page: /games/page/{page}/

# Page number as query param
page: /games/?p={page}
```

### Search styles

```yaml
# Query param
search: /search/?q={query}

# Path segment
search: /search/{query}/
```

---

## Rate limiting

Prevents hammering the remote server. Always enable this — it keeps your config working long-term.

```yaml
rate_limit:
  enabled: true
  min_delay_ms: 600         # Minimum gap between requests (ms)
  window_seconds: 10        # Rolling window for burst detection
  burst_threshold: 4        # Max requests allowed in the window before throttling
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `enabled` | bool | `false` | Must be `true` to activate |
| `min_delay_ms` | integer | `500` | Floor for inter-request delay |
| `window_seconds` | integer | `10` | Length of the burst detection window |
| `burst_threshold` | integer | `5` | Requests before throttling kicks in |

### Recommended values by site type

| Site type | `min_delay_ms` | `burst_threshold` |
|---|---|---|
| Small / independent | 1000–1500 | 3 |
| Medium site | 600–800 | 4 |
| Large / CDN-backed | 300–500 | 6 |

```yaml
# Polite defaults for an independent site
rate_limit:
  enabled: true
  min_delay_ms: 1000
  window_seconds: 10
  burst_threshold: 3
```
