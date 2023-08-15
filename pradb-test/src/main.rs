extern crate pradb;
use std::path::PathBuf;
use std::process::exit;
fn main() {
    env_logger::init();
    let mut obj = pradb::pradb::Adb::new().unwrap();
    let dev_res = obj.devices();
    if dev_res.is_err() {
        log::error!("Couldnt get devices");
        exit(1);
    }
    let mut devices = dev_res.unwrap();
    let emu = devices.get_mut(0).unwrap();
    emu.use_device();

    let package_path = PathBuf::from("package.apk");
    println!("{:?}", emu.install_package(package_path));
}
