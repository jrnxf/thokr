#!/bin/sh
# Point Formula/thokr.rb at the source archive of a release tag.
# usage: scripts/bump-formula.sh <version>   (e.g. 0.7.0 or v0.7.0)
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <version>" >&2
  exit 1
fi

version="${1#v}"
formula="$(cd "$(dirname "$0")/.." && pwd)/Formula/thokr.rb"
url="https://github.com/jrnxf/thokr/archive/refs/tags/v${version}.tar.gz"

tmp="$(mktemp)"
trap 'rm -f "$tmp" "$tmp.new"' EXIT

curl -fsSL "$url" -o "$tmp"

if command -v sha256sum >/dev/null 2>&1; then
  sha="$(sha256sum "$tmp" | cut -d ' ' -f 1)"
else
  sha="$(shasum -a 256 "$tmp" | cut -d ' ' -f 1)"
fi

sed -e "s|^  url \".*\"|  url \"${url}\"|" \
    -e "s|^  sha256 \".*\"|  sha256 \"${sha}\"|" \
    "$formula" > "$tmp.new"
cat "$tmp.new" > "$formula"

echo "Formula/thokr.rb -> v${version} (${sha})"
