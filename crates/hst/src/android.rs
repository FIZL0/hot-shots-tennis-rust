//! M102: what the Android app (`tools/apk.sh`, NativeActivity) asks the Java side for, through JNI: the window's safe
//! area (system bars and the camera cutout, which an edge-to-edge window draws under) and the on-screen pad's haptic
//! tick (`play/touch_pad.rs`).

use bevy::log::warn;
use jni::objects::{JObject, JValue};
use jni::{JNIEnv, JavaVM};
use std::sync::OnceLock;

/// The activity's VM, attached to the calling thread for good (the pad's systems run on a few pool threads).
fn with_env<T>(f: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<T>) -> Option<T> {
    static VM: OnceLock<Option<JavaVM>> = OnceLock::new();
    let app = bevy::android::ANDROID_APP.get()?;
    // SAFETY: the activity's VM pointer, valid for the process
    let vm = VM.get_or_init(|| unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()).as_ref()?;
    let mut env = vm.attach_current_thread_permanently().ok()?;
    // SAFETY: the NativeActivity's global reference, alive as long as the activity
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
    // a frame for the local references: a permanently attached thread never returns to Java to free them
    let out = env.with_local_frame(16, |env| f(env, &activity));
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    out.map_err(|e| warn!("android: JNI call failed: {e}")).ok()
}

fn decor<'a>(env: &mut JNIEnv<'a>, activity: &JObject) -> jni::errors::Result<JObject<'a>> {
    let window = env.call_method(activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
    env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()
}

/// The window's insets, left, top, right, bottom, in physical px: the visible system bars and the display cutout
/// (0 where the window doesn't reach under them).
pub fn safe_area() -> Option<[f32; 4]> {
    with_env(|env, activity| {
        let decor = decor(env, activity)?;
        let insets = env.call_method(&decor, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?.l()?;
        if insets.is_null() {
            return Ok([0.0; 4]);
        }
        let sdk = env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()?;
        let side = |env: &mut JNIEnv, o: &JObject, name: &str| env.get_field(o, name, "I").and_then(|v| v.i());
        if sdk >= 30 {
            let bars = env.call_static_method("android/view/WindowInsets$Type", "systemBars", "()I", &[])?.i()?;
            let cutout = env.call_static_method("android/view/WindowInsets$Type", "displayCutout", "()I", &[])?.i()?;
            let i = env.call_method(&insets, "getInsets", "(I)Landroid/graphics/Insets;", &[JValue::Int(bars | cutout)])?.l()?;
            Ok([side(env, &i, "left")?, side(env, &i, "top")?, side(env, &i, "right")?, side(env, &i, "bottom")?].map(|v| v as f32))
        } else {
            // before 30: the system windows' insets, which take in the cutout once the window lays out into it
            let mut v = [0.0; 4];
            for (v, m) in v.iter_mut().zip(["getSystemWindowInsetLeft", "getSystemWindowInsetTop", "getSystemWindowInsetRight", "getSystemWindowInsetBottom"]) {
                *v = env.call_method(&insets, m, "()I", &[])?.i()? as f32;
            }
            Ok(v)
        }
    })
}

/// M102: the light haptic tick of an on-screen pad press, as a key's (the phone's own touch-feedback setting holds too).
pub fn haptic() {
    with_env(|env, activity| {
        let decor = decor(env, activity)?;
        // HapticFeedbackConstants.VIRTUAL_KEY
        env.call_method(&decor, "performHapticFeedback", "(I)Z", &[JValue::Int(1)]).map(drop)
    });
}
