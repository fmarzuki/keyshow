fn main() {
    println!("cargo:rerun-if-changed=packaging/windows/keyshow.ico");
    println!("cargo:rerun-if-changed=packaging/windows/keyshow.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // Icon and version info for keyshow.exe (Explorer, taskbar, Add/Remove Programs).
    let mut res = winresource::WindowsResource::new();
    res.set_icon("packaging/windows/keyshow.ico")
        .set_manifest_file("packaging/windows/keyshow.manifest")
        .set("ProductName", "Keyshow")
        .set(
            "FileDescription",
            "Keyshow - show your keystrokes on screen",
        )
        .set("OriginalFilename", "keyshow.exe");
    if let Err(e) = res.compile() {
        // Lets `cargo check --target x86_64-pc-windows-msvc` work from Linux without a resource compiler.
        println!("cargo:warning=could not embed the Windows icon: {e}");
    }
}
