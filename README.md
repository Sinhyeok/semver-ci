![lint workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/lint.yml/badge.svg)
![publish workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/publish.yml/badge.svg)
# semver-ci
Semantic Versioning for CI/CD

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
      - name: Check out the repo
        uses: actions/checkout@v4
      - name: Set upcoming version
        id: set_upcoming_version
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

  tag:
    runs-on: ubuntu-latest
    container: tartar4s/semver-ci
    if: startsWith(github.ref_name, 'release/') || startsWith(github.ref_name, 'hotfix/')
    needs: [upcoming_version, build]
    permissions:
      contents: write
    steps:
      - name: Check out the repo
        uses: actions/checkout@v4
      - name: Tag
        run: svci tag "$TAG_NAME"
    env:
      TAG_NAME: ${{needs.upcoming_version.outputs.UPCOMING_VERSION}}
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```
### GitLab CI/CD
- [example](https://gitlab.com/attar.sh/semver-ci-example)
> [!NOTE]
> For tagging, "SEMVER_CI_TOKEN" with read_repository/write_repository permissions must be set in CI/CD variables 
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

# "SEMVER_CI_TOKEN" with read_repository/write_repository permissions must be set in CI/CD variables
tag:
  stage: after_build
  image:
    name: tartar4s/semver-ci
    entrypoint: [""]
  variables:
    TAG_NAME: $UPCOMING_VERSION
  script:
    - svci tag $TAG_NAME
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

# GitLab
## develop
GITLAB_CI=true
CI_COMMIT_BRANCH=develop
CI_COMMIT_SHORT_SHA=g9i0tlab
## hotfix
#GITLAB_CI=true
#CI_COMMIT_BRANCH=hotfix/0.2.34
#CI_COMMIT_SHORT_SHA=b08640bd

# Git Repo
#GIT_SSH_KEY_PATH=$HOME/.ssh/id_rsa
#GIT_SSH_KEY_PASSPHRASE={PASSWORD}
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
