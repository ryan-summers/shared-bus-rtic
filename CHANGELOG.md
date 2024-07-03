# Changelog

This file contains all changes to the shared-bus-rtic library.

## [0.2.3+deprecated]

### Added
- This crate is now deprecated. Users should use
[`embedded-hal-bus`](https://crates.io/crates/embedded-hal-bus) instead.

### Fixed
- Removed a call to `expect` which pulled in unnecessary fmt bloat

## [0.2.2] - 2020-07-21

### Added
- Initial release of library on crates.io
- AtomicBool support for thumbv6 platforms
- SPI full-duplex support
- SPI data-types support
