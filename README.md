# Winbang

Winbang provides Nix-like shebang support for Windows. This program makes it possible to execute scripts without needing to specify the interpreter in the command line and without the need for file extensions. It uses the shebang line to determine the interpreter to use, and it can also handle file associations based on file extensions.

When run from a command prompt, it will always attempt to execute the script.
When run from a GUI such as `explorer.exe`, it will by default prompt the user for an action.

This program is extensible and can be configured to associate files by extension or by shebang line, and allowing for shebang runtimes to effectively be proxied. Such as `#!/bin/bash` being proxied to `C:\msys64\msys2_shell.cmd` with the appropriate arguments.

> **NOTICE**
>
> If a file association does not exist in the config it will attempt to find the interpreter in `PATH`.
> A prompt for action is shown by default when launched from a GUI shell, disabling this behavior could lead to an increased security risk. The user is responsible for ensuring that files are correct and safe to execute.

`env` shebangs are supported, but `env` itself is ignored and the optional argument is used as the interpreter. For example, `#!/usr/bin/env python3` will be interpreted as `python3`.

## Setup

**EXAMPLE CONFIG**

`%PROGRAMDATA%/Winbang/config.toml` or `%APPDATA%/Winbang/config.toml`

The configuration files will not merge, only one will be used. If `allow_user_config` is set to true in the `%PROGRAMDATA%` config, then the user config will be used. If not, the `%PROGRAMDATA%` config will be used.

```toml
# allow_user_config = true              # Optional, default is false, only valid in %PROGRAMDATA% config.
# List of GUI shells to use when launching files in GUI mode
gui_shells = ["explorer.exe", "dopus.exe"]

# Default operation if no file association matches
default_operation = "prompt"            # Optional, default: "prompt", only affects when launched via GUI.

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
# exec_runtime = "deno"                 # Required
# view_runtime = "code"                 # Optional
# shebang_interpreter = "deno"          # Optional
# extension = ".ts"                     # Optional
# exec_argv_override = "run @{script}"  # Optional

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
exec_runtime = "C:\\msys64\\msys2_shell.cmd"
view_runtime = "code"
shebang_interpreter = "bash"
extension = "sh"
exec_argv_override = "-defterm -here -no-start -ucrt64 -shell bash -c \"$(cygpath -u @{script_unix})\""

[[file_associations]]
exec_runtime = "zsh"
shebang_interpreter = "zsh"
extension = "sh"
default_operation = "execute"

[[file_associations]]
exec_runtime = "deno"
extension = "ts"
shebang_interpreter = "deno"
exec_argv_override = "run -A @{script}"

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

`exec_argv_override` will expand the following special variables:
- `@{script}`: The full script file path with double-backslashes (e.g., `C:\\Users\\username\\test.sh`).
- `@{script_unix}`: The script file path with forward slashes (e.g., `C:/Users/username/test.sh`).

**EXTENSIONLESS FILE ASSOCIATION**

This step is needed to unlock the full potential of this application. Allowing for a Nix like file experience making file extensions much less important and taking advantage of standard shebang lines and shebang-like lines.

1. Run in an elevated command prompt:
```batch
assoc .="No Extension"
ftype "No Extension"=^"^%PROGRAMFILES%\Winbang\winbang.exe^" "%1"
assoc "No Extension"\DefaultIcon=%SystemRoot%\System32\imageres.dll,-68
```

`DefaultIcon` `imageres.dll` indexes:
- `15` application icon
- `68` script icon
- `102` file icon

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