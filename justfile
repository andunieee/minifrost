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

# Build the coordinator debug APK and install it on the connected phone.
# Install only, no attached shell (no logcat / hot reload). For logs:
# `adb -s <device> logcat`. Picks the first connected android device (see `just devices`).
coordinator-android:
    cd apps/coordinator && flutter build apk --debug && adb -s "$(flutter devices --machine | jq -r '[.[] | select(.targetPlatform | startswith("android-")) | .id] | first')" install -r -t build/app/outputs/flutter-apk/app-debug.apk

# --- signer ----------------------------------------------------------------------------------
# On desktop the key share lives in a plain file (see lib/vault.dart), webcam scanning
# comes from the shared `desktop_camera` crate. Not for serious funds.

# Run the signer on this machine (debug build, hot reload with `r`).
signer-desktop:
    cd apps/signer && flutter run -d linux

# Build the signer debug APK and install it on the connected phone.
# Install only, no attached shell (no logcat / hot reload). For logs:
# `adb -s <device> logcat`. Picks the first connected android device (see `just devices`).
signer-android:
    cd apps/signer && flutter build apk --debug && adb -s "$(flutter devices --machine | jq -r '[.[] | select(.targetPlatform | startswith("android-")) | .id] | first')" install -r -t build/app/outputs/flutter-apk/app-debug.apk

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

# --- packaging -------------------------------------------------------------------------------

# Build the release bundle of a Linux desktop app (coordinator|signer) and install it, with its
# launcher entry and icon, under ~/.local.
install-desktop app:
    cd apps/{{app}} && flutter build linux --release
    rm -rf ~/.local/opt/minifrost_{{app}}
    mkdir -p ~/.local/opt ~/.local/bin ~/.local/share/applications ~/.local/share/icons/hicolor/512x512/apps
    cp -r apps/{{app}}/build/linux/x64/release/bundle ~/.local/opt/minifrost_{{app}}
    ln -sf ~/.local/opt/minifrost_{{app}}/minifrost_{{app}} ~/.local/bin/minifrost_{{app}}
    cp assets/logo.png ~/.local/share/icons/hicolor/512x512/apps/minifrost.png
    cp packaging/minifrost_{{app}}.desktop ~/.local/share/applications/
