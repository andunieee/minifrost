package dev.minifrost.minifrost_signer

import android.content.Context
import android.content.pm.ApplicationInfo
import android.os.Bundle
import android.view.WindowManager
import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity() {
    // Lets the Rust sound code reach the Java audio APIs, see sound_transport::android.
    private external fun initAudio(context: Context)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        System.loadLibrary("minifrost_signer")
        initAudio(applicationContext)
        // No screenshots, no screen recording, blank thumbnail in the app switcher. Debug builds
        // are exempt so the screens can be captured over adb while testing.
        val debuggable = applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
        if (!debuggable) window.setFlags(WindowManager.LayoutParams.FLAG_SECURE, WindowManager.LayoutParams.FLAG_SECURE)
    }
}
