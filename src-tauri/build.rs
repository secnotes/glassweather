fn main() {
    copy_deskband_dll();
    tauri_build::build()
}

/// deskband11lib 的 build.rs 会把 deskband11lib.dll 复制到 "OUT_DIR 三级之上"
/// 的目录，但新版 cargo 的 build 目录是嵌套布局（build/<pkg>/<hash>/out），
/// 三级之上落在 build/ 而非 profile 目录——DLL 不在 exe 旁，运行时加载失败，
/// tauri.conf.json 的 resources 校验也找不到文件。
/// 此处直接从 cargo 注册表源码目录把 DLL 复制到本 profile 目录（release/debug），
/// 注册表源码在任何 build script 运行前就已解包，顺序可靠；复制完成后
/// tauri_build::build() 的资源路径校验即能通过，安装包与本地运行均能找到 DLL。
fn copy_deskband_dll() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86_64") => "win-x64",
        Ok("aarch64") => "win-arm64",
        _ => return,
    };
    let Some(home) = std::env::var_os("CARGO_HOME").map(std::path::PathBuf::from) else {
        return;
    };
    let src_root = home.join("registry").join("src");
    let Ok(registries) = std::fs::read_dir(&src_root) else {
        return;
    };

    // 注册表/*/deskband11lib-<version>/bin/<arch>/deskband11lib.dll
    let mut dll: Option<std::path::PathBuf> = None;
    for reg in registries.flatten() {
        let Ok(packages) = std::fs::read_dir(reg.path()) else {
            continue;
        };
        for pkg in packages.flatten() {
            if pkg
                .file_name()
                .to_string_lossy()
                .starts_with("deskband11lib-")
            {
                let candidate = pkg.path().join("bin").join(arch).join("deskband11lib.dll");
                if candidate.is_file() {
                    dll = Some(candidate);
                }
            }
        }
    }
    let Some(dll) = dll else {
        return;
    };

    // 从 OUT_DIR 向上找 profile 目录（目录名为 release / debug，
    // 兼容 --target 布局下带三元组后缀的 release-<triple> 等变体）
    let Some(out_dir) = std::env::var_os("OUT_DIR").map(std::path::PathBuf::from) else {
        return;
    };
    let mut dir = out_dir.as_path();
    while let Some(name) = dir.file_name().and_then(|s| s.to_str()) {
        if name == "release" || name == "debug" || name.starts_with("release-") || name.starts_with("debug-") {
            let dest = dir.join("deskband11lib.dll");
            if let Err(e) = std::fs::copy(&dll, &dest) {
                println!("cargo:warning=could not copy deskband11lib.dll to {}: {e}", dest.display());
            }
            return;
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return,
        }
    }
}
