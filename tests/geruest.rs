//! B0001 Crate-Gerüst: die Crate baut ein Binärprogramm mit dem Namen `localchatA`.

use std::path::Path;

#[test]
fn das_binaerprogramm_heisst_localchat_a() {
    let programm = Path::new(env!("CARGO_BIN_EXE_localchatA"));
    assert!(programm.is_file(), "{} fehlt", programm.display());
    assert_eq!(
        programm.file_name().and_then(|n| n.to_str()),
        Some("localchatA")
    );
}
