//! Available-disk-space probe wrapped for the receiver pre-check.
//! `fs2::available_space` resolves the volume holding the given path.

use std::path::Path;

pub fn available_for(path: &Path) -> Option<u64> {
    // fs2 needs an existing path; if the dir doesn't exist yet, walk up
    // to the first ancestor that does.
    let mut cur: Option<&Path> = Some(path);
    while let Some(p) = cur {
        if p.exists() {
            return fs2::available_space(p).ok();
        }
        cur = p.parent();
    }
    None
}
