Bats
====

Bats is a free and open source (WIP) groovebox for Linux.

Global Key Bindings
-------------------

| Key   | Action                           |
|:------|:---------------------------------|
| `ESC` | Go to main menu.                 |
| `m`   | Go to metrenome page.            |
| `t`   | Go to tracks page.               |
| `q`   | Quit.                            |
| `M`   | Toggle the metronome on and off. |

Plugins
-------

### Toof

A polyphonic sawtooth wave instrument.

Building
--------

Bats is built with the `cargo build` command. To build and run the relase version, run `cargo run --release`. The required dependencies to build Bats are libraries and development libraries for `jack`, `sdl2`, and `sdl2-ttf`. See the "Building" section of `./.github/workflows/testing.yml` for the specific dependencies on Ubuntu Linux.

Tools
-----

-	Cargo - Cargo is used for building, testing, and other utilities.
-	Cargo Flamegraph - Cargo utility for benchmarking the binary live and producing a visualization.

### Code Coverage

```shell
CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test
grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html
find . -name "*.profraw" -delete
xdg-open target/coverage/html/html/index.html
```
