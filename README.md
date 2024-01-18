![lint workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/lint.yml/badge.svg)
# semver-ci
Semantic Versioning for CI/CD

## Development
### Install rustup
#### Mac
```shell
brew install rustup
```
### Setup Project
```shell
# Clone project
git clone git@github.com:Sinhyeok/semver-ci.git
cd semver-ci

# Create .env
touch .env
vi .env
```
#### Example `.env`
```dotenv
#GITHUB_ACTIONS=true
#GITHUB_REF_NAME=develop
#GITHUB_SHA=g9i8thubrt290384egrfy2837

GITLAB_CI=true
CI_COMMIT_BRANCH=develop
CI_COMMIT_SHORT_SHA=g9i0tlab
```

### Run
```shell
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
