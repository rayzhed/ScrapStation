---
title: "Settings UI"
sidebar_position: 10
---

# Settings UI

`setting_sections` lets you define a custom settings panel for your source — shown in the app's Settings modal. Use it for login flows, API keys, toggles, and any per-user configuration your source needs.

---

## Structure

```yaml
setting_sections:
  - id: account
    title: "Account"
    icon: user
    description: "Manage your login"
    components:
      - type: button
        ...

  - id: preferences
    title: "Preferences"
    icon: sliders
    components:
      - type: toggle
        ...
```

---

## Visibility conditions

Every component supports `show_when` and `hide_when`:

```yaml
show_when:
  key: cookies
  exists: true

hide_when:
  key: cookies
  exists: true

show_when:
  key: mode
  equals: "advanced"
```

---

## Component types

### `button`

```yaml
- type: button
  label: "Open Dashboard"
  icon: external-link
  variant: default            # "default", "danger", "success"
  action: open_external
  action_config:
    link: "https://example.com/dashboard"
```

---

### `button_group`

Multiple buttons side by side — ideal for login options.

```yaml
- type: button_group
  hide_when:
    key: cookies
    exists: true
  buttons:
    - label: "Sign In"
      icon: log-in
      color: "#0a84ff"
      action: open_webview
      action_config:
        url: /login
        wait_for_cookie: session_id
        close_on_domains:
          - example.com
        store_cookies_as: cookies
```

---

### `toggle`

```yaml
- type: toggle
  id: auto_resolve
  label: "Auto-resolve download links"
  description: "Automatically follow redirect chains"
  default: true
  store_as: auto_resolve
```

---

### `input`

```yaml
- type: input
  id: api_key
  label: "API Key"
  placeholder: "sk-..."
  secret: true
  store_as: api_key
```

---

### `select`

```yaml
- type: select
  id: quality
  label: "Preferred Quality"
  default: "1080p"
  store_as: quality
  options:
    - value: "4k"
      label: "4K Ultra HD"
    - value: "1080p"
      label: "1080p Full HD"
    - value: "720p"
      label: "720p HD"
```

---

### `status_card`

```yaml
- type: status_card
  variant: success          # "success", "warning", "error", "info"
  icon: check-circle
  text: "Logged in successfully"
  show_when:
    key: cookies
    exists: true
```

---

### `text`

```yaml
- type: text
  content: "You need an account to access downloads."
  variant: info             # "info", "warning", "muted"
```

---

### `divider`

```yaml
- type: divider
```

---

## Action types

### `open_webview`

```yaml
action: open_webview
action_config:
  url: /login
  wait_for_cookie: session_id
  close_on_domains:
    - example.com
  store_cookies_as: cookies
```

### `clear_storage`

```yaml
action: clear_storage
action_config:
  keys:
    - cookies
  clear_webview: true
```

### `open_external`

```yaml
action: open_external
action_config:
  link: "https://example.com/register"
```

---

## Full example

```yaml
setting_sections:
  - id: account
    title: "Account"
    icon: user
    description: "Log in to access downloads"
    components:
      - type: text
        content: "Create a free account to get started."
        variant: muted
        hide_when:
          key: cookies
          exists: true

      - type: button_group
        hide_when:
          key: cookies
          exists: true
        buttons:
          - label: "Sign In"
            icon: log-in
            color: "#0a84ff"
            action: open_webview
            action_config:
              url: /login
              wait_for_cookie: auth_token
              close_on_domains:
                - example.com
              store_cookies_as: cookies

      - type: status_card
        variant: success
        icon: check-circle
        text: "You are logged in"
        show_when:
          key: cookies
          exists: true

      - type: button
        label: "Sign Out"
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

  - id: preferences
    title: "Preferences"
    icon: sliders
    components:
      - type: toggle
        id: auto_resolve
        label: "Auto-resolve links"
        description: "Follow redirect chains automatically before downloading"
        default: true
        store_as: auto_resolve
```
