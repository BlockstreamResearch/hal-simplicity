# hal-simplicity-daemon

A JSON-RPC daemon for Simplicity operations. Listens for HTTP POST requests and handles Simplicity-related operations via JSON-RPC.

## Usage

Run with default address (`127.0.0.1:28579`):

```bash
cargo run --bin hal-simplicity-daemon
```

Run with custom address:

```bash
cargo run --bin hal-simplicity-daemon -- -a 0.0.0.0:3000
```

## daemon feature

You can disable the `daemon` feature to exclude heavy dependencies if daemon functionality is not required:

```toml
hal-simplicity-daemon = { version = "0.1.0", default-features = false }
```
