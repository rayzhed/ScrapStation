---
title: "Authentication"
sidebar_position: 11
---

# Authentication

Some sources require users to be logged in before content is accessible. ScrapStation handles this through **WebView sessions** — a real browser window that stores cookies just like a normal browser.

---

## How it works

1. You define a login button in `setting_sections` with `action: open_webview`
2. The user clicks it — a browser window opens pointing to the login page
3. The user logs in normally in that browser window
4. The app detects the target cookie and closes the window automatically
5. All subsequent requests to that source include the captured cookies

The session persists between app restarts.

---

## Requiring authentication to load the source

If the site uses Cloudflare, DDoS-Guard, or any JavaScript challenge that blocks direct HTTP requests:

```yaml
auth:
  requires_webview_fetch: true
```

With this flag, the app will route all page fetches through the embedded WebView and refuse to load the source until an active session exists.

---

## Setting up the login UI

```yaml
setting_sections:
  - id: account
    title: "Account"
    icon: user
    components:
      - type: button
        label: "Log In"
        icon: log-in
        action: open_webview
        hide_when:
          key: cookies
          exists: true
        action_config:
          url: /login
          wait_for_cookie: session_id
          close_on_domains:
            - example.com
          store_cookies_as: cookies

      - type: status_card
        variant: success
        icon: check-circle
        text: "Authenticated"
        show_when:
          key: cookies
          exists: true

      - type: button
        label: "Log Out"
        icon: log-out
        variant: danger
        show_when:
          key: cookies
          exists: true
        action: clear_storage
        action_config:
          keys:
            - cookies
          clear_webview: true
```

### `action_config` for `open_webview`

| Field | Required | Notes |
|---|---|---|
| `url` | ✅ | Login page URL (relative or absolute) |
| `wait_for_cookie` | ✅ | Cookie name that signals successful login |
| `close_on_domains` | — | Auto-close window when navigating to these domains |
| `store_cookies_as` | ✅ | Storage key for captured cookies (use `cookies`) |

---

## Multiple login methods

```yaml
- type: button_group
  hide_when:
    key: cookies
    exists: true
  buttons:
    - label: "Google"
      icon: chrome
      color: "#4285F4"
      action: open_webview
      action_config:
        url: /auth?provider=google
        wait_for_cookie: auth_session
        close_on_domains: [example.com]
        store_cookies_as: cookies

    - label: "Discord"
      icon: message-circle
      color: "#5865F2"
      action: open_webview
      action_config:
        url: /auth?provider=discord
        wait_for_cookie: auth_session
        close_on_domains: [example.com]
        store_cookies_as: cookies
```

---

## Using cookies in requests

The engine reads the `cookies` stored value and automatically attaches it as a `Cookie:` header on every request. No extra config needed — just store them under the key `cookies`.

---

## Manual cookie entry

Users can paste cookies manually from their browser's DevTools:

```yaml
- type: input
  id: cookies
  label: "Manual Cookies"
  description: "Paste your browser cookies directly (from DevTools → Application → Cookies)"
  placeholder: "session_id=abc123; other_cookie=xyz"
  secret: true
  store_as: cookies
  show_when:
    key: cookies
    exists: false
```

---

## Security notes

- Cookies are stored in the app's local config directory — never transmitted elsewhere
- Clearing a source's settings wipes its cookies
- The WebView session is sandboxed inside the app — does not share cookies with your system browser
