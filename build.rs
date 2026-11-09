fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    // Embed the app icon + version info into the .exe when building on Windows.
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "RAM Altman");
        res.set("FileDescription", "RAM Altman: Memory Is All You Need");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed Windows icon: {e}");
        }
    }
}
