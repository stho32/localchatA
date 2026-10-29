//! Abweisung eines Host-Namens (ADR-0045 Regel 7 mit Ergänzung E4) — rein: die deutsche Seite für ein
//! Browser-Dokument und der Text für alle anderen, beide mit der Adresse, die sofort funktioniert.

use std::net::SocketAddr;

/// HTML ohne Stil und Skript (die CSP aus ADR-0014 bleibt): Überschrift, Satz für den Teilnehmer mit Link auf
/// `http://<lokal>/`, Satz für den Betreiber (nur mit Namen), Satz zum Warum. Der Name erscheint nur maskiert und
/// nie in einem Attribut.
pub fn seite(name: Option<&str>, lokal: SocketAddr) -> String {
    let adresse = format!("http://{lokal}/");
    let teilnehmer = match name {
        Some(name) => format!(
            "Unter dem Namen {} antwortet diese Instanz nicht. Öffne sie über ihre Adresse: ",
            maskiert(name)
        ),
        None => {
            "Diese Anfrage nennt keinen Namen. Öffne die Instanz über ihre Adresse: ".to_string()
        }
    };
    let betreiber = name.map_or_else(String::new, |name| {
        let name = maskiert(name);
        format!(
            "<p>Der Betreiber kann den Namen {name} mit --erreichbar-als {name} beim Start freigeben.</p>\n"
        )
    });
    format!(
        concat!(
            "<!DOCTYPE html>\n",
            "<html lang=\"de\">\n",
            "<head><meta charset=\"utf-8\"><title>localchatA — Adresse nicht freigegeben</title></head>\n",
            "<body>\n",
            "<h1>Diese Adresse ist für localchatA nicht freigegeben</h1>\n",
            "<p>{teilnehmer}<a href=\"{adresse}\">{adresse}</a></p>\n",
            "{betreiber}",
            "<p>Namen des eigenen Netzes gelten automatisch. Andere Namen sind gesperrt, damit fremde Websites nicht über einen umgeleiteten Namen mitlesen.</p>\n",
            "</body>\n",
            "</html>\n"
        ),
        teilnehmer = teilnehmer,
        adresse = adresse,
        betreiber = betreiber,
    )
}

/// `&`, `<`, `>`, `"` und `'` als Entität.
fn maskiert(text: &str) -> String {
    text.chars()
        .map(|zeichen| match zeichen {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            anderes => anderes.to_string(),
        })
        .collect()
}

