fn main() {
    println!("cargo:rustc-link-arg-bin=winbang=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bin=winbang=/MANIFESTINPUT:app.manifest");
}
