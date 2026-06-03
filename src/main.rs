use std::io::{self, Write};

fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let command = input.trim();
    if !command.is_empty() {
        println!("{}: command not found", command);
    }
}
