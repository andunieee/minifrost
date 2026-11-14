package dev.minifrost;

import android.app.Activity;
import android.graphics.ImageFormat;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.Image;
import android.media.ImageReader;
import android.os.Handler;
import android.os.HandlerThread;
import android.util.Size;

import java.nio.ByteBuffer;
import java.util.Collections;

/**
 * The back camera, streaming the luma plane of every frame to Rust, which decodes the QR codes in it
 * and shows a preview. No preview surface of its own: the UI draws it.
 */
final class QrCamera {
    /** Frames about this size: enough for dense codes, small enough to decode quickly. */
    private static final int WANTED_PIXELS = 1280 * 720;

    private final Activity activity;
    private boolean wanted;
    private HandlerThread thread;
    private Handler handler;
    private CameraDevice device;
    private CameraCaptureSession session;
    private ImageReader reader;
    private byte[] buffer = new byte[0];

    QrCamera(Activity activity) {
        this.activity = activity;
    }

    void start() {
        wanted = true;
        open();
    }

    void stop() {
        wanted = false;
        close();
    }

    void pause() {
        close();
    }

    void resume() {
        if (wanted) open();
    }

    private void open() {
        if (thread != null) return;
        try {
            CameraManager manager = activity.getSystemService(CameraManager.class);
            String id = null;
            for (String candidate : manager.getCameraIdList()) {
                Integer facing = manager.getCameraCharacteristics(candidate).get(CameraCharacteristics.LENS_FACING);
                if (id == null || (facing != null && facing == CameraCharacteristics.LENS_FACING_BACK)) {
                    id = candidate;
                    if (facing != null && facing == CameraCharacteristics.LENS_FACING_BACK) break;
                }
            }
            if (id == null) {
                Native.cameraError("This phone has no camera.");
                return;
            }
            CameraCharacteristics characteristics = manager.getCameraCharacteristics(id);
            Integer sensor = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION);
            int rotation = sensor == null ? 0 : sensor;
            StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
            Size size = pickSize(map.getOutputSizes(ImageFormat.YUV_420_888));

            thread = new HandlerThread("qr-camera");
            thread.start();
            handler = new Handler(thread.getLooper());
            reader = ImageReader.newInstance(size.getWidth(), size.getHeight(), ImageFormat.YUV_420_888, 2);
            reader.setOnImageAvailableListener(r -> {
                Image image = r.acquireLatestImage();
                if (image == null) return;
                try {
                    Image.Plane y = image.getPlanes()[0];
                    ByteBuffer bytes = y.getBuffer();
                    if (buffer.length != bytes.remaining()) buffer = new byte[bytes.remaining()];
                    bytes.get(buffer);
                    Native.cameraFrame(buffer, image.getWidth(), image.getHeight(), y.getRowStride(), rotation);
                } finally {
                    image.close();
                }
            }, handler);
            manager.openCamera(id, new CameraDevice.StateCallback() {
                @Override
                public void onOpened(CameraDevice camera) {
                    device = camera;
                    startSession(camera);
                }

                @Override
                public void onDisconnected(CameraDevice camera) {
                    camera.close();
                    if (device == camera) device = null;
                }

                @Override
                public void onError(CameraDevice camera, int error) {
                    camera.close();
                    if (device == camera) device = null;
                    Native.cameraError("Camera error " + error + ".");
                }
            }, handler);
        } catch (SecurityException e) {
            close();
            Native.cameraError("Camera permission is needed to read QR codes.");
        } catch (CameraAccessException | RuntimeException e) {
            close();
            Native.cameraError("Camera error: " + e.getMessage());
        }
    }

    @SuppressWarnings("deprecation")
    private void startSession(CameraDevice camera) {
        try {
            camera.createCaptureSession(Collections.singletonList(reader.getSurface()),
                    new CameraCaptureSession.StateCallback() {
                        @Override
                        public void onConfigured(CameraCaptureSession s) {
                            session = s;
                            try {
                                CaptureRequest.Builder request = camera.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
                                request.addTarget(reader.getSurface());
                                request.set(CaptureRequest.CONTROL_AF_MODE, CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE);
                                s.setRepeatingRequest(request.build(), null, handler);
                            } catch (CameraAccessException | RuntimeException e) {
                                Native.cameraError("Camera error: " + e.getMessage());
                            }
                        }

                        @Override
                        public void onConfigureFailed(CameraCaptureSession s) {
                            Native.cameraError("The camera could not be set up.");
                        }
                    }, handler);
        } catch (CameraAccessException | RuntimeException e) {
            Native.cameraError("Camera error: " + e.getMessage());
        }
    }

    private static Size pickSize(Size[] sizes) {
        Size best = sizes[0];
        for (Size s : sizes) {
            long d = Math.abs((long) s.getWidth() * s.getHeight() - WANTED_PIXELS);
            long b = Math.abs((long) best.getWidth() * best.getHeight() - WANTED_PIXELS);
            if (d < b) best = s;
        }
        return best;
    }

    private void close() {
        try {
            if (session != null) session.close();
            if (device != null) device.close();
            if (reader != null) reader.close();
        } catch (RuntimeException ignored) {
        }
        session = null;
        device = null;
        reader = null;
        if (thread != null) thread.quitSafely();
        thread = null;
        handler = null;
    }
}
