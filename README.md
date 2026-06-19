# Winbang

Winbang adds Unix-like [shebang](https://en.wikipedia.org/wiki/Shebang_(Unix))
support to Windows, Allowing scripts to run without specifying an interpreter or
requiring file extensions. It selects the interpreter from the shebang and can
also use file-extension associations when present.

## Installation

Cargo

```powershell
cargo install winbang
```

## Setup

To get the full benefits of Winbang, the following is required:

1. Manually associate desired filetypes to `winbang.exe` (ideally `.sh`, `.zsh`,
   `.py`, and other common script extensions.)
2. Follow **Extensionless File Association** instructions
3. Set up a config file (optional, recommended)

### Extensionless File Association

This unlocks the true potential of Winbang. Allowing for a Unix-like file
experience making file extensions much less important and taking advantage of
standard shebang lines and shebang-like lines.

1. Run in an **elevated** command prompt:

   > **NOTICE**
   >
   > `winbang.exe` path may need adjustment based on your installation.

```batch
assoc .="No Extension"
ftype "No Extension"=^"^%USERPROFILE%\.cargo\bin\winbang.exe^" "%1"
assoc "No Extension"\DefaultIcon=%SystemRoot%\System32\imageres.dll,-68
```

`DefaultIcon` `imageres.dll` indexes:

- `15` application icon
- `68` script icon
- `102` file icon

2. Restart computer.

## Behavior

When invoked from a command prompt, Winbang always executes the script. When
invoked from a GUI context such as `explorer.exe`, it prompts the user for an
action by default.

Winbang is extensible and supports file association by extension or shebang.
Shebang runtimes can be proxied, for example, mapping `#!/bin/bash` to
`C:\msys64\msys2_shell.cmd` with appropriate arguments.

If no matching association exists in the configuration, Winbang searches for the
interpreter in `PATH`.

`env` shebangs are supported via basic emulation rather than invoking the `env`
binary. For example, `#!/usr/bin/env python3` directly executes `python3`. The
`-S` flag is supported, allowing multiple interpreter arguments (e.g.
`#!/usr/bin/env -S python3 -u -O`). No other `env` flags are supported.

> **WARNING**
>
> By default, an action prompt is shown when launched from a GUI shell;
> disabling this behavior could lead to an increased security risk. (The same
> risk as running any untrusted application/script.)

## Shell verb pass-through

When Winbang is associated with an extension, Windows routes every shell verb:
`open`, `edit`, `print`, `printto`, `runas`, `UIAccess` through Winbang's
ProgID. Winbang owns all of these and proxies non-`open` verbs to whatever
handler the extension would have used before Winbang was selected.

How it works:

1. For a non-open verb, Winbang resolves the extension's underlying ProgID via
   `HKCR\.<ext>\(Default)`, then executes that ProgID's `shell\<verb>\command`
   value.
2. If that verb is absent on the underlying ProgID, Winbang falls back to its
   `open` verb. If that is also absent, Winbang exits non-zero.

> **Tip** When associating Winbang with an extension, pick it via **Explorer**'s
> "Open with -> Always" rather than `assoc .<ext>=Applications\winbang.exe`.
> Explorer writes UserChoice, which leaves the original ProgID intact at
> `HKCR\.<ext>\(Default)`, which is what pass-through reads to find the previous
> handler.
>
> For example: `assoc .py=Applications\winbang.exe` would overwrite `.py`'s
> ProgID pointer (`Python.File` -> `Winbang`), making the original handler's
> verbs unreachable through `.py`.

You can read more about Windows Shell verbs at:
https://learn.microsoft.com/en-us/windows/win32/shell/fa-verbs

### Repairing Winbang Verbs

The verb subkeys under `HKCU\Software\Classes\Applications\winbang.exe\shell`
can become stale if you move the executable after using it.

To recover, invoke the moved exe directly with `--reinstall-verbs`:

```powershell
C:\new\location\winbang.exe --reinstall-verbs
```

This force-resets every Winbang verb subkey.

## Config File Template

`%PROGRAMDATA%/Winbang/config.toml` or `%APPDATA%/Winbang/config.toml`

The configuration files will not merge; only one will be used. If
`allow_user_config` is set to true in the `%PROGRAMDATA%` config, then the user
config will be used. If not, the `%PROGRAMDATA%` config will be used.

`config.toml`

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
# exec_runtime = "deno"                              # Required
# view_runtime = "code"                              # Optional
# shebang_interpreter = "deno"                       # Optional
# extension = ".ts"                                  # Optional
# exec_argv_override = "run @{script}"               # Optional
# verb_edit = "\"C:\\Windows\\Notepad.exe\" \"%1\""  # Optional
# verb_print = "notepad /p \"%1\""                   # Optional
# verb_printto = "notepad /pt \"%1\" \"%2\""         # Optional
# verb_runas = "..."                                 # Optional
# verb_uiaccess = "..."                              # Optional

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

[[file_associations]]
exec_runtime = "C:\\Program Files\\PowerShell\\7\\pwsh.exe"
view_runtime = "code"
shebang_interpreter = "powershell"
extension = "ps1"
exec_argv_override = "-NoProfile -ExecutionPolicy Bypass -File \"@{script}\""
```

`exec_argv_override` will expand the following special variables:

- `@{script}`: The full script file path with double-backslashes (e.g.,
  `C:\\Users\\username\\test.sh`).
- `@{script_unix}`: The script file path with forward slashes (e.g.,
  `C:/Users/username/test.sh`).
- `@{passed_args}`: Additional arguments passed from the runtime to the script
  interpreter.

## Example/Test Files

**Deno Script**

`./denotest`

```typescript
#!/usr/bin/env deno

async function main() {
  console.log("Hello from Deno!");

  const fileName = "test.txt";

  try {
    await Deno.writeTextFile(fileName, "Hello from Deno!");
    console.log(`File ${fileName} created successfully.`);
  } catch (error) {
    console.error(`Failed to create file ${fileName}:`, error);
  }

  const promptExit = prompt("Press Enter to exit...");
}

main();
```

**Ruby Script**

`./rubytest`

```ruby
#!/usr/bin/env ruby
puts "Hello from Ruby!"

File.open("test.txt", "w") do |file|
  file.write("Hello from Ruby!")
end

puts "Press Enter to exit..."
STDIN.gets
```
