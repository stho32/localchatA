//! Eingebettete Dateien der Oberfläche (ADR-0002): zur Bauzeit per `include_bytes!` im Programm.

/// Eine Datei der Oberfläche mit dem Content-Type, unter dem sie ausgeliefert wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EingebetteteDatei {
    pub inhalt: &'static [u8],
    pub typ: &'static str,
}

/// Schlägt eine Datei unter `/web/<name>` nach; unbekannte Namen gibt es nicht.
pub fn nachschlagen(name: &str) -> Option<EingebetteteDatei> {
    let (inhalt, typ): (&'static [u8], _) = match name {
        "index.html" => (include_bytes!("../../web/index.html"), HTML),
        "lc-tokens.css" => (include_bytes!("../../web/lc-tokens.css"), CSS),
        "lc-seite.css" => (include_bytes!("../../web/lc-seite.css"), CSS),
        "start.js" => (include_bytes!("../../web/start.js"), JS),
        "api.js" => (include_bytes!("../../web/api.js"), JS),
        "sitzung.js" => (include_bytes!("../../web/sitzung.js"), JS),
        "namensregel.js" => (include_bytes!("../../web/namensregel.js"), JS),
        "beitreten.js" => (include_bytes!("../../web/beitreten.js"), JS),
        "kopfzeile.js" => (include_bytes!("../../web/kopfzeile.js"), JS),
        "teilnehmerliste.js" => (include_bytes!("../../web/teilnehmerliste.js"), JS),
        "hauptansicht.js" => (include_bytes!("../../web/hauptansicht.js"), JS),
        "live.js" => (include_bytes!("../../web/live.js"), JS),
        "sse.js" => (include_bytes!("../../web/sse.js"), JS),
        "verbindung.js" => (include_bytes!("../../web/verbindung.js"), JS),
        "strom.js" => (include_bytes!("../../web/strom.js"), JS),
        "verteiler.js" => (include_bytes!("../../web/verteiler.js"), JS),
        "restzeit.js" => (include_bytes!("../../web/restzeit.js"), JS),
        "sekundentakt.js" => (include_bytes!("../../web/sekundentakt.js"), JS),
        "channels.js" => (include_bytes!("../../web/channels.js"), JS),
        "seitenleiste.js" => (include_bytes!("../../web/seitenleiste.js"), JS),
        "channel-anlegen.js" => (include_bytes!("../../web/channel-anlegen.js"), JS),
        "auswahl.js" => (include_bytes!("../../web/auswahl.js"), JS),
        "gliederung.js" => (include_bytes!("../../web/gliederung.js"), JS),
        "nachricht.js" => (include_bytes!("../../web/nachricht.js"), JS),
        "verlauf.js" => (include_bytes!("../../web/verlauf.js"), JS),
        "fenster.js" => (include_bytes!("../../web/fenster.js"), JS),
        "frist.js" => (include_bytes!("../../web/frist.js"), JS),
        "channel-einstellungen.js" => (include_bytes!("../../web/channel-einstellungen.js"), JS),
        "eingabe.js" => (include_bytes!("../../web/eingabe.js"), JS),
        "zustellung.js" => (include_bytes!("../../web/zustellung.js"), JS),
        "senden.js" => (include_bytes!("../../web/senden.js"), JS),
        "live-verlauf.js" => (include_bytes!("../../web/live-verlauf.js"), JS),
        "nachholen.js" => (include_bytes!("../../web/nachholen.js"), JS),
        "nachhollauf.js" => (include_bytes!("../../web/nachhollauf.js"), JS),
        "verbindungsleiste.js" => (include_bytes!("../../web/verbindungsleiste.js"), JS),
        "textregel.js" => (include_bytes!("../../web/textregel.js"), JS),
        "hinweis.js" => (include_bytes!("../../web/hinweis.js"), JS),
        "fokus.js" => (include_bytes!("../../web/fokus.js"), JS),
        "ansage.js" => (include_bytes!("../../web/ansage.js"), JS),
        "entwurf.js" => (include_bytes!("../../web/entwurf.js"), JS),
        "wiederverbinden.js" => (include_bytes!("../../web/wiederverbinden.js"), JS),
        "wiederbeitritt.js" => (include_bytes!("../../web/wiederbeitritt.js"), JS),
        "bildschirmlauf.js" => (include_bytes!("../../web/bildschirmlauf.js"), JS),
        "bearbeiten.js" => (include_bytes!("../../web/bearbeiten.js"), JS),
        "loeschen.js" => (include_bytes!("../../web/loeschen.js"), JS),
        "tippen.js" => (include_bytes!("../../web/tippen.js"), JS),
        "aktivitaetsanzeige.js" => (include_bytes!("../../web/aktivitaetsanzeige.js"), JS),
        "kopieren.js" => (include_bytes!("../../web/kopieren.js"), JS),
        "menuelage.js" => (include_bytes!("../../web/menuelage.js"), JS),
        "zitat.js" => (include_bytes!("../../web/zitat.js"), JS),
        "dateiliste.js" => (include_bytes!("../../web/dateiliste.js"), JS),
        _ => return None,
    };
    Some(EingebetteteDatei { inhalt, typ })
}

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";

