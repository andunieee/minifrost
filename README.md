# minifrost

A small FROST (threshold Schnorr) Bitcoin wallet where the signers are **air-gapped phones** and
everything between them and the coordinator travels as **animated QR codes**.

| | Coordinator | Signer |
|---|---|---|
| App | `apps/coordinator` (Flutter, Linux desktop) | `apps/signer` (Flutter, Android; also runs on Linux desktop for dev/testing) |
| Rust | `crates/minifrost_coordinator` | `crates/minifrost_signer` |
| Network | Esplora only (balances, fee estimates, broadcast) | **none** — no `INTERNET` permission, no networking crates |
| Permissions | webcam, microphone | `CAMERA`, `RECORD_AUDIO` |
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

### Sound instead of QR codes

Where a camera is awkward, any QR code can be sent as sound instead ([ggwave](https://github.com/ggerganov/ggwave),
`crates/sound_transport`). On the screen that shows a code press **Play sound**; on the device that
scans press **Listen (sound)** first. The player loops the message as audible chirps (`AUDIBLE_FAST`,
100 payload bytes each, roughly 15 bytes/s), the listener's microphone hears them in any order and
with repeats, and the frames go into the same assembler and SHA-256 check as scanned QR frames.
Keep the devices close in a quiet room. Raise the phone's media volume first (many phones keep
it at 0 for the speaker). A desktop without a microphone can play sound but not listen: feed it the
phone's "Show raw" text through the **Paste QR text** field instead. The signer asks for the `RECORD_AUDIO` permission the
first time you press Listen; like the camera it is only used while the screen is open.

### Backups and restoration

These follow frostsnap's flows (`frostsnap_core`'s restoration state machine, with the same
messages); only the transport and the screens are minifrost's. A share's backup is **25 BIP39 words
plus a key number**. The words carry a checksum tied to the wallet's public polynomial, so a backup
typed into the wrong wallet is caught.

**Backup** (after key generation every share is "not backed up" until its signer confirmed it)
1. Coordinator broadcasts `DisplayBackup` to one signer, with its decryption contribution (see
   below). The signer decrypts its share with it and shows the 25 words with the key number, threshold and wallet name; the user writes them down and
   confirms on the signer, then presses the coordinator's confirmation button to mark that share
   backed up. No reply QR is needed. "Mark as backed up" does the same by hand from the wallet page.
2. **Check** (`CheckBackup`): the signer quizzes the user with 26 multiple-choice questions (key
   number, then each word; the wrong options are the closest BIP39 words) answered from the paper.
   Wrong answers change nothing. When all 26 are right the signer says the check is complete and
   the user presses the coordinator's confirmation button. No reply QR is needed.

**Restoration from physical backups** (new coordinator and/or new phones)
1. Coordinator creates a restoration (wallet name, network, optional threshold) and broadcasts
   `EnterPhysicalBackup` to a signer. The user types the key number and 25 words; the signer checks
   the word checksums and replies with the share's public *image* only.
2. The coordinator checks it fits (not a share it already has, not a different wallet's), then
   broadcasts `SavePhysicalBackup` → the signer keeps the backup and confirms locally; the user
   presses the coordinator's confirmation button instead of scanning a `PhysicalSaved` reply.
3. Repeat with other signers until the shares determine the key (`Restore wallet` becomes available).
   `finish_restoration` creates the wallet, with the same addresses as before.
4. **Consolidation**: each signer that was restored from a typed-in backup is asked (`Consolidate`)
   to turn it into a normal share of the wallet; it does so by itself and replies. Until then it
   cannot sign. The same path adds a replaced signer to a wallet the coordinator already has.

**Restoration from signers** (the coordinator was lost, the signers still hold their shares): the
coordinator broadcasts `RequestHeldShares`, each signer answers with the shares it holds (public
images, wallet name, threshold), and the user picks them into a restoration. A signer that rejoins
an existing wallet this way needs no consolidation.

As in frostsnap, **a signer can't decrypt its own share without the coordinator**. Only the
coordinator keeps the wallet's root key (the public point at the root of the key polynomial,
encrypted in its own file), and every message that needs a signer's share (sign, display or check
a backup, consolidate) carries `CoordShareDecryptionContrib`, a hash of the device id, share index
and root polynomial that the signer mixes into its share-encryption key. The signer computes the
same hash once, right after keygen, to encrypt the share, then forgets the polynomial.

The coordinator's file is still **not** needed to recover funds: the root key is public data that
a threshold of shares rebuilds (either restoration flow above), after which the new coordinator
can give the signers their contributions again. What *is* needed is a threshold of backups (or
signer phones), and knowing the wallet's name, network and threshold to check the result against.

## Where the signer keeps its secret

- One 32-byte random `device_secret`; the identity key is HMAC-derived from it, and the
  share-encryption keys and nonce seeds come from `frostsnap_core`'s `DevicePrf` derivation (the
  same byte layouts as a frostsnap device) with HMAC-SHA256 under `device_secret` as the PRF. The
  share-encryption key also needs the coordinator's contribution (see above). That contribution is
  derived from public wallet data, so, as frostsnap says of its own devices, it forces the signer to
  be talking to a coordinator that knows the whole wallet rather than hiding the share: the phone's
  storage plus the wallet's public polynomial can sign.
- Vaults written before this scheme (snapshot version 1) are refused: their shares were encrypted
  under the old key and can't be moved without the coordinator. Clear the app's data and restore
  the keys from their backup words.
- Rust hands the app a snapshot, which `lib/vault.dart` writes **unencrypted** to a file in the
  app-private directory (atomic temp file + rename). Backups and device transfer are disabled;
  `FLAG_SECURE` blocks screenshots (release builds only). Minimum Android is 7 (API 24). No screen lock, biometrics or
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
- If a phone is wiped, its share is gone unless its 25-word backup was written down (that, and the
  threshold, is what backups are for). The backup words are shown on the phone's screen: nothing
  stops someone looking over the shoulder, and a typed-in backup sits in the signer's plain file
  until it is consolidated. A backup typed in but not yet saved is kept in memory only, so an app
  restart in the middle of entering one means entering it again.
- Restoration state on the coordinator (shares gathered so far) is persisted; which shares each
  signer *reported* holding is not, so ask the signer again after a restart.
- The `CheckBackup` quiz compares against the signer's own share, so it proves the paper matches
  *that signer*, not that the wallet itself is recoverable.
- Wallet: single taproot account, largest-first coin selection, one signing session at a time,
  gap limit 10. Change addresses are not reserved across concurrent drafts.
- The Linux desktop signer is for dev/testing only: the webcam replaces the phone camera
  (`desktop_camera`). The QR protocol is identical to the phone, so coordinator↔signer flows can be driven
  entirely on one machine.
- The Flutter UI has not been run against a physical phone or webcam in the environment this was
  written in: everything below the UI is covered by tests, the Android and Linux builds are
  verified to compile, the camera and screens need a real device pass.
