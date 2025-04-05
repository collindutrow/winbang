# Winbang

Winbang provides Nix-like shebang support for Windows.

## Setup

**EXAMPLE CONFIG**

`%PROGRAMDATA%/Winbang/config.toml`
```toml
# List of known GUI parent processes
gui_shells = ["explorer.exe", "dopus.exe"]

# File extension to interpreter/editor mappings
[[file_associations]]
extension = "rb"
interpreter = "ruby"
editor = "code"

[[file_associations]]
extension = "py"
interpreter = "python"
editor = "code"

[[file_associations]]
extension = "pl"
interpreter = "perl"
editor = "notepad"

[[file_associations]]
extension = "sh"
interpreter = "bash"
editor = "notepad"

[[dispatch_overrides]]
interpreter = "deno"
args_override = "run -A $script"
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
    console.log("Hello from Deno");

    // Test if deno has write permission (requires an override in config.toml)
    const fileName = "hello.txt";

    try {
        await Deno.writeTextFile(fileName, "Hello, Deno!");
        console.log(`File ${fileName} created successfully.`);
    }
    catch (error) {
        console.error(`Failed to create file ${fileName}:`, error);
    }

    // Press enter to exit
    const promptExit = prompt("Press Enter to exit...");
}

main();
```

**Ruby Script**

```ruby
#!/usr/bin/env ruby
puts "Hello from Ruby!"

# Write "Hello, world!" to a file named hello.txt
File.open("hello.txt", "w") do |file|
  file.write("Hello, Ruby!")
end

# Press the Enter key to exit the program
puts "Press Enter to exit..."
STDIN.gets
```