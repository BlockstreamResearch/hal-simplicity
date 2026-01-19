# 0.2.0 - 2026-01-06

- Remove second `simplicity` in all commands, so e.g. `hal-simplicity simplictiy simplicity info`
  becomes `hal-simplicity simplicity info`.
  [#13](https://github.com/BlockstreamResearch/hal-simplicity/pull/13)
- Change the default NUMS key to match BIP-0341.
  [#18](https://github.com/BlockstreamResearch/hal-simplicity/pull/18)
- Refactor transaction construction to use PSET rather than raw transactions; add
  the ability to compute sighashes and execute programs with trace output.
  [#12](https://github.com/BlockstreamResearch/hal-simplicity/pull/12)
  [#32](https://github.com/BlockstreamResearch/hal-simplicity/pull/32)
  [#37](https://github.com/BlockstreamResearch/hal-simplicity/pull/37)
- Extend `address inspect` to support Liquid Testnet.
  [#27](https://github.com/BlockstreamResearch/hal-simplicity/pull/27)
- Add basic state commitment support.
  [#33](https://github.com/BlockstreamResearch/hal-simplicity/pull/33)
- Begin refactoring process to allow use as a RPC server.
  [#37](https://github.com/BlockstreamResearch/hal-simplicity/pull/37)
  [#38](https://github.com/BlockstreamResearch/hal-simplicity/pull/38)

# 0.1.0 - 2025-07-29

- Initial release, including hal-elements functions and the `simplicity info` command
