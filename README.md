<div align="center">
  <h1><code>cargo fuzz</code></h1>

  <p><b>A <code>cargo</code> subcommand for fuzzing with <code>libFuzzer</code>! Easy to use!</b></p>
</div>

## Installation

```sh
$ cargo install cargo-fuzz
```

Note: `libFuzzer` needs LLVM sanitizer support, so this only works on x86-64 Linux, x86-64 macOS
and Apple-Silicon (aarch64) macOS for now. You'll also need a C++ compiler with C++11 support.

If you have an old version of `cargo fuzz`, you can upgrade with this command:

```sh
$ cargo install -f cargo-fuzz
```

## Usage

### `cargo fuzz init`

Initialize a `cargo fuzz` project for your crate!

### If your crate uses cargo workspaces, add `fuzz` directory to `workspace.members` in root `Cargo.toml`

`fuzz` directory can be either a part of an existing workspace (default)
or use an independent workspace. If latter is desired, you can use
`cargo fuzz init --fuzzing-workspace=true`.

### `cargo fuzz add <target>`

Create a new fuzzing target!

### `cargo fuzz run <target>`

Run a fuzzing target and find bugs!

### `cargo fuzz fmt <target> <input>`

Print the `std::fmt::Debug` output for a test case. Useful when your fuzz target
takes an `Arbitrary` input!

### `cargo fuzz tmin <target> <input>`

Found a failing input? Minify it to the smallest input that causes that failure
for easier debugging!

### `cargo fuzz cmin <target>`

Minify your corpus of input files!

### `cargo fuzz coverage <target>`

Generate coverage information on the fuzzed program!

## Documentation

Documentation can be found in the [Rust Fuzz
Book](https://rust-fuzz.github.io/book/cargo-fuzz.html).

You can also always find the full command-line options that are available with
`--help`:

```sh
$ cargo fuzz --help
```

## Trophy case

[The trophy case](https://github.com/rust-fuzz/trophy-case) has a list of bugs
found by `cargo fuzz` (and others). Did `cargo fuzz` and libFuzzer find a bug
for you? Add it to the trophy case!

## License

`cargo-fuzz` is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for
details.
