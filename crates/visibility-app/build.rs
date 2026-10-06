//! Embeds the application icon and version info into the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        if let Err(e) = resource.compile() {
            // Without a resource compiler only the icon is lost, so keep building.
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}
