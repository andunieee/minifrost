package dev.minifrost.minifrost_signer

import android.os.Bundle
import android.view.WindowManager
import io.flutter.embedding.android.FlutterFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

// FragmentActivity because BiometricPrompt needs one.
class MainActivity : FlutterFragmentActivity() {
    private lateinit var vault: Vault

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // No screenshots, no screen recording, blank thumbnail in the app switcher.
        window.setFlags(WindowManager.LayoutParams.FLAG_SECURE, WindowManager.LayoutParams.FLAG_SECURE)
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        vault = Vault(this)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "minifrost/vault").setMethodCallHandler { call, result ->
            when (call.method) {
                "status" -> result.success(
                    mapOf(
                        "exists" to vault.exists(),
                        "secureLockScreen" to vault.hasSecureLockScreen(),
                        "hardware" to vault.hardwareLevel(),
                    )
                )
                "load" -> vault.load({ result.success(it) }, { code, msg -> result.error(code, msg, null) })
                "save" -> {
                    val bytes = call.arguments as ByteArray
                    vault.save(bytes, { result.success(null) }, { code, msg -> result.error(code, msg, null) })
                }
                "wipe" -> { vault.wipe(); result.success(null) }
                else -> result.notImplemented()
            }
        }
    }
}
