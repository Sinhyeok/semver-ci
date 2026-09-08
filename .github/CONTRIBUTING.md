# Branch strategy
Git flow

# Semantic Commit Messages

See how a minor change to your commit message style can make you a better programmer.

Format: `<type>(<scope>): <subject>`

`<scope>` is optional

## Example

```
feat: add hat wobble
^--^  ^------------^
|     |
|     +-> Summary in present tense.
|
+-------> Type: chore, docs, feat, fix, refactor, style, or test.
```

More Examples:

- `feat`: (new feature for the user, not a new feature for build script)
- `fix`: (bug fix for the user, not a fix to a build script)
- `docs`: (changes to the documentation)
- `style`: (formatting, missing semi colons, etc; no production code change)
- `refactor`: (refactoring production code, eg. renaming a variable)
- `test`: (adding missing tests, refactoring tests; no production code change)
- `chore`: (updating grunt tasks etc; no production code change)

References:

- https://gist.github.com/joshbuchea/6f47e86d2510bce28f8e7f42ae84c716
- https://www.conventionalcommits.org/
- https://seesparkbox.com/foundry/semantic_commit_messages
- http://karma-runner.github.io/1.0/dev/git-commit-msg.html

# CI and container builds

For local setup, test commands, and container checks, see
[Development](../README.md#development).

## Toolchain and build profiles

Local development, CI, and Docker builds use
[rust-toolchain.toml](../rust-toolchain.toml), which pins Rust and Cargo to 1.98.1.
The [Dockerfile](../Dockerfile) uses `rust:1.98-bookworm` as its bootstrap image,
then installs and selects the pinned toolchain through Rustup.

Docker builds use `CARGO_PROFILE=release` by default. Set `CARGO_PROFILE=dev` for
a debug build; this replaces the former `debug.Dockerfile`. The CI workflows use
the dev profile, preserving the profile used for previously published images.
Both Alpine (musl) and Debian (GNU) images target `linux/amd64`.

## Pull request checks

[PR CI](workflows/pr-review.yml) calls the reusable
[Rust CI workflow](workflows/ci.yml) for formatting, Clippy, and tests on non-draft
pull requests. Formatting and lint findings can be posted to same-repository
pull requests; fork pull requests still run the checks with a read-only token.

PR CI also builds and tests both container variants when the full PR diff changes
Dockerfiles, `.dockerignore`, Cargo manifests or lockfiles, the Rust toolchain,
`.cargo/`, workflow definitions, or `tests/container.sh`. These checks do not push
images. PR CI can also be dispatched manually; container change detection runs
only for pull requests.

## Release workflow

The [release workflow](workflows/release.yml) runs on pushes to `develop`, `main`,
`release/**`, and `hotfix/**`. Both the PR and release workflows ignore changes
limited to README and license files.

The release workflow runs Rust CI, calculates the version using a pinned
published Semver-CI image, and builds each container variant once. It runs
`tests/container.sh` against those images before pushing them to Docker Hub.
Alpine receives both `<version>` and `<version>-alpine` tags; Debian receives
`<version>-debian`. The workflow does not update `latest`.

After the image checks and pushes succeed, the workflow creates a GitHub release
for `main`, `release/**`, and `hotfix/**` branches, with the image tags listed in
the release description.