#[cfg(test)]
mod tests {
    use super::*;

    fn text(datei: EingebetteteDatei) -> String {
        String::from_utf8(datei.inhalt.to_vec()).expect("UTF-8")
    }

    #[test]
    fn index_ist_die_deutsche_seite_mit_dem_titel_localchat_a() {
        let index = nachschlagen("index.html").expect("index.html eingebettet");

        assert_eq!(index.typ, "text/html; charset=utf-8");
        let html = text(index);
        assert!(html.contains("<html lang=\"de\">"), "{html}");
        assert!(html.contains("<title>localchatA</title>"), "{html}");
    }

    #[test]
    fn die_tokens_sind_die_des_designsystems_als_css() {
        let tokens = nachschlagen("lc-tokens.css").expect("lc-tokens.css eingebettet");

        assert_eq!(tokens.typ, "text/css; charset=utf-8");
        assert_eq!(
            text(tokens),
            include_str!("../../Dokumentation/Wireframes/designsystem/tokens.css")
        );
    }

    #[test]
    fn der_seitenstil_ist_eine_eigene_css_datei() {
        let seite = nachschlagen("lc-seite.css").expect("lc-seite.css eingebettet");

        assert_eq!(seite.typ, "text/css; charset=utf-8");
        assert!(text(seite).contains("var(--kd-bg)"));
    }

    #[test]
    fn die_module_der_oberflaeche_sind_javascript() {
        for name in [
            "start.js",
            "api.js",
            "sitzung.js",
            "namensregel.js",
            "beitreten.js",
            "kopfzeile.js",
            "teilnehmerliste.js",
            "hauptansicht.js",
            "live.js",
            "sse.js",
            "verbindung.js",
            "strom.js",
            "verteiler.js",
            "restzeit.js",
            "sekundentakt.js",
        ] {
            let modul = nachschlagen(name).unwrap_or_else(|| panic!("{name} eingebettet"));
            assert_eq!(modul.typ, "text/javascript; charset=utf-8", "{name}");
            assert!(!modul.inhalt.is_empty(), "{name}");
        }
    }

    /// Ein neues Modul unter `web/` ohne Eintrag in `nachschlagen` liefe nur im Browser auf einen 404 (Karte #62).
    #[test]
    fn jedes_javascript_unter_web_ist_eingebettet() {
        let ordner = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("web");
        for eintrag in std::fs::read_dir(ordner).expect("web/") {
            let name = eintrag
                .expect("Eintrag")
                .file_name()
                .into_string()
                .expect("Name");
            if name.ends_with(".js") {
                assert!(
                    nachschlagen(&name).is_some(),
                    "{name} ist nicht eingebettet"
                );
            }
        }
    }

    #[test]
    fn unbekannte_und_ausbrechende_namen_gibt_es_nicht() {
        for name in [
            "gibtsnicht.js",
            "",
            "../Cargo.toml",
            "index.html/",
            "INDEX.HTML",
            "package.json",
        ] {
            assert_eq!(nachschlagen(name), None, "{name}");
        }
    }

    /// Jede Datei der Oberfläche unter `web/` ist eingebettet — ein vergessenes Modul fiele sonst erst im Browser
    /// auf (der Import scheitert, der Worker startet nicht). `package.json` gehört nur zu den Node-Tests.
    #[test]
    fn jede_datei_der_oberflaeche_ist_eingebettet() {
        let web = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("web");
        let mut fehlend = Vec::new();
        for eintrag in std::fs::read_dir(web).expect("web/") {
            let name = eintrag
                .expect("Eintrag")
                .file_name()
                .to_string_lossy()
                .into_owned();
            if name != "package.json" && nachschlagen(&name).is_none() {
                fehlend.push(name);
            }
        }
        assert_eq!(fehlend, Vec::<String>::new());
    }
}
