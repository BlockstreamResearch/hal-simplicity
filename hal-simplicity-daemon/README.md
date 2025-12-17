# hal-simplicity-daemon

A JSON-RPC daemon for Simplicity operations. Listens for HTTP POST requests and handles Simplicity-related operations via JSON-RPC.

## Usage

Run with default address (`127.0.0.1:8080`):

```bash
cargo run --bin hal-simplicity-daemon
```

Run with custom address:

```bash
cargo run --bin hal-simplicity-daemon -- -a 0.0.0.0:3000
```
