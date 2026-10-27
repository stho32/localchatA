//! F0003 Oberfläche ausliefern — über das echte Programm (US-9, US-10).

mod gemeinsam;

use gemeinsam::wegwerf_starten;

struct Antwort {
    status: u16,
    typ: String,
    cache: String,
    inhalt: String,
}

async fn holen(url: String) -> Antwort {
    let antwort = reqwest::get(url).await.expect("Antwort");
    let kopf = |name: &str| {
        antwort
            .headers()
            .get(name)
            .and_then(|wert| wert.to_str().ok())
            .unwrap_or_default()
            .to_string()
    };
    let (typ, cache) = (kopf("content-type"), kopf("cache-control"));
    Antwort {
        status: antwort.status().as_u16(),
        typ,
        cache,
        inhalt: antwort.text().await.expect("Inhalt"),
    }
}

#[tokio::test]
async fn us9_die_wurzel_liefert_die_deutsche_seite_mit_titel_ohne_zwischenspeichern() {
    let instanz = wegwerf_starten();

    let seite = holen(instanz.adresse.clone()).await;

    assert_eq!(seite.status, 200);
    assert!(seite.typ.starts_with("text/html"), "{}", seite.typ);
    assert_eq!(seite.cache, "no-cache");
    assert!(seite.inhalt.contains("<html lang=\"de\">"));
    assert!(seite.inhalt.contains("<title>localchatA</title>"));
}

#[tokio::test]
async fn die_tokens_kommen_aus_dem_programm_als_css_ohne_zwischenspeichern() {
    let instanz = wegwerf_starten();

    let tokens = holen(format!("{}web/lc-tokens.css", instanz.adresse)).await;

    assert_eq!(tokens.status, 200);
    assert!(tokens.typ.starts_with("text/css"), "{}", tokens.typ);
    assert_eq!(tokens.cache, "no-cache");
    assert!(tokens.inhalt.contains("--kd-bg"));
}

#[tokio::test]
async fn us10_unbekannte_dateien_und_pfade_ergeben_404() {
    let instanz = wegwerf_starten();

    for pfad in [
        "web/gibtsnicht.js",
        "web/",
        "web/../Cargo.toml",
        "gibtsnicht",
        "index.html",
    ] {
        let antwort = holen(format!("{}{pfad}", instanz.adresse)).await;
        assert_eq!(antwort.status, 404, "{pfad}");
    }
}
