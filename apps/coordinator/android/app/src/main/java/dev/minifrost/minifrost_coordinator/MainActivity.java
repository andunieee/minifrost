package dev.minifrost.minifrost_coordinator;

import dev.minifrost.MinifrostActivity;

public class MainActivity extends MinifrostActivity {
    static {
        // NativeActivity opens the same library (android.app.lib_name) for android_main; loading it here
        // first lets the JVM find the native methods of dev.minifrost.Native.
        System.loadLibrary("minifrost_coordinator_app");
    }
}
