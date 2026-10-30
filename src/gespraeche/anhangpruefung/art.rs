//! Art einer Datei, wie die Instanz sie bestimmt (ADR-0047 Frage 3): Inhaltstyp, Disposition und — nur bei
//! Bildern — die angezeigten Maße.

/// Inhaltstyp für alles, was nicht dargestellt wird.
pub const DATEI: &str = "application/octet-stream";

/// Die eine Stelle der Disposition (R00029 „Disposition“, Befund #53 A1): `inline` für alle Arten der Tabelle außer
/// `application/octet-stream`, HEIC und HEIF (Download, ADR-0047 Ergänzung „Karte #9 Nacharbeit F3 und F4“),
/// `video/ogg` (Theora spielt im Browser nur den Ton, ADR-0014 Ergänzung „Karte #71 (Chrome 154)“ Regel 5) und Text
/// über 16 MiB — gilt beim Erkennen wie beim Ausliefern.
pub fn inline(typ: &str, bytes: u64) -> bool {
    let nur_laden = [
        DATEI,
        super::medien::HEIC_TYP,
        super::medien::HEIF_TYP,
        "video/ogg",
    ]
    .contains(&typ);
    let grosser_text = typ == super::erkenner::TEXT && bytes > super::text::TEXT_INLINE_HOECHSTENS;
    !(nur_laden || grosser_text)
}

/// Die Medienarten, deren Direktlink im Medien-Dokument des Browsers abspielt (ADR-0014 Ergänzung „Karte #71
/// (Chrome 154)“ Regel 5): nur ihre Antworten bekommen den Marker für die Medien-CSP. `video/ogg` steht nicht darin,
/// es wird geladen. Eine neue Art der Tabelle ohne Eintrag hier spielt nicht (fail-closed).
pub const MEDIENARTEN: [&str; 5] = [
    "video/mp4",
    "audio/mp4",
    "video/webm",
    "audio/ogg",
    "audio/mpeg",
];

/// Ob die vom Server bestimmte Art eine der Medienarten ist — exakt, ohne Präfix oder Parameter.
pub fn medienart(typ: &str) -> bool {
    MEDIENARTEN.contains(&typ)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Art {
    /// Inhaltstyp aus der Tabelle der ADR-0047, z. B. `image/png` oder `text/plain; charset=utf-8`.
    pub typ: &'static str,
    /// `inline` oder `attachment`.
    pub inline: bool,
    pub breite: Option<u32>,
    pub hoehe: Option<u32>,
}

impl Art {
    /// Der sichere Ausgang: eine Datei zum Laden, ohne Maße.
    pub fn datei() -> Art {
        Art {
            typ: DATEI,
            inline: false,
            breite: None,
            hoehe: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::erkenner::ARTEN;
    use super::*;

    /// B0663, B0728: über alle Arten der Tabelle (ADR-0047 Frage 3) tragen genau diese fünf den Marker (ADR-0014
    /// Ergänzung „Karte #71 (Chrome 154)“ Regel 5) — `video/ogg` nicht, eine neue Art nicht ohne Entscheidung.
    #[test]
    fn genau_die_fuenf_medienarten_der_tabelle_bekommen_den_marker() {
        let fuenf = [
            "video/mp4",
            "audio/mp4",
            "video/webm",
            "audio/ogg",
            "audio/mpeg",
        ];
        for typ in ARTEN {
            assert_eq!(medienart(typ), fuenf.contains(&typ), "{typ}");
        }
        assert_eq!(ARTEN.iter().filter(|typ| medienart(typ)).count(), 5);
        assert!(ARTEN.contains(&"video/ogg"));
    }

    /// B0728 (Regel 5, O1): `video/ogg` wird geladen (`attachment`), die fünf Medienarten bleiben `inline`.
    #[test]
    fn video_ogg_wird_geladen_die_medienarten_spielen() {
        assert!(!inline("video/ogg", 1_000));
        for typ in MEDIENARTEN {
            assert!(inline(typ, 1_000), "{typ}");
        }
    }

    #[test]
    fn nur_die_exakte_art_ist_medienart() {
        for typ in [
            "VIDEO/MP4",
            "video/mp4; codecs=avc1",
            " video/mp4",
            "video/",
            "video/x-matroska",
            "audio/wav",
            "",
        ] {
            assert!(!medienart(typ), "{typ}");
        }
    }
}
