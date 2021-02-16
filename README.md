<div align="center">
  <h1><code>cargo fuzz</code></h1>

  <p><b>A <code>cargo</code> subcommand for using <code>libFuzzer</code>! Easy to use! No need to recompile LLVM!</b></p>
</div>

## Installation

```sh
$ cargo install cargo-fuzz
```

Note: `libFuzzer` needs LLVM sanitizer support, so this only works on x86-64
Linux and x86-64 macOS for now. This also needs a nightly Rust toolchain since
it uses some unstable command-line flags. Finally, you'll also need a C++
compiler with C++11 support.

If you have an old version of `cargo fuzz`, you can upgrade with this command:

```sh
$ cargo install -f cargo-fuzz
```

## Usage

### `cargo fuzz init`

Initialize a `cargo fuzz` project for your crate!

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

## Documentation

Documentation can be found in the [Rust Fuzz
Book](https://rust-fuzz.github.io/book/cargo-fuzz.html).

You can also always find the full command-line options that are available with
`--help`:

```sh
$ cargo fuzz --help
```

## Generating code coverage information

Use the `--coverage` build option to generate precise
[source-based code coverage](https://blog.rust-lang.org/inside-rust/2020/11/12/source-based-code-coverage.html)
information. This compiles your project using the `-Zinstrument-coverage` Rust compiler flag.

Running the generated binary creates raw profiling data in a file called `default.profraw`.
This file can be used to generate coverage reports and visualize code-coverage information
as described in the [Unstable book](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/source-based-code-coverage.html#installing-llvm-coverage-tools).

Minimal example of visualizing code coverage:

1. Run the fuzzer using
   
   `$ cargo fuzz run --coverage <target>`
2. Integrate 
  
   `$llvm-profdata merge -sparse default.profraw -o default.profdata`
3. 

Note: we recommend using LLVM 11 and a recent nightly version of the Rust toolchain.
This code was tested with `1.51.0-nightly (2021-02-10)`.

## Trophy case

[The trophy case](https://github.com/rust-fuzz/trophy-case) has a list of bugs
found by `cargo fuzz` (and others). Did `cargo fuzz` and libFuzzer find a bug
for you? Add it to the trophy case!

## License

`cargo-fuzz` is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for
details.
