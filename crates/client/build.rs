//! 에셋 폴더(`assets`)에 있는 파일의 목록을 만들어서 클라이언트에 넣습니다.
//! 클라이언트는 이 목록에 있는 파일만 불러오고, 없는 아이콘은 자리 표시로 그립니다. 파일을 넣고 다시 빌드하면 그 에셋이 쓰입니다.

use std::path::Path;
use std::{env, fs};

fn collect(root: &Path, dir: &Path, files: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files);
        } else if let Ok(relative) = path.strip_prefix(root) {
            let parts: Vec<String> = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect();
            files.push(parts.join("/"));
        }
    }
}

fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("cargo sets this")).join("assets");
    let mut files = Vec::new();
    collect(&root, &root, &mut files);
    files.sort();
    let list: String = files
        .iter()
        .map(|file| format!("    {file:?},\n"))
        .collect();
    let code = format!("pub const FILES: &[&str] = &[\n{list}];\n");
    let out = Path::new(&env::var("OUT_DIR").expect("cargo sets this")).join("asset_files.rs");
    fs::write(out, code).expect("the build folder is writable");
    println!("cargo:rerun-if-changed=assets");
}
