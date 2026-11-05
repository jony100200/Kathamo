//! Android lifecycle helpers.
//!
//! winit 0.30 drops Android's `onDestroy` notification (it only logs a TODO). But
//! `NativeActivity.onDestroy` blocks the Java main thread until the Rust
//! `android_main` thread returns. A Kathamo app that never leaves its event loop
//! therefore freezes the activity on Back or `finish()`, and Android reports "app
//! isn't responding". The runner asks the activity `isFinishing()` when its surface
//! goes away; if it is, the event loop exits and the process ends.
//!
//! stdout/stderr already go to logcat (tag `RustStdoutStderr`) through
//! android-activity, so `eprintln!` and panic messages are visible with
//! `adb logcat -s RustStdoutStderr`.

use std::ffi::c_void;
use std::ptr::{null, null_mut};

use jni_sys::{jclass, jobject, JNIEnv, JNINativeInterface_, JavaVM, JNI_OK};

use crate::native::AndroidApp;

/// True when the activity is closing for good (Back, `finish()`), false when it
/// is only going to the background or the check cannot be made.
pub(crate) fn activity_is_finishing(app: &AndroidApp) -> bool {
    // SAFETY: see `with_activity`.
    unsafe {
        with_activity(app, |env, activity, f, class| {
            let (Some(get_method), Some(call_bool)) = (f.GetMethodID, f.CallBooleanMethodA) else {
                return false;
            };
            let method = get_method(env, class, c"isFinishing".as_ptr(), c"()Z".as_ptr());
            !method.is_null() && call_bool(env, activity, method, null()) != 0
        })
    }
    .unwrap_or(false)
}

/// Asks Android to close the activity (`Activity.finish()`), as the Back button
/// does in a Java app. The surface then goes away, `activity_is_finishing`
/// turns true and the runner exits its loop.
pub(crate) fn finish_activity(app: &AndroidApp) {
    // SAFETY: see `with_activity`.
    unsafe {
        with_activity(app, |env, activity, f, class| {
            let (Some(get_method), Some(call_void)) = (f.GetMethodID, f.CallVoidMethodA) else {
                return;
            };
            let method = get_method(env, class, c"finish".as_ptr(), c"()V".as_ptr());
            if !method.is_null() {
                call_void(env, activity, method, null());
            }
        });
    }
}

/// Exits the process at once, without `atexit` handlers or C++ static
/// destructors. `std::process::exit` runs those, and on Android the GPU
/// driver's (EGL) destructors then lock mutexes the driver already destroyed,
/// which aborts the process with a tombstone instead of a clean exit.
pub(crate) fn exit_process_now(code: i32) -> ! {
    extern "C" {
        fn _exit(code: i32) -> !;
    }
    // SAFETY: `_exit` is a bionic libc function with no preconditions.
    unsafe { _exit(code) }
}

/// Runs `body` with a JNI env attached to this thread, the activity object, the
/// JNI function table and the activity's class. Pending Java exceptions are
/// cleared and the class ref released afterwards. `None` when JNI is unavailable.
///
/// SAFETY: `vm` and `activity` come from android-activity and stay valid while
/// the activity exists, which it does until the `android_main` thread returns.
/// Every JNI function pointer is checked before use.
unsafe fn with_activity<R>(
    app: &AndroidApp,
    body: impl FnOnce(*mut JNIEnv, jobject, &JNINativeInterface_, jclass) -> R,
) -> Option<R> {
    let vm = app.vm_as_ptr() as *mut JavaVM;
    let activity = app.activity_as_ptr() as jobject;
    if vm.is_null() || activity.is_null() {
        return None;
    }
    let attach = (**vm).AttachCurrentThread?;
    let mut env: *mut JNIEnv = null_mut();
    if attach(vm, &mut env as *mut *mut JNIEnv as *mut *mut c_void, null_mut()) != JNI_OK
        || env.is_null()
    {
        return None;
    }
    let f = &**env;
    let (Some(get_class), Some(check), Some(clear), Some(del)) =
        (f.GetObjectClass, f.ExceptionCheck, f.ExceptionClear, f.DeleteLocalRef)
    else {
        return None;
    };
    let class = get_class(env, activity);
    if class.is_null() {
        return None;
    }
    let result = body(env, activity, f, class);
    if check(env) != 0 {
        clear(env);
    }
    del(env, class);
    Some(result)
}
