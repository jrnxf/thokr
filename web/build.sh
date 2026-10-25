#!/bin/sh
# Builds the website into web/dist: the page from web/public, and its engine,
# thokr's own Rust (src/) compiled to WebAssembly from this checkout, named by
# its content so browsers can keep it for good. The word lists come with it:
# lang.rs includes src/lang/*.txt.
#
# It runs as is in Cloudflare's Workers Builds, whose image has no Rust: it then
# installs rustup, at the version and checksum below, and the toolchain.
set -eu

RUST_VERSION=1.99.1
RUSTUP_VERSION=1.29.2
# from https://static.rust-lang.org/rustup/archive/$RUSTUP_VERSION/x86_64-unknown-linux-gnu/rustup-init.sha256
RUSTUP_INIT_SHA256=5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4

web=$(cd "$(dirname "$0")" && pwd)
root=$(dirname "$web")

sha256() {
    if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d ' ' -f 1
}

if ! command -v rustup >/dev/null; then
    cargo_home=${CARGO_HOME:-$HOME/.cargo}
    if [ ! -x "$cargo_home/bin/rustup" ]; then
        case "$(uname -s)-$(uname -m)" in
            Linux-x86_64) ;;
            *) echo "build.sh needs rustup; see https://rustup.rs" >&2; exit 1 ;;
        esac
        # rustup-init reads its own file name, so it must keep it
        tmp=$(mktemp -d)
        trap 'rm -rf "$tmp"' EXIT
        init=$tmp/rustup-init
        echo "Installing rustup $RUSTUP_VERSION"
        curl -sSfL --retry 3 -o "$init" \
            "https://static.rust-lang.org/rustup/archive/$RUSTUP_VERSION/x86_64-unknown-linux-gnu/rustup-init"
        if [ "$(sha256 "$init")" != "$RUSTUP_INIT_SHA256" ]; then
            echo "rustup-init does not match its checksum" >&2
            exit 1
        fi
        chmod +x "$init"
        "$init" -y --no-modify-path --profile minimal --default-toolchain none
    fi
    PATH=$cargo_home/bin:$PATH
fi
rustup toolchain install "$RUST_VERSION" --profile minimal --target wasm32-wasip1

target=$root/target/web
rustup run "$RUST_VERSION" cargo build --release --locked --target wasm32-wasip1 \
    --manifest-path "$web/engine/Cargo.toml" --target-dir "$target"

dist=$web/dist
rm -rf "$dist"
mkdir -p "$dist"
cp -R "$web/public/." "$dist/"
engine=$target/wasm32-wasip1/release/thokr_web_engine.wasm
wasm=thokr.$(sha256 "$engine" | cut -c 1-16).wasm
cp "$engine" "$dist/kit/$wasm"
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)
sed "s/__THOKR_WASM__/$wasm/" "$web/public/kit/engine.js" > "$dist/kit/engine.js"
sed "s/__THOKR_VERSION__/$version/" "$web/public/index.html" > "$dist/index.html"
if grep -rq "__THOKR_" "$dist"; then
    echo "a placeholder was left in dist" >&2
    exit 1
fi
ls -lR "$dist"
