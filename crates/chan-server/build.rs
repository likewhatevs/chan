// build.rs
//
// rust-embed bakes `web/dist/` into the binary at compile time, but
// Cargo doesn't track changes inside the embedded folder. Without
// this script, `npm run build` followed by `cargo build --release`
// produces a binary with the OLD bundle because Cargo decides
// nothing has changed and skips compilation.
//
// We emit `cargo:rerun-if-changed=PATH` for every file under
// web/dist so Cargo re-links chan-server whenever the frontend
// bundle is rebuilt.
//
// We also `create_dir_all` web/dist on first build because rust-
// embed errors if the folder doesn't exist. A fresh clone has no
// dist (it's gitignored as a build artifact); the empty dir lets
// the macro succeed and the binary just serves nothing useful
// until the user runs `cd web && npm install && npm run build`.

use std::path::Path;

fn main() {
    let dist = Path::new("../../web/dist");
    let _ = std::fs::create_dir_all(dist);
    println!("cargo:rerun-if-changed={}", dist.display());
    walk(dist);
}

fn walk(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        println!("cargo:rerun-if-changed={}", p.display());
        if p.is_dir() {
            walk(&p);
        }
    }
}
