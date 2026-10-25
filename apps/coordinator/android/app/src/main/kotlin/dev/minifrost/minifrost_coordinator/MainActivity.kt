package dev.minifrost.minifrost_coordinator

import android.content.Context
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity() {
    // Lets the Rust sound code reach the Java audio APIs, see sound_transport::android.
    private external fun initAudio(context: Context)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        System.loadLibrary("minifrost_coordinator")
        initAudio(applicationContext)
    }
}
