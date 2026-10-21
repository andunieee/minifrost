# Run `just` to list recipes.

# Flutter lives in ~/sdk/flutter on this machine; override with FLUTTER_HOME=/path/to/flutter.
flutter_home := env("FLUTTER_HOME", env("HOME") / "sdk/flutter")
export PATH := flutter_home / "bin" + ":" + env("PATH")
export ANDROID_HOME := env("ANDROID_HOME", "/opt/android-sdk")

default:
    @just --list

# --- coordinator -----------------------------------------------------------------------------

# Run the coordinator on this machine (debug build, hot reload with `r`).
coordinator-desktop:
    cd apps/coordinator && flutter run -d linux

# Run the coordinator on the connected phone (debug build).
# Picks the first connected android device (see `just devices`).
coordinator-android:
    cd apps/coordinator && flutter run -d "$(flutter devices --machine | jq -r '[.[] | select(.targetPlatform | startswith("android-")) | .id] | first')"

# --- signer ----------------------------------------------------------------------------------
# On desktop the key share lives in a plain file (see lib/vault.dart), webcam scanning
# comes from the shared `desktop_camera` crate. Not for serious funds.

# Run the signer on this machine (debug build, hot reload with `r`).
signer-desktop:
    cd apps/signer && flutter run -d linux

# Run the signer on the connected phone (debug build).
# Picks the first connected android device (see `just devices`).
signer-android:
    cd apps/signer && flutter run -d "$(flutter devices --machine | jq -r '[.[] | select(.targetPlatform | startswith("android-")) | .id] | first')"

# --- development -----------------------------------------------------------------------------

# Protocol, wallet and full QR-flow tests (no UI, no devices needed).
test:
    cargo test --workspace

# Regenerate the Dart bindings after editing crates/*/src/api.
codegen:
    cd apps/signer && flutter_rust_bridge_codegen generate
    cd apps/coordinator && flutter_rust_bridge_codegen generate

analyze:
    cd packages/minifrost_ui && flutter analyze
    cd apps/signer && flutter analyze
    cd apps/coordinator && flutter analyze

# Fetch Dart dependencies for everything.
setup:
    cd packages/minifrost_ui && flutter pub get
    cd apps/signer && flutter pub get
    cd apps/coordinator && flutter pub get

devices:
    adb devices -l
    flutter devices
