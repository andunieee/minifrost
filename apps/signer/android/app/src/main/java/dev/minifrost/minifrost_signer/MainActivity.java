package dev.minifrost.minifrost_signer;

import android.content.Intent;

import dev.minifrost.MinifrostActivity;

public class MainActivity extends MinifrostActivity {
    static {
        // NativeActivity opens the same library (android.app.lib_name) for android_main; loading it here
        // first lets the JVM find the native methods of dev.minifrost.Native.
        System.loadLibrary("minifrost_signer_app");
    }

    private final ShareKeystore keystore = new ShareKeystore(this);

    /** No screenshots, no screen recording, blank in the app switcher. Debug builds are exempt so the
     * screens can be captured over adb while testing. */
    @Override
    protected boolean secure() {
        return !debuggable();
    }

    @Override
    protected void handle(String op, long id, byte[] data) throws Exception {
        if (!keystore.handle(op, id, data)) super.handle(op, id, data);
    }

    @Override
    @SuppressWarnings("deprecation")
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        if (!keystore.onActivityResult(requestCode, resultCode)) super.onActivityResult(requestCode, resultCode, data);
    }
}
