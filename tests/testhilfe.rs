//! B0007 Testhilfe: eine Instanz im Wegwerf-Ordner endet mit dem Drop — Prozess weg, Port frei, Ordner weg.

mod gemeinsam;

use std::net::TcpStream;

#[tokio::test]
async fn wegwerf_instanz_laeuft_bis_zum_drop_und_hinterlaesst_nichts() {
    let instanz = gemeinsam::wegwerf_starten();
    let port = instanz.port();
    let ordner = instanz.ordner.clone();
    assert!(ordner.is_dir());
    assert_eq!(gemeinsam::gesundheit(&instanz.adresse).await.0, 200);

    drop(instanz);

    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_err(),
        "Port {port} antwortet noch"
    );
    assert!(
        !ordner.exists(),
        "Wegwerf-Ordner {} liegt noch",
        ordner.display()
    );
}
