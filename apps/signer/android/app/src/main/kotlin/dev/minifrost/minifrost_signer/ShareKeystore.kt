package dev.minifrost.minifrost_signer

import android.app.Activity
import android.app.KeyguardManager
import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.UserNotAuthenticatedException
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.security.KeyStore
import java.security.ProviderException
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Seals key shares with an AES-GCM key kept in the Android key store. The key never leaves it and
 * only works for [AUTH_SECONDS] after the screen lock was confirmed; Android deletes it for good
 * when the screen lock is removed. One key serves every share on the key store.
 *
 * Sealed bytes: version byte, 12-byte IV, ciphertext with its tag.
 */
class ShareKeystore(private val activity: Activity) {
    companion object {
        const val CHANNEL = "minifrost/keystore"
        const val REQUEST_CODE = 0x4d46
        private const val ALIAS = "minifrost-shares"
        private const val AUTH_SECONDS = 30
        private const val VERSION: Byte = 1
        private const val IV_BYTES = 12
    }

    private class NoScreenLock : Exception()
    private class KeyGone : Exception()

    /** What to do once the user confirmed (true) or dismissed (false) the screen lock. */
    private var afterScreenLock: ((Boolean) -> Unit)? = null

    fun handle(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "screenLockSet" -> result.success(keyguard().isDeviceSecure)
            "seal" -> run(result) { seal(call.arguments as ByteArray) }
            "open" -> run(result) { open(call.arguments as ByteArray) }
            else -> result.notImplemented()
        }
    }

    /** From `onActivityResult`; true when it was the screen lock this class asked for. */
    fun onActivityResult(requestCode: Int, resultCode: Int): Boolean {
        if (requestCode != REQUEST_CODE) return false
        val then = afterScreenLock ?: return true
        afterScreenLock = null
        then(resultCode == Activity.RESULT_OK)
        return true
    }

    /** Runs [op], asking for the screen lock once when the key wants it. */
    private fun run(result: MethodChannel.Result, retried: Boolean = false, op: () -> ByteArray) {
        try {
            result.success(op())
        } catch (e: UserNotAuthenticatedException) {
            if (retried) return result.error("cancelled", "The screen lock was not confirmed.", null)
            confirmScreenLock { ok ->
                if (ok) run(result, true, op)
                else result.error("cancelled", "The screen lock was not confirmed.", null)
            }
        } catch (e: NoScreenLock) {
            result.error("no_screen_lock", "This phone has no screen lock (PIN, pattern or password).", null)
        } catch (e: KeyPermanentlyInvalidatedException) {
            result.error("key_gone", "The device key store no longer has its key.", null)
        } catch (e: KeyGone) {
            result.error("key_gone", "The device key store no longer has its key.", null)
        } catch (e: AEADBadTagException) {
            // sealed with a key that was deleted and replaced since
            result.error("key_gone", "The device key store no longer has its key.", null)
        } catch (e: Exception) {
            result.error("failed", e.toString(), null)
        }
    }

    private fun confirmScreenLock(then: (Boolean) -> Unit) {
        @Suppress("DEPRECATION")
        val intent = keyguard().createConfirmDeviceCredentialIntent(
            "Minifrost Signer",
            "Confirm your screen lock to use the key share on the device key store.",
        ) ?: return then(false)
        afterScreenLock = then
        activity.startActivityForResult(intent, REQUEST_CODE)
    }

    private fun seal(plain: ByteArray): ByteArray {
        if (!keyguard().isDeviceSecure) throw NoScreenLock()
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        try {
            cipher.init(Cipher.ENCRYPT_MODE, existingKey() ?: newKey())
        } catch (e: KeyPermanentlyInvalidatedException) {
            // The screen lock was removed since: the shares sealed with this key can't be opened
            // any more anyway. Start over with a new key.
            keyStore().deleteEntry(ALIAS)
            cipher.init(Cipher.ENCRYPT_MODE, newKey())
        }
        val sealed = byteArrayOf(VERSION) + cipher.iv + cipher.doFinal(plain)
        // still inside the screen lock window: make sure it opens before the share leaves the disk
        check(open(sealed).contentEquals(plain)) { "the sealed share does not open again" }
        return sealed
    }

    private fun open(sealed: ByteArray): ByteArray {
        require(sealed.size > 1 + IV_BYTES && sealed[0] == VERSION) { "not a sealed share" }
        val key = existingKey() ?: throw KeyGone()
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, sealed, 1, IV_BYTES))
        return cipher.doFinal(sealed, 1 + IV_BYTES, sealed.size - 1 - IV_BYTES)
    }

    private fun keyStore(): KeyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    private fun existingKey(): SecretKey? = keyStore().getKey(ALIAS, null) as SecretKey?

    private fun newKey(): SecretKey {
        fun spec(strongBox: Boolean): KeyGenParameterSpec {
            val b = KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setUserAuthenticationRequired(true)
            if (Build.VERSION.SDK_INT >= 30) {
                b.setUserAuthenticationParameters(
                    AUTH_SECONDS,
                    KeyProperties.AUTH_DEVICE_CREDENTIAL or KeyProperties.AUTH_BIOMETRIC_STRONG,
                )
            } else {
                @Suppress("DEPRECATION")
                b.setUserAuthenticationValidityDurationSeconds(AUTH_SECONDS)
            }
            // adding a fingerprint must not cost the shares; only removing the screen lock does
            b.setInvalidatedByBiometricEnrollment(false)
            if (strongBox && Build.VERSION.SDK_INT >= 28) b.setIsStrongBoxBacked(true)
            return b.build()
        }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        return try {
            gen.init(spec(strongBox = true))
            gen.generateKey()
        } catch (e: ProviderException) {
            // StrongBoxUnavailableException: this phone has no separate security chip
            if (Build.VERSION.SDK_INT < 28) throw e
            gen.init(spec(strongBox = false))
            gen.generateKey()
        }
    }

    private fun keyguard() = activity.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
}
