package io.github.fizl0.hst;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;

/** Runs in its own process (":restart" in the manifest): ends the game's process and starts it again. */
public class Restart extends Activity {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        int pid = getIntent().getIntExtra("pid", 0);
        if (pid > 0) android.os.Process.killProcess(pid);
        startActivity(new Intent(this, HstActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK));
        finish();
        Runtime.getRuntime().exit(0);
    }
}
