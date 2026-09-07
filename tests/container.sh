#!/usr/bin/env bash
set -euo pipefail

image=${1:?Usage: bash tests/container.sh IMAGE VARIANT}
variant=${2:?Usage: bash tests/container.sh IMAGE VARIANT}

docker run --rm --platform linux/amd64 "$image" --version
docker run --rm --platform linux/amd64 --entrypoint /bin/sh "$image" -ec '
    . /etc/os-release
    test "$ID" = "$1"
    test -s /etc/ssl/certs/ca-certificates.crt
    curl --version
    svci version --help
' sh "$variant"

# Exercise libgit2 in the final image, including its native runtime dependencies.
repo=$(mktemp -d)
trap 'rm -rf "$repo"' EXIT
git -C "$repo" init -q --initial-branch develop
git -C "$repo" -c user.name=Test -c user.email=test@example.com \
    -c commit.gpgsign=false -c core.hooksPath=/dev/null \
    commit -q --allow-empty -m 'chore: init'
git -C "$repo" -c tag.gpgsign=false tag v1.2.3
sha=$(git -C "$repo" rev-parse HEAD)

run_cli() {
    docker run --rm --platform linux/amd64 --user "$(id -u):$(id -g)" \
        -v "$repo:/app:ro" -e GIT_TOKEN=test-token "$image" "$@"
}

test "$(run_cli scope)" = minor
actual=$(run_cli version --scope minor)
expected=$(printf 'UPCOMING_VERSION=v1.3.0-dev.1.%s\nLAST_VERSION=v1.2.3' "${sha:0:8}")
if [[ "$actual" != "$expected" ]]; then
    printf 'Unexpected version output:\n%s\nExpected:\n%s\n' "$actual" "$expected" >&2
    exit 1
fi
printf 'Container checks passed: %s\n' "$variant"
