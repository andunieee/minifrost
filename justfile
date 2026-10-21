# Run `just` to list recipes.

# Flutter lives in ~/sdk/flutter on this machine; override with FLUTTER_HOME=/path/to/flutter.
flutter_home := env("FLUTTER_HOME", env("HOME") / "sdk/flutter")
export PATH := flutter_home / "bin" + ":" + env("PATH")
export ANDROID_HOME := env("ANDROID_HOME", "/opt/android-sdk")

apk := "apps/signer/build/app/outputs/flutter-apk/app-release.apk"

default:
    @just --list

# --- coordinator (desktop) -------------------------------------------------------------------

# Run the coordinator on this machine (debug build, hot reload with `r`).
coordinator:
    cd apps/coordinator && flutter run -d linux

# Run the coordinator as a release build.
coordinator-release:
    cd apps/coordinator && flutter run -d linux --release

# Build the Linux bundle: apps/coordinator/build/linux/x64/release/bundle/
coordinator-build:
    cd apps/coordinator && flutter build linux --release

# --- signer (android) ------------------------------------------------------------------------

# Build the signer APK for arm64 phones. No INTERNET permission means no hot reload, so release only.
signer-build:
    cd apps/signer && flutter build apk --release --target-platform android-arm64

# Run the signer on the connected phone (release mode, no hot reload).
signer:
    cd apps/signer && flutter run --release

# Build, verify offline guarantees, and install on the connected phone via adb.
signer-install: signer-build check-offline
    adb install -r {{apk}}

# Same, but pick a device when several are connected: just signer-install-to <serial>
signer-install-to serial: signer-build check-offline
    adb -s {{serial}} install -r {{apk}}

# Remove the signer from the phone. This DELETES its key share (Keystore key goes with it).
signer-uninstall:
    adb uninstall dev.minifrost.minifrost_signer

# Fail if the signer has networking crates or the APK asks for unexpected permissions.
check-offline: signer-build
    ./scripts/check-signer-offline.sh {{apk}}

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
