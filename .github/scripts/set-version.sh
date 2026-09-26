#!/bin/bash
# Stamps the release version into Cargo.toml (so the app, its update check and the
# Linux packages all report it) and exports VERSION for later steps.
# Tag builds use the tag (v1.3.0 -> 1.3.0); manual runs keep Cargo.toml's version.
set -e

if [[ "$GITHUB_REF" == refs/tags/v* ]]; then
    VERSION="${GITHUB_REF_NAME#v}"
    perl -0pi -e "s/^version = \"[^\"]*\"/version = \"$VERSION\"/m" Cargo.toml
else
    VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
fi

echo "VERSION=$VERSION" >> "$GITHUB_ENV"
echo "Building version $VERSION"
