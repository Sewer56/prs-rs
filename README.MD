# prs-rs

[![Crates.io](https://img.shields.io/crates/v/prs-rs.svg)](https://crates.io/crates/prs-rs)
[![Docs.rs](https://docs.rs/prs-rs/badge.svg)](https://docs.rs/prs-rs)
[![CI](https://github.com/Sewer56/prs-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/prs-rs/actions)
[![codecov](https://codecov.io/gh/Sewer56/prs-rs/graph/badge.svg?token=4Xsufoi2VD)](https://codecov.io/gh/Sewer56/prs-rs)

## About

Rust port of SEGA PRS Compression scheme.

You can learn more about this project in the [dedicated documentation page][docs].

## Quick Start

This project uses a cargo workspace structure with all source code organized under the `src/` directory.

### Building

```bash
# Navigate to workspace root
cd src/

# Build the main library
cargo build

# Build the CLI tool
cargo build -p prs-rs-cli

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Development

To start working on the code, open the `src/` folder in your IDE. The `doc/` folder can be opened separately for documentation work.

For detailed contribution guidelines and development practices, see the [documentation](docs/).

## C# Bindings

This project provides C# bindings as [prs_rs.Net.Sys](https://www.nuget.org/packages/prs_rs.Net.Sys).

These are raw bindings to C exports of this Rust library, automatically generated in `src/bindings/csharp/`.

## License

Licensed under [GPL v3 (with Reloaded FAQ)](./LICENSE).

[Learn more about Reloaded's general choice of licensing for projects.][reloaded-license]

[docs]: https://sewer56.dev/prs-rs/
[reloaded-license]: https://reloaded-project.github.io/Reloaded.MkDocsMaterial.Themes.R2/Pages/license/