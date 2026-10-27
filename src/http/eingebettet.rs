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
        _ => return None,
    };
    Some(EingebetteteDatei { inhalt, typ })
}

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";

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
    fn unbekannte_und_ausbrechende_namen_gibt_es_nicht() {
        for name in [
            "gibtsnicht.js",
            "",
            "../Cargo.toml",
            "index.html/",
            "INDEX.HTML",
        ] {
            assert_eq!(nachschlagen(name), None, "{name}");
        }
    }
}
