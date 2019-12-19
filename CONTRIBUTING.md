# Contributing to `cargo fuzz`

## Testing

Run all the tests the usual way:

```
cargo test
```

The test suite is located in `tests/tests/main.rs`. The tests use a test project
builder in `tests/tests/project.rs` to create `cargo fuzz` projects that we can
run various commands on and verify that they behave as expected. These test
projects end up in `cargo-fuzz/target/tests/*`, so if a test fails and you want
to debug it, you can find its test project directory in there. Run `cargo test
-- --nocapture --test-threads 1` to get the logs saying which directory is the
one used by your failing test in particular.

## Code Style

We use the current stable Rust channel's `rustfmt`, and enforce that code is
formatted this way in CI.

```sh
# If you don't already have `rustfmt` installed, run this:
rustup component add rustfmt

# Format the code!
cargo +stable fmt
```
