---
title: Installing
index: 0
---

# Installing Docgen

There are a few installation options for Docgen. If you would like another installation option,
please open an issue for it.

### Precompiled binaries

Docgen provides precompiled binaries for Mac, Linux, and Windows, which you can download from the
[latest release page](https://github.com/thecodrr/docgen/releases/latest).

### Homebrew

Docgen maintains its own [homebrew tap](https://github.com/Docgen/homebrew-docgen), and you can
install Docgen via the following command:

```
$ brew install docgen/docgen/docgen
```

This will take a few minutes as Docgen is compiled from scratch for your machine.

### Cargo (Rust package manager)

You can also use the Rust package manager, Cargo, to install Docgen. Currently Docgen is not
listed on crates.io, but you can install it directly from GitHub:

```
$ cargo install --git https://github.com/thecodrr/docgen --tag 0.4.2
```
