# Winbang

Winbang provides Nix-like shebang support for Windows.

## Setup

**EXAMPLE CONFIG**

`%PROGRAMDATA%/Winbang/config.toml` or `%APPDATA%/Winbang/config.toml`
```toml
# allow_user_config = true            # Optional, default is false, only valid in higher configs
# List of GUI shells to use when launching files in GUI mode
gui_shells = ["explorer.exe", "dopus.exe"]

# Default operation if no file association matches
default_operation = "prompt"

[default]
# Viewer used for regular files
view_runtime = "code"
#args = "$script"

[default_large]
# Viewer used for large files (>= 5MB)
size_mb_threshold = 5
view_runtime = "notepad++"
#args = "$script"

# [[file_associations]]
# exec_runtime = "deno"               # Required
# view_runtime = "code"               # Optional
# shebang_interpreter = "deno"        # Optional
# extension = ".ts"                   # Optional
# exec_argv_override = "run $script"  # Optional

# File associations
[[file_associations]]
exec_runtime = "python"
view_runtime = "thonny"
shebang_interpreter = "python"
extension = "py"

[[file_associations]]
exec_runtime = "ruby"
view_runtime = "code"
shebang_interpreter = "ruby"
extension = "rb"
default_operation = "prompt"

[[file_associations]]
exec_runtime = "bash"
shebang_interpreter = "bash"
extension = "sh"
default_operation = "execute"

[[file_associations]]
exec_runtime = "zsh"
shebang_interpreter = "zsh"
extension = "sh"
default_operation = "execute"

[[file_associations]]
exec_runtime = "deno"
extension = "ts"
shebang_interpreter = "deno"
exec_argv_override = "run -A $script"

[[file_associations]]
exec_runtime = "node"
shebang_interpreter = "node"
extension = "js"

[[file_associations]]
exec_runtime = "perl"
view_runtime = "runemacs"
shebang_interpreter = "perl"
extension = "pl"
```

**EXTENSIONLESS FILE ASSOCIATION**

This step is needed to unlock the full potential of this application. Allowing for a Nix like file experience making file extensions much less important and taking advantage of standard shebang lines and shebang-like lines.

1. Run in an elevated command prompt:
```batch
assoc .="No Extension"
ftype "No Extension"=^"^%PROGRAMFILES%\Winbang\winbang.exe^" "%1"
assoc "No Extension"\DefaultIcon=%SystemRoot%\System32\imageres.dll,-102
```

2. Restart computer.

## Example/Test Files

**Deno Script**

```typescript
#!/usr/bin/env deno

async function main() {
    console.log("Hello from Deno!");

    const fileName = "test.txt";

    try {
        await Deno.writeTextFile(fileName, "Hello from Deno!");
        console.log(`File ${fileName} created successfully.`);
    }
    catch (error) {
        console.error(`Failed to create file ${fileName}:`, error);
    }

    const promptExit = prompt("Press Enter to exit...");
}

main();
```

**Ruby Script**

```ruby
#!/usr/bin/env ruby
puts "Hello from Ruby!"

File.open("test.txt", "w") do |file|
  file.write("Hello from Ruby!")
end

puts "Press Enter to exit..."
STDIN.gets
```