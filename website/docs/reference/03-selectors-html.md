---
title: "HTML Selectors"
sidebar_position: 3
---

# HTML Selectors

Used when `type: html_scraper`. Defines CSS selectors for extracting game data from HTML pages.

---

## Structure

```yaml
selectors:
  main_page:           # Used for the browse / listing page
    container: "..."   # CSS selector for one game entry (repeating element)
    fields:            # Fields to extract from each entry
      fieldname: ...

  search_page:         # Used for search results (falls back to main_page if omitted)
    container: "..."
    fields:
      fieldname: ...
```

The engine finds every element matching `container`, then within each one runs the field selectors. The result is one game object per container match.

---

## Field selector formats

### Simple — just a CSS selector, extracts text content

```yaml
fields:
  title: "h3.game-title"
  genre: ".meta .genre"
```

### With attribute — extract an HTML attribute instead of text

```yaml
fields:
  cover:
    selector: "img.thumbnail"
    attribute: src          # extract src= instead of text

  cover_lazy:
    selector: "img.lazy"
    attribute: data-src     # lazy-loaded images often store the real URL here

  link:
    selector: "a.game-link"
    attribute: href
```

### Multiple — collect all matches as a comma-separated string

```yaml
fields:
  genres:
    selector: "span.tag"
    multiple: true          # joins all matched elements' text
```

### Static — hardcode a value for every game from this source

```yaml
fields:
  platform:
    static: "PC"
```

---

## Practical examples

### Typical game card

```yaml
selectors:
  main_page:
    container: ".game-card"
    fields:
      title:
        selector: "h3.title"
      cover:
        selector: "img"
        attribute: src
      link:
        selector: "a.card-link"
        attribute: href
      size:
        selector: ".file-size"
      date:
        selector: "time"
        attribute: datetime
```

### Lazy-loaded images

Many sites put the real image URL in `data-src` or `data-lazy-src` to defer loading:

```yaml
cover:
  selector: "img.lazy"
  attribute: data-src
```

### Nested selectors

CSS selectors scope within the container — you can still use any valid CSS:

```yaml
fields:
  title:
    selector: "div.info > h2:first-child"
  version:
    selector: "ul.meta li:nth-child(2)"
  author:
    selector: ".author a[rel='author']"
```

### Search vs browse — different layouts

Sites often use a different HTML structure for search results vs the main listing:

```yaml
selectors:
  main_page:
    container: ".game-grid-item"
    fields:
      title:
        selector: "h3"
      cover:
        selector: "img"
        attribute: src
      link:
        selector: "a"
        attribute: href

  search_page:
    container: ".search-result-row"   # different element!
    fields:
      title:
        selector: ".result-name"
      cover:
        selector: ".result-thumb img"
        attribute: data-src
      link:
        selector: "a.result-link"
        attribute: href
```

---

## Tips

- Use your browser's DevTools to find the right selectors — right-click → Inspect
- `container` should be the **smallest** element that wraps exactly one game
- If covers don't load, check for `data-src`, `data-lazy`, or `data-original` attributes
- If links are relative (e.g. `/game/123`), add a `url_normalize` transformation — see [Transformations](06-transformations)
