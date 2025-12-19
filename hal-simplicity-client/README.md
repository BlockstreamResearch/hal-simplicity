# hal-simplicity-client

A CLI and library client for Simplicity operations.

## Usage

Run the client:

```bash
cargo run
```

## embed_daemon feature

You can disable the `embed_daemon` feature to exclude heavy dependencies if daemon embedding functionality is not required,
then you are forced to connect to an external daemon for Simplicity operations:

```toml
hal-simplicity = { version = "0.1.0", default-features = false }
```

## Tests

Run tests:

```bash
cargo test -- --test-threads=1
```
