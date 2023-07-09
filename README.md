# Overview

bats! is a DAW that is a work in progress.

![./assets/logo.png](./assets/logo.png)

## Goals

### Lightweight

bats! should be lightweight enough to run on a Raspberry Pi.

# Build

## Dependencies

bats! depends on Guile Scheme, LV2, and Jack. To see a more detailed
set of dependencies, see the "Install Dependencies" section of
`./.github/workflows/test.yml`.

## Build Command

```shell
cargo build
```

## Running

### Cargo Run

bats! can be run with `cargo run`.

```bash
cargo run
```

### From Guile Scheme

bats! can also be run directly from Guile. This method may be more
reliable to integrate with other toos like Emacs Geiser.

```bash
cargo build
guile
(load "main.scm")
```
