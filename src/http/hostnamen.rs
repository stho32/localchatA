//! Host-Namen nach ADR-0045 — rein, ohne Netz: Normalform (Regel 1 mit Ergänzung E2) und Urteil über einen
//! Namen (Regeln 2, 3, 5).

/// Längste Länge eines Labels und eines ganzen Namens (ohne abschließenden Punkt), RFC 1035.
const LABEL_HOECHSTENS: usize = 63;
const NAME_HOECHSTENS: usize = 253;

/// Host-Anteil ohne Port → Normalform: ASCII klein, ein abschließender Punkt entfernt, nur `[a-z0-9.-]`, Labels
/// 1–63 Zeichen, gesamt höchstens 253. `None` heißt abgewiesen, ohne dass etwas aufgelöst wird — auch, wenn das
/// letzte Label eine Zahl ist (`2130706433`, `0x7f000001`, `127.1`): `getaddrinfo` läse ihn als IPv4, ein Browser
/// sendet ihn nie (E2). Internationalisierte Namen kommen als `xn--…` und bleiben so.
pub fn normalform(host_anteil: &str) -> Option<String> {
    let klein = host_anteil.to_ascii_lowercase();
    let name = klein.strip_suffix('.').unwrap_or(&klein);
    let zeichen_erlaubt = name.bytes().all(|zeichen| {
        zeichen.is_ascii_lowercase() || zeichen.is_ascii_digit() || b".-".contains(&zeichen)
    });
    let labels_erlaubt = name
        .split('.')
        .all(|label| (1..=LABEL_HOECHSTENS).contains(&label.len()));
    let laenge_erlaubt = name.len() <= NAME_HOECHSTENS;
    let letztes_label = name.rsplit('.').next().unwrap_or_default();
    (zeichen_erlaubt && labels_erlaubt && laenge_erlaubt && !ist_zahl(letztes_label))
        .then(|| name.to_string())
}

/// „ends in a number“ des WHATWG-URL-Standards: nur Ziffern, oder `0x`/`0X` gefolgt von höchstens Hexziffern.
fn ist_zahl(label: &str) -> bool {
    let hex = label
        .strip_prefix("0x")
        .or_else(|| label.strip_prefix("0X"));
    match hex {
        Some(ziffern) => ziffern.bytes().all(|zeichen| zeichen.is_ascii_hexdigit()),
        None => !label.is_empty() && label.bytes().all(|zeichen| zeichen.is_ascii_digit()),
    }
}

/// Endet `name` (in Normalform) auf `suffix`, und zwar an einer Label-Grenze? `evil-local` und
/// `local.evil.example` enden nicht auf `local`.
#[cfg_attr(not(test), allow(dead_code))] // vorübergehend: verdrahtet mit B0372
pub fn endet_auf_label(name: &str, suffix: &str) -> bool {
    name.strip_suffix(suffix)
        .is_some_and(|davor| davor.ends_with('.'))
}

/// Warum ein Host-Name erlaubt ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // vorübergehend: LokalerNamensraum und AlleNamen entstehen mit B0373 und B0377
pub enum Grund {
    /// Regel 2: `localhost`, Rechnername samt `.local`, `--erreichbar-als`.
    Fest,
    /// Regel 3: Rückwärtsname der angesprochenen Adresse.
    Rueckwaertsname,
    /// Regel 4: Name im lokalen Namensraum, der auf eine eigene Adresse zeigt.
    LokalerNamensraum,
    /// Regel 9: `--alle-namen`.
    AlleNamen,
}

/// Die beim Start festgelegten Namen aus Regel 2, klein und ohne abschließenden Punkt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FesteNamen {
    namen: Vec<String>,
}

impl FesteNamen {
    /// `localhost`, der Rechnername und `<rechnername>.local` (falls es einen gibt) und jeder Name aus
    /// `--erreichbar-als`; leere Namen gelten nicht.
    pub fn neu(rechnername: Option<&str>, erreichbar_als: &[String]) -> FesteNamen {
        let rechner = rechnername
            .into_iter()
            .flat_map(|name| [name.to_string(), format!("{name}.local")]);
        let namen = std::iter::once("localhost".to_string())
            .chain(rechner)
            .chain(erreichbar_als.iter().cloned())
            .map(|name| klein_ohne_punkt(&name))
            .filter(|name| !name.is_empty() && name != ".local")
            .collect();
        FesteNamen { namen }
    }

    fn enthaelt(&self, name: &str) -> bool {
        self.namen.iter().any(|fest| fest == name)
    }
}

fn klein_ohne_punkt(name: &str) -> String {
    let klein = name.to_ascii_lowercase();
    klein.strip_suffix('.').unwrap_or(&klein).to_string()
}

