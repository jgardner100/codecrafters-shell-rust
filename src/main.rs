use std::io::Write;
use std::process;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// --- Rustyline Imports ---
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

// Define our helper struct
struct ShellHelper;

// Manually implement the traits instead of deriving them
impl Helper for ShellHelper {}
impl Hinter for ShellHelper {
    type Hint = String;
}
impl Highlighter for ShellHelper {}
impl Validator for ShellHelper {}

// Keep your existing Completer implementation
impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let builtins = vec!["echo ", "exit "];
        let mut candidates = Vec::new();
        let slice = &line[..pos];
        
        if !slice.contains(' ') {
            for builtin in &builtins {
                if builtin.starts_with(slice) {
                    candidates.push(Pair {
                        display: builtin.trim_end().to_string(),
                        replacement: builtin.to_string(),
                    });
                }
            }
        }
        
        Ok((0, candidates))
    }
}

fn is_builtin(cmd: &str) -> bool {
    matches!(cmd, "echo" | "exit" | "type" | "pwd" | "cd")
}

fn find_executable_in_path(command: &str) -> Option<String> {
    if let Ok(path_var) = env::var("PATH") {
        let path_delimiter = if cfg!(windows) { ";" } else { ":" };
        for dir in path_var.split(path_delimiter) {
            let full_path = Path::new(dir).join(command);
            if full_path.exists() && is_executable(&full_path) {
                return Some(full_path.to_string_lossy().to_string());
            }
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let mode = metadata.permissions().mode();
            (mode & 0o111) != 0
        } else {
            false
        }
    }
    #[cfg(windows)]
    {
        path.exists()
    }
}

#[derive(Debug, Clone)]
struct Redirection {
    stdout_target: Option<(String, bool)>,
    stderr_target: Option<(String, bool)>,
}

fn parse_with_redirection(input: &str) -> (Vec<String>, Redirection) {
    let tokens = parse_command_with_quotes(input);
    let mut command_parts = Vec::new();
    let mut redirection = Redirection {
        stdout_target: None,
        stderr_target: None,
    };
    let mut i = 0;
    
    while i < tokens.len() {
        let token = &tokens[i];
        if token == ">>" || token == "1>>" {
            if i + 1 < tokens.len() {
                redirection.stdout_target = Some((tokens[i + 1].clone(), true));
                i += 2;
            } else {
                command_parts.push(token.clone());
                i += 1;
            }
        } else if token == ">" || token == "1>" {
            if i + 1 < tokens.len() {
                redirection.stdout_target = Some((tokens[i + 1].clone(), false));
                i += 2;
            } else {
                command_parts.push(token.clone());
                i += 1;
            }
        } else if token == "2>>" {
            if i + 1 < tokens.len() {
                redirection.stderr_target = Some((tokens[i + 1].clone(), true));
                i += 2;
            } else {
                command_parts.push(token.clone());
                i += 1;
            }
        } else if token == "2>" {
            if i + 1 < tokens.len() {
                redirection.stderr_target = Some((tokens[i + 1].clone(), false));
                i += 2;
            } else {
                command_parts.push(token.clone());
                i += 1;
            }
        } else {
            command_parts.push(token.clone());
            i += 1;
        }
    }
    (command_parts, redirection)
}

fn execute_external_program(cmd: &str, args: &[String], redirection: Redirection) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(program_path) = find_executable_in_path(cmd) {
        let mut command = process::Command::new(&program_path);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.arg0(cmd);
        }
        for arg in args {
            command.arg(arg);
        }
        if let Some((filename, is_append)) = &redirection.stdout_target {
            let file = fs::OpenOptions::new().create(true).append(*is_append).write(!is_append).truncate(!is_append).open(filename)?;
            command.stdout(file);
        }
        if let Some((filename, is_append)) = &redirection.stderr_target {
            let file = fs::OpenOptions::new().create(true).append(*is_append).write(!is_append).truncate(!is_append).open(filename)?;
            command.stderr(file);
        }
        let mut child = command.spawn()?;
        child.wait()?;
        Ok(())
    } else {
        Err(format!("{}: command not found", cmd).into())
    }
}

fn resolve_relative_path(target_dir: &str) -> PathBuf {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let mut path = current_dir.clone();
    for component in target_dir.split('/') {
        match component {
            "" | "." => {}
            ".." => { path.pop(); }
            _ => { path.push(component); }
        }
    }
    path
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Ok(home) = env::var("HOME") {
            if path == "~" { home }
            else if path.starts_with("~/") { format!("{}{}", home, &path[1..]) }
            else { path.to_string() }
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    }
}

