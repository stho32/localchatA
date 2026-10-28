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

/// Stellen im HTML, die eine CSP mit `script-src 'self'; style-src 'self'` verletzen würden (ADR-0014 Regel 6).
fn inline_code(html: &str) -> Vec<String> {
    let klein = html.to_lowercase();
    let mut funde = Vec::new();
    if klein.contains("<style") {
        funde.push("<style>".to_string());
    }
    for (stelle, _) in klein.match_indices("<script") {
        let tag = &klein[stelle
            ..klein[stelle..]
                .find('>')
                .map_or(klein.len(), |ende| stelle + ende)];
        if !tag.contains(" src=") {
            funde.push(format!("Skript ohne src: {tag}"));
        }
    }
    let woerter: Vec<&str> = klein
        .split(|zeichen: char| zeichen.is_whitespace())
        .collect();
    for wort in woerter {
        let attribut = wort.split('=').next().unwrap_or_default();
        let ist_on_attribut = wort.contains('=')
            && attribut.len() > 2
            && attribut.starts_with("on")
            && attribut
                .chars()
                .all(|zeichen| zeichen.is_ascii_alphabetic());
        if attribut == "style" && wort.contains('=') || ist_on_attribut {
            funde.push(wort.to_string());
        }
    }
    funde
}

#[test]
fn die_pruefung_auf_inline_code_findet_jede_art() {
    let html = "<style>a{}</style><p style=\"x\" onclick=\"y\"><script>1</script><script type=module src=/a.js></script>";
    assert_eq!(inline_code(html).len(), 4, "{:?}", inline_code(html));
}

#[tokio::test]
async fn die_seite_kommt_ohne_inline_stil_und_skript_aus_der_seitenstil_als_css_datei() {
    let instanz = wegwerf_starten();

    let seite = holen(instanz.adresse.clone()).await;
    let stil = holen(format!("{}web/lc-seite.css", instanz.adresse)).await;

    assert_eq!(inline_code(&seite.inhalt), Vec::<String>::new());
    assert!(
        seite.inhalt.contains("href=\"/web/lc-seite.css\""),
        "{}",
        seite.inhalt
    );
    assert_eq!(stil.status, 200);
    assert!(stil.typ.starts_with("text/css"), "{}", stil.typ);
    assert!(stil.inhalt.contains("var(--kd-bg)"), "{}", stil.inhalt);
}

#[tokio::test]
async fn die_seite_laedt_genau_ein_modul_skript_und_die_module_kommen_als_javascript() {
    let instanz = wegwerf_starten();

    let seite = holen(instanz.adresse.clone()).await;

    assert_eq!(
        seite.inhalt.matches("<script").count(),
        1,
        "{}",
        seite.inhalt
    );
    assert!(
        seite
            .inhalt
            .contains("<script type=\"module\" src=\"/web/start.js\"></script>"),
        "{}",
        seite.inhalt
    );
    for modul in [
        "start.js",
        "api.js",
        "sitzung.js",
        "namensregel.js",
        "beitreten.js",
        "kopfzeile.js",
    ] {
        let antwort = holen(format!("{}web/{modul}", instanz.adresse)).await;
        assert_eq!(antwort.status, 200, "{modul}");
        assert_eq!(antwort.typ, "text/javascript; charset=utf-8", "{modul}");
    }
}

/// R00004 F0014, B0093 (ADR-0015 Regel 3, Pre-Mortem 1): die ausgelieferte Seite nennt die Programmversion,
/// die Oberfläche legt den Worker unter `/web/strom.js?v=<version>` an, `<version>` gleich `version` aus
/// `/api/gesundheit`; die Worker-Datei kommt unabhängig vom Query.
#[tokio::test]
async fn die_worker_adresse_traegt_die_programmversion_aus_der_gesundheit() {
    let instanz = wegwerf_starten();

    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;
    let version = gesundheit["version"].as_str().expect("version").to_string();
    let seite = holen(instanz.adresse.clone()).await;
    let live = holen(format!("{}web/live.js", instanz.adresse)).await;
    let worker = holen(format!("{}web/strom.js?v={version}", instanz.adresse)).await;
    let ohne_query = holen(format!("{}web/strom.js", instanz.adresse)).await;

    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    assert!(
        seite
            .inhalt
            .contains(&format!("<meta name=\"lc-version\" content=\"{version}\">")),
        "{}",
        seite.inhalt
    );
    assert!(
        live.inhalt.contains("`/web/strom.js?v=${version}`"),
        "{}",
        live.inhalt
    );
    assert_eq!(worker.status, 200);
    assert!(worker.typ.starts_with("text/javascript"), "{}", worker.typ);
    assert_eq!(worker.inhalt, ohne_query.inhalt);
}
