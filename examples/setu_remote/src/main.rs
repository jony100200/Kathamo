//! Desktop preview of the phone controller (phone-sized window).
fn main() {
    kathamo::run_native(kathamo_setu_remote::SetuRemote::default()).expect("remote could not start");
}
