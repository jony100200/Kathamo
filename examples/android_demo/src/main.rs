//! Desktop preview of the phone demo. `KATHAMO_SNAPSHOT=out.png` writes a frame instead.
fn main() {
    kathamo::run_native(kathamo_android_demo::MobileDemo::default()).expect("demo could not start");
}
