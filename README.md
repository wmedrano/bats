# Bats

Bats is a free and open source (WIP) groovebox for Linux.

## Global Key Bindings

| Key   | Action                           |
|:------|:---------------------------------|
| `ESC` | Go to main menu.                 |
| `m`   | Go to metrenome page.            |
| `t`   | Go to tracks page.               |
| `q`   | Quit.                            |
| `M`   | Toggle the metronome on and off. |

## Plugins

### Toof

A polyphonic sawtooth wave instrument.


## Building

Bats is built with the `cargo build` command. To build and run the
relase version, run `cargo run --release`. The required dependencies
to build Bats are libraries and development libraries for `jack`,
`sdl2`, and `sdl2-ttf`. See the "Building" section of
`./.github/workflows/testing.yml` for the specific dependencies on
Ubuntu Linux.
