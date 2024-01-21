![lint workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/lint.yml/badge.svg)
![publish workflow](https://github.com/Sinhyeok/semver-ci/actions/workflows/publish.yml/badge.svg)
# semver-ci
Semantic Versioning for CI/CD

## Getting Started
### GitHub Actions
```yaml
```
### GitLab CI/CD
- [example](https://gitlab.com/attar.sh/semver-ci-example)
```yaml
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
  script:
    - echo "$UPCOMING_VERSION"
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
