![release workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/release.yml/badge.svg)

# Semver-CI

Welcome to Semver-CI, an open-source project designed to seamlessly integrate semantic versioning into your continuous integration (CI) workflow. This tool automates the process of versioning releases, ensuring that every new build adheres strictly to the [Semantic Versioning](https://semver.org/) guidelines. With Semver-CI, developers can focus more on their code and less on the intricacies of version management.

## Key Features:

- **Automated Version Management**: Automatically increments your project's version based on branch names, tags and predefined rules.

  | **Branch (default)** | **Stage** | **Format** | **Example** |
  | --- | --- | --- | --- |
  | `develop`, `feature/*` | dev | `vX.Y.Z-dev.N.SHA` | `v0.1.0-dev.1.c8ae805d` |
  | `release/*`, `hotfix/*` | rc | `vX.Y.Z-rc.N` | `v0.1.0-rc.1` |
  | `main`, `master` | stable | `vX.Y.Z` | `v0.1.0` |

  See [Automatic scope and stage](#automatic-scope-and-stage) for supported branch names and bump rules.

- **Customizable Rules**: Define how your version numbers increase (major, minor, patch) through simple configuration settings.
- **CI Integration**: Supports GitHub Actions, GitLab CI, and local Git repositories.
- **Release Creation**: Creates GitHub and GitLab releases, with optional release note generation.

## Why Semver-CI?

In today's fast-paced development environment, managing version numbers can be tedious and error-prone. Semver-CI takes the hassle out of versioning, ensuring your project's releases are consistent, predictable, and in compliance with semantic versioning principles. It's the perfect tool for teams looking to automate their release process and maintain high-quality software.

Start integrating semantic versioning into your CI workflow with Semver-CI today and make your release process as efficient and error-free as possible.

## Getting Started
Run `svci version` to calculate `UPCOMING_VERSION` and `LAST_VERSION` from your
branch and version tags. Scope and stage are inferred automatically.

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

permissions:
  contents: read

jobs:
  upcoming_version:
    runs-on: ubuntu-latest
    container: tartar4s/semver-ci
    outputs:
      UPCOMING_VERSION: ${{ steps.set_upcoming_version.outputs.UPCOMING_VERSION }}
    steps:
      - id: set_upcoming_version
        run: svci version >> "$GITHUB_OUTPUT"
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
  variables:
    GIT_DEPTH: "0"
  image:
    name: tartar4s/semver-ci
    entrypoint: [""]
  script:
    - svci version > version.env
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
    - if: $CI_COMMIT_BRANCH =~ /^(develop|feature\/.+|release\/.+|hotfix\/.+|main|master)$/

release:
  stage: release
  image:
    name: tartar4s/semver-ci
    entrypoint: [""]
  script:
    - svci release -g -p "$LAST_VERSION" "$UPCOMING_VERSION"
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
Print the upcoming and previous versions based on version tags, scope, and stage.
```shell
Usage: svci version [OPTIONS]

Options:
  -s, --scope <SCOPE>    Version increase or candidate promotion. Inferred from branch rules when omitted [env: SCOPE=] [possible values: major, minor, patch, release]
      --stage <STAGE>    Version stage. Inferred from branch rules when omitted [env: STAGE=] [possible values: dev, rc, stable]
      --candidate <TAG>  Promote this exact prerelease tag (official version calculation only) [env: CANDIDATE=]
  -h, --help             Print help
  -V, --version          Print version
```
#### Example
```shell
% svci version
UPCOMING_VERSION=v0.8.0-dev.1.c8ae805d
LAST_VERSION=v0.7.1
```

#### Automatic scope and stage

Both options are optional. Each value uses the command-line option first, then
its environment variable (`SCOPE` or `STAGE`), then the branch rules below.
GitHub Actions, GitLab CI, and local Git repositories share these rules.

| Branch | Scope | Stage |
| --- | --- | --- |
| `develop`, `feature/*` | minor | dev |
| `release/2.x.x` | major | rc |
| `release/1.2.x` | minor | rc |
| `hotfix/1.2.3` | patch | rc |
| `main`, `master` | release | stable |

`major`, `minor`, and `patch` increase the previous official version. `release`
selects a prerelease candidate for promotion and requires stage `stable`.
Stage controls the resulting format: `dev` adds `-dev.N.SHA`, `rc` adds `-rc.N`,
and `stable` has no prerelease suffix. For example, from official `v1.2.3`:

```shell
svci version --scope patch --stage stable # UPCOMING_VERSION=v1.2.4
svci version --scope patch --stage rc     # first candidate: v1.2.4-rc.1
svci version --scope patch                # infer only stage from the branch
svci version --stage rc                  # infer only scope from the branch
```

A stable bump uses the previous official version even when prerelease candidates
exist. For example, `--scope patch --stage stable` produces `v1.2.4` from `v1.2.3`,
even if `v2.0.0-rc.1` exists. It does not promote or increment the candidate.

For `dev` and `rc`, the bump starts from the highest official version among all
available tags, without filtering by commit ancestry. The prerelease number is
one greater than the highest existing number for the calculated version and
stage, or `1` if none exists. `LAST_VERSION` is that previous prerelease tag,
falling back to the highest official version when there is no matching prerelease.

For compatibility, an explicitly supplied `--scope release` or `SCOPE=release`
defaults the stage to `stable`, including on development branches. Explicitly
combining it with stage `dev` or `rc` is an error. When a value needed for
calculation cannot be inferred, the command fails without version outputs and
asks for an option or a branch pattern. An unmapped branch is never implicitly
treated as stable. On main/master, a prerelease stage override also needs an
increase scope, for example `--scope patch --stage rc`.

#### Custom branch patterns

Configure branch regular expressions through environment variables. Scope rules
are checked in `MAJOR`, `MINOR`, `PATCH`, `RELEASE` order; stage rules are checked
in `DEV`, `RC`, `STABLE` order. The first matching rule wins. Patterns replace
their defaults; use alternatives to keep the default branches as well.

| Variable | Default pattern / value |
| --- | --- |
| `MAJOR` | `^release/[0-9]+.x.x$` |
| `MINOR` | `^(develop\|feature/.*\|release/[0-9]+.[0-9]+.x)$` |
| `PATCH` | `^hotfix/[0-9]+.[0-9]+.[0-9]+$` |
| `RELEASE` | `^(main\|master)$` |
| `DEV` | `^(develop\|feature/.*)$` |
| `RC` | `^(release\|hotfix)/.*$` |
| `STABLE` | `^(main\|master)$` |

`RELEASE` and `STABLE` use the same built-in main/master pattern, but their
environment overrides are independent. `RELEASE` configures scope matching;
`STABLE` configures stage matching.

```shell
# On integration: scope=minor, stage=dev
MINOR='^integration$' DEV='^integration$' svci version

# On candidate: scope=patch, stage=rc
PATCH='^candidate$' RC='^candidate$' svci version

# On production: scope=release, stage=stable
RELEASE='^production$' STABLE='^production$' svci version

# Or specify values directly on any named branch
svci version --scope minor --stage dev
```

Only missing values need branch rules. Supplying an increase scope does not
configure its stage: custom development or RC branches also need a matching stage pattern or
an explicit `--stage`/`STAGE`. Invalid patterns used for inference are errors.
The legacy scope regex syntax and precedence are preserved. Version-bearing
branch names select a bump scope, not a maintenance release line.

#### Official version selection

With stage `stable`, the previous official version (`LAST_VERSION`) and automatic
prerelease candidates are selected from tags at the target commit or its ancestors.
The target is local `HEAD`, `GITHUB_SHA` in GitHub Actions, or `CI_COMMIT_SHA` in
GitLab CI. Explicit `--candidate` selection can use a tag outside this history,
as described below. Annotated and lightweight tags are supported.

With scope `release`, unmerged release branches do not affect candidate selection.
For example, a merged `v1.2.4-rc.1` is promoted to `v1.2.4` even if an unmerged
branch has `v2.0.0-rc.1`.
Newer reachable prereleases must agree on a single `major.minor.patch` version.
For example, `v1.2.4-rc.1` and `v1.2.4-rc.2` both produce `v1.2.4`. If candidates
for both `v1.2.4` and `v2.0.0` are reachable, the command fails with the candidate
tag names and prints no version outputs. It does not choose the highest version.
Use `--candidate <tag>` to resolve this ambiguity explicitly.
Prereleases at or below the last official version do not create ambiguity.

With scope `release`, if there is no newer reachable prerelease, the existing
minor bump is retained:
`v1.2.3` becomes `v1.3.0`. With no version tags, the initial version is `v0.1.0`
and `LAST_VERSION` is `v0.0.0`.

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

#### Explicit candidate selection

To promote a specific candidate, including after squash/rebase, pass its exact
tag name (with or without `v`, matching the existing tag):

```shell
svci version --stage stable --candidate v1.2.4-rc.1
# UPCOMING_VERSION=v1.2.4
# LAST_VERSION=v1.2.3
```

`--candidate` (or the `CANDIDATE` environment variable) selects only that tag;
the command-line option takes precedence over the environment. It requires stage
`stable`. If scope is omitted, candidate selection uses scope `release` instead
of inferring a bump from the branch. Explicit scope `major`, `minor`, or `patch`
(including `SCOPE`) conflicts with candidate selection and is an error. The
compatible `--scope release --candidate <tag>` form remains supported. Using a
candidate during prerelease generation is an error.

The tag must exist, point to a commit, and use a supported prerelease format
(`X.Y.Z-rc.N` or `X.Y.Z-dev.N.SHA`, optionally prefixed with `v`). Its official
version must be newer than `LAST_VERSION` and must not already have an official
tag anywhere in the repository, including an equivalent unprefixed tag.
Invalid or missing candidates fail without version outputs or a minor fallback.

Explicit selection declares the caller's intent to associate that candidate
with the target release. The candidate need not be an ancestor of the target;
the command does not verify equivalent source changes or build artifacts.
Pipelines that intend to promote a particular RC should always pass it explicitly,
especially when using squash/rebase. Full history is still required to determine
`LAST_VERSION`, and all version tags must be available to detect existing releases.

### scope
Print scope based on branch name. This command remains supported for independent
use and existing CI workflows. It prints one unchanged value: `major`, `minor`,
`patch`, or `release` (on main/master by default). It does not print stage.
```shell
Usage: svci scope [OPTIONS]

Options:
      --major <MAJOR>      [env: MAJOR=] [default: ^release/[0-9]+.x.x$]
      --minor <MINOR>      [env: MINOR=] [default: ^(develop|feature/.*|release/[0-9]+.[0-9]+.x)$]
      --patch <PATCH>      [env: PATCH=] [default: ^hotfix/[0-9]+.[0-9]+.[0-9]+$]
      --release <RELEASE>  [env: RELEASE=] [default: ^(main|master)$]
  -h, --help               Print help
  -V, --version            Print version
```
#### Example
```shell
% svci scope
minor
```

Existing command composition continues to work:

```shell
export SCOPE=$(svci scope)
svci version
```

The scope command and version command use the same scope rules. Custom patterns
passed as flags to `scope` affect that invocation; configure the stage with an
environment pattern or `--stage` when passing its result to `version` on a custom
branch. The scope command is not deprecated.
Its release pattern uses `--release`, then `RELEASE`, then the default main/master
pattern. Existing `RELEASE` configurations can still be used
with `scope` followed by `SCOPE=release` for version calculation.

### release
Create a release in GitHub or GitLab using the supplied name and tag.
If `--tag-name` is omitted, the name is also used as the tag name.
Version calculation and promotion belong to
`version`; stage `stable` is distinct from creating a provider release.
Releases are published immediately; there is no draft option.
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

The current GitHub integration does not set the release's `prerelease` flag,
even for a dev or RC tag ([#57](https://github.com/Sinhyeok/semver-ci/issues/57)).
`--tag-message` is used only by GitLab. The release command currently accepts
`--strip-prefix-v` but does not apply it
([#56](https://github.com/Sinhyeok/semver-ci/issues/56)); supply an unprefixed
name and `--tag-name` directly when needed.

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

For local development, use the current Git checkout. CI providers supply their
own branch, commit, and job variables when running a real pipeline.

```dotenv
GITHUB_ACTIONS=false
GITLAB_CI=false

# Required for local version calculation; may be empty without remote access.
GIT_TOKEN=
FORCE_FETCH_TAGS=false

# Optional credentials when fetching or pushing over SSH
#GIT_SSH_KEY_PATH=$HOME/.ssh/id_rsa
#GIT_SSH_KEY_PASSPHRASE={YOUR_PASSWORD}
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
`hotfix/**`. Feature pull requests run PR CI. In addition to Rust lint and tests,
PR CI builds and checks both container variants when the PR changes Dockerfiles,
`.dockerignore`, Cargo manifests or lockfiles, the Rust toolchain, `.cargo/`,
workflow definitions, or `tests/container.sh`. These PR checks do not push images.

## Troubleshooting
- Detached HEAD: Local runs require a checked-out branch. GitHub Actions and GitLab CI use the branch and commit variables supplied by the CI provider.
- Auth/token errors: GitHub requires `GITHUB_TOKEN`. GitLab uses `CI_JOB_TOKEN`; `SEMVER_CI_TOKEN` overrides credentials for Git operations, while release API requests still use `CI_JOB_TOKEN`. Local `version` and `tag` commands require `GIT_TOKEN`, which may be empty when no remote authentication is needed.
- Tags not up to date: Local runs can set `FORCE_FETCH_TAGS=true` to fetch remote tags. GitHub Actions and GitLab CI runs always fetch tags before version calculation.
- SSH auth: Set GIT_SSH_KEY_PATH and, if needed, GIT_SSH_KEY_PASSPHRASE.

## Contributing & License
- Contributing: See `.github/CONTRIBUTING.md`
- License: See `LICENSE`
