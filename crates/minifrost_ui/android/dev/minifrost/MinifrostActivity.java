package dev.minifrost;

import android.app.NativeActivity;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.view.SurfaceView;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowManager;

import androidx.core.view.ViewCompat;
import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsCompat;

import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;

/**
 * What both apps' activities share. The UI is native code (Freya, started by NativeActivity in
 * `android_main`); this does what it can't do from there: the window insets, runtime permissions and
 * the camera. Rust asks through {@link #request} and gets its answer through {@link Native#result}.
 */
public abstract class MinifrostActivity extends NativeActivity {
    private final Map<Integer, Long> permissionRequests = new HashMap<>();
    private int nextRequestCode = 1;
    private QrCamera camera;

    /** Whether to hide the window from screenshots, screen recording and the app switcher. */
    protected boolean secure() {
        return false;
    }

    protected boolean debuggable() {
        return (getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0;
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        if (secure()) {
            getWindow().setFlags(WindowManager.LayoutParams.FLAG_SECURE, WindowManager.LayoutParams.FLAG_SECURE);
        }
        edgeToEdge();
    }

    /**
     * NativeActivity gives the UI the whole window, under the system bars: their size (and the
     * keyboard's) goes to Rust, which keeps the content clear of them. The surface needs focus for
     * key events to arrive.
     */
    @SuppressWarnings("deprecation")
    private void edgeToEdge() {
        WindowCompat.setDecorFitsSystemWindows(getWindow(), false);
        if (Build.VERSION.SDK_INT < 35) {
            getWindow().setStatusBarColor(Color.TRANSPARENT);
            getWindow().setNavigationBarColor(Color.TRANSPARENT);
        }
        if (Build.VERSION.SDK_INT >= 29) {
            getWindow().setStatusBarContrastEnforced(false);
            getWindow().setNavigationBarContrastEnforced(false);
        }
        View decor = getWindow().getDecorView();
        ViewCompat.setOnApplyWindowInsetsListener(decor, (v, insets) -> {
            float density = getResources().getDisplayMetrics().density;
            androidx.core.graphics.Insets bars = insets.getInsets(
                    WindowInsetsCompat.Type.systemBars() | WindowInsetsCompat.Type.displayCutout());
            int ime = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom;
            Native.insets(bars.left / density, bars.top / density, bars.right / density,
                    Math.max(bars.bottom, ime) / density);
            return insets;
        });
        decor.post(() -> {
            View surface = findSurface(decor);
            if (surface != null) {
                surface.setFocusable(true);
                surface.setFocusableInTouchMode(true);
                surface.requestFocus();
            }
        });
    }

    private static View findSurface(View view) {
        if (view instanceof SurfaceView) return view;
        if (view instanceof ViewGroup) {
            ViewGroup group = (ViewGroup) view;
            for (int i = 0; i < group.getChildCount(); i++) {
                View found = findSurface(group.getChildAt(i));
                if (found != null) return found;
            }
        }
        return null;
    }

    /** Called from Rust, on its thread. `id` 0 wants no answer. */
    public void request(String op, long id, byte[] data) {
        runOnUiThread(() -> {
            try {
                handle(op, id, data);
            } catch (Exception e) {
                fail(id, "failed", e.toString());
            }
        });
    }

    /** Runs a request on the main thread; subclasses add their own and pass the rest on. */
    protected void handle(String op, long id, byte[] data) throws Exception {
        switch (op) {
            case "permission": {
                String permission = new String(data, StandardCharsets.UTF_8);
                if (checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED) {
                    reply(id, new byte[]{1});
                } else {
                    int code = nextRequestCode++;
                    permissionRequests.put(code, id);
                    requestPermissions(new String[]{permission}, code);
                }
                break;
            }
            case "camera_start":
                if (camera == null) camera = new QrCamera(this);
                camera.start();
                reply(id, new byte[0]);
                break;
            case "camera_stop":
                if (camera != null) camera.stop();
                reply(id, new byte[0]);
                break;
            case "move_to_back":
                moveTaskToBack(true);
                reply(id, new byte[0]);
                break;
            default:
                fail(id, "unknown", "Unknown request " + op);
        }
    }

    protected static void reply(long id, byte[] data) {
        if (id != 0) Native.result(id, data, null);
    }

    protected static void fail(long id, String code, String message) {
        if (id != 0) Native.result(id, null, code + "\n" + message);
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        Long id = permissionRequests.remove(requestCode);
        if (id == null) return;
        boolean granted = grantResults.length > 0 && grantResults[0] == PackageManager.PERMISSION_GRANTED;
        reply(id, new byte[]{(byte) (granted ? 1 : 0)});
    }

    @Override
    protected void onPause() {
        // the camera is the system's while the app is out of sight
        if (camera != null) camera.pause();
        super.onPause();
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (camera != null) camera.resume();
    }

    /**
     * The UI's event loop cannot be started again in this process (winit allows one per process), and
     * NativeActivity waits here for it to end, which it never does. So the activity's end is the
     * process's (the manifest handles every configuration change, so that is only when the user or the
     * system closes it); the next launch starts afresh.
     */
    @Override
    protected void onDestroy() {
        if (camera != null) camera.stop();
        android.os.Process.killProcess(android.os.Process.myPid());
        super.onDestroy();
    }
}
