//! Embeds the Windows version resource (product name and version) into the DLL.
//! Code signing (SignPath) requires every signed file to carry the same product name and version.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set("ProductName", "Hangul IME")
        .set("FileDescription", "Hangul IME (2-beolsik) text input service")
        .set("LegalCopyright", "Copyright (c) 2026 ironsand. MIT License.")
        .set("OriginalFilename", "hangul_ime.dll")
        .set("InternalName", "hangul_ime");
    res.compile().expect("failed to compile the Windows version resource");
}
