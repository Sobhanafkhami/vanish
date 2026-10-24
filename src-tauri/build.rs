fn main() {
    // tauri-build does not watch the icon files, so without this the exe keeps the icon it was first built with.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
