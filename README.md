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

Use the `--coverage` option to generate precise
[source-based code coverage](https://blog.rust-lang.org/inside-rust/2020/11/12/source-based-code-coverage.html)
information:
```
$ cargo fuzz run --coverage <coverage data output file> <target>
```
This compiles your project using the `-Zinstrument-coverage` Rust compiler flag and generates coverage data in the
specified file. This file can be used to generate coverage reports and visualize code-coverage information.
If you run the fuzzer multiple times, you can specify different coverage-output file names and subsequently merge them
into one data file.
Read more in the [Unstable book](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/source-based-code-coverage.html#installing-llvm-coverage-tools).

### Example

Suppose we have a `compiler` fuzz target for which we want to visualize code coverage.

1. Run the fuzzer on the `compiler` target:
   
   `$ cargo fuzz build --coverage "run1" compiler`
   
   This will generate a file named `run1.profraw` in the same directory.

2. You can run the fuzzer again, generating more profiler data in another output file:

   `$ cargo fuzz run --coverage "run2" compiler`

3. Merge the coverage data files and index them with the instrumented compiler code: 
  
   `$llvm-profdata merge -sparse run1.profraw run2.profraw -o runs.profdata`

4. Visualize the coverage data in HTML:

   `llvm-cov show target/.../compiler --format=html -instr-profile=runs.profdata > index.html`
   
   There are many visualization and coverage-report options available (see `llvm-cov show --help`).

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
