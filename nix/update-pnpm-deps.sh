#!/usr/bin/env bash
# Recompute the `pnpmDeps` fixed-output hash in nix/axolotl.nix from the current
# pnpm lockfile. Run it from the repository root, e.g.
# `nix develop . --command update-pnpm-deps`.
set -euo pipefail

readonly repository_root="$PWD"
readonly target="$repository_root/nix/axolotl.nix"

if [[ ! -f "$target" ]]; then
  echo "error: run update-pnpm-deps from the Axolotl repository root" >&2
  exit 1
fi

expression_file="$(mktemp)"
build_log="$(mktemp)"
trap 'rm -f "$expression_file" "$build_log"' EXIT

cat >"$expression_file" <<EOF
let
  flake = builtins.getFlake "$repository_root";
  pkgs = import flake.inputs.nixpkgs {
    system = builtins.currentSystem;
    overlays = [ flake.inputs.rust-overlay.overlays.default ];
  };
in
  (pkgs.callPackage (flake.outPath + "/nix/axolotl.nix") { inputs = flake.inputs; }).pnpmDeps
EOF

if nix build --no-link --impure --file "$expression_file" >"$build_log" 2>&1; then
  echo "pnpm dependency hash is up to date"
  exit 0
fi

new_hash="$(grep -m1 -oE 'got: +sha256-[A-Za-z0-9+/=]+' "$build_log" | sed 's/.*got: *//' || true)"

if [[ -z "$new_hash" ]]; then
  cat "$build_log" >&2
  echo "error: could not determine the new pnpm dependency hash" >&2
  exit 1
fi

sed -i \
  "/pnpmDeps = fetchPnpmDeps {/,/^    };/ s|hash = \"sha256-[A-Za-z0-9+/=]*\";|hash = \"$new_hash\";|" \
  "$target"

echo "pnpm dependency hash updated to $new_hash"
