async function main() {
    console.log("Hello from Deno");

    // Test if deno has write permission
    const fileName = "hello.txt";
    try {
    await Deno.writeTextFile(fileName, "Hello from Deno!");
    console.log(`File ${fileName} created successfully.`);
    }
    catch (error) {
    console.error(`Failed to create file ${fileName}:`, error);
    }

    // Press enter to exit
    const promptExit = prompt("Press Enter to exit...");
}

main();