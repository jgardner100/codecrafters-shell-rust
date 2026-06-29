# Changes Summary: Adding -C and -p Completion Support

## Overview
Modified `src/main.rs` to add support for registering and displaying command completions using the `-C` and `-p` flags.

## Key Changes

### 1. **Added Global Completion Storage**
```rust
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref COMPLETIONS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}
```
- Uses `lazy_static` crate for thread-safe global mutable state
- Stores completions as a HashMap mapping command names to completer script paths

### 2. **Updated Cargo.toml**
- Added `lazy_static = "1.4"` dependency for global state management

### 3. **Enhanced `complete` Command Handler**
The code now handles three scenarios:

#### Scenario A: Register Completion with `-C` flag
```rust
if parts[1] == "-C" {
    if parts.len() < 4 {
        eprintln!("complete: -C: option requires an argument");
        continue;
    }
    
    let completer_path = &parts[2];
    let command_name = &parts[3];
    
    // Register the completion
    let mut completions = COMPLETIONS.lock().unwrap();
    completions.insert(command_name.clone(), completer_path.clone());
    
    // The -C flag produces no output on success
}
```
- Accepts: `complete -C <path> <command>`
- Stores the path and command name
- Produces no output on success

#### Scenario B: Display Completion with `-p` flag
```rust
else if parts[1] == "-p" {
    if parts.len() < 3 {
        eprintln!("complete: -p: option requires an argument");
        continue;
    }
    
    let command_name = &parts[2];
    let completions = COMPLETIONS.lock().unwrap();
    
    if let Some(completer_path) = completions.get(command_name) {
        // Print in normalized format: complete -C '<path>' <command>
        println!("complete -C '{}' {}", completer_path, command_name);
    } else {
        // Print error message for no completion specification
        eprintln!("complete: {}: no completion specification", command_name);
    }
}
```
- Accepts: `complete -p <command>`
- Outputs normalized format: `complete -C '<path>' <command>` if registered
- Outputs error message if not registered
- Path is wrapped in single quotes
- Arguments separated by exactly one space

## Features

✅ **Register completions** with `-C flag`, handling extra whitespace
✅ **Display completions** with `-p flag` in normalized format
✅ **Proper error handling** when completion not found
✅ **Single quotes** around script path in output
✅ **Single space separation** in output format
✅ **No output** on successful registration

## Example Usage

```bash
# Register a completer
$ complete -C /path/to/git/completer git

# Display registered completer (normalized output)
$ complete -p git
complete -C '/path/to/git/completer' git

# Try to display unregistered completer
$ complete -p unknown
complete: unknown: no completion specification
```

## Behavior with Extra Whitespace

Input:
```bash
complete   -C    /path/to/completer    mycommand
```

Output:
```bash
$ complete -p mycommand
complete -C '/path/to/completer' mycommand
```

The extra whitespace in the input is normalized to single spaces in the output.
