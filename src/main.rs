mod adb;

fn main() {
    let mut ok = adb::Adb::new().unwrap();
    println!("{:?}", ok.devices().unwrap());
}


// fn main() {
//     let app = Application::builder()
//         .application_id("com.adb_gui.XtremeTHN")
//         .build();
    
//     app.connect_startup(|_| {
//         adw::init();
//     });

//     app.connect_activate(|app: &Application| {
//         let window = ApplicationWindow::builder()
//             .default_height(800)
//             .default_width(600)
//             .title("AdbGUI")
//             .application(app)
//             .build();

//         let layout = Box::builder()
//             .orientation(gtk::Orientation::Vertical)
//             .margin_bottom(15)
//             .margin_end(15)
//             .margin_start(15)
//             .margin_top(15)
//             .build();

//         let label1 = Label::new(Some("Waiting for any device..."));

//         window.set_content(Some(&layout));
//         window.present();
//     });

//     app.run();
// }