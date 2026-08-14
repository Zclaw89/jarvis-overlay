#[cfg(windows)]
use std::path::PathBuf;

fn main() {
    copy_runtime_libs();
    tauri_build::build()
}

/// Copy native runtime DLLs that dependency build scripts drop into the cargo
/// target directory (notably `ort`'s `DirectML.dll`) into a stable
/// `runtime-libs/` folder. The Tauri bundler then ships them next to the
/// executable (see `tauri.windows.conf.json`), so the installer is fully
/// self-contained. At dev time the DLLs already sit next to the dev exe, so the
/// app runs regardless.
fn copy_runtime_libs() {
    #[cfg(windows)]
    {
        // OUT_DIR = <target>/<profile>/build/<pkg>-<hash>/out — the profile dir
        // (where dependency build scripts place DLLs) is three levels up.
        let Ok(out_dir) = std::env::var("OUT_DIR") else {
            return;
        };
        let Some(profile_dir) =
            PathBuf::from(&out_dir).ancestors().nth(3).map(|p| p.to_path_buf())
        else {
            return;
        };

        let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") else {
            return;
        };
        let dest_dir = PathBuf::from(&manifest).join("runtime-libs");
        let _ = std::fs::create_dir_all(&dest_dir);

        // DLLs that must travel next to the executable at runtime.
        for dll in ["DirectML.dll"] {
            let src = profile_dir.join(dll);
            if src.exists() {
                let _ = std::fs::copy(&src, dest_dir.join(dll));
            }
        }
    }
}
