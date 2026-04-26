use rust_sample::{greet, add, User};

fn main() {
    println!("{}", greet("Rust"));
    println!("1 + 2 = {}", add(1, 2));

    let user = User::new("Alice", 30);
    println!("{}", user.info());
}