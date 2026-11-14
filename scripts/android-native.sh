#!/usr/bin/env bash
# Builds an app's Rust library (the Freya UI and the engine) for Android with cargo-ndk, into the
# app's jniLibs. Run by Gradle (apps/*/android/app/build.gradle.kts); usable by hand too:
#   scripts/android-native.sh <signer|coordinator> [debug|release] [abi ...]   (default: debug arm64-v8a)
set -euo pipefail
cd "$(dirname "$0")/.."
app=$1; shift
profile=${1:-debug}; shift || true
abis=("${@:-arm64-v8a}")
export ANDROID_HOME=${ANDROID_HOME:-/opt/android-sdk}
export ANDROID_NDK_HOME=${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.2.12479018}
# Freya's mundy (the system's light/dark preference) compiles a little Java against it.
export ANDROID_JAR=${ANDROID_JAR:-$(ls -d "$ANDROID_HOME"/platforms/android-*/android.jar | sort -V | tail -1)}

if ! command -v cargo-ndk >/dev/null; then
    echo "cargo-ndk is not installed; install it with: cargo install cargo-ndk" >&2
    exit 1
fi

out=apps/$app/android/app/src/main/jniLibs
targets=(); for abi in "${abis[@]}"; do targets+=(-t "$abi"); done
flags=(); [ "$profile" = release ] && flags+=(--release)
rm -rf "$out"
# Freya's skia compiles against API 26 (freya-skia-bindings hardcodes -D__ANDROID_API__=26), and so
# does AAudio (data over sound). Keep this in step with minSdk in app/build.gradle.kts.
cargo ndk "${targets[@]}" -P 26 -o "$out" build -p "minifrost_${app}_app" --lib "${flags[@]}"
# cargo-ndk copies every cdylib of the build; the app only loads its own.
find "$out" -name '*.so' ! -name "libminifrost_${app}_app.so" -delete
