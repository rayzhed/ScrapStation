---
title: "Notices"
sidebar_position: 15
---

# Notices

Notices are popup modals shown to the user at specific moments — for example, the first time they start a download. They're useful for communicating one-time instructions or warnings without cluttering the UI permanently.

---

## Structure

```yaml
notices:
  - id: unique_notice_id
    trigger: download_start
    once: true
    style: info             # "info", "warning", "danger", "success"
    title: "Heads Up"
    message: "Your message here."
```

Multiple notices can be defined. Each is identified by its `id`.

---

## Fields

| Field | Required | Notes |
|---|---|---|
| `id` | ✅ | Unique string — used to track whether the user has seen it |
| `trigger` | ✅ | When to show the notice (see below) |
| `once` | — | Default `false`. Set `true` to only show it once ever |
| `style` | — | `info` (default), `warning`, `danger`, `success` |
| `title` | ✅ | Bold heading of the modal |
| `message` | ✅ | Body text of the modal |

---

## Triggers

| Trigger | When it fires |
|---|---|
| `download_start` | Just before the user's first download from this source in the session (or ever, if `once: true`) |

---

## Example — first-download warning

```yaml
notices:
  - id: mysite_first_download
    trigger: download_start
    once: true
    style: info
    title: "First Download — Heads Up"
    message: "A browser window will open when you start downloading. This is normal — just close it and return to the app. This message only appears once."
```

---

## Example — login required

```yaml
notices:
  - id: mysite_login_required
    trigger: download_start
    once: false
    style: warning
    title: "Login Required"
    message: "You must be logged in to download from this source. Go to Settings → Account to sign in."
```
