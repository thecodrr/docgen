# Docgen

**Docgen is a fork of [Doctave](https://github.com/Doctave/doctave/) intending to continue the development. All credit goes to the original Docgen authors.**

Docgen is an opinionated documentation site generator that converts your Markdown files into
a beautiful documentation site with minimal effort.

Docgen is not a generic static site generator - it is only meant for generating documentation sites
from Markdown. This allows the tool to be much simpler than other solutions, with fewer
configuration steps.

This open source tool is built and maintained by [Streetwriters.co](https://streetwriters.co).

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