/// Text für Nicht-Browser: Name, Adresse, Ausweg; ohne lokale Adresse wie vor ADR-0045.
pub fn text(name: Option<&str>, lokal: Option<SocketAddr>) -> String {
    let ip_adresse = match lokal {
        Some(lokal) => format!("die IP-Adresse http://{lokal}/"),
        None => "die IP-Adresse".to_string(),
    };
    match (name, lokal) {
        (Some(name), Some(_)) => format!(
            "Diese Instanz ist unter dem Namen {name} nicht freigegeben — über {ip_adresse} öffnen oder mit --erreichbar-als {name} starten. Namen des eigenen Netzes gelten automatisch."
        ),
        (Some(name), None) => format!(
            "Diese Instanz ist unter dem Namen {name} nicht freigegeben — über {ip_adresse} öffnen oder mit --erreichbar-als {name} starten."
        ),
        (None, _) => format!("Diese Anfrage nennt keinen Host — über {ip_adresse} öffnen."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lan() -> SocketAddr {
        "192.168.178.65:7430".parse().expect("Adresse")
    }

    /// Kriterium F0079 „Seite“ (Matrix G1).
    #[test]
    fn die_seite_ist_deutsch_mit_ueberschrift_link_und_beiden_saetzen() {
        let html = seite(Some("boese.example"), lan());

        assert!(html.contains(r#"<html lang="de">"#), "{html}");
        assert!(html.contains("<meta charset=\"utf-8\">"), "{html}");
        assert!(
            html.contains("<h1>Diese Adresse ist für localchatA nicht freigegeben</h1>"),
            "{html}"
        );
        assert!(
            html.contains(
                r#"<a href="http://192.168.178.65:7430/">http://192.168.178.65:7430/</a>"#
            ),
            "{html}"
        );
        assert!(
            html.contains(
                "Der Betreiber kann den Namen boese.example mit --erreichbar-als boese.example beim Start freigeben."
            ),
            "{html}"
        );
        assert!(html.contains("fremde Websites"), "{html}");
        assert!(
            html.contains("Namen des eigenen Netzes gelten automatisch"),
            "{html}"
        );
    }

    fn ohne_stil_und_skript(html: &str) {
        let klein = html.to_ascii_lowercase();
        for verboten in ["<style", "style=", "<script", "<img", "<link"] {
            assert!(!klein.contains(verboten), "{verboten}: {html}");
        }
        for tag in klein.split('<').skip(1) {
            let im_tag = tag.split('>').next().unwrap_or_default();
            let attribute: Vec<&str> = im_tag.split_whitespace().skip(1).collect();
            assert!(
                !attribute.iter().any(|attribut| attribut.starts_with("on")),
                "on…-Attribut in <{im_tag}>"
            );
        }
    }

    #[test]
    fn die_seite_hat_keinen_stil_kein_skript_und_kein_on_attribut() {
        ohne_stil_und_skript(&seite(Some("boese.example"), lan()));
        ohne_stil_und_skript(&seite(None, lan()));
    }

    #[test]
    fn ipv6_steht_im_link_in_klammern() {
        let html = seite(
            Some("boese.example"),
            "[::1]:7430".parse().expect("Adresse"),
        );

        assert!(
            html.contains(r#"<a href="http://[::1]:7430/">http://[::1]:7430/</a>"#),
            "{html}"
        );
    }

    /// Matrix G2: der Name erscheint nur maskiert, nie in einem Attribut.
    #[test]
    fn der_name_wird_maskiert() {
        for (name, maskiert) in [
            ("<script>x", "&lt;script&gt;x"),
            (
                "<img src=x onerror=alert(1)>",
                "&lt;img src=x onerror=alert(1)&gt;",
            ),
            ("a\"b'c&d.local", "a&quot;b&#39;c&amp;d.local"),
            (
                "</h1><script>x</script>",
                "&lt;/h1&gt;&lt;script&gt;x&lt;/script&gt;",
            ),
        ] {
            let html = seite(Some(name), lan());
            assert!(html.contains(maskiert), "{name}: {html}");
            assert!(!html.contains(name), "{name}: {html}");
            ohne_stil_und_skript(&html);
            let attribute: String = html
                .split('<')
                .filter_map(|tag| tag.split('>').next())
                .collect();
            assert!(!attribute.contains(maskiert), "Name in einem Tag: {html}");
        }
    }

    /// E4, Matrix G5: ohne Namen kein Satz für den Betreiber.
    #[test]
    fn ohne_namen_ueberschrift_link_und_warum_ohne_betreiber_satz() {
        let html = seite(None, lan());

        assert!(html.contains("Diese Adresse ist für localchatA nicht freigegeben"));
        assert!(
            html.contains(r#"href="http://192.168.178.65:7430/""#),
            "{html}"
        );
        assert!(html.contains("fremde Websites"), "{html}");
        assert!(!html.contains("--erreichbar-als"), "{html}");
    }

    /// Kriterium F0079 „Agent bekommt Text“ (US-8), Matrix ABW-T.
    #[test]
    fn der_text_nennt_name_adresse_ausweg_und_die_eigenen_namen() {
        let text = text(Some("boese.example"), Some(lan()));

        for teil in [
            "boese.example",
            "http://192.168.178.65:7430/",
            "--erreichbar-als boese.example",
            "Namen des eigenen Netzes gelten automatisch",
        ] {
            assert!(text.contains(teil), "{teil}: {text}");
        }
        assert!(!text.contains("<html"), "{text}");
        assert!(!text.contains('\n'), "{text}");
    }

    #[test]
    fn ohne_host_nennt_der_text_die_adresse() {
        let text = text(None, Some(lan()));

        assert!(text.contains("nennt keinen Host"), "{text}");
        assert!(text.contains("http://192.168.178.65:7430/"), "{text}");
    }

    #[test]
    fn ohne_lokale_adresse_bleibt_der_text_wie_vor_adr_0045() {
        assert_eq!(
            text(Some("chat.lan"), None),
            "Diese Instanz ist unter dem Namen chat.lan nicht freigegeben — über die IP-Adresse öffnen oder mit --erreichbar-als chat.lan starten."
        );
        assert_eq!(
            text(None, None),
            "Diese Anfrage nennt keinen Host — über die IP-Adresse öffnen."
        );
    }
}
