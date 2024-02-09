![lint workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/lint.yml/badge.svg)
![publish workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/publish.yml/badge.svg)
# semver-ci
Semantic Versioning for CI/CD

## Getting Started
### GitHub Actions
- [example](https://github.com/Sinhyeok/semver-ci-example)
```yaml
# .github/workflows/build.yml

name: BUILD
on:
  push:
    branches:
      - 'develop'
      - 'feature/*'
      - 'release/[0-9]*.[0-9]*.x'
      - 'release/[0-9]*.x.x'
      - 'hotfix/[0-9]*.[0-9]*.[0-9]*'
jobs:
  upcoming_version:
    runs-on: ubuntu-latest
    container: tartar4s/semver-ci
    outputs:
      UPCOMING_VERSION: ${{ steps.set_upcoming_version.outputs.UPCOMING_VERSION }}
    steps:
      - name: Check out the repository to the runner
        uses: actions/checkout@v4
      - run: git config --global --add safe.directory .
      - name: Set upcoming version
        id: set_upcoming_version
        run: |
          if [[ $GITHUB_REF == refs/heads/release/[0-9]*.x.x ]]; then
            export SCOPE=major
          elif [[ $GITHUB_REF == refs/heads/develop || $GITHUB_REF == refs/heads/feature/* || $GITHUB_REF == refs/heads/release/[0-9]*.[0-9]*.x ]]; then
            export SCOPE=minor
          elif [[ $GITHUB_REF == refs/heads/hotfix/* ]]; then
            export SCOPE=patch
          else
            echo "Unsupported branch for versioning"
            exit 1
          fi
          echo "UPCOMING_VERSION=$(svci version)" >> "$GITHUB_OUTPUT"
    env:
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  build:
    runs-on: ubuntu-latest
    needs: upcoming_version
    steps:
      - env:
          RELEASE_TAG: ${{needs.upcoming_version.outputs.UPCOMING_VERSION}}
        run: echo "$RELEASE_TAG"
```
### GitLab CI/CD
- [example](https://gitlab.com/attar.sh/semver-ci-example)
```yaml
# .gitlab-ci.yml

stages:
  - before_build
  - build

.upcoming_version:
  stage: before_build
  image:
    name: tartar4s/semver-ci
    entrypoint: [""]
  script:
    - echo "UPCOMING_VERSION=$(svci version)" >> version.env
  artifacts:
    reports:
      dotenv: version.env

upcoming_version:minor:
  extends: .upcoming_version
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^(develop|feature\/.*|release\/[0-9]+\.[0-9]+\.x)$/

upcoming_version:patch:
  extends: .upcoming_version
  variables:
    SCOPE: patch
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^hotfix\/.*$/

upcoming_version:major:
  extends: .upcoming_version
  variables:
    SCOPE: major
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^release\/[0-9]+\.x\.x$/

build:
  stage: build
  variables:
    RELEASE_TAG: $UPCOMING_VERSION
  script:
    - echo "$RELEASE_TAG"
```
### Git Repo
> [!NOTE]
> The Git HEAD must be pointing to the branch. If it's a detached head, semver-ci won't work because it can't find the target branch.
```shell
# help
docker run tartar4s/semver-ci

# version command
docker run -v .:/app tartar4s/semver-ci version --help
```

## Commands
### version
Print upcoming version based on last semantic version tag and branch

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
