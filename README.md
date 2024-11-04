# k8sfg

Displays feature gates across Kubernetes versions.

## ToDo

- [ ] Mark when a feature gate is deprecated and/or removed (i.e. - GA and no longer a gate)

## Local Development

`k8sfg` uses Rust stable for production builds, but nightly for local development for formatting and linting. It is not a requirement to use nightly, but if running `fmt` you may see a few warnings on certain features only being available on nightly.

Build the project to pull down dependencies and ensure everything is setup properly:

```sh
cargo build
```

To format the codebase:

If using nightly to use features defined in [rustfmt.toml](rustfmt.toml), run the following:

```sh
cargo +nightly fmt --all
```

If using stable, run the following:

```sh
cargo fmt --all
```

To execute lint checks:

```sh
cargo clippy --all-targets --all-features
```

To run `k8sfg` locally for development:

```sh
cargo run
```

### Running Tests

To execute the tests provided, run the following from the project root directory:

```sh
cargo test --all
```
