![lint workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/lint.yml/badge.svg)
# semver-ci
Semantic Versioning for CI/CD

## Development
### Install rustup
#### Mac
```shell
brew install rustup
```
### Run
```shell
cargo run version --help
cargo run version
cargo run version --scope major
cargo run version --scope patch
```
### Install lint tools
```shell
rustup component add clippy
rustup component add rustfmt
```
### Run lint
```shell
cargo clippy
cargo fmt
```
