fn main() {
    tauri_build::build();

    #[cfg(windows)]
    {
        let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
        let res = std::path::Path::new(&out_dir).join("libresource.a");
        if res.exists() {
            println!("cargo:rustc-link-arg-tests={}", res.display());
        }

        // Ensure WebView2Loader.dll is accessible in the deps/ folder for unit test runners
        for ancestor in std::path::Path::new(&out_dir).ancestors() {
            let src_dll = ancestor.join("WebView2Loader.dll");
            let dst_dll = ancestor.join("deps").join("WebView2Loader.dll");
            if src_dll.exists() && ancestor.join("deps").exists() {
                let _ = std::fs::copy(&src_dll, &dst_dll);
                break;
            }
        }
    }
}
