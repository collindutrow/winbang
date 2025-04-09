#[macro_export]
macro_rules! log_debug {
    ($msg:expr) => {
        #[cfg(debug_assertions)]
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            use std::path::Path;

            let msg = $msg;

            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| String::from("."));
            let debug_log_dir = Path::new(&appdata).join("Winbang");
            let debug_log_path = debug_log_dir.join("debug.log");

            if !debug_log_dir.exists() {
                std::fs::create_dir_all(&debug_log_dir).unwrap_or_else(|_| {
                    eprintln!("Failed to create debug log directory: {:?}", debug_log_dir);
                });
            }

            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(debug_log_path)
            {
                let _ = writeln!(file, "[DEBUG] {}", msg);
                let _ = file.flush();
            }

            println!("[DEBUG] {}", msg);
        }
    };
    ($fmt:literal, $($arg:tt)*) => {
        log_debug!(format!($fmt, $($arg)*));
    };
}
