# Quick Reference: Complete -C and -p Implementation

## What Was Added

### 1. **Global Completion Storage**
```rust
lazy_static::lazy_static! {
    static ref COMPLETIONS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}
```
- Thread-safe HashMap storing command → completer_path mappings
- Persists for the lifetime of the shell session

### 2. **Register Completions with `-C`**
```bash
complete -C /path/to/completer command_name
```
- **Effect**: Stores the completer script path
- **Output**: None (silent success)
- **Error**: If < 4 arguments

### 3. **Display Completions with `-p`**
```bash
complete -p command_name
```
- **Success Output**: `complete -C '/path/to/completer' command_name`
- **Error Output**: `complete: command_name: no completion specification`

## Example Workflow

```bash
# 1. Register git completer
$ complete -C /usr/bin/git-completer git
# (no output)

# 2. Register docker completer (with extra spaces)
$ complete   -C   /usr/bin/docker-completer   docker
# (no output - spaces are normalized internally)

# 3. Display git completion
$ complete -p git
complete -C '/usr/bin/git-completer' git

# 4. Display docker completion (normalized output)
$ complete -p docker
complete -C '/usr/bin/docker-completer' docker

# 5. Try to display unregistered completion
$ complete -p unknown
complete: unknown: no completion specification

# 6. Register a new one
$ complete -C /path/to/my-completer mycommand
# (no output)

# 7. Display the new one
$ complete -p mycommand
complete -C '/path/to/my-completer' mycommand
```

## Implementation Highlights

| Feature | Implementation |
|---------|-----------------|
| Storage | `HashMap<String, String>` in lazy_static |
| Thread Safety | `Mutex<>` wrapper |
| Registration | `-C path command` → insert into HashMap |
| Retrieval | `-p command` → get from HashMap |
| Output Format | `complete -C '<path>' <command>` |
| Quote Style | Single quotes around path |
| Spacing | Exactly one space between parts |
| Whitespace Handling | Automatically normalized |

## Files Modified

1. **Cargo.toml**
   - Added: `lazy_static = "1.4"`

2. **src/main.rs**
   - Added: Import `HashMap`
   - Added: Global `COMPLETIONS` HashMap
   - Modified: `complete` command handler

## Edge Cases Handled

✅ Extra whitespace in registration command
✅ Multiple registrations for different commands
✅ Overwriting existing registrations
✅ Querying unregistered commands
✅ Missing arguments to `-C` or `-p`

## Output Format Guarantee

Input (with extra spaces):
```
complete   -C    /very/long/path/to/completer    mycommand
```

Output (normalized):
```
complete -C '/very/long/path/to/completer' mycommand
```

Notice:
- Single quotes around path
- Exactly one space between: `complete`, `-C`, `'path'`, and `mycommand`
- No matter the input spacing, output is always consistent
