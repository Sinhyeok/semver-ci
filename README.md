# Semver-CI

![release workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/release.yml/badge.svg)

Calculate semantic versions from Git branches and tags, and create releases in GitHub or GitLab.

## Getting Started

Run `svci version` to calculate `UPCOMING_VERSION` and `LAST_VERSION` from your
branch and version tags. Scope and stage are inferred automatically.
The CI examples pass the calculated version to build and release jobs.

### GitHub Actions

[Example repository](https://github.com/Sinhyeok/semver-ci-example)

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
      RELEASE_TAG: ${{ needs.upcoming_version.outputs.UPCOMING_VERSION }}

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
      RELEASE_NAME: ${{ needs.upcoming_version.outputs.UPCOMING_VERSION }}
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### GitLab CI/CD

[Example repository](https://gitlab.com/attar.sh/semver-ci-example)

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

### Local Git Repository

Run from a Git repository with a branch checked out. For version calculation and
tagging, configure the [local Git credentials](#authentication) as needed.

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

### Docker

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

## Versioning

### Scope and stage

**Scope** chooses how the core version changes: `major`, `minor`, and `patch`
increment an official version or select the base release line for an exact target;
`release` promotes a prerelease candidate.
**Stage** chooses the version format: `dev`, `rc`, or `stable`.
Scope `release` requires stage `stable`.

`svci version` calculates these values and prints versions. The separate
[`release` command](#release) creates a GitHub or GitLab release using a supplied
name and tag.

### Default branch rules

GitHub Actions, GitLab CI, and local Git repositories use the same rules.
These examples assume a previous official version of `v1.2.3` and no prerelease tags.

| Branch | Scope | Stage | Format | Example |
| --- | --- | --- | --- | --- |
| `develop`, `feature/*` | minor | dev | `vX.Y.Z-dev.N.SHA` | `v1.3.0-dev.1.c8ae805d` |
| `release/2.x.x` | major | rc | `vX.Y.Z-rc.N` | `v2.0.0-rc.1` |
| `release/1.3.x` | minor | rc | `vX.Y.Z-rc.N` | `v1.3.0-rc.1` |
| `hotfix/1.2.4` | patch | rc | `vX.Y.Z-rc.N` | `v1.2.4-rc.1` |
| `main`, `master` | release | stable | `vX.Y.Z` | `v1.3.0` |

### Exact release targets and maintenance branches

Version-bearing branches select an exact target as well as scope and stage:

| Branch | Target | Previous official version |
| --- | --- | --- |
| `hotfix/M.m.p` | `M.m.p` (`p > 0`) | Highest reachable official version with the same major and minor. |
| `release/M.m.x` | `M.m.0` (`m > 0`) | Highest reachable official version with the same major. |
| `release/M.x.x` | `M.0.0` (`M > 0`) | Highest reachable official version. |

Reachable means tagged at the target commit or one of its ancestors. The commit
is local `HEAD`, `GITHUB_SHA` in GitHub Actions, or `CI_COMMIT_SHA` in GitLab CI.
All targets must be newer than the selected previous official version and must
not already have an official tag anywhere in the repository, with or without `v`.
These constraints apply to dev, RC, stable bumps, and candidate promotion.

For example, `hotfix/1.2.4` branched from `v1.2.3` produces `v1.2.4-rc.1` and
`LAST_VERSION=v1.2.3` even if a separate branch has already released `v2.0.0`.
`release/1.3.x` targets `1.3.0`, using the previous reachable `1.*` official
version; it fails after `1.3.0` has been published. Use `hotfix/1.3.1` for the next
patch release. Targets may skip numbers: `hotfix/1.2.9` can start from `v1.2.3`.

When no official tag is reachable, `v0.0.0` can serve as the base only if it fits
the scope's release line, such as `hotfix/0.0.1`, `release/0.1.x`, or `release/1.x.x`.
A missing base for `hotfix/1.2.4` or `release/1.3.x` is an error. Fetch complete
history and tags and check where the branch was created.

Use `--target X.Y.Z` or `TARGET` for an exact target on a custom branch. A leading
`v` is accepted. Scope and stage still follow their usual option/pattern rules:

```shell
# On a custom branch: prepare or directly calculate the 1.2.4 maintenance release
svci version --scope patch --stage rc --target 1.2.4
svci version --scope patch --stage stable --target 1.2.4
```

A target with nonzero patch requires scope `patch`; a target ending in `.0` with
nonzero minor requires `minor`; a target ending in `.0.0` requires `major`.
Scope `release` is also supported for stable candidate promotion. Conflicting
scope values from flags, environment variables, or custom patterns fail. An
explicit target must match a version-bearing branch's target. For example,
`release/2.x.x --scope patch` and `hotfix/1.2.4 --target 1.2.5` are errors.

The `release/` and `hotfix/` prefixes require the supported target formats above,
with decimal components, no leading zeroes, and no `v` prefix. Custom regexes and
explicit options cannot bypass this validation in `version`.

### Dev and RC versions

With an exact target, prerelease calculation uses that core version and the
previous official version selected from its release line and commit history.
Without a target, it applies the scope bump to the highest official tag at the
target commit or one of its ancestors. The target commit is local `HEAD`,
`GITHUB_SHA` in GitHub Actions, or `CI_COMMIT_SHA` in GitLab CI.

The highest reachable prerelease number for the exact core version and stage is
incremented. Tags outside the target's history do not affect the prerelease number.
The number starts at `1` when no matching reachable prerelease exists.
Dev versions also include the target commit's short SHA.

For example, when `develop` includes `v1.2.3` and an unmerged release branch has
`v2.0.0`, `develop` still calculates `v1.3.0-dev.N.SHA`. Once the `v2.0.0` commit
is merged into `develop`, it calculates `v2.1.0-dev.N.SHA`.

For example, with official `v1.2.3` and candidate `v1.3.0-rc.2`, scope `minor`
and stage `rc` produce `v1.3.0-rc.3`. Dev and RC counters are independent.
Without an exact target and with no reachable official tags, the base is `v0.0.0`.

### Stable versions and promotion

Without an exact target, stable calculation selects the highest official tag at the target commit or
one of its ancestors. The target is local `HEAD`, `GITHUB_SHA` in GitHub Actions,
or `CI_COMMIT_SHA` in GitLab CI. Annotated and lightweight tags are supported.
Complete commit history and available version tags are required; see
[shallow clones and missing tags](#shallow-clones-and-missing-tags).

Without an exact target, the resolved scope determines the next version:

| Scope | Behavior | Example from `v1.2.3` |
| --- | --- | --- |
| `major`, `minor`, `patch` | Apply the bump to the previous official version, even when candidates exist. | `patch` produces `v1.2.4`, even with `v2.0.0-rc.1`. |
| `release` | Promote newer reachable prereleases when they agree on one core version. | `v1.2.4-rc.1` and `v1.2.4-rc.2` produce `v1.2.4`. |
| `release`, with no newer reachable candidate | Fall back to a minor bump. | `v1.3.0`; with no version tags, `v0.1.0`. |

Automatic promotion ignores unmerged candidates and prereleases at or below the
previous official version. If newer reachable candidates target different core
versions, such as `v1.2.4` and `v2.0.0`, calculation fails and lists the candidates.
Use explicit selection to choose one.

With an exact target, bump scopes produce that target directly. Scope `release`
promotes only a reachable prerelease with the matching core version. If none
exists, calculation fails; it does not fall back to a minor bump. Use
`--candidate` to select a matching tag outside the commit's history, or the
target's bump scope with `--stage stable` to calculate it directly.

### Explicit candidate selection

Pass the exact existing tag name to promote a particular candidate, including
after a squash merge or rebase:

```shell
svci version --stage stable --candidate v1.2.4-rc.1
```

This produces `v1.2.4` when the candidate is valid. Candidate selection requires
stage `stable` and defaults an omitted scope to `release`. Explicit bump scopes
(`major`, `minor`, `patch`), including values supplied through `SCOPE`, conflict
with `--candidate`. The legacy `--scope release --candidate <tag>` form also works.

The candidate must:

- Exist as a tag pointing to a commit, using its exact name, including any `v` prefix.
- Use `X.Y.Z-rc.N` or `X.Y.Z-dev.N.SHA`, optionally prefixed with `v`.
- Have a core version newer than the target's previous official version.
- Have no corresponding official tag anywhere in the repository, with or without `v`.
- Match the branch or explicit target when one is selected.

The candidate can be outside the target's ancestry. Explicit selection associates
it with the target release without verifying equivalent source changes or build
artifacts. Full history is still required to find the previous official version,
and all version tags must be available to detect an existing release.
Invalid candidates fail without version outputs or a minor fallback.

### Version outputs

Before printing outputs, every stage checks the final `UPCOMING_VERSION` against
all available repository tags, including tags outside the target commit's history.
The check compares the full version, including the SHA for dev versions, and treats
tags with and without the `v` prefix as equivalent. A collision fails with the
existing tag's name and no version outputs; it does not select another candidate
or increment the version again. Base selection and prerelease counters still use
only reachable tags. Existing target and explicit-candidate checks also apply.

`version` writes two `KEY=value` lines to standard output:

```text
UPCOMING_VERSION=v1.3.0-rc.3
LAST_VERSION=v1.3.0-rc.2
```

| Output | Dev / RC | Stable |
| --- | --- | --- |
| `UPCOMING_VERSION` | The next prerelease for the resolved scope and stage. | The bumped or promoted official version. |
| `LAST_VERSION` | The highest reachable prerelease for the calculated core version and stage, or the selected previous official version if none matches. | The selected previous official version from the commit's history and, when specified, the release line. |

When no previous version applies, `LAST_VERSION` is `v0.0.0`.
Diagnostics go to standard error, so CI jobs can capture standard output directly.

## Configuration

### Configuration precedence

For each option, the command-line value takes precedence over its environment
variable. For scope and stage, an omitted value is then inferred from branch
rules. Only missing values require matching rules.

`--target` takes precedence over `TARGET`. A target inferred from the branch is
always a constraint: explicit target and scope values must agree with it.

An explicit `--scope release` or `SCOPE=release` defaults an omitted stage to
`stable` on any named branch for compatibility. An explicit candidate defaults
an omitted scope to `release`, as described in [candidate selection](#explicit-candidate-selection).

### Override scope and stage

```shell
# Choose both values directly
svci version --scope patch --stage stable
svci version --scope patch --stage rc

# On develop: override scope and infer stage=dev
svci version --scope patch

# On develop: override stage and infer scope=minor
svci version --stage rc
```

The corresponding environment variables are `SCOPE` and `STAGE`.
On main/master, a dev or RC override also needs a bump scope because the default
scope is `release`. Unknown branches require every missing value to be configured;
they are never implicitly treated as stable.

### Custom branch patterns

Set environment variables to replace the default regular expressions. Scope
rules are checked in `MAJOR`, `MINOR`, `PATCH`, `RELEASE` order; stage rules in
`DEV`, `RC`, `STABLE` order. The first matching rule in each group wins.

| Variable | Default pattern |
| --- | --- |
| `MAJOR` | `^release/[0-9]+.x.x$` |
| `MINOR` | `^(develop\|feature/.*\|release/[0-9]+.[0-9]+.x)$` |
| `PATCH` | `^hotfix/[0-9]+.[0-9]+.[0-9]+$` |
| `RELEASE` | `^(main\|master)$` |
| `DEV` | `^(develop\|feature/.*)$` |
| `RC` | `^(release\|hotfix)/.*$` |
| `STABLE` | `^(main\|master)$` |

`RELEASE` and `STABLE` have the same built-in pattern, but independent environment
overrides: `RELEASE` configures scope, and `STABLE` configures stage.
The legacy scope regex syntax and precedence are preserved. Invalid patterns
in a group needed for inference cause an error, even if an earlier rule matches.
These patterns select scope/stage only; target parsing and validation are
independent. Custom branches outside `release/` and `hotfix/` can use `--target`.

```shell
# On integration: scope=minor, stage=dev
MINOR='^integration$' DEV='^integration$' svci version

# On candidate: scope=patch, stage=rc
PATCH='^candidate$' RC='^candidate$' svci version

# On production: scope=release, stage=stable
RELEASE='^production$' STABLE='^production$' svci version
```

Patterns replace their defaults; use regex alternatives to keep the default
branches as well. Custom development and RC branches need both a scope and a
stage rule unless the corresponding value is supplied directly.

### Authentication

| Environment | Git credentials | Release API credentials |
| --- | --- | --- |
| GitHub Actions | `GITHUB_TOKEN` | `GITHUB_TOKEN` |
| GitLab CI | `SEMVER_CI_TOKEN` when set; otherwise `CI_JOB_TOKEN` | `CI_JOB_TOKEN` |
| Local Git | `GIT_TOKEN` for HTTPS, or the SSH settings below | Provider release creation is unavailable in local Git mode. |

For GitHub Actions, grant `contents: read` for version calculation and
`contents: write` for release creation or tag pushes. For GitLab tag pushes,
provide a `SEMVER_CI_TOKEN` with `read_repository` and `write_repository` permissions.
The GitLab runner supplies `CI_JOB_TOKEN`; overriding Git credentials does not
change the token used for release API requests.

Local `version` and `tag` commands require `GIT_TOKEN` to be set. It can be empty
when remote authentication is unnecessary, including version calculation using
only existing local tags. Local tagging also uses Git's `user.name` and `user.email`.

### Git settings and tag fetching

| Variable | Default | Purpose |
| --- | --- | --- |
| `CLONE_TARGET_PATH` | `.` | Repository directory. |
| `FORCE_FETCH_TAGS` | `false` for local Git | Fetch tags from `origin` before local version calculation when `true`. |
| `GIT_SSH_KEY_PATH` | Unset | Private key path; required when Git authenticates over SSH. |
| `GIT_SSH_KEY_PASSPHRASE` | Unset | Optional passphrase for the SSH key. |

GitHub Actions and GitLab CI always fetch tags before calculating a version.
`FORCE_FETCH_TAGS` controls local Git runs only. Fetching tags alone does not
complete a shallow repository's commit history.

## Commands

Use `svci <command> --help` for usage and `svci --version` for the installed
version. Each command also supports `-h`/`--help` and `-V`/`--version`.

### version

Calculate and print the [upcoming and previous versions](#version-outputs).
This command reads Git data; creating tags or provider releases is a separate step.

```shell
svci version
svci version --scope patch --stage stable
svci version --stage stable --candidate v1.2.4-rc.1
```

| Option | Environment variable | Behavior / default |
| --- | --- | --- |
| `-s`, `--scope <SCOPE>` | `SCOPE` | `major`, `minor`, `patch`, or `release`; inferred when omitted. |
| `--stage <STAGE>` | `STAGE` | `dev`, `rc`, or `stable`; inferred when omitted. |
| `--candidate <TAG>` | `CANDIDATE` | Promote the exact tag; unset by default. Requires stable calculation. |
| `--target <VERSION>` | `TARGET` | Exact official core version, optionally prefixed with `v`; inferred from supported release/hotfix branch names when omitted. |

### release

Create a published GitHub or GitLab release. The required `<NAME>` is also the
tag name unless `--tag-name` is supplied.

```shell
# GitHub Actions
svci release -g v1.3.0

# GitLab CI: compare against the previous version for release notes
svci release -g -p v1.2.3 v1.3.0

# Use a display name distinct from the tag
svci release --tag-name v1.3.0 "Version 1.3.0"

# Use 1.3.0 for both the release name and tag
svci release --strip-prefix-v v1.3.0
```

| Option | Environment variable | Behavior / default |
| --- | --- | --- |
| `--description <DESCRIPTION>` | `DESCRIPTION` | Release description; empty by default. |
| `--tag-name <TAG_NAME>` | `TAG_NAME` | Defaults to `<NAME>`. |
| `--tag-message <TAG_MESSAGE>` | `TAG_MESSAGE` | Message for creating an annotated tag in GitLab; empty by default. Ignored by GitHub. |
| `-g`, `--generate-release-notes` | `GENERATE_RELEASE_NOTES` | Generate notes and prepend any description; `false` by default. |
| `-p`, `--previous-tag <PREVIOUS_TAG>` | `PREVIOUS_TAG` | GitLab comparison base for generated notes; empty by default. Pass `v0.0.0` for an initial release. |
| `-s`, `--strip-prefix-v` | `STRIP_PREFIX_V` | Remove one leading lowercase `v` from the release name and tag name; `false` by default. |

GitHub requests explicitly set `prerelease` from the final tag name, after
`--tag-name` / `TAG_NAME` selection and any `--strip-prefix-v` processing.
Tags in the supported `X.Y.Z-rc.N` or `X.Y.Z-dev.N.SHA` format, optionally
prefixed with one lowercase `v`, use `prerelease: true`. The entire tag must
match the supported format, and dev tags require a nonempty SHA. Official
versions and all other tags, including custom or malformed version tags, use
`prerelease: false` and are still submitted. The display name does not affect
this decision. There is no prerelease override or draft option.

### tag

Create an annotated tag at the repository's `HEAD` and push it to `origin`.
The tag name is a required argument.

```shell
svci tag v1.3.0 --tag-message "Release v1.3.0"
```

| Option | Environment variable | Behavior / default |
| --- | --- | --- |
| `--tag-message <TAG_MESSAGE>` | `TAG_MESSAGE` | Annotation message; empty by default. |
| `-s`, `--strip-prefix-v` | `STRIP_PREFIX_V` | Remove a leading `v` from the tag name; `false` by default. |

See [authentication](#authentication) for Git push credentials.

### scope

Print `major`, `minor`, `patch`, or `release` from the branch name. This command
remains supported for independent use and existing CI workflows; it prints no stage.

```shell
svci scope

# Existing command composition
export SCOPE=$(svci scope)
svci version
```

| Option | Environment variable | Behavior / default |
| --- | --- | --- |
| `--major <MAJOR>` | `MAJOR` | Major branch regex. |
| `--minor <MINOR>` | `MINOR` | Minor branch regex. |
| `--patch <PATCH>` | `PATCH` | Patch branch regex. |
| `--release <RELEASE>` | `RELEASE` | Release scope regex. |

Defaults and matching order are shared with the
[scope branch patterns](#custom-branch-patterns). Flags override environment values
for this invocation only. When passing the result to `version` on a custom branch,
configure the stage separately. An explicit `SCOPE=release` defaults the stage to
stable, so existing `RELEASE` patterns still work with this composition.

## Troubleshooting

Application failures print `Error: <context>` and the complete `Caused by:` chain
to stderr, then exit with code **1**. Version calculation failures do not print
version assignments to stdout. Invalid CLI arguments retain clap's exit code
**2** and usage diagnostics.

Missing optional configuration uses its documented default. Invalid values,
unreadable configuration, malformed `.env` files, and version component overflow
are reported as errors. A missing `.env` file is allowed.

### Unknown branches and invalid combinations

For an unmapped branch, configure both scope and stage through
[options or branch patterns](#configuration). Scope `release` cannot be combined
with stage `dev` or `rc`; select a bump scope instead. `--candidate` requires
stable calculation and cannot be combined with an explicit bump scope.
Invalid option values or patterns needed for inference cause an error without
version outputs.

Version-bearing branches must use the supported [target formats](#exact-release-targets-and-maintenance-branches).
Conflicting scope, target, or candidate values are errors, including explicit
overrides. Published targets, a target at or below the previous official version
in its line, and missing bases also fail without version outputs.

### Shallow clones and missing tags

All version calculation requires full history.
Use `GIT_DEPTH: "0"` in GitLab CI or
`fetch-depth: 0` with `actions/checkout`. Semver-CI's internal GitHub Actions clone
fetches full history when it creates the checkout itself.

For an existing shallow clone:

```shell
git fetch --unshallow --tags
```

For a complete local clone with stale tags, run `git fetch --tags` or set
`FORCE_FETCH_TAGS=true`. All version tags must be available to select prerelease
numbers, detect a duplicate upcoming version, and check whether an explicit
candidate or target has already been released.

### Multiple candidates, squash merges, and rebases

When automatic promotion reports multiple core versions, pass the intended tag
with [`--candidate`](#explicit-candidate-selection). Normal merges and fast-forwards
preserve candidate ancestry; squash merges and rebases can remove it. If no newer
candidate remains reachable, automatic promotion without an exact target falls
back to a minor bump. With an exact target, a missing matching candidate is an error.
Pass a candidate explicitly when the pipeline must promote a particular RC.

### Detached HEAD

Local runs require a checked-out branch. GitHub Actions and GitLab CI use the
branch and commit variables supplied by their provider, so CI checkouts may be detached.

### Authentication errors

Check the [credentials for your environment](#authentication) and repository
permissions. For SSH remotes, also set `GIT_SSH_KEY_PATH` and the key passphrase
if needed. Local `version` requires `GIT_TOKEN` even when its value is empty.

## Development

### Set up the environment

Install Rustup and CMake. On macOS:

```shell
brew install rustup cmake
```

Clone the repository as shown in [Installation](#build-from-source).
[rust-toolchain.toml](rust-toolchain.toml) pins Rust and Cargo to **1.98.1** and
includes Clippy and rustfmt. Rustup installs the configured toolchain when Cargo runs.

For local development, create `.env` in the repository root:

```dotenv
GITHUB_ACTIONS=false
GITLAB_CI=false
GIT_TOKEN=
FORCE_FETCH_TAGS=false
```

This uses the current Git checkout and existing local tags. CI providers supply
their own branch, commit, and job variables during pipeline runs. See
[Configuration](#configuration) when remote access or custom branches are needed.

### Run, test, and lint

```shell
cargo run -- version --help
cargo run -- version
cargo run -- version --scope patch --stage stable
cargo test --locked --all-features
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
```

### Source organization

- `src/models/` owns shared types and their value-level behavior, including
  version parsing, comparison, increments, and release target validation.
- `src/branch_rules.rs` interprets branches and validates scope, stage, and
  target combinations using supplied values and patterns. It does not read
  environment variables or access Git.
- `src/commands/` handles CLI/environment precedence and loads only the patterns
  needed for inference. `src/versioning_service.rs` calculates versions using
  Git history and the selected policy.
- `src/errors/` owns shared application errors and diagnostic messages.

Models do not depend on commands, branch rules, configuration, or I/O services.
Keep value-level operations on the models; branch interpretation belongs in
branch rules, and environment reads belong at the command/configuration boundary.

### Error handling

Keep application error and recovery messages in `src/errors/messages.rs`, using
constants for fixed text and functions for messages with parameters. Return
`errors::Result<T>` from fallible application functions. Create validation
errors with `DefaultError::new(...)`; use `ResultExt::context(...)` to attach
operation context to external failures, and propagate existing errors with `?`.
Preserve original errors through `with_source(...)` rather than converting them
to strings. `main` prints `report()` once and selects the exit code.

Intentional recovery paths may log `report()` and continue, such as GitLab's
compare-link fallback. The libgit2 credentials callback requires `git2::Error`,
so it converts the report at that API boundary. Programmer invariants may use
`unreachable!`; configuration, I/O, parsing, and arithmetic failures return errors.

### Build and test containers

```shell
docker build --platform linux/amd64 --build-arg VARIANT=alpine -t semver-ci:alpine .
docker build --platform linux/amd64 --build-arg VARIANT=debian -t semver-ci:debian .
bash tests/container.sh semver-ci:alpine alpine
bash tests/container.sh semver-ci:debian debian
```

Local builds use the release profile by default. Add `--build-arg CARGO_PROFILE=dev`
for a debug build. See [Contributing](.github/CONTRIBUTING.md#ci-and-container-builds)
for CI triggers, toolchain setup, and image publishing details.

## Contributing & License

See the [contribution guidelines](.github/CONTRIBUTING.md) for branch and commit
conventions and [LICENSE](LICENSE) for the license terms.
