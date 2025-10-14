hal-simplicity -- a Simplictiy-enabled extension of hal
============================================

This is a fork of Steven Roose's [hal-elements](https://github.com/stevenroose/hal-elements/)
which in turn is an extension of his Bitcoin tool [hal](https://github.com/stevenroose/hal).

# Installation

```
$ cargo install --locked hal hal-simplicity
```

You can also run it directly with `cargo run -- <command>`.

---

## HAL-SIMPLICITY Commands

### hal-simplicity simplicity address create
Create Simplicity addresses
```bash
hal-simplicity simplicity address create <program>
```

### hal-simplicity simplicity address inspect
Inspect Simplicity addresses
```bash
hal-simplicity simplicity address inspect <address>
```

### hal-simplicity simplicity keypair generate
Generate a random private/public keypair
```bash
hal-simplicity simplicity keypair generate
```

### hal-simplicity simplicity simplicity info
Parse a base64-encoded Simpliicity program and decode it
```bash
hal-simplicity simplicity simplcitiy info <base64-program>
```

### hal-simplicity simplicity sighash
Compute sighash for a Simplicity transaction input (draft PR #9)
```bash
hal-simplicity simplicity sighash <tx-hex> <input-index> <cmr> <control-block> -i <input-utxo> [-g <genesis-hash>] [-s <secret-key>]
```

### hal-simplicity simplicity tx create
Create a raw Simplicity transaction from JSON
```bash
hal-simplicity simplicity tx create <tx-info-json>
hal-simplicity simplicity tx create --raw-stdout <tx-info-json>
```

### hal-simplicity simplicity tx decode
Decode a raw Simplicity transaction to JSON
```bash
hal-simplicity simplicity tx decode <tx-hex>
```

### hal-simplicity simplicity block create
Create a raw block from JSON
```bash
hal-simplicity simplicity block create <block-info-json>
hal-simplicity simplicity block create --raw-stdout <block-info-json>
```

### hal-simplicity simplicity block decode
Decode a Simplicity block
```bash
hal-simplicity simplicity block decode <block-hex>
```

