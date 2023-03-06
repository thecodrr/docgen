---
title: Docgen
---

<p align="center">
<img alt="Docgen logo" src="./logo.png" width="200px" />
</p>

<h2 align="center">
Generate static docs from markdown files at the speed of light ⚡
</h2>

---

## What?

Docgen is an opinionated documentation site generator that converts your Markdown files into a beautiful documentation site really, _really_ fast (you can see how fast in these benchmarks). It is **not** a generic static site generator i.e. it is only meant for generating docs.

> This open source tool was initially built by [Doctave](https://github.com/Doctave/doctave/). Since October, 2022 [Streetwriters](https://streetwriters.co) has taken up the maintenance.

### What "speed of light" means

Docgen was built with these goals in mind:

1. Users should be able to quickly get started with minimal configuration or setup — Docgen is a single self-contained binarythat works all platforms and enforces no special directory structures or markdown syntax.
2. Generating the final result should be _fast_ — Docgen can easily take a directory of 10,000 markdown files & turn it into beautiful documentation in under a second.
3. Accessing the docs should be quick — Docgen tries very hard to keep the individual HTML files slim. A 4kB markdown file takes only 9kB after generation.

## Features

Docgen comes with a number of documentation-specific features out of the box. No plugins needed.

- [Mermaid.js](https://mermaid-js.github.io/) diagrams
- Offline full-text search
- Local live-reloading server
- Broken links checking
- Typesetting for mathematical formulas
- Responsive design
- Dark mode
- GitHub flavored markdown
- Minimal configuration
- Symlinked files **[new]**
- Tabs **[new]**
- Static code block syntax highlighting (thanks to syntect) **[new]**
- Fast build times (Docgen is built with Rust)

## Hosting

Docgen-generated sites can be hosted on any static site hosting provider, such as [GitHub
Pages](https://pages.github.com/).

## Screenshots

You can customize the color scheme and logo to match your own style. Below are two examples: one
with Docgen's own color scheme, and another customized color scheme.

| Light | Dark |
| ----- | ---- |
| TBD   | TBD  |
| TBD   | TBD  |

## Installation

There are a few installation options for Docgen. If you would like another installation option,
please open an issue for it.

### Precompiled binaries

Docgen provides precompiled binaries for Mac, Linux, and Windows, which you can download from the
[latest release page](https://github.com/thecodrr/docgen/releases/latest).

### Cargo (Rust package manager)

You can also use the Rust package manager, Cargo, to install Docgen. Currently Docgen is not
listed on crates.io, but you can install it directly from GitHub:

```
$ cargo install --git https://github.com/thecodrr/docgen
```

## Getting started

Once you have Docgen installed, you can run the `init` command to create an initial docs site:

```
$ docgen init
```

Then, run the `serve` command to preview your site locally.

```
$ docgen serve

Docgen | Serve
Starting development server...

Server running on http://0.0.0.0:4001/

```
