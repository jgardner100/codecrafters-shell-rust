# Complete Code Sections - Before and After

## Section 1: Imports and Global State

### BEFORE
```rust
use std::io::Write;
use std::process;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::Mutex;
use rustyline::config::CompletionType;
```

### AFTER
```rust
use std::io::Write;
use std::process;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};  // ← Added HashMap
use std::sync::Mutex;
use rustyline::config::CompletionType;

// Global storage for registered completions        ← NEW
lazy_static::lazy_static! {                         ← NEW
    static ref COMPLETIONS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());  ← NEW
}                                                   ← NEW
```

---

## Section 2: Complete Command Handler

### BEFORE (Lines ~540-560)
```rust
} else if cmd == "complete" {
    // Handle the complete builtin command
    if parts.len() < 2 {
        // No arguments provided to complete
        continue;
    }
    
    // Check if -p flag is provided
    if parts[1] == "-p" {
        // -p flag requires a command name
        if parts.len() < 3 {
            eprintln!("complete: -p: option requires an argument");
            continue;
        }
        
        let command_name = &parts[2];
        // Print the error message for no completion specification
        eprintln!("complete: {}: no completion specification", command_name);
    }
    // For other cases or future flags, we can add more handling here
}
```

### AFTER (Lines ~612-662)
```rust
} else if cmd == "complete" {
    // Handle the complete builtin command
    if parts.len() < 2 {
        // No arguments provided to complete
        continue;
    }
    
    // Check if -C flag is provided (register completion)
    if parts[1] == "-C" {
        // -C flag requires at least: complete -C <path> <command>
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
    // Check if -p flag is provided (display completion)
    else if parts[1] == "-p" {
        // -p flag requires a command name
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
}
```

---

## Section 3: Cargo.toml Dependencies

### BEFORE
```toml
[dependencies]
anyhow = "1.0.68"                                # error handling
bytes = "1.3.0"                                  # helps manage buffers
thiserror = "1.0.38"                             # error handling
rustyline = "14.0.0"                             # Cmdline editing
```

### AFTER
```toml
[dependencies]
anyhow = "1.0.68"                                # error handling
bytes = "1.3.0"                                  # helps manage buffers
thiserror = "1.0.38"                             # error handling
rustyline = "14.0.0"                             # Cmdline editing
lazy_static = "1.4"                              # global mutable state
```

---

## Key Differences Highlighted

### Addition 1: Global State
**Purpose**: Persist completion registrations  
**Type**: Thread-safe mutable HashMap  
**Scope**: Application lifetime  
```rust
lazy_static::lazy_static! {
    static ref COMPLETIONS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}
```

### Addition 2: Registration Handler
**Trigger**: `parts[1] == "-C"`  
**Action**: Store completer path  
**Output**: None (silent success)  
```rust
if parts[1] == "-C" {
    let completer_path = &parts[2];
    let command_name = &parts[3];
    let mut completions = COMPLETIONS.lock().unwrap();
    completions.insert(command_name.clone(), completer_path.clone());
}
```

### Addition 3: Display Handler Enhancement
**Trigger**: `parts[1] == "-p"`  
**Action**: Retrieve and display completer  
**Output**: Normalized format or error  
```rust
else if parts[1] == "-p" {
    let command_name = &parts[2];
    let completions = COMPLETIONS.lock().unwrap();
    if let Some(completer_path) = completions.get(command_name) {
        println!("complete -C '{}' {}", completer_path, command_name);
    } else {
        eprintln!("complete: {}: no completion specification", command_name);
    }
}
```

---

## Line Count Summary

| File | Before | After | Change |
|------|--------|-------|--------|
| Cargo.toml | 8 | 9 | +1 line |
| src/main.rs | ~540 | ~662 | +122 lines |
| **Total** | **548** | **671** | **+123 lines** |

---

## Functional Flow Diagram

```
User Input: "complete -C /path/to/completer git"
                    ↓
         parse_command_with_quotes()
                    ↓
         parts = ["complete", "-C", "/path/to/completer", "git"]
                    ↓
         cmd == "complete" && parts[1] == "-C" ?
                    ↓ YES
    completer_path = "/path/to/completer"
    command_name = "git"
    COMPLETIONS.lock().insert("git", "/path/to/completer")
                    ↓
         (no output - silent success)


User Input: "complete -p git"
                    ↓
         parse_command_with_quotes()
                    ↓
         parts = ["complete", "-p", "git"]
                    ↓
         cmd == "complete" && parts[1] == "-p" ?
                    ↓ YES
    command_name = "git"
    COMPLETIONS.lock().get("git")
                    ↓ FOUND
    println!("complete -C '/path/to/completer' git")
```

---

## Thread Safety Mechanism

```rust
COMPLETIONS: Mutex<HashMap<String, String>>
     │
     ├─ Mutex: Ensures only one thread accesses at a time
     │
     └─ HashMap<String, String>:
         ├─ Key: Command name ("git", "docker", etc.)
         └─ Value: Completer path ("/usr/bin/git-completer", etc.)

Operation Flow:
1. Lock mutex: let mut completions = COMPLETIONS.lock().unwrap();
2. Access HashMap: completions.insert(...) or completions.get(...)
3. Auto-unlock when completions goes out of scope
```