fn parse_command_with_quotes(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_single_quotes = false;
    let mut in_double_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double_quotes => { in_single_quotes = !in_single_quotes; }
            '"' if !in_single_quotes => { in_double_quotes = !in_double_quotes; }
            '\\' if in_double_quotes => {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '"' | '\\' | '$' | '`' => {
                            chars.next();
                            current_arg.push(next_ch);
                        }
                        '\n' => { chars.next(); }
                        _ => { current_arg.push('\\'); }
                    }
                } else {
                    current_arg.push('\\');
                }
            }
            '\\' if !in_single_quotes && !in_double_quotes => {
                if let Some(next_ch) = chars.next() {
                    current_arg.push(next_ch);
                }
            }
            ' ' | '\t' => {
                if in_single_quotes || in_double_quotes {
                    current_arg.push(ch);
                } else if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            _ => { current_arg.push(ch); }
        }
    }
    if !current_arg.is_empty() {
        args.push(current_arg);
    }
    args
}

fn main() {
    // Initialize rustyline with our custom ShellHelper
    let mut rl = Editor::<ShellHelper, _>::new().unwrap();
    rl.set_helper(Some(ShellHelper));

    loop {
        // Read user input via rustyline (handles prompt and tab-completion automatically)
        let readline = rl.readline("$ ");
        match readline {
            Ok(line) => {
                let command = line.trim();
                if command.is_empty() {
                    continue;
                }
                
                let (parts, redirection) = parse_with_redirection(command);
                if parts.is_empty() {
                    continue;
                }
                
                let cmd = &parts[0];
                
                if cmd == "exit" {
                    process::exit(0);
                }
                else if cmd == "echo" {
                    let args = &parts[1..];
                    let output = args.join(" ");
                    
                    if let Some((stderr_filename, is_append)) = &redirection.stderr_target {
                        let _ = fs::OpenOptions::new().create(true).append(*is_append).write(!is_append).truncate(!is_append).open(stderr_filename);
                    }
                    
                    if let Some((filename, is_append)) = &redirection.stdout_target {
                        let result = fs::OpenOptions::new().create(true).append(*is_append).write(!is_append).truncate(!is_append).open(filename);
                        match result {
                            Ok(mut file) => { let _ = writeln!(file, "{}", output); }
                            Err(e) => { eprintln!("echo: {}: {}", filename, e); }
                        }
                    } else {
                        println!("{}", output);
                    }
                }
                else if cmd == "type" {
                    if parts.len() < 2 {
                        println!("type: missing argument");
                        continue;
                    }
                    let target_cmd = &parts[1];
                    if is_builtin(target_cmd) {
                        println!("{} is a shell builtin", target_cmd);
                    } else if let Some(full_path) = find_executable_in_path(target_cmd) {
                        println!("{} is {}", target_cmd, full_path);
                    } else {
                        println!("{}: not found", target_cmd);
                    }
                }
                else if cmd == "pwd" {
                    match env::current_dir() {
                        Ok(path) => {
                            let output = format!("{}", path.display());
                            if let Some((stderr_filename, is_append)) = &redirection.stderr_target {
                                let _ = fs::OpenOptions::new().create(true).append(*is_append).write(!is_append).truncate(!is_append).open(stderr_filename);
                            }
                            if let Some((filename, is_append)) = &redirection.stdout_target {
                                let result = fs::OpenOptions::new().create(true).append(*is_append).write(!is_append).truncate(!is_append).open(filename);
                                match result {
                                    Ok(mut file) => { let _ = writeln!(file, "{}", output); }
                                    Err(e) => { eprintln!("pwd: {}: {}", filename, e); }
                                }
                            } else {
                                println!("{}", output);
                            }
                        }
                        Err(e) => { eprintln!("pwd: {}", e); }
                    }
                }
                else if cmd == "cd" {
                    if parts.len() < 2 {
                        eprintln!("cd: missing argument");
                        continue;
                    }
                    let target_dir = &parts[1];
                    let expanded_target = expand_tilde(target_dir);
                    let path = if expanded_target.starts_with('/') {
                        PathBuf::from(&expanded_target)
                    } else {
                        resolve_relative_path(&expanded_target)
                    };
                    
                    if path.exists() && path.is_dir() {
                        if let Err(e) = env::set_current_dir(&path) {
                            eprintln!("cd: {}: {}", target_dir, e);
                        }
                    } else {
                        eprintln!("cd: {}: No such file or directory", target_dir);
                    }
                }
                else {
                    let args = parts[1..].to_vec();
                    if let Err(e) = execute_external_program(cmd, &args, redirection) {
                        eprintln!("{}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
}
