# covidd


Render COVID-19 case data for Dresden in the terminal

## Installation

### Prerequisites

This tool is build with Rust so you need to have a rust toolchain and cargo installed.
If you don't, please visit [https://rustup.rs/](https://rustup.rs/) and follow their instructions.

### Building

The preferred way is to run:

```rust
make install
```

If you do not have a fairly recent `make` (on macOS, homebrew can install a newer version),
or don't want to use make, you can also run `cargo install --path .`.

### Already built binaries

If you don't want to compile on your own, you can find binaries at [the Github release page](https://github.com/knutwalker/covidd/releases).

## Usage

Run `covidd`.

- Press Up/Right to zoom in
- Press Down/Left to zoom out
- Press Home/End to fully zoom in/out
- Press 1 through 9 to zoom to the latest <n> weeks
- Press q to quit

Run `covidd --help` for an overview of more available options.

#### Screenshot

![have a look at doc/screenshot.png](https://knutwalker.s3.eu-central-1.amazonaws.com/covidd/doc/screenshot.png)


License: MIT OR Apache-2.0
