package io.github.fizl0.hst;

import android.app.NativeActivity;
import android.content.Intent;
import android.net.wifi.WifiManager;
import android.os.Build;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.view.WindowManager;

/** The game's NativeActivity (`tools/apk.sh`), plus what the native side can't do on its own (M99): the system's file
 *  picker for the disc image, and a restart once the disc is imported. Called from `crates/hst-base/src/android.rs`. */
public class HstActivity extends NativeActivity {
    private static final int PICK = 1;
    /** The picked document's file descriptor (the native side owns it), -1 when the picker was cancelled, -2 when
     *  there's nothing new. */
    private volatile int picked = -2;
    /** B455: Wi-Fi drops multicast an app hasn't asked for, so mDNS (the LAN's lobbies, `net.rs` `watch_lan`) heard
     *  nothing: held while the game is in front. */
    private WifiManager.MulticastLock lan;

    /** M106: the window reaches under the camera cutout on every Android, as it must from Android 15 on; the game keeps
     *  its picture inside the safe area itself (`android.rs` `safe_area`, `widescreen.rs`). */
    @Override
    protected void onCreate(Bundle state) {
        if (Build.VERSION.SDK_INT >= 28) {
            WindowManager.LayoutParams a = getWindow().getAttributes();
            a.layoutInDisplayCutoutMode = Build.VERSION.SDK_INT >= 30
                ? WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
                : WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            getWindow().setAttributes(a);
        }
        super.onCreate(state);
        try {
            WifiManager w = (WifiManager) getApplicationContext().getSystemService(WIFI_SERVICE);
            if (w != null) {
                lan = w.createMulticastLock("hst-lan");
                lan.setReferenceCounted(false);
            }
        } catch (Exception e) {
            lan = null;
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        try {
            if (lan != null) lan.acquire();
        } catch (Exception e) {
            // no CHANGE_WIFI_MULTICAST_STATE: the LAN list stays empty, the internet's still works
        }
    }

    @Override
    protected void onPause() {
        if (lan != null && lan.isHeld()) lan.release();
        super.onPause();
    }

    /** Opens the system's document picker (any file: a disc image has no MIME type of its own). */
    public void pick() {
        runOnUiThread(new Runnable() {
            public void run() {
                Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                i.addCategory(Intent.CATEGORY_OPENABLE);
                i.setType("*/*");
                startActivityForResult(i, PICK);
            }
        });
    }

    /** The picker's answer since the last call (see `picked`), then -2 again. */
    public int taken() {
        int fd = picked;
        if (fd != -2) picked = -2;
        return fd;
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        if (request != PICK) {
            super.onActivityResult(request, result, data);
            return;
        }
        int fd = -1;
        if (result == RESULT_OK && data != null && data.getData() != null) {
            try {
                ParcelFileDescriptor p = getContentResolver().openFileDescriptor(data.getData(), "r");
                if (p != null) fd = p.detachFd();
            } catch (Exception e) {
                fd = -1;
            }
        }
        picked = fd;
    }

    /** A fresh process for the game: the native side's window can only be made once per process. */
    public void restart() {
        startActivity(new Intent(this, Restart.class).putExtra("pid", android.os.Process.myPid()));
    }
}
