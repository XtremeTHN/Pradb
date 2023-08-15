fn main() {
    let string = String::from("Hello, world!");
    println!("{:?}", string.split(',').collect::<String>());
}
