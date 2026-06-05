use std::io::{self, Write};
use std::process;

fn main() {
    loop {
        // Display the prompt
        print!("$ ");
        io::stdout().flush().unwrap();

        // Read user input
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                // EOF reached (Ctrl+D), exit gracefully
                break;
            }
            Ok(_) => {
                // Parse and execute the command
                let command = input.trim();
                if !command.is_empty() {
                    // Check for exit builtin
                    if command == "exit" {
                        process::exit(0);
                    } else {
                        // Command not found
                        println!("{}: command not found", command);
                    }
                }
            }
            Err(_) => {
                // Error reading input, exit
                break;
            }
        }
    }
}
