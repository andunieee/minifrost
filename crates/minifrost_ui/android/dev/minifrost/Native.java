package dev.minifrost;

/**
 * The native side (crates/minifrost_ui/src/platform/android.rs). One class shared by both apps, so the
 * symbols are the same in both libraries.
 */
public final class Native {
    private Native() {}

    /** The answer to {@link MinifrostActivity#request}: `data`, or `error` ("code\nmessage" or a message). */
    public static native void result(long id, byte[] data, String error);

    /** Room taken by the system bars and the keyboard, in logical pixels, and whether the keyboard is up. */
    static native void insets(float left, float top, float right, float bottom, boolean keyboard);

    /** The luma plane of a camera frame; `rotation` turns it upright (clockwise degrees). */
    static native void cameraFrame(byte[] luma, int width, int height, int rowStride, int rotation);

    static native void cameraError(String message);
}
