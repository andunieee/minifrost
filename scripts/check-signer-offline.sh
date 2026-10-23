#!/usr/bin/env bash
# Fails if the signer could talk to a network.
#   scripts/check-signer-offline.sh [path/to/signer.apk]
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== Rust dependency tree of the signer"
tree=$(cargo tree -p minifrost_signer -e normal --prefix none | sed 's/ v.*//' | sort -u)
bad=$(echo "$tree" | grep -E '^(ureq|rustls|hyper|reqwest|openssl|native-tls|socket2|mio|curl|h2|tungstenite|minifrost_coordinator)$' || true)
if [ -n "$bad" ]; then echo "FAIL: networking/coordinator crates in the signer:"; echo "$bad"; exit 1; fi
if cargo tree -p minifrost_signer -e features -i tokio 2>/dev/null | grep -qE 'tokio feature "(net|process|signal)"'; then
  echo "FAIL: tokio networking features enabled in the signer"; exit 1
fi
echo "ok: no networking crates"

if [ $# -ge 1 ]; then
  apk=$1
  aapt2=$(ls "${ANDROID_HOME:-/opt/android-sdk}"/build-tools/*/aapt2 | sort -V | tail -1)
  echo "== Permissions in $apk"
  perms=$("$aapt2" dump permissions "$apk" | grep -E "^uses-permission" || true)
  echo "$perms"
  # Only CAMERA, to read QR codes.
  extra=$(echo "$perms" | grep -vE "android.permission.(CAMERA)'|DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION" || true)
  if [ -n "$extra" ]; then echo "FAIL: unexpected permissions:"; echo "$extra"; exit 1; fi
  echo "ok: only camera"
fi
