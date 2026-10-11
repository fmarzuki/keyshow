#!/usr/bin/env sh
# Build release packages inside Docker, so only Docker is needed on the host:
#
#   dist/keyshow_<version>-1_amd64.deb                     Debian 12+ / Ubuntu 22.04+
#   dist/keyshow-<version>-windows-x86_64-setup.exe        Windows installer (NSIS, per-user)
#   dist/keyshow-<version>-windows-x86_64-portable.exe     Windows, no install
#
# Usage: packaging/build-packages.sh [deb|windows|all]   (default: all)
#
# The image uses Debian 12 so the .deb depends on an older glibc than a
# bleeding-edge host would produce. Windows builds are cross-compiled with mingw-w64.
set -eu

what=${1:-all}
root=$(cd "$(dirname "$0")/.." && pwd)
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n1)
image=keyshow-packager:bookworm

docker build -t "$image" - <<'EOF'
FROM rust:1-bookworm
RUN apt-get update \
 && apt-get install -y --no-install-recommends mingw-w64 nsis dpkg-dev \
 && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-pc-windows-gnu \
 && cargo install cargo-deb --locked \
 && chmod -R a+rwX /usr/local/cargo /usr/local/rustup
EOF

mkdir -p "$root/dist"
# Separate target dir: binaries built against the container's glibc must not mix
# with host builds in ./target.
docker run --rm -u "$(id -u):$(id -g)" \
    -v "$root:/src" -w /src \
    -e HOME=/tmp -e CARGO_TARGET_DIR=/src/target/packager \
    -e WHAT="$what" -e VERSION="$version" \
    "$image" sh -euc '
        if [ "$WHAT" = all ] || [ "$WHAT" = deb ]; then
            cargo deb --locked --output dist/
        fi
        if [ "$WHAT" = all ] || [ "$WHAT" = windows ]; then
            cargo build --locked --release --target x86_64-pc-windows-gnu
            exe=$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release/keyshow.exe
            cp "$exe" "dist/keyshow-$VERSION-windows-x86_64-portable.exe"
            makensis -V2 -DVERSION="$VERSION" -DBINARY="$exe" \
                -DOUTFILE="/src/dist/keyshow-$VERSION-windows-x86_64-setup.exe" \
                packaging/windows/keyshow.nsi
        fi
    '
ls -l "$root/dist"
