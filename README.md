# minifrost

A small FROST (threshold Schnorr) Bitcoin wallet where the signers are **air-gapped phones** and
everything between them and the coordinator travels as **animated QR codes**.

| | Coordinator | Signer |
|---|---|---|
| App | `apps/coordinator` (Flutter, Linux desktop) | `apps/signer` (Flutter, Android; also runs on Linux desktop for dev/testing) |
| Rust | `crates/minifrost_coordinator` | `crates/minifrost_signer` |
| Network | Esplora only (balances, fee estimates, broadcast) | **none** — no `INTERNET` permission, no networking crates |
| Permissions | webcam | `CAMERA` |
| Holds | coordinator half of the share-decryption secret, public wallet data | the secret key share |

The cryptography and protocol are [frostsnap](https://github.com/frostsnap/frostsnap)'s: the
workspace depends on `frostsnap_core` from GitHub, pinned to a commit in the root `Cargo.toml`
(bump `rev` to update). `crates/desktop_camera` is frostsnap's webcam capture, trimmed to Linux and
copied in because it isn't published as a library we can depend on cleanly.

## Why Flutter + Rust (and not Slint, or Kotlin + a Rust library)

All the protocol logic is Rust and is exercised by `cargo test` without any UI. Flutter only draws
screens, shows QR codes and runs the camera; `flutter_rust_bridge` generates the glue. One UI
toolkit covers both Android and Linux, `camera` + `qr_flutter` cover the QR needs, and storage is a plain file.
Kotlin-only would have duplicated the protocol or required hand-written JNI.

## How the QR protocol works

Every message is a `minifrost_wire::Packet` (bincode, wrapping frostsnap's own
coordinator↔device messages), split by `qr_transport` into a loop of frames such as
`MF1:3FA9C2E1:2:7:<base32>`. The scanner collects frames in any order, repeats are fine, and the
whole payload is checked against its SHA-256 prefix. Coordinator → signers is always one
**broadcast** QR that every signer scans; each signer acts only on entries addressed to its id.

**Enrolment** — signer shows `Hello{id, name}`; coordinator scans it.

**Key generation** (ChillDKG / certpedpop, from frostsnap)
1. Coordinator broadcasts `Begin`; each signer user approves → signer shows its keygen contribution.
2. Coordinator scans all, broadcasts `CertifyPlease`; signers answer automatically with a certificate.
3. Coordinator scans all, broadcasts `Check`; **every signer and the coordinator show the same check
   code** — humans compare — each signer approves → shows an ack.
4. Coordinator scans all acks, stores the key, broadcasts `Finalize`; each signer stores its share.

**Signing** (taproot key-path spend of a tweaked FROST key)
1. Coordinator broadcasts `SignPrepare`: the unsigned transaction and a nonce request. Each chosen
   signer **reviews outputs and fee**, approves, and shows its **nonces**.
2. Coordinator scans the nonces, aggregates them, and broadcasts one **bundle** (aggregate nonces +
   a per-signer request). Each signer re-derives the transaction from its *own* key (so change
   outputs are verified, not trusted), refuses if it differs from the one reviewed in step 1,
   approves, and shows its **partial signature**.
3. Coordinator scans the signature shares, verifies each, combines them into one BIP340 signature,
   builds the witness, and offers **Publish via Esplora**.

Frostsnap pre-shares batches of nonces (a nonce stream) so that, after the first session, step 1
returns "no new nonces needed" and the nonces already held by the coordinator are used. The round
trip still happens, which is what binds the transaction review to the signature.

## Where the signer keeps its secret

- One 32-byte random `device_secret`; the identity key, the share-encryption key and the nonce
  seeds are all HMAC-derived from it. The share is encrypted under a key that also needs a value
  only the coordinator holds, so the phone's storage alone cannot sign.
- Rust hands the app a snapshot, which `lib/vault.dart` writes **unencrypted** to a file in the
  app-private directory (atomic temp file + rename). Backups and device transfer are disabled;
  `FLAG_SECURE` blocks screenshots. Minimum Android is 7 (API 24). No screen lock, biometrics or
  Keystore are involved, so anyone with file access to the phone (root, forensics) can read the share.
- State is persisted **before** any QR derived from it is shown (`controller.dart`). After
  signing, the snapshot contains the burned nonce — so a crash cannot lead to nonce reuse.
- The manifest asks for `CAMERA` and explicitly removes `INTERNET`, network-state, audio
  and storage permissions (also in debug/profile builds). QR decoding is done in Rust (`rqrr`), so there is no ML Kit or
  Google Play Services dependency.

`scripts/check-signer-offline.sh [signer.apk]` fails if a networking crate is in the signer's
dependency tree or the APK requests anything beyond those three.

## Build and run

Needs: Rust, Flutter (3.47+), `cargo install flutter_rust_bridge_codegen --version 2.13.0`,
Android SDK + NDK 27.2 for the signer, GTK/cmake/clang for the Linux coordinator.

```sh
cargo test --workspace                       # protocol, wallet logic, full QR flow end to end

cd apps/coordinator && flutter run -d linux  # coordinator
cd apps/signer && flutter run -d linux       # signer, desktop (dev/testing)
cd apps/signer && flutter build apk --release --target-platform android-arm64
adb install build/app/outputs/flutter-apk/app-release.apk
../../scripts/check-signer-offline.sh build/app/outputs/flutter-apk/app-release.apk
```

After changing `crates/*/src/api`, regenerate the bindings with
`flutter_rust_bridge_codegen generate` inside the matching app directory. The signer has no
`INTERNET` permission even in debug builds, so there is no hot reload: iterate with release builds.
(`cargo test` covers the logic; the e2e test lives in the coordinator crate and `#[path]`-includes
the signer engine because two FFI crates cannot be linked into one binary.)

The release APK is signed with the debug key; set up your own signing config before distributing.

## Known limits

- Signer state mid-keygen is in memory only: if the app is killed during a key generation, redo
  that keygen. Finished keys, nonce state and the "reviewed transaction" survive restarts.
- If the phone is wiped, its share is gone (that is what the
  threshold is for). Frostsnap's backup / restoration flows are not ported.
- Wallet: single taproot account, largest-first coin selection, one signing session at a time,
  gap limit 10. Change addresses are not reserved across concurrent drafts.
- The Linux desktop signer is for dev/testing only: the webcam replaces the phone camera
  (`desktop_camera`). The QR protocol is identical to the phone, so coordinator↔signer flows can be driven
  entirely on one machine.
- The Flutter UI has not been run against a physical phone or webcam in the environment this was
  written in: everything below the UI is covered by tests, the Android and Linux builds are
  verified to compile, the camera and screens need a real device pass.
