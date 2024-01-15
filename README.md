![lint workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/lint.yml/badge.svg)
# semver-ci
Semantic Versioning for CI/CD

## Development
### Install rustup
#### Mac
```shell
brew install rustup
```
### Clone Project and Run
```shell
# Clone project
git clone git@github.com:Sinhyeok/semver-ci.git
cd semver-ci

# Show help
cargo run version --help

# Run
cargo run version
cargo run version --scope major
cargo run version --scope patch
```
### Install Lint Tools
```shell
rustup component add clippy
rustup component add rustfmt
```
### Run lint
```shell
cargo clippy
cargo fmt
```
