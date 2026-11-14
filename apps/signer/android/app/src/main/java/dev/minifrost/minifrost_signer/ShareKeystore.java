package dev.minifrost.minifrost_signer;

import android.app.Activity;
import android.app.KeyguardManager;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyPermanentlyInvalidatedException;
import android.security.keystore.KeyProperties;
import android.security.keystore.UserNotAuthenticatedException;

import java.security.KeyStore;
import java.security.ProviderException;
import java.util.Arrays;

import javax.crypto.AEADBadTagException;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

import dev.minifrost.Native;

/**
 * Seals key shares with an AES-GCM key kept in the Android key store. The key never leaves it and only
 * works for {@link #AUTH_SECONDS} after the screen lock was confirmed; Android deletes it for good when
 * the screen lock is removed. One key serves every share on the key store.
 *
 * Sealed bytes: version byte, 12-byte IV, ciphertext with its tag.
 */
final class ShareKeystore {
    static final int REQUEST_CODE = 0x4d46;
    private static final String ALIAS = "minifrost-shares";
    private static final int AUTH_SECONDS = 30;
    private static final byte VERSION = 1;
    private static final int IV_BYTES = 12;

    private static final class NoScreenLock extends Exception {}

    private static final class KeyGone extends Exception {}

    private interface Op {
        byte[] run() throws Exception;
    }

    private interface AfterScreenLock {
        void then(boolean confirmed);
    }

    private final Activity activity;
    /** What to do once the user confirmed (true) or dismissed (false) the screen lock. */
    private AfterScreenLock afterScreenLock;

    ShareKeystore(Activity activity) {
        this.activity = activity;
    }

    /** Requests from Rust (src/keystore.rs); false for ones that aren't the key store's. */
    boolean handle(String op, long id, byte[] data) {
        switch (op) {
            case "keystore_screen_lock":
                answer(id, new byte[]{(byte) (keyguard().isDeviceSecure() ? 1 : 0)});
                return true;
            case "keystore_seal":
                run(id, false, () -> seal(data));
                return true;
            case "keystore_open":
                run(id, false, () -> open(data));
                return true;
            default:
                return false;
        }
    }

    /** From onActivityResult; true when it was the screen lock this class asked for. */
    boolean onActivityResult(int requestCode, int resultCode) {
        if (requestCode != REQUEST_CODE) return false;
        AfterScreenLock then = afterScreenLock;
        afterScreenLock = null;
        if (then != null) then.then(resultCode == Activity.RESULT_OK);
        return true;
    }

    private static void answer(long id, byte[] data) {
        Native.result(id, data, null);
    }

    private static void error(long id, String code, String message) {
        Native.result(id, null, code + "\n" + message);
    }

    /** Runs `op`, asking for the screen lock once when the key wants it. */
    private void run(long id, boolean retried, Op op) {
        try {
            answer(id, op.run());
        } catch (UserNotAuthenticatedException e) {
            if (retried) {
                error(id, "cancelled", "The screen lock was not confirmed.");
                return;
            }
            confirmScreenLock(ok -> {
                if (ok) run(id, true, op);
                else error(id, "cancelled", "The screen lock was not confirmed.");
            });
        } catch (NoScreenLock e) {
            error(id, "no_screen_lock", "This phone has no screen lock (PIN, pattern or password).");
        } catch (KeyPermanentlyInvalidatedException | KeyGone | AEADBadTagException e) {
            // AEADBadTag: sealed with a key that was deleted and replaced since
            error(id, "key_gone", "The device key store no longer has its key.");
        } catch (Exception e) {
            error(id, "failed", e.toString());
        }
    }

    @SuppressWarnings("deprecation")
    private void confirmScreenLock(AfterScreenLock then) {
        Intent intent = keyguard().createConfirmDeviceCredentialIntent(
                "Minifrost Signer",
                "Confirm your screen lock to use the key share on the device key store.");
        if (intent == null) {
            then.then(false);
            return;
        }
        afterScreenLock = then;
        activity.startActivityForResult(intent, REQUEST_CODE);
    }

    private byte[] seal(byte[] plain) throws Exception {
        if (!keyguard().isDeviceSecure()) throw new NoScreenLock();
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        SecretKey key = existingKey();
        try {
            cipher.init(Cipher.ENCRYPT_MODE, key != null ? key : newKey());
        } catch (KeyPermanentlyInvalidatedException e) {
            // The screen lock was removed since: the shares sealed with this key can't be opened any
            // more anyway. Start over with a new key.
            keyStore().deleteEntry(ALIAS);
            cipher.init(Cipher.ENCRYPT_MODE, newKey());
        }
        byte[] iv = cipher.getIV();
        byte[] body = cipher.doFinal(plain);
        byte[] sealed = new byte[1 + iv.length + body.length];
        sealed[0] = VERSION;
        System.arraycopy(iv, 0, sealed, 1, iv.length);
        System.arraycopy(body, 0, sealed, 1 + iv.length, body.length);
        // still inside the screen lock window: make sure it opens before the share leaves the disk
        if (!Arrays.equals(open(sealed), plain)) throw new IllegalStateException("the sealed share does not open again");
        return sealed;
    }

    private byte[] open(byte[] sealed) throws Exception {
        if (sealed.length <= 1 + IV_BYTES || sealed[0] != VERSION) throw new IllegalArgumentException("not a sealed share");
        SecretKey key = existingKey();
        if (key == null) throw new KeyGone();
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.DECRYPT_MODE, key, new GCMParameterSpec(128, sealed, 1, IV_BYTES));
        return cipher.doFinal(sealed, 1 + IV_BYTES, sealed.length - 1 - IV_BYTES);
    }

    private static KeyStore keyStore() throws Exception {
        KeyStore store = KeyStore.getInstance("AndroidKeyStore");
        store.load(null);
        return store;
    }

    private static SecretKey existingKey() throws Exception {
        return (SecretKey) keyStore().getKey(ALIAS, null);
    }

    @SuppressWarnings("deprecation")
    private static KeyGenParameterSpec spec(boolean strongBox) {
        KeyGenParameterSpec.Builder b = new KeyGenParameterSpec.Builder(ALIAS,
                KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setUserAuthenticationRequired(true);
        if (Build.VERSION.SDK_INT >= 30) {
            b.setUserAuthenticationParameters(AUTH_SECONDS,
                    KeyProperties.AUTH_DEVICE_CREDENTIAL | KeyProperties.AUTH_BIOMETRIC_STRONG);
        } else {
            b.setUserAuthenticationValidityDurationSeconds(AUTH_SECONDS);
        }
        // adding a fingerprint must not cost the shares; only removing the screen lock does
        b.setInvalidatedByBiometricEnrollment(false);
        if (strongBox && Build.VERSION.SDK_INT >= 28) b.setIsStrongBoxBacked(true);
        return b.build();
    }

    private static SecretKey newKey() throws Exception {
        KeyGenerator gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
        try {
            gen.init(spec(true));
            return gen.generateKey();
        } catch (ProviderException e) {
            // StrongBoxUnavailableException: this phone has no separate security chip
            if (Build.VERSION.SDK_INT < 28) throw e;
            gen.init(spec(false));
            return gen.generateKey();
        }
    }

    private KeyguardManager keyguard() {
        return (KeyguardManager) activity.getSystemService(Context.KEYGUARD_SERVICE);
    }
}
