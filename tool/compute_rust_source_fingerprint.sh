#!/usr/bin/env bash
# Compute deterministic SHA-256 fingerprint for Rust source/build inputs.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

python3 - "$ROOT" <<'PY'
import hashlib
import os
import sys

# NOTE: inputs must match the inline Python in .github/workflows/build-prebuilds.yml
# exactly, otherwise the locally computed fingerprint will differ from the release tag.
root = sys.argv[1]

files = []
for dirpath, _, filenames in os.walk(os.path.join(root, "src")):
    for fn in filenames:
        files.append(os.path.join(dirpath, fn))
for extra in ("Cargo.toml", "Cargo.lock"):
    p = os.path.join(root, extra)
    if os.path.isfile(p):
        files.append(p)

sha = hashlib.sha256()
for abs_path in sorted(set(files)):
    rel = os.path.relpath(abs_path, root).replace(os.sep, "/")
    with open(abs_path, "rb") as f:
        data = f.read()
    sha.update(rel.encode("utf-8"))
    sha.update(b"\0")
    sha.update(hashlib.sha256(data).digest())

print(sha.hexdigest())
PY
