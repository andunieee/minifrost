# Run `just` to list recipes.

export ANDROID_HOME := env("ANDROID_HOME", "/opt/android-sdk")

default:
    @just --list

# --- coordinator -----------------------------------------------------------------------------

# Run the coordinator on this machine (debug build).
coordinator-desktop:
    cargo run -p minifrost_coordinator_app

# Build the coordinator debug APK and install it on all connected phones.
# Install only, no attached shell. For logs: `adb -s <device> logcat`.
# Loops over all connected android devices (see `just devices`).
coordinator-android: (android "coordinator")

# --- signer ----------------------------------------------------------------------------------
# On desktop the key share lives in a plain file (see apps/signer/src/ctl.rs), webcam
# scanning comes from the shared `desktop_camera` crate. Not for serious funds.

# Run the signer on this machine (debug build).
signer-desktop:
    cargo run -p minifrost_signer_app

# Build the signer debug APK and install it on all connected phones.
# Install only, no attached shell. For logs: `adb -s <device> logcat`.
# Loops over all connected android devices (see `just devices`).
signer-android: (android "signer")

# --- development -----------------------------------------------------------------------------

# Protocol, wallet and full QR-flow tests (no UI, no devices needed).
test:
    cargo test --workspace

# Lint everything.
check:
    cargo clippy --workspace --all-targets

# Fails if the signer could reach a network (Rust dependencies; pass an APK to check its permissions too).
check-signer-offline *apk:
    scripts/check-signer-offline.sh {{apk}}

devices:
    adb devices -l

# Build an app's debug APK (Rust library via cargo-ndk, see scripts/android-native.sh) and install it.
[private]
android app:
    cd apps/{{app}}/android && ./gradlew assembleDebug
    for d in $(adb devices -l | awk 'NR > 1 && $2 == "device" {print $1}'); do adb -s "$d" install -r -t apps/{{app}}/android/app/build/outputs/apk/debug/app-debug.apk; done

# --- packaging -------------------------------------------------------------------------------

# Build the release binary of a Linux desktop app (coordinator|signer) and install it, with its
# launcher entry and icon, under ~/.local.
install-desktop app:
    cargo build --release -p minifrost_{{app}}_app
    rm -rf ~/.local/opt/minifrost_{{app}}  # the old Flutter bundle
    mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/icons/hicolor/512x512/apps
    install -m755 "$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)/release/minifrost_{{app}}" ~/.local/bin/minifrost_{{app}}
    magick assets/logo.png -gravity center -crop 800x800+0+0 +repage -resize 512x512 ~/.local/share/icons/hicolor/512x512/apps/minifrost.png
    cp packaging/minifrost_{{app}}.desktop ~/.local/share/applications/
