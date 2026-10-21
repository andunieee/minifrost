package dev.minifrost.minifrost_signer

import android.app.KeyguardManager
import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import android.security.keystore.UserNotAuthenticatedException
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import java.io.File
import java.io.FileOutputStream
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec

/**
 * Where the signer's secrets live at rest.
 *
 * The Rust side hands us an opaque snapshot (device secret, key shares, nonce state). We encrypt
 * it with an AES-256-GCM key that:
 *   - is generated inside the Android Keystore (StrongBox secure element when the phone has one,
 *     otherwise the TEE) and can never be exported,
 *   - only works shortly after the user unlocked with biometrics or their screen lock,
 *   - only works while the device is unlocked.
 * The ciphertext sits in noBackupFilesDir, which is excluded from every backup/transfer path.
 *
 * Consequence worth knowing: if the user removes their screen lock or wipes the phone, the key is
 * gone and so is this signer's share. That is what the t-of-n threshold is for.
 */
class Vault(private val activity: FragmentActivity) {
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private val file = File(activity.noBackupFilesDir, "signer.vault")
    private val aad = "minifrost-signer-vault-v1".toByteArray()

    companion object {
        private const val ALIAS = "minifrost.signer.vault"
        private const val AUTH_VALID_SECONDS = 120
        private const val VERSION: Byte = 1
    }

    private val authenticators =
        BiometricManager.Authenticators.BIOMETRIC_STRONG or BiometricManager.Authenticators.DEVICE_CREDENTIAL

    fun hasSecureLockScreen(): Boolean =
        (activity.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager).isDeviceSecure

    fun exists(): Boolean = file.exists()

    /** "strongbox", "tee", "software" or "none" (no key created yet). */
    fun hardwareLevel(): String {
        val key = keyStore.getKey(ALIAS, null) as? SecretKey ?: return "none"
        val info = SecretKeyFactory.getInstance(key.algorithm, "AndroidKeyStore").getKeySpec(key, KeyInfo::class.java) as KeyInfo
        return when {
            android.os.Build.VERSION.SDK_INT >= 31 && info.securityLevel == KeyProperties.SECURITY_LEVEL_STRONGBOX -> "strongbox"
            info.isInsideSecureHardware -> "tee"
            else -> "software"
        }
    }

    private fun createKey(strongBox: Boolean): SecretKey {
        val spec = KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setUserAuthenticationRequired(true)
            .setUserAuthenticationParameters(
                AUTH_VALID_SECONDS,
                KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL
            )
            .setUnlockedDeviceRequired(true)
            .setRandomizedEncryptionRequired(true)
            .apply { if (strongBox) setIsStrongBoxBacked(true) }
            .build()
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").run {
            init(spec)
            generateKey()
        }
    }

    private fun key(): SecretKey {
        (keyStore.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        return try {
            createKey(strongBox = true)
        } catch (e: StrongBoxUnavailableException) {
            createKey(strongBox = false)
        }
    }

    private fun seal(plain: ByteArray): ByteArray {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key())
        cipher.updateAAD(aad)
        val ct = cipher.doFinal(plain)
        return byteArrayOf(VERSION) + cipher.iv + ct
    }

    private fun unseal(blob: ByteArray): ByteArray {
        require(blob.isNotEmpty() && blob[0] == VERSION) { "unknown vault version" }
        val iv = blob.copyOfRange(1, 13)
        val ct = blob.copyOfRange(13, blob.size)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, iv))
        cipher.updateAAD(aad)
        return cipher.doFinal(ct)
    }

    /** Runs [block]; if the keystore wants a fresh user authentication, asks for it and retries once. */
    private fun <T> withAuth(block: () -> T, ok: (T) -> Unit, fail: (String, String) -> Unit) {
        try {
            ok(block())
            return
        } catch (e: UserNotAuthenticatedException) {
            // fall through to prompt
        } catch (e: KeyPermanentlyInvalidatedException) {
            fail("key_invalidated", "The vault key was invalidated (screen lock removed or changed). The share on this signer is lost."); return
        } catch (e: Exception) {
            fail("vault_error", e.message ?: e.toString()); return
        }
        val executor = ContextCompat.getMainExecutor(activity)
        val prompt = BiometricPrompt(activity, executor, object : BiometricPrompt.AuthenticationCallback() {
            override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                try { ok(block()) } catch (e: Exception) { fail("vault_error", e.message ?: e.toString()) }
            }
            override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                fail("auth_cancelled", errString.toString())
            }
        })
        prompt.authenticate(
            BiometricPrompt.PromptInfo.Builder()
                .setTitle("Unlock signer")
                .setSubtitle("Needed to read or update your key share")
                .setAllowedAuthenticators(authenticators)
                .build()
        )
    }

    fun load(ok: (ByteArray?) -> Unit, fail: (String, String) -> Unit) {
        if (!file.exists()) { ok(null); return }
        val blob = file.readBytes()
        withAuth({ unseal(blob) }, ok, fail)
    }

    fun save(plain: ByteArray, ok: () -> Unit, fail: (String, String) -> Unit) {
        withAuth({
            val blob = seal(plain)
            val tmp = File(file.parentFile, "signer.vault.tmp")
            FileOutputStream(tmp).use { it.write(blob); it.fd.sync() }
            Files.move(tmp.toPath(), file.toPath(), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
        }, { ok() }, fail)
    }

    fun wipe() {
        file.delete()
        if (keyStore.containsAlias(ALIAS)) keyStore.deleteEntry(ALIAS)
    }
}
