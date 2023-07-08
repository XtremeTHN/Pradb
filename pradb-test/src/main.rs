extern crate pradb;
use std::process::exit;
fn main() {
    env_logger::init();
    let mut obj = pradb::pradb::Adb::new().unwrap();
    let dev_res = obj.devices();
    if dev_res.is_err() {
        log::error!("Couldnt get devices");
        exit(1);
    }
    let devices = dev_res.unwrap();
    for dev in devices {
        println!("Serial number: {}", dev.getserial_no());
    }
}
