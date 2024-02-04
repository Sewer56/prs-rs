# prs-rs

[![Crates.io](https://img.shields.io/crates/v/prs-rs.svg)](https://crates.io/crates/prs-rs)
[![Docs.rs](https://docs.rs/prs-rs/badge.svg)](https://docs.rs/prs-rs)
[![CI](https://github.com/Sewer56/prs-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/prs-rs/actions)

-----------------------

## Additional Setup Required!!

Additional setup might be required.  
Once you do the stuff under this text, delete this section.  

### Code Coverage
To setup code coverage for this project. Login into [Codecov][codecov] and add this repository from your list of tracked repos.

### Setup Documentation

If you don't have a `docs/Reloaded` folder, an error occurred, and you might need to add it manually as submodule:

```
git submodule add https://github.com/Reloaded-Project/Reloaded.MkDocsMaterial.Themes.R2.git docs/Reloaded
```

Once you make your first commit to GitHub, you'll need to do the following:
- Go to `Settings -> Pages` in your repo and select `gh-pages` branch to enable GitHub Pages.


### Setup API Key

You'll need to set up an API key to publish to `crates.io` on tag.

- Go to `Settings -> Secrets -> Actions` in your repo and add environment secret named `CRATES_IO_TOKEN`.  
- Paste your API key generated in [crates.io (Account Settings -> API Tokens)][crates-io-key].  


-----------------------

## About

Rust port of the SEGA PRS Compression scheme

You can learn more about this project in the [dedicated documentation page][docs].

## Development

How to develop this project.

***Clone this Repository:***
```bash
# When cloning, make sure symlinks are enabled
git clone -c core.symlinks=true https://github.com/Sewer56/prs-rs.git
```

***Install Rust:***
- Install the [Rust Toolchain.][rust-toolchain]***Setup IDE***
- This repository is fully with VSCode. [Guidance below](#visual-studio-code-integration).
### Visual Studio Code Integration

`Code`/`VSCode` is the de-facto Rust development environment.  

The following extensions are required:  
- [rust-analyzer][rust-analyzer] for Rust support.  
- [coverage-gutters][coverage-gutters] for Coverage support.  
- [CodeLLDB][codelldb] for debugging.  
- [crates](https://marketplace.visualstudio.com/items?itemName=serayuzgur.crates) easier dependency management.  

The VSCode configuration in Reloaded projects (`.vscode`) contain the following:  
- Run Rust linter `clippy` on Save.  
- Run code format `rustfmt` on Save.  
- Tasks for common operations (generate documentation, active CI/CD etc.).  

These configurations are in the `.vscode` folder; and the tasks can be ran via `Ctrl+Shift+P -> Run Task`.  

#### Test Coverage

To run Coverage, run task (`Ctrl+Shift+P -> Run Task`), you should see: 

| Task                   | Description                                                                |
| ---------------------- | -------------------------------------------------------------------------- |
| Cargo Watch Tarpaulin  | Automatically runs tests and updates coverage on save.                     |
| Generate Code Coverage | Manually generate code coverage (`cobertura.xml`, `tarpaulin-report.html`) |

The `tarpaulin-report.html` file can be opened in VSCode (`Show Preview`) for a live view.

For GUI integration, run action `Coverage Gutter: Watch` (in `Ctrl+Shift+P` actions menu).

## Debugging Benchmarks

If you wish to debug benchmarks in VSCode, go to `Run and Debug` Menu and generate the launch 
profiles, you should get one for debugging benchmarks.

## Profiling Benchmarks

### Linux/OSX

Execute the following:

```
cargo bench --bench my_benchmark --profile profile -- --profile-time 10
```

This should give you a flamegraph in `target/criterion/<method_name>/profile`. You can open that flamegraph in a web browser.

### Windows

Execute the following:

```
cargo bench --bench my_benchmark --no-run --profile profile
```

Navigate to the executable listed in the commandline:

```
target/profile/deps/my_benchmark-eced832ac8f31257.exe
```

And run with command `my_benchmark-eced832ac8f31257.exe --bench --profile-time 10` under an external profiler, such as Visual Studio.  

![example](./assets/profile_example.png)

## Optimizing for Size when Creating C Libraries

1. Add `"cdylib"` crate type to `Cargo.toml` (if not already present)

```
[lib]
crate-type = ["cdylib"]
```

Install `cargo-bloat`, `nightly toolchain` and `build-std`:

```
cargo install cargo-bloat
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

Run `cargo-bloat` the following command to calculate package size:

```
RUSTFLAGS="-C panic=abort -C lto=fat -C embed-bitcode=yes" cargo +nightly bloat -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --target x86_64-pc-windows-gnu --profile profile --crate-type cdylib -n 100
```

Change `--target` if needed for your platform.  
This should produce binaries more appropriate for dynamic linking from C.  

## PGO (Profile Guided Optimization) on C Libraries

This Reloaded-based library is built with [Profile Guided Optimization (PGO)](https://doc.rust-lang.org/rustc/profile-guided-optimization.html).

PGO is a compiler optimization technique that uses data from profiling runs to improve the quality of the generated code.

Details of the PGO implementation in this project are as follows:

- We collect PGO data by running the `benchmarks` with the `pgo` feature enabled.
- This is done in CI, before building the final C library.

You should ensure that only realistic representative workloads are used to collect the PGO data.

For example, if this was a compression library, you should run the 'compress' and 'decompress' methods
on real files (not random data) as part of your benchmarks. 

Non-realistic/representative workloads in benchmarks should be excluded through the 'pgo' feature flag,
for example an unrealistic benchmark can be excluded like this:

```rust
#[cfg(not(feature = "pgo"))]
{
    bench_create_dict(c);
}
```

### Testing PGO

PGO isn't guaranteed to always provide an improvement, after adding representative workloads, always test.

We will test with `cargo pgo`.

First, install the following:

```
cargo install cargo-pgo
rustup toolchain install nightly
rustup component add llvm-tools-preview
```

Then run an 'instrumented' benchmark, this will run your code in `pgo_benchmark` and collect some data:

```
cargo +nightly pgo instrument bench
```

After that run a regular benchmark to create a 'baseline' number:

```
cargo +nightly bench
```

And run the PGO optimized build:

```
cargo +nightly pgo optimize bench
```

If most of the results are equal or show an improvement, PGO has helped.
Otherwise disable PGO from the library by editing the [rust.yml](./.github/workflows/rust.yml) workflow.

## File Layout

The following is the expected file layout for your project:

```
.vscode/
docs/
src/
Cargo.toml
mkdocs.yml
```

The `docs` folder, and `mkdocs.yml` contain [MkDocs Material documentation][mkdocs-material] for your project.  
The `src` folder should contains all source code for your project.  

`Cargo.toml` should be in the root of the project.  
## Cross Platform Targeting

Some templates allow for cross platform development.  

To work with cross-platform code, where you need to access OS specific APIs, some helper scripts are provided.  

### Including All Code Paths

To include all code paths for local builds, consider editing `.cargo/config.toml`.

```toml
[build]
# Note: This breaks IntelliJ Rust. Remove this line temporarily if working from that IDE.
target = ['x86_64-unknown-linux-gnu','x86_64-apple-darwin','x86_64-pc-windows-gnu']
```

You might need to install the targets first:

```bash
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-apple-darwin
rustup target add x86_64-pc-windows-gnu
```

Now when you run `cargo build`, it will build code for all platforms; and you'll get your compiler errors, warnings etc.

### Cross Testing on Local Machine

#### Prerequisites (Windows)

- Install [Docker Desktop](https://www.docker.com/products/docker-desktop/).  
- Disable WSL 2 (Docker Desktop -> Settings -> General -> Use the WSL 2 based engine).  

#### Prerequisites (Linux)

- Install [Podman](https://podman.io) from your package manager.  

#### Prerequisites (Common)

Install cross

```
cargo install cross
```

#### Running Cross-Platform Tests

Use the provided `pwsh` scripts in `scripts` folder.

- `./test-wine-x64.ps1`: Tests your code in Wine on x86_64.  
- `./test-linux-x64.ps1`: Tests your code in Linux on x86_64.  
- `./test-linux-x86.ps1`: Tests your code in Linux on x86.  

These scripts can be used on any platform given the prerequisites are met.  
If you need to test Apple stuff without an Apple machine, you're generally out of luck outside of using CI/CD for testing.  

## Contributing

See [CONTRIBUTING](CONTRIBUTING.MD) for guidance on how to contribute to this project.  

## License

Licensed under [GPL v3 (with Reloaded FAQ)](./LICENSE).  

[Learn more about Reloaded's general choice of licensing for projects.][reloaded-license].  

[codecov]: https://about.codecov.io/
[codelldb]: https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb
[coverage-gutters]: https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters 
[crates-io-key]: https://crates.io/settings/tokens
[docs]: https://prs-rs.github.io/prs-rs
[mkdocs-material]: https://squidfunk.github.io/mkdocs-material/
[reloaded-license]: https://reloaded-project.github.io/Reloaded.MkDocsMaterial.Themes.R2/Pages/license/
[rust-analyzer]: https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer
[rust-toolchain]: https://www.rust-lang.org/tools/install
