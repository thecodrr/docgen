---
title: Tabs
---

# Tabs

Docgen supports tabbed content out of the box. There is no unique syntax for tabs — just normal headings wrapped in a special link. Docgen automatically detects & renders the tabs using that.

## Basic example

Tabs start when you create a heading `# [<Tab Title>](#/tab/<id>)` & end either at the end of the document or anywhere you specify a horizontal rule. Here's an example demonstrating just that:

```md
# [Desktop](#/tab/desktop)

1. Right click to open menu

# [Mobile](#/tab/mobile)

1. Long tap to open menu

---
```

# [Desktop](#/tab/desktop)

1. Right click to open menu

# [Mobile](#/tab/mobile)

1. Long tap to open menu

---

## Synced tabs

Tabs with the same `id` are synced by default across the page. This can be useful if you have platform/device specific content split across tabs.

```md
# [Desktop](#/tab/desktop)

1. Right click to open menu

# [Mobile](#/tab/mobile)

1. Long tap to open menu

---

# [Desktop](#/tab/desktop)

1. Click on the close button to close the window

# [Mobile](#/tab/mobile)

1. Swipe up from the bottom to move the application to the background.

---
```

# [Desktop](#/tab/desktop)

1. Right click to open menu

# [Mobile](#/tab/mobile)

1. Long tap to open menu

---

# [Desktop](#/tab/desktop)

1. Click on the close button to close the window

# [Mobile](#/tab/mobile)

1. Swipe up from the bottom to move the application to the background.

---

## Nested tabs

Nested tabs are not current supported.
