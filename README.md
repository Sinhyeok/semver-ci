![release workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/release.yml/badge.svg)

# Semver-CI

Welcome to Semver-CI, an open-source project designed to seamlessly integrate semantic versioning into your continuous integration (CI) workflow. This tool automates the process of versioning releases, ensuring that every new build adheres strictly to the [Semantic Versioning](https://semver.org/) guidelines. With Semver-CI, developers can focus more on their code and less on the intricacies of version management.

## Key Features:

- **Automated Version Management**: Automatically increments your project's version based on branch names, tags and predefined rules.

  | **Branch** | **Format** | **Example** |
  | --- | --- | --- |
  | develop, feature/* | v\<version>-<pre-release_stage>.<pre-release_number>.<short_commit_sha> | v0.1.0-dev.1.dfh890fd |
  | release/\*, hotfix/\* | v\<version>-<pre-release_stage>.<pre-release_number> | v0.1.0-rc.1 |
  | main, master | v\<version> | v0.1.0 |
- **Customizable Rules**: Define how your version numbers increase (major, minor, patch) through simple configuration settings.
- **Integration with CI Tools**: Easily integrates with popular CI services like GitHub Actions, GitLab CI, and Jenkins to streamline your development pipeline.
- **Release Drafting**: Automatically generates release notes and drafts new releases with the updated version numbers.

## Why Semver-CI?

In today's fast-paced development environment, managing version numbers can be tedious and error-prone. Semver-CI takes the hassle out of versioning, ensuring your project's releases are consistent, predictable, and in compliance with semantic versioning principles. It's the perfect tool for teams looking to automate their release process and maintain high-quality software.

Start integrating semantic versioning into your CI workflow with Semver-CI today and make your release process as efficient and error-free as possible.

## Getting Started
### GitHub Actions
- [example](https://github.com/Sinhyeok/semver-ci-example)
```yaml
# .github/workflows/build.yml

name: Build

on:
  push:
    branches:
      - 'develop'
      - 'feature/*'
      - 'release/*'
      - 'hotfix/*'
      - 'main'
      - 'master'

jobs:
  upcoming_version:
    runs-on: ubuntu-latest
    container: tartar4s/semver-ci
    outputs:
      UPCOMING_VERSION: ${{ steps.set_upcoming_version.outputs.UPCOMING_VERSION }}
    steps:
      - id: set_upcoming_version
          #export MAJOR='^release/[0-9]+.x.x$'
          #export MINOR='^(develop|feature/.*|release/[0-9]+.[0-9]+.x)$'
          #export PATCH='^hotfix/[0-9]+.[0-9]+.[0-9]+$'
          #export RELEASE='^(main|master)$'
        run: |
          export SCOPE=$(svci scope)
          svci version >> "$GITHUB_OUTPUT"
    env:
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  build:
    runs-on: ubuntu-latest
    needs: upcoming_version
    steps:
      - run: echo "build $RELEASE_TAG"
    env:
      RELEASE_TAG: ${{needs.upcoming_version.outputs.UPCOMING_VERSION}}

  release:
    runs-on: ubuntu-latest
    container: tartar4s/semver-ci
    if: github.ref_name == 'main' || github.ref_name == 'master' || startsWith(github.ref_name, 'release/') || startsWith(github.ref_name, 'hotfix/')
    needs: [upcoming_version, build]
    permissions:
      contents: write
    steps:
      - run: svci release -g "$RELEASE_NAME"
    env:
      RELEASE_NAME: ${{needs.upcoming_version.outputs.UPCOMING_VERSION}}
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```
### GitLab CI/CD
- [example](https://gitlab.com/attar.sh/semver-ci-example)
```yaml
# .gitlab-ci.yml

stages:
  - before_build
  - build
  - release

upcoming_version:
  stage: before_build
  image:
    name: tartar4s/semver-ci
    entrypoint: [""]
  script:
      #export MAJOR='^release/[0-9]+.x.x$'
      #export MINOR='^(develop|feature/.*|release/[0-9]+.[0-9]+.x)$'
      #export PATCH='^hotfix/[0-9]+.[0-9]+.[0-9]+$'
      #export RELEASE='^(main|master)$'
    - |
      export SCOPE=$(svci scope)
      svci version >> version.env
  artifacts:
    reports:
      # version.env:
      #   UPCOMING_VERSION=v1.7.0-rc.1
      #   LAST_VERSION=v1.6.0
      dotenv: version.env
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^(develop|feature\/.+|release\/.+|hotfix\/.+|main|master)$/

build:
  stage: build
  script:
    - echo "build $UPCOMING_VERSION"
  rules:
    - if: $CI_COMMIT_BRANCH

release:
  stage: release
  image:
    name: tartar4s/semver-ci
    entrypoint: [""]
  script:
    - svci release -g -p $LAST_VERSION $UPCOMING_VERSION
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^(main|master|release\/.+|hotfix\/.+)$/
```
### Git Repo
> [!NOTE]
> The Git HEAD must be pointing to the branch. If it's a detached head, semver-ci won't work because it can't find the target branch.
```shell
# help
docker run tartar4s/semver-ci

# version command
docker run -v .:/app tartar4s/semver-ci version --help

# scope command
docker run -v .:/app tartar4s/semver-ci scope --help

# tag command
docker run -v .:/app tartar4s/semver-ci tag --help
```

## Installation
### Using Docker (recommended for CI)
```shell
docker pull tartar4s/semver-ci
```

Versioned images are available for `linux/amd64` in two variants:

| Tag | Base OS | Rust target |
| --- | --- | --- |
| `<version>`, `<version>-alpine` | Alpine | `x86_64-unknown-linux-musl` |
| `<version>-debian` | Debian Bookworm | `x86_64-unknown-linux-gnu` |

Replace `<version>` with a release tag that lists these variants in its artifacts.
The tag without an OS suffix remains an alias for Alpine.

### Build from source
```shell
git clone https://github.com/Sinhyeok/semver-ci.git
cd semver-ci
cargo build --locked --release
./target/release/svci --version
```

## Commands
### version
Print upcoming version based on last semantic version tag and branch
```shell
Usage: svci version [OPTIONS]

Options:
  -s, --scope <SCOPE>  [env: SCOPE=] [default: minor]
  -h, --help           Print help
  -V, --version        Print version
```
#### Example
```shell
% svci version
UPCOMING_VERSION=v0.8.0-dev.1.c8ae805d
LAST_VERSION=v0.7.1
```

#### Official version selection

On branches that produce official versions (including `main` and `master`), or
with `--scope release`, version calculation uses only tags at the target commit
or its ancestors. The target is local `HEAD`, `GITHUB_SHA` in GitHub Actions, or
`CI_COMMIT_SHA` in GitLab CI. Both the previous official version (`LAST_VERSION`)
and automatic prerelease candidates are selected from this history. Annotated
and lightweight tags are supported.

Unmerged release branches do not affect this calculation. For example, a merged
`v1.2.4-rc.1` is promoted to `v1.2.4` even if an unmerged branch has `v2.0.0-rc.1`.
If there is no newer reachable prerelease, the existing minor bump is retained:
`v1.2.3` becomes `v1.3.0`. With no version tags, the initial version is `v0.1.0`
and `LAST_VERSION` is `v0.0.0`. Prerelease generation on development and release
branches keeps its existing version and counter selection rules.

Official calculation requires complete commit history and available version
tags. Shallow repositories fail without printing version outputs; fetching tags
alone does not remove a shallow history boundary. Use a full checkout (for
example, `fetch-depth: 0` with `actions/checkout`, or `GIT_DEPTH: "0"` in GitLab
CI), or run `git fetch --unshallow --tags` for an existing shallow clone. The
internal GitHub Actions clone fetches full history. Local runs should fetch tags
first, or set `FORCE_FETCH_TAGS=true` to refresh them through Semver-CI.

Normal merges and fast-forwards preserve candidate ancestry. Squash merges and
rebases that rewrite tagged commits can remove that relationship; automatic
selection cannot infer the original candidate from equivalent changes. If no
newer candidate remains reachable, the minor fallback applies.

### scope
Print scope based on branch name
```shell
Usage: svci scope [OPTIONS]

Options:
      --major <MAJOR>  [env: MAJOR=] [default: ^release/[0-9]+.x.x$]
      --minor <MINOR>  [env: MINOR=] [default: ^(develop|feature/.*|release/[0-9]+.[0-9]+.x)$]
      --patch <PATCH>  [env: PATCH=] [default: ^hotfix/[0-9]+.[0-9]+.[0-9]+$]
  -h, --help           Print help
  -V, --version        Print version
```
#### Example
```shell
% svci scope
minor
```
### release
Create a release in GitHub or GitLab
```shell
Usage: svci release [OPTIONS] <NAME>

Arguments:
  <NAME>  Release name

Options:
      --description <DESCRIPTION>    Release description [env: DESCRIPTION=] [default: ]
      --tag-name <TAG_NAME>          [env: TAG_NAME=]
      --tag-message <TAG_MESSAGE>    Specify tag_message to create an annotated tag [env: TAG_MESSAGE=] [default: ]
  -g, --generate-release-notes       Automatically generate the body for this release. If description is specified, the description will be pre-pended to the automatically generated notes [env: GENERATE_RELEASE_NOTES=]
  -p, --previous-tag <PREVIOUS_TAG>  (Only for GitLab CI) tag from previous releases to compare when automatically generating release notes [env: PREVIOUS_TAG=] [default: ]
  -s, --strip-prefix-v               Strip prefix "v" from release name and tag name. ex) v0.1.0 => 0.1.0 [env: STRIP_PREFIX_V=]
  -h, --help                         Print help
  -V, --version                      Print version
```
### tag
Create and push git tag to origin
```shell
Usage: svci tag [OPTIONS] <TAG_NAME>

Arguments:
  <TAG_NAME>  

Options:
      --tag-message <TAG_MESSAGE>  [env: TAG_MESSAGE=] [default: ]
  -s, --strip-prefix-v             [env: STRIP_PREFIX_V=]
  -h, --help                       Print help
  -V, --version                    Print version
```
> [!NOTE]
> For tagging on GitLab CI, "SEMVER_CI_TOKEN" with read_repository/write_repository permissions must be set in CI/CD variables


## Development
### Install rustup and cmake
#### Mac
```shell
brew install rustup cmake
```
### Setup Project
The project pins Rust and Cargo to **1.98.1** in `rust-toolchain.toml`.
Rustup installs that toolchain and the configured targets when you run Cargo.
Local development, CI, and Docker builds all use this file.
The Docker bootstrap image uses `rust:1.98-bookworm` to track published patch
updates within Rust 1.98. Rustup installs and selects the project's **1.98.1**
toolchain inside that builder.

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
# GitHub
## develop
#GITHUB_ACTIONS=true
#GITHUB_REF_NAME=develop
#GITHUB_SHA=g9i8thubrt290384egrfy2837
#GITHUB_ACTOR=Sinhyeok
#GITHUB_TOKEN=github_pat_asd897fytaw7890efh2394hef9asdhp9fas8ydfh
#GITHUB_SERVER_URL=https://github.com
#GITHUB_REPOSITORY=Sinhyeok/semver-ci

# GitLab
## develop
GITLAB_CI=true
CI_COMMIT_REF_NAME=develop
CI_COMMIT_SHORT_SHA=g9i0tlab
GITLAB_USER_EMAIL=user@mail.com
SEMVER_CI_TOKEN=glpat_908d21yh0ewfd98h
CI_JOB_TOKEN=vn0w9e7dfgy97esd8f
CI_PROJECT_URL=https://gitlab.com/attar.sh/semver-ci
## hotfix
#GITLAB_CI=true
#CI_COMMIT_REF_NAME=hotfix/0.2.34
#CI_COMMIT_SHORT_SHA=b08640bd

# Git Repo
#GIT_SSH_KEY_PATH=$HOME/.ssh/id_rsa
#GIT_SSH_KEY_PASSPHRASE={YOUR_PASSWORD}
#FORCE_FETCH_TAGS=true
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
rustup component add clippy rustfmt
```
### Run lint
```shell
cargo clippy
cargo fmt
```

### Build and test container images
```shell
docker build --platform linux/amd64 --build-arg VARIANT=alpine -t semver-ci:alpine .
docker build --platform linux/amd64 --build-arg VARIANT=debian -t semver-ci:debian .
bash tests/container.sh semver-ci:alpine alpine
bash tests/container.sh semver-ci:debian debian
```

Local Docker builds use the release profile by default. For a debug build, add
`--build-arg CARGO_PROFILE=dev` to either command; this replaces `debug.Dockerfile`.
The release workflow explicitly uses `CARGO_PROFILE=dev` to preserve the debug
builds previously produced by `debug.Dockerfile`. It builds each variant once,
runs the container checks, and pushes the tested image. The reusable CI workflow
runs Rust lint and tests.
The release workflow runs only on pushes to `develop`, `main`, `release/**`, and
`hotfix/**`. Feature pull requests run PR CI.

## Troubleshooting
- Detached HEAD: Ensure a branch is checked out. In CI, the ref is fetched and checked out automatically.
- Auth/token errors: GitHub requires GITHUB_TOKEN; GitLab requires CI_JOB_TOKEN or SEMVER_CI_TOKEN.
- Tags not up to date: Set FORCE_FETCH_TAGS=true to force-sync remote tags.
- SSH auth: Set GIT_SSH_KEY_PATH and, if needed, GIT_SSH_KEY_PASSPHRASE.

## Contributing & License
- Contributing: See `.github/CONTRIBUTING.md`
- License: See `LICENSE`
