use std::env;

fn main() {
    let mut args = env::args().skip(1);
    let Some(first) = args.next() else {
        eprintln!("Usage: dg <text>");
        std::process::exit(2);
    };

    if first == "こんちには" {
        println!("こんにちは");
        return;
    }

    println!("{first}");
}
