# Detailed Changes Diff

## File: `Cargo.toml`

```diff
  [dependencies]
  anyhow = "1.0.68"                                # error handling
  bytes = "1.3.0"                                  # helps manage buffers
  thiserror = "1.0.38"                             # error handling
  rustyline = "14.0.0"                             # Cmdline editing
+ lazy_static = "1.4"                              # global mutable state
```

## File: `src/main.rs`

### Change 1: Updated Imports (Line 6)

```diff
- use std::collections::HashSet;
+ use std::collections::{HashSet, HashMap};
```

### Change 2: Added Global Completion Storage (Lines 10-13)

```diff
+ // Global storage for registered completions
+ lazy_static::lazy_static! {
+     static ref COMPLETIONS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
+ }
```

### Change 3: Updated Complete Command Handler (Lines 612-662)

**OLD CODE:**
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

**NEW CODE:**
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

## Summary of Changes

| Component | Type | Details |
|-----------|------|---------|
| Cargo.toml | Added Dependency | `lazy_static = "1.4"` |
| src/main.rs | Import | Added `HashMap` to imports |
| src/main.rs | Global State | New `COMPLETIONS` lazy_static HashMap |
| src/main.rs | Handler Logic | Added `-C` flag registration logic |
| src/main.rs | Handler Logic | Enhanced `-p` flag with retrieval logic |

## Functionality Additions

1. **Registration (`-C` flag)**
   - Accepts: `complete -C <path> <command>`
   - Stores completion script path for a command
   - No output on success
   - Handles extra whitespace gracefully

2. **Display (`-p` flag)**
   - Accepts: `complete -p <command>`
   - Retrieves and displays registered completions
   - Format: `complete -C '<path>' <command>`
   - Returns error if not registered
   - Normalizes all spacing to single spaces

## Backward Compatibility

✅ All existing functionality preserved
✅ Error handling for unregistered commands unchanged
✅ All other shell commands work as before
