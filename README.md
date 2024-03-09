![development workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/development.yml/badge.svg)
![publish workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/publish.yml/badge.svg)

# Semver-CI

Welcome to Semver-CI, an open-source project designed to seamlessly integrate semantic versioning into your continuous integration (CI) workflow. This tool automates the process of versioning releases, ensuring that every new build adheres strictly to the [Semantic Versioning](https://semver.org/) guidelines. With Semver-CI, developers can focus more on their code and less on the intricacies of version management.

## Key Features:

- **Automated Version Management**: Automatically increments your project's version based on branch names, tags and predefined rules.
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
        run: |
          export SCOPE=$(svci scope)
          echo "UPCOMING_VERSION=$(svci version)" >> "$GITHUB_OUTPUT"
    env:
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  build:
    runs-on: ubuntu-latest
    needs: upcoming_version
    steps:
      - run: echo "$RELEASE_TAG"
    env:
      RELEASE_TAG: ${{needs.upcoming_version.outputs.UPCOMING_VERSION}}

  release_candidate:
    runs-on: ubuntu-latest
    container: tartar4s/semver-ci
    if: startsWith(github.ref_name, 'release/') || startsWith(github.ref_name, 'hotfix/')
    needs: [upcoming_version, build]
    permissions:
      contents: write
    steps:
      - run: svci release "$RELEASE_NAME"
    env:
      RELEASE_NAME: ${{needs.upcoming_version.outputs.UPCOMING_VERSION}}
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      #GENERATE_RELEASE_NOTES: true
```
### GitLab CI/CD
- [example](https://gitlab.com/attar.sh/semver-ci-example)
```yaml
# .gitlab-ci.yml

stages:
  - before_build
  - build
  - after_build

upcoming_version:
  stage: before_build
  image:
    name: tartar4s/semver-ci
    entrypoint: [""]
  script:
      #export MAJOR='^release/[0-9]+.x.x$'
      #export MINOR='^(develop|feature/.*|release/[0-9]+.[0-9]+.x)$'
      #export PATCH='^hotfix/[0-9]+.[0-9]+.[0-9]+$'
    - |
      export SCOPE=$(svci scope)
      echo "UPCOMING_VERSION=$(svci version)" >> version.env
  artifacts:
    reports:
      dotenv: version.env
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^(develop|feature\/.+|release\/.+|hotfix\/.+)$/

build:
  stage: build
  variables:
    RELEASE_TAG: $UPCOMING_VERSION
  script:
    - echo "$RELEASE_TAG"
  rules:
    - if: $CI_COMMIT_BRANCH

release_candidate:
  stage: after_build
  image:
    name: tartar4s/semver-ci
    entrypoint: [""]
  variables:
    RELEASE_NAME: $UPCOMING_VERSION
  script:
    - svci release $RELEASE_NAME
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^(release\/.+|hotfix\/.+)$/
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
### release
Create a release in Github or GitLab
```shell
Usage: svci release [OPTIONS] <NAME>

Arguments:
  <NAME>  Release name

Options:
      --description <DESCRIPTION>  Release description [env: DESCRIPTION=] [default: ]
      --tag-name <TAG_NAME>        [env: TAG_NAME=]
      --tag-message <TAG_MESSAGE>  Specify tag_message to create an annotated tag [env: TAG_MESSAGE=] [default: ]
  -g, --generate-release-notes     (Only for Github Actions) Automatically generate the body for this release. If body is specified, the body will be pre-pended to the automatically generated notes [env: GENERATE_RELEASE_NOTES=]
  -s, --strip-prefix-v             Strip prefix "v" from release name and tag name. ex) v0.1.0 => 0.1.0 [env: STRIP_PREFIX_V=]
  -h, --help                       Print help
  -V, --version                    Print version
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
# Github
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
CI_COMMIT_BRANCH=develop
CI_COMMIT_SHORT_SHA=g9i0tlab
GITLAB_USER_EMAIL=user@mail.com
SEMVER_CI_TOKEN=glpat_908d21yh0ewfd98h
CI_JOB_TOKEN=vn0w9e7dfgy97esd8f
CI_PROJECT_URL=https://gitlab.com/attar.sh/semver-ci
## hotfix
#GITLAB_CI=true
#CI_COMMIT_BRANCH=hotfix/0.2.34
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
