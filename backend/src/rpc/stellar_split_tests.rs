//! Guards for the `rpc/` split (#2775).

use std::path::Path;

fn walk(dir: &Path, out: &mut Vec<(String, usize)>) {
    for entry in std::fs::read_dir(dir).expect("read rpc dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("read file");
            out.push((path.display().to_string(), text.lines().count()));
        }
    }
}

#[test]
fn no_rpc_file_exceeds_800_lines() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rpc");
    let mut files = Vec::new();
    walk(&dir, &mut files);
    let too_big: Vec<_> = files.iter().filter(|(_, n)| *n > 800).collect();
    assert!(too_big.is_empty(), "files over 800 lines: {too_big:?}");
}

#[test]
fn legacy_stellar_paths_still_resolve() {
    // Re-exports kept for one release: both paths name the same type.
    fn same<T>(_: Option<T>, _: Option<T>) {}
    same::<crate::rpc::stellar::Payment>(None, None::<crate::rpc::horizon::models::Payment>);
    same::<crate::rpc::stellar::RpcLedger>(None, None::<crate::rpc::soroban::models::RpcLedger>);
}
