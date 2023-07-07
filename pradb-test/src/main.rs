extern crate pradb;

fn main() {
    let mut obj = pradb::pradb::Adb::new().unwrap();
    println!("{:?}", obj.devices());
}