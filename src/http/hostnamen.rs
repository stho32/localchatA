//! Host-Namen nach ADR-0045 — rein, ohne Netz: Normalform (Regel 1 mit Ergänzung E2).

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
pub fn endet_auf_label(name: &str, suffix: &str) -> bool {
    name.strip_suffix(suffix)
        .is_some_and(|davor| davor.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
