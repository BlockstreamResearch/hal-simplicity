hal-simplicity -- a Simplicity-enabled extension of hal
============================================

This is a fork of Steven Roose's [hal-elements](https://github.com/stevenroose/hal-elements/)
which in turn is an extension of his Bitcoin tool [hal](https://github.com/stevenroose/hal).

# Installation

```
$ cargo install --locked hal hal-simplicity
```

You can also run it directly with `cargo run -- <command>`.

# Example: Validating a Signature Hash

Consider transaction [e54d31ce544b65a3768d7dc44a9caf1142eb1ce9bb46707f5a83cb1ccf9b77f9](https://blockstream.info/liquidtestnet/nojs/tx/e54d31ce544b65a3768d7dc44a9caf1142eb1ce9bb46707f5a83cb1ccf9b77f9?expand)
on Liquid Testnet, which spends a simple "pay to public key" Simplicity program.

We can obtain the raw transaction from the Block Explorer by replacing `/nojs/` with
`/api/` in the above URL and adding `/hex` to the end, [as so](https://liquid.network/liquidtestnet/api/tx/e54d31ce544b65a3768d7dc44a9caf1142eb1ce9bb46707f5a83cb1ccf9b77f9/hex):

```
020000000101b33e0e092f2f229bb472f7ac15b22783908ebf5b70d9fefa13fc627979b2ca6c0000000000000000000201499a818545f6bae39fc03b637f2a4e1e64e590cac1bc3a6f6d71aa4443654c140100000000000182b800160014b58c22151f4ba159e2255767472ac89137e8183001499a818545f6bae39fc03b637f2a4e1e64e590cac1bc3a6f6d71aa4443654c140100000000000003e8000000000000000004609bef8d556d80e43ae7e0becb3a7e6838b95defe45896ed6075bb9035d06c9964e09e91b2ee81dd61d97ec6e83bfdb03c28f79e0e7038a98964ea5c29cde0b2319878a86dc9e5a0d00269215a43754755a6e173246ad7d330eb82d27e779fd90afd7201e4fba0509b4df120e1d320451f14172c46476646daf8d0d6da80e84c986cc5e073f80ed4dcf0210284187248126ac8e671544245742660022ae160c5e14b09ec0c2a17584bf5c548c85961c02b6efc010c03109ad2420c3f00140b16ab91cd75dcbc1e84ea7a320719cbfc6dc95e5194f9eca996d55a7b2d768c511e2a310e1806240a1241b70a35627302ef7da851f75a1f471748121a2b6978930a58ccaee2309401bd1b6e9fcbb0018601881a80e12071190284906e2a37159c2a162cdba0e67e0aad66c82658ec0c7f2a5a2cc38c3f61a892acd0da3a133ff9ead668873dc60c0310b5b0730445fea038d226980c2e6f7e4be9e895848d1fd97f2100db43004cb4eaddefc50601885c078170e6f13a1848e019ef88de2e7a3c1561d1828b3be0f290def9feebf54da94249472c0c0312050920fc8238dc861438a059b630e6ef256702d23cf92f32979f4fcd9ff3909cf7b32538aafb0e3a23ec40079b1d130c03103785c207e4c8201c5a072580e4e0207fd424f70498ef2fb6dd05ffbb7368dc796e6c47f24404e0b1ff138cfce89a7a21bff5919fa64ce45f8306849072b26c1bfdd2937e6b81774796ff372bd1eb5362d20000000000
```

We see that this transaction spends the transaction output with outpoint
`6ccab2797962fc13fafed9705bbf8e908327b215acf772b49b222f2f090e3eb3:0`, which contains
the Simplicity program, and that this is the 0th input of the transaction. From its
witness stack, we see (reading bottom to top):

* Its control block is `bff5919fa64ce45f8306849072b26c1bfdd2937e6b81774796ff372bd1eb5362d2`
* The CMR of the Simplicity program is `7fd424f70498ef2fb6dd05ffbb7368dc796e6c47f24404e0b1ff138cfce89a7a`
* The Simplicity program itself starts with `e4fba`...
* The program's witness is `9bef8d556d80e43ae7e0becb3a7e6838b95defe45896ed6075bb9035d06c9964e09e91b2ee81dd61d97ec6e83bfdb03c28f79e0e7038a98964ea5c29cde0b2319878a86dc9e5a0d00269215a43754755a6e173246ad7d330eb82d27e779fd90a`,
  which for this program we know consists of a public key `9bef8d556d80e43ae7e0becb3a7e6838b95defe45896ed6075bb9035d06c9964`
  and a signature `e09e91b2ee81dd61d97ec6e83bfdb03c28f79e0e7038a98964ea5c29cde0b2319878a86dc9e5a0d00269215a43754755a6e173246ad7d330eb82d27e779`.

If you are unsure which components of the witness stack are what, you can attempt to parse each one
of them as a Simplicity program. When we get the correct one, it will parse and produce a CMR which
appears elsewhere in the witness stack:

```
$ hal-simplicity simplicity info e4fba0509b4df120e1d320451f14172c46476646daf8d0d6da80e84c986cc5e073f80ed4dcf0210284187248126ac8e671544245742660022ae160c5e14b09ec0c2a17584bf5c548c85961c02b6efc010c03109ad2420c3f00140b16ab91cd75dcbc1e84ea7a320719cbfc6dc95e5194f9eca996d55a7b2d768c511e2a310e1806240a1241b70a35627302ef7da851f75a1f471748121a2b6978930a58ccaee2309401bd1b6e9fcbb0018601881a80e12071190284906e2a37159c2a162cdba0e67e0aad66c82658ec0c7f2a5a2cc38c3f61a892acd0da3a133ff9ead668873dc60c0310b5b0730445fea038d226980c2e6f7e4be9e895848d1fd97f2100db43004cb4eaddefc50601885c078170e6f13a1848e019ef88de2e7a3c1561d1828b3be0f290def9feebf54da94249472c0c0312050920fc8238dc861438a059b630e6ef256702d23cf92f32979f4fcd9ff3909cf7b32538aafb0e3a23ec40079b1d130c03103785c207e4c8201c5a072580e4e0
{
  "jets": "core",
  "commit_base64": "5PugUJtN8SDh0yBFHxQXLEZHZkba+NDW2oDoTJhsxeBz+A7U3PAhAoQYckgSasjmcVRCRXQmYAIq4WDF4UsJ7AwqF1hL9cVIyFlhwCtu/AEMAxCa0kIMPwAUCxarkc113LwehOp6MgcZy/xtyV5RlPnsqZbVWnstdoxRHioxDhgGJAoSQbcKNWJzAu99qFH3Wh9HF0gSGitpeJMKWMyu4jCUAb0bbp/LsAGGAYgagOEgcRkChJBuKjcVnCoWLNug5n4KrWbIJljsDH8qWizDjD9hqJKs0No6Ez/56tZohz3GDAMQtbBzBEX+oDjSJpgMLm9+S+nolYSNH9l/IQDbQwBMtOrd78UGAYhcB4Fw5vE6GEjgGe+I3i56PBVh0YKLO+DykN75/uv1TalCSUcsDAMSBQkg/II43IYUOKBZtjDm7yVnAtI8+S8yl59PzZ/zkJz3syU4qvsOOiPsQAebHRMMAxA3hcIH5MggHFoHJYDk4A==",
  "commit_decode": "(witness  & iden); (((unit; const 0xbe241c3a6408a3e282e588c8ecc8db5f1a1adb501d09930d98bc0e7f01da9b9e ) & iden); (((IOH; ((((false & unit); assertl drop jet_sha_256_ctx_8_init ) & iden); ((((false & (OH & IH)); assertl drop jet_sha_256_ctx_8_add_32 ) & iden); ((false & OH); assertl drop jet_sha_256_ctx_8_finalize )))) & iden); ((((false & ((false & (OH & IOH)); assertl drop jet_eq_256 )); assertl drop jet_verify ) & ((((false & unit); assertl drop jet_sig_all_hash ) & iden); ((false & ((IIIOH & OH) & witness )); assertl drop jet_bip_0340_verify ))); IH)))",
  "type_arrow": "1 → 1",
  "cmr": "7fd424f70498ef2fb6dd05ffbb7368dc796e6c47f24404e0b1ff138cfce89a7a",
  "liquid_address_unconf": "ex1pyuvwaqedernfdc7c6qf7r67en3szas6s0sdegzq3jxduhj4mhles29dz23",
  "liquid_testnet_address_unconf": "tex1pyuvwaqedernfdc7c6qf7r67en3szas6s0sdegzq3jxduhj4mhlestul9m7",
  "is_redeem": false
}
```

This gives us the canonical base64 encoding of the program, which the Block Explorer does not provide,
and lets us confirm the CMR. We can then guess which part of the witness stack is the witness for
the Simplicity program, by passing various blobs as the second argument of `simplicity info`:

```
$ hal-simplicity simplicity info 5PugUJtN8SDh0yBFHxQXLEZHZkba+NDW2oDoTJhsxeBz+A7U3PAhAoQYckgSasjmcVRCRXQmYAIq4WDF4UsJ7AwqF1hL9cVIyFlhwCtu/AEMAxCa0kIMPwAUCxarkc113LwehOp6MgcZy/xtyV5RlPnsqZbVWnstdoxRHioxDhgGJAoSQbcKNWJzAu99qFH3Wh9HF0gSGitpeJMKWMyu4jCUAb0bbp/LsAGGAYgagOEgcRkChJBuKjcVnCoWLNug5n4KrWbIJljsDH8qWizDjD9hqJKs0No6Ez/56tZohz3GDAMQtbBzBEX+oDjSJpgMLm9+S+nolYSNH9l/IQDbQwBMtOrd78UGAYhcB4Fw5vE6GEjgGe+I3i56PBVh0YKLO+DykN75/uv1TalCSUcsDAMSBQkg/II43IYUOKBZtjDm7yVnAtI8+S8yl59PzZ/zkJz3syU4qvsOOiPsQAebHRMMAxA3hcIH5MggHFoHJYDk4A== 9bef8d556d80e43ae7e0becb3a7e6838b95defe45896ed6075bb9035d06c9964e09e91b2ee81dd61d97ec6e83bfdb03c28f79e0e7038a98964ea5c29cde0b2319878a86dc9e5a0d00269215a43754755a6e173246ad7d330eb82d27e779fd90a`
{
  "jets": "core",
  "commit_base64": "5PugUJtN8SDh0yBFHxQXLEZHZkba+NDW2oDoTJhsxeBz+A7U3PAhAoQYckgSasjmcVRCRXQmYAIq4WDF4UsJ7AwqF1hL9cVIyFlhwCtu/AEMAxCa0kIMPwAUCxarkc113LwehOp6MgcZy/xtyV5RlPnsqZbVWnstdoxRHioxDhgGJAoSQbcKNWJzAu99qFH3Wh9HF0gSGitpeJMKWMyu4jCUAb0bbp/LsAGGAYgagOEgcRkChJBuKjcVnCoWLNug5n4KrWbIJljsDH8qWizDjD9hqJKs0No6Ez/56tZohz3GDAMQtbBzBEX+oDjSJpgMLm9+S+nolYSNH9l/IQDbQwBMtOrd78UGAYhcB4Fw5vE6GEjgGe+I3i56PBVh0YKLO+DykN75/uv1TalCSUcsDAMSBQkg/II43IYUOKBZtjDm7yVnAtI8+S8yl59PzZ/zkJz3syU4qvsOOiPsQAebHRMMAxA3hcIH5MggHFoHJYDk4A==",
  "commit_decode": "(witness  & iden); (((unit; const 0xbe241c3a6408a3e282e588c8ecc8db5f1a1adb501d09930d98bc0e7f01da9b9e ) & iden); (((IOH; ((((false & unit); assertl drop jet_sha_256_ctx_8_init ) & iden); ((((false & (OH & IH)); assertl drop jet_sha_256_ctx_8_add_32 ) & iden); ((false & OH); assertl drop jet_sha_256_ctx_8_finalize )))) & iden); ((((false & ((false & (OH & IOH)); assertl drop jet_eq_256 )); assertl drop jet_verify ) & ((((false & unit); assertl drop jet_sig_all_hash ) & iden); ((false & ((IIIOH & OH) & witness )); assertl drop jet_bip_0340_verify ))); IH)))",
  "type_arrow": "1 → 1",
  "cmr": "7fd424f70498ef2fb6dd05ffbb7368dc796e6c47f24404e0b1ff138cfce89a7a",
  "liquid_address_unconf": "ex1pyuvwaqedernfdc7c6qf7r67en3szas6s0sdegzq3jxduhj4mhles29dz23",
  "liquid_testnet_address_unconf": "tex1pyuvwaqedernfdc7c6qf7r67en3szas6s0sdegzq3jxduhj4mhlestul9m7",
  "is_redeem": true,
  "redeem_base64": "5PugUJtN8SDh0yBFHxQXLEZHZkba+NDW2oDoTJhsxeBz+A7U3PAhAoQYckgSasjmcVRCRXQmYAIq4WDF4UsJ7AwqF1hL9cVIyFlhwCtu/AEMAxCa0kIMPwAUCxarkc113LwehOp6MgcZy/xtyV5RlPnsqZbVWnstdoxRHioxDhgGJAoSQbcKNWJzAu99qFH3Wh9HF0gSGitpeJMKWMyu4jCUAb0bbp/LsAGGAYgagOEgcRkChJBuKjcVnCoWLNug5n4KrWbIJljsDH8qWizDjD9hqJKs0No6Ez/56tZohz3GDAMQtbBzBEX+oDjSJpgMLm9+S+nolYSNH9l/IQDbQwBMtOrd78UGAYhcB4Fw5vE6GEjgGe+I3i56PBVh0YKLO+DykN75/uv1TalCSUcsDAMSBQkg/II43IYUOKBZtjDm7yVnAtI8+S8yl59PzZ/zkJz3syU4qvsOOiPsQAebHRMMAxA3hcIH5MggHFoHJYDk4A==",
  "witness_hex": "9bef8d556d80e43ae7e0becb3a7e6838b95defe45896ed6075bb9035d06c9964e09e91b2ee81dd61d97ec6e83bfdb03c28f79e0e7038a98964ea5c29cde0b2319878a86dc9e5a0d00269215a43754755a6e173246ad7d330eb82d27e779fd90a",
  "amr": "463d72225db55f4bb9811a9d4d921854ef8893fa92b6358f6d47719ecfb1ced5",
  "ihr": "8d3f7748cf509656920ecdb9cb424cac4f55f5e9cc0a1083f98fa87e74fca3e1"
}
```

We see that the witness parsed correctly and we now get some extra fields in the output. (Though be
aware that any blob of the correct size would have been accepted; this serves as a sanity check but
is not a reliable way to determine what the correct witness for a program is.)

Next, we look at the input transaction, [6ccab2797962fc13fafed9705bbf8e908327b215acf772b49b222f2f090e3eb3](https://blockstream.info/liquidtestnet/nojs/tx/6ccab2797962fc13fafed9705bbf8e908327b215acf772b49b222f2f090e3eb3).
Its 0th output has:

* scriptPubKey `51202718ee832dc8e696e3d8d013e1ebd99c602ec3507c1b940811919bcbcabbbff3`
* asset ID `144c654344aa716d6f3abcc1ca90e5641e4e2a7f633bc09fe3baf64585819a49`
* amount 0.00100000 BTC

Putting all this data together, we can invoke `simplicity sighash`:

```
$ hal-simplicity simplicity sighash 020000000101b33e0e092f2f229bb472f7ac15b22783908ebf5b70d9fefa13fc627979b2ca6c0000000000000000000201499a818545f6bae39fc03b637f2a4e1e64e590cac1bc3a6f6d71aa4443654c140100000000000182b800160014b58c22151f4ba159e2255767472ac89137e8183001499a818545f6bae39fc03b637f2a4e1e64e590cac1bc3a6f6d71aa4443654c140100000000000003e8000000000000000004609bef8d556d80e43ae7e0becb3a7e6838b95defe45896ed6075bb9035d06c9964e09e91b2ee81dd61d97ec6e83bfdb03c28f79e0e7038a98964ea5c29cde0b2319878a86dc9e5a0d00269215a43754755a6e173246ad7d330eb82d27e779fd90afd7201e4fba0509b4df120e1d320451f14172c46476646daf8d0d6da80e84c986cc5e073f80ed4dcf0210284187248126ac8e671544245742660022ae160c5e14b09ec0c2a17584bf5c548c85961c02b6efc010c03109ad2420c3f00140b16ab91cd75dcbc1e84ea7a320719cbfc6dc95e5194f9eca996d55a7b2d768c511e2a310e1806240a1241b70a35627302ef7da851f75a1f471748121a2b6978930a58ccaee2309401bd1b6e9fcbb0018601881a80e12071190284906e2a37159c2a162cdba0e67e0aad66c82658ec0c7f2a5a2cc38c3f61a892acd0da3a133ff9ead668873dc60c0310b5b0730445fea038d226980c2e6f7e4be9e895848d1fd97f2100db43004cb4eaddefc50601885c078170e6f13a1848e019ef88de2e7a3c1561d1828b3be0f290def9feebf54da94249472c0c0312050920fc8238dc861438a059b630e6ef256702d23cf92f32979f4fcd9ff3909cf7b32538aafb0e3a23ec40079b1d130c03103785c207e4c8201c5a072580e4e0207fd424f70498ef2fb6dd05ffbb7368dc796e6c47f24404e0b1ff138cfce89a7a21bff5919fa64ce45f8306849072b26c1bfdd2937e6b81774796ff372bd1eb5362d20000000000 0 7fd424f70498ef2fb6dd05ffbb7368dc796e6c47f24404e0b1ff138cfce89a7a bff5919fa64ce45f8306849072b26c1bfdd2937e6b81774796ff372bd1eb5362d2 -i '51202718ee832dc8e696e3d8d013e1ebd99c602ec3507c1b940811919bcbcabbbff3:144c654344aa716d6f3abcc1ca90e5641e4e2a7f633bc09fe3baf64585819a49:0.001' -p 9bef8d556d80e43ae7e0becb3a7e6838b95defe45896ed6075bb9035d06c9964 -s e09e91b2ee81dd61d97ec6e83bfdb03c28f79e0e7038a98964ea5c29cde0b2319878a86dc9e5a0d00269215a43754755a6e173246ad7d330eb82d27e779fd90a`
{
  "sighash": "d832133eba9525e9e452752e6b2193b3c71084af75832b385a4c108f8100947d",
  "signature": null,
  "valid_signature": true
}
```

Which tells us the `SIGHASH_ALL` sighash for the transaction input, as well as validating the signature
we provided with the public key we provided, which confirms that our witness was well-formed.

If we had not provided the public key and signature, this command would have simply output the sighash
with no further information. If we had instead provided a *secret* key with `-s`, the command would have
produced a signature for us.

# Command Reference

### hal-simplicity address create
Create Simplicity addresses
```bash
hal-simplicity address create <program>
```

### hal-simplicity address inspect
Inspect Simplicity addresses
```bash
hal-simplicity address inspect <address>
```

### hal-simplicity keypair generate
Generate a random private/public keypair
```bash
hal-simplicity keypair generate
```

### hal-simplicity simplicity info
Parse a base64-encoded Simplicity program and decode it
```bash
hal-simplicity simplicity info <base64-program>
```

### hal-simplicity simplicity sighash
Compute sighash for a Simplicity transaction input (draft PR #9)
```bash
hal-simplicity simplicity sighash <tx-hex> <input-index> <cmr> <control-block> -i <input-utxo> [-g <genesis-hash>] [-s <secret-key>]
```

### hal-simplicity tx create
Create a raw Simplicity transaction from JSON
```bash
hal-simplicity tx create <tx-info-json>
hal-simplicity tx create --raw-stdout <tx-info-json>
```

### hal-simplicity tx decode
Decode a raw Simplicity transaction to JSON
```bash
hal-simplicity tx decode <tx-hex>
```

### hal-simplicity block create
Create a raw block from JSON
```bash
hal-simplicity block create <block-info-json>
hal-simplicity block create --raw-stdout <block-info-json>
```

### hal-simplicity block decode
Decode a Simplicity block
```bash
hal-simplicity block decode <block-hex>
```

