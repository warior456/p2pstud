fn main() {
    tauri_build::build();

    #[cfg(windows)]
    {
        // 1. Link Windows resources/manifest to all binaries and tests so Common Controls v6 is activated
        let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
        let res = std::path::Path::new(&out_dir).join("libresource.a");
        if res.exists() {
            println!("cargo:rustc-link-arg={}", res.display());
        }

        // 2. Under Windows GNU target, ensure GCC runtime library search path is provided
        if std::env::var("TARGET")
            .map(|t| t.contains("windows-gnu"))
            .unwrap_or(false)
        {
            let winlibs = "C:\\Users\\matte\\AppData\\Local\\Microsoft\\WinGet\\Packages\\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\\mingw64\\lib\\gcc\\x86_64-w64-mingw32\\16.2.0";
            if std::path::Path::new(winlibs).exists() {
                println!("cargo:rustc-link-search=native={winlibs}");
            }
        }
    }
}