/// Urteil über einen Namen in Normalform nach Regeln 2, 3 und 5: fest erlaubt, sonst erlaubt, wenn er gleich
/// einem Rückwärtsnamen der angesprochenen Adresse ist (beide in Normalform), sonst nicht (`None`). Die
/// Rückwärtsnamen werden nur geholt, wenn die feste Liste scheitert; vorwärts wird hier nie aufgelöst.
pub fn host_urteil(
    name: &str,
    feste: &FesteNamen,
    rueckwaertsnamen: impl FnOnce() -> Vec<String>,
) -> Option<Grund> {
    if feste.enthaelt(name) {
        return Some(Grund::Fest);
    }
    let ist_rueckwaertsname = rueckwaertsnamen()
        .iter()
        .filter_map(|rueckwaerts| normalform(rueckwaerts))
        .any(|rueckwaerts| rueckwaerts == name);
    ist_rueckwaertsname.then_some(Grund::Rueckwaertsname)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::namensauskunft::{Namensauskunft, Tabelle};
    use std::net::IpAddr;

    #[test]
    fn grossbuchstaben_werden_klein_und_ein_abschliessender_punkt_faellt() {
        assert_eq!(normalform("Chat.LAN.").as_deref(), Some("chat.lan"));
        assert_eq!(
            normalform("NUC5.Fritz.Box").as_deref(),
            Some("nuc5.fritz.box")
        );
        assert_eq!(normalform("localhost.").as_deref(), Some("localhost"));
        assert_eq!(normalform("chat").as_deref(), Some("chat"));
    }

    #[test]
    fn punycode_bleibt_unveraendert() {
        assert_eq!(
            normalform("xn--bcher-kva.example").as_deref(),
            Some("xn--bcher-kva.example")
        );
        assert_eq!(
            normalform("XN--BCHER-KVA.local").as_deref(),
            Some("xn--bcher-kva.local")
        );
    }

    /// Matrix D2, D3, D8: Zeichen außerhalb `[a-z0-9.-]` und leere Labels.
    #[test]
    fn zeichen_ausserhalb_und_leere_labels_werden_abgewiesen() {
        for name in [
            "a_b.example",
            "x_y.local",
            "x%2elocal",
            "a@nuc5.local",
            "nuc5 .local",
            "nuc5\t.local",
            "<img src=x onerror=alert(1)>",
            "bücher.local",
            "a..b",
            "a..local",
            ".local",
            "nuc5.local..",
            ".",
            "",
        ] {
            assert_eq!(normalform(name), None, "{name:?}");
        }
    }

    /// Matrix D5–D7: Label 63 ja, 64 nein; Name 253 ja, 254 nein (ohne Punkt am Ende).
    #[test]
    fn grenzen_label_63_und_name_253() {
        let label_63 = format!("{}.local", "a".repeat(63));
        let label_64 = format!("{}.local", "a".repeat(64));
        assert_eq!(normalform(&label_63).as_deref(), Some(label_63.as_str()));
        assert_eq!(normalform(&label_64), None);

        let name_253 = format!(
            "{}.{}.{}.{}.local",
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(55)
        );
        assert_eq!(name_253.len(), 253);
        assert_eq!(normalform(&name_253).as_deref(), Some(name_253.as_str()));
        assert_eq!(
            normalform(&format!("{name_253}.")).as_deref(),
            Some(name_253.as_str())
        );
        let name_254 = format!("e{name_253}");
        assert_eq!(name_254.len(), 254);
        assert_eq!(normalform(&name_254), None);
    }

    /// E2, Matrix C13, C14: das letzte Label ist eine Zahl — abgewiesen ohne Auflösung.
    #[test]
    fn numerische_namen_werden_abgewiesen() {
        for name in [
            "2130706433",
            "0x7f000001",
            "0X7F000001",
            "127.1",
            "1.2.3.4.5",
            "0x",
            "nuc5.0x1f",
        ] {
            assert_eq!(normalform(name), None, "{name}");
        }
        for name in ["a1.local", "1a.local", "0xg1", "x.0xg"] {
            assert!(normalform(name).is_some(), "{name}");
        }
    }

    #[test]
    fn suffixe_nur_an_label_grenzen() {
        assert!(endet_auf_label("chat.local", "local"));
        assert!(endet_auf_label("nas.home.arpa", "home.arpa"));
        assert!(!endet_auf_label("evil-local", "local"));
        assert!(!endet_auf_label("local.evil.example", "local"));
        assert!(!endet_auf_label("local", "local"));
        assert!(!endet_auf_label("xlocal", "local"));
    }

    fn feste() -> FesteNamen {
        FesteNamen::neu(Some("nuc5"), &["chat.lan".to_string(), "Nuc.".to_string()])
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("Adresse")
    }

    /// Urteil über einen Host-Namen, der über `lokal` kam, mit der Tabelle als Namensauskunft.
    fn urteil(tabelle: &Tabelle, host: &str, lokal: &str) -> Option<Grund> {
        let name = normalform(host).expect("Normalform");
        host_urteil(&name, &feste(), || tabelle.namen_der_adresse(ip(lokal)))
    }

    /// Erster Test aus A00004 (Kriterium „Host-Urteil“): öffentlich nie vorwärts gefragt, Rückwärtsname erlaubt.
    #[test]
    fn oeffentlicher_name_nie_vorwaerts_gefragt_rueckwaertsname_erlaubt() {
        let tabelle =
            Tabelle::aus_wert("192.168.178.65>nuc5.fritz.box;boese.example=192.168.178.65")
                .expect("Tabelle");

        assert_eq!(urteil(&tabelle, "boese.example", "192.168.178.65"), None);
        assert_eq!(tabelle.fragen("vorwaerts", "boese.example"), 0);

        assert_eq!(
            urteil(&tabelle, "nuc5.fritz.box", "192.168.178.65"),
            Some(Grund::Rueckwaertsname)
        );
        assert_eq!(
            urteil(&tabelle, "nuc5.fritz.box.", "192.168.178.65"),
            Some(Grund::Rueckwaertsname)
        );
        assert_eq!(
            urteil(&tabelle, "NUC5.Fritz.Box", "192.168.178.65"),
            Some(Grund::Rueckwaertsname)
        );
        assert_eq!(urteil(&tabelle, "nuc5.fritz.box", "127.0.0.1"), None);
        assert_eq!(tabelle.fragen("vorwaerts", "nuc5.fritz.box"), 0);
    }

    /// Regel 2: ohne Frage an die Auskunft erlaubt.
    #[test]
    fn feste_namen_fragen_nicht_rueckwaerts() {
        let tabelle = Tabelle::default();
        for host in [
            "localhost",
            "LocalHost.",
            "nuc5",
            "NUC5.local",
            "nuc5.local.",
            "chat.lan",
            "nuc",
        ] {
            assert_eq!(
                urteil(&tabelle, host, "127.0.0.1"),
                Some(Grund::Fest),
                "{host}"
            );
        }
        assert_eq!(tabelle.fragen("rueckwaerts", "127.0.0.1"), 0);
    }

    /// Matrix A1–A3, A6–A8, B2, B4, B5: nur der Rückwärtsname selbst, kein Suffix, keine Verlängerung.
    #[test]
    fn nur_der_rueckwaertsname_der_angesprochenen_adresse_gilt() {
        let tabelle = Tabelle::aus_wert(
            "127.0.0.2>nuc5.fritz.box;127.0.0.1>localhost;boese.example=127.0.0.1;andere.fritz.box=127.0.0.2;chat.lan=127.0.0.1",
        )
        .expect("Tabelle");
        for (host, lokal) in [
            ("boese.example", "127.0.0.1"),
            ("boese.example", "127.0.0.2"),
            ("boese.example", "::1"),
            ("andere.fritz.box", "127.0.0.2"),
            ("fritz.box", "127.0.0.2"),
            ("chat.lan.boese.example", "127.0.0.1"),
            ("nuc5.fritz.box", "127.0.0.1"),
            ("nuc5.fritz.box.boese.example", "127.0.0.2"),
            ("nuc5.fritz", "127.0.0.2"),
            ("nuc5.fritz.box", "127.0.0.3"),
            ("rebind.boese.example", "127.0.0.1"),
            ("localtest.me", "127.0.0.1"),
            ("127.0.0.1.nip.io", "127.0.0.1"),
            ("x.localhost", "127.0.0.1"),
        ] {
            assert_eq!(urteil(&tabelle, host, lokal), None, "{host} über {lokal}");
            assert_eq!(tabelle.fragen("vorwaerts", host), 0, "{host}");
        }
        assert_eq!(
            urteil(&tabelle, "nuc5.fritz.box", "127.0.0.2"),
            Some(Grund::Rueckwaertsname)
        );
    }

    /// Matrix B3: Normalform beidseitig — der Rückwärtsname kommt groß und mit Punkt.
    #[test]
    fn der_rueckwaertsname_wird_ebenfalls_normalisiert() {
        let tabelle = Tabelle::aus_wert("127.0.0.2>CHAT.fritz.box.;127.0.0.3>a_b.fritz.box")
            .expect("Tabelle");

        assert_eq!(
            urteil(&tabelle, "Chat.Fritz.Box.", "127.0.0.2"),
            Some(Grund::Rueckwaertsname)
        );
        assert_eq!(urteil(&tabelle, "a.fritz.box", "127.0.0.3"), None);
    }

    #[test]
    fn ohne_rechnername_und_mit_leeren_freigaben_gelten_nur_localhost() {
        let feste = FesteNamen::neu(None, &[String::new(), ".".to_string()]);
        let keine = || Vec::new();
        assert_eq!(host_urteil("localhost", &feste, keine), Some(Grund::Fest));
        assert_eq!(host_urteil("local", &feste, keine), None);
        assert_eq!(host_urteil("nuc5", &feste, keine), None);
    }
}
