//! Frist-Meldungen als Einträge des Betriebsprotokolls (R00017, ADR-0030 Ergänzung „wie I0018 an das
//! Betriebsprotokoll anbindet“): rein, ohne Uhr und Protokoll. Kennungen nur im Kontext (ADR-0040 Regel 3).

use super::Kennung;
use super::pflege::{Pflegeanlass, Pflegebericht};
use crate::protokoll::{Bereich, Eintrag, Kontext, Schluessel, Stufe};
use chrono::NaiveDate;

/// Text der Zeile, wenn die Anhänge eines Tags nicht entfernt wurden (ADR-0046 Ergänzung „Review #53 B1“ Regel 4).
pub const ANHAENGE_NICHT_ENTFERNT: &str =
    "Anhänge eines Tags nicht entfernt — erneuter Versuch bei der nächsten Pflege";

/// Der Eintrag zu einem Pflegebericht: Entfallen als INFORMATION, ein nicht entfallener Tag und ein nicht
/// auflistbarer Ordner als ERROR, defekte Einstellungen als WARNING (R00017, Abbildung Zeilen 2–5; R00019). Im
/// Bereich `frist` ist `grund` immer der Anlass, ein Plattenfehler steht in `ursache` (R00019).
pub fn pflege_eintrag(bericht: &Pflegebericht) -> Eintrag {
    match bericht {
        Pflegebericht::Entfallen {
            channel,
            tage,
            frist,
            anlass,
            ..
        } => {
            let kontext = mit_anlass(
                Kontext::neu()
                    .mit(Schluessel::Channel, channel)
                    .mit(Schluessel::Anzahl, tage.len()),
                anlass,
            );
            Eintrag::neu(
                Stufe::Information,
                Bereich::Frist,
                format!(
                    "{} entfallen ({}; Frist {})",
                    tage_text(tage.len()),
                    spanne(tage),
                    tage_text(*frist as usize)
                ),
                kontext,
            )
        }
        Pflegebericht::NichtEntfallen {
            channel,
            datei,
            anlass,
            ursache,
        } => Eintrag::neu(
            Stufe::Error,
            Bereich::Frist,
            "Tag nicht entfallen — erneuter Versuch beim nächsten Anlass",
            mit_anlass(
                Kontext::neu()
                    .mit(Schluessel::Channel, channel)
                    .mit(Schluessel::Datei, datei),
                anlass,
            )
            .mit(Schluessel::Ursache, ursache),
        ),
        Pflegebericht::AnhaengeNichtEntfernt {
            channel,
            tag,
            anlass,
            ursache,
        } => Eintrag::neu(
            Stufe::Error,
            Bereich::Frist,
            ANHAENGE_NICHT_ENTFERNT,
            mit_anlass(
                Kontext::neu()
                    .mit(Schluessel::Channel, channel)
                    .mit(Schluessel::Tag, tag.format("%Y-%m-%d")),
                anlass,
            )
            .mit(Schluessel::Ursache, ursache),
        ),
        Pflegebericht::TageNichtErmittelt {
            channel,
            ordner,
            anlass,
            ursache,
        } => Eintrag::neu(
            Stufe::Error,
            Bereich::Frist,
            "Tage nicht ermittelt — Ordner nicht lesbar; erneuter Versuch beim nächsten Anlass",
            mit_anlass(
                Kontext::neu()
                    .mit(Schluessel::Channel, channel)
                    .mit(Schluessel::Datei, ordner),
                anlass,
            )
            .mit(Schluessel::Ursache, ursache),
        ),
        Pflegebericht::Gesperrt {
            channel,
            datei,
            zeile,
        } => {
            let kontext = Kontext::neu()
                .mit(Schluessel::Channel, channel)
                .mit(Schluessel::Datei, datei);
            let kontext = match zeile {
                Some(zeile) => kontext.mit(Schluessel::Zeile, zeile),
                None => kontext,
            };
            Eintrag::neu(
                Stufe::Warning,
                Bereich::Frist,
                "Nichts entfällt — Einstellungen defekt",
                kontext,
            )
        }
    }
}

/// `INFORMATION frist Frist <alt> → <neu> Tage gesetzt | channel= kennung=` (Abbildung Zeile 1).
pub fn frist_gesetzt_eintrag(channel: &str, alt: u32, neu: u32, von: &Kennung) -> Eintrag {
    Eintrag::neu(
        Stufe::Information,
        Bereich::Frist,
        format!("Frist {alt} → {} gesetzt", tage_text(neu as usize)),
        Kontext::neu()
            .mit(Schluessel::Channel, channel)
            .mit(Schluessel::Kennung, von),
    )
}

/// `ERROR frist Frist nicht gesetzt — … | channel= kennung= datei= ursache=` (Abbildung Zeile 6): ohne Anlass,
/// darum ohne `grund`; der Plattenfehler steht in `ursache` (ADR-0040 Ergänzung R00019, Option B).
pub fn frist_nicht_gesetzt_eintrag(
    channel: &str,
    von: &Kennung,
    datei: &str,
    ursache: &str,
) -> Eintrag {
    nicht_gesetzt(
        "Frist nicht gesetzt — Einstellungen ließen sich nicht schreiben",
        channel,
        von,
        datei,
        ursache,
    )
}

/// `ERROR frist Frist nicht gesetzt — Tage nicht ermittelt | channel= kennung= datei= ursache=` (ADR-0042
/// Ergänzung R00019 Punkt 1): verkürzt ohne Bestätigung, der Ordner ließ sich nicht auflisten.
pub fn tage_nicht_ermittelt_eintrag(
    channel: &str,
    von: &Kennung,
    ordner: &str,
    ursache: &str,
) -> Eintrag {
    nicht_gesetzt(
        "Frist nicht gesetzt — Tage nicht ermittelt",
        channel,
        von,
        ordner,
        ursache,
    )
}

/// Die eine Form der beiden „Frist nicht gesetzt“: ERROR mit Channel, Kennung, Datei und `ursache`.
fn nicht_gesetzt(text: &str, channel: &str, von: &Kennung, datei: &str, ursache: &str) -> Eintrag {
    Eintrag::neu(
        Stufe::Error,
        Bereich::Frist,
        text,
        Kontext::neu()
            .mit(Schluessel::Channel, channel)
            .mit(Schluessel::Kennung, von)
            .mit(Schluessel::Datei, datei)
            .mit(Schluessel::Ursache, ursache),
    )
}

/// Der Anlass in `grund`, beim Setzen dazu die Kennung dessen, der gesetzt hat (ADR-0040 Ergänzung R00019).
fn mit_anlass(kontext: Kontext, anlass: &Pflegeanlass) -> Kontext {
    let kontext = kontext.mit(Schluessel::Grund, grund(anlass));
    match anlass {
        Pflegeanlass::FristGesetzt { von } => kontext.mit(Schluessel::Kennung, von),
        Pflegeanlass::Start | Pflegeanlass::Tageswechsel => kontext,
    }
}

/// Der Anlass als Wert von `grund`.
fn grund(anlass: &Pflegeanlass) -> &'static str {
    match anlass {
        Pflegeanlass::Start => "start",
        Pflegeanlass::Tageswechsel => "tageswechsel",
        Pflegeanlass::FristGesetzt { .. } => "frist_gesetzt",
    }
}

/// „1 Tag“, sonst „n Tage“.
fn tage_text(anzahl: usize) -> String {
    if anzahl == 1 {
        "1 Tag".to_string()
    } else {
        format!("{anzahl} Tage")
    }
}

/// Erster und letzter Tag („von bis bis“), ein einzelner Tag allein.
fn spanne(tage: &[NaiveDate]) -> String {
    match (tage.first(), tage.last()) {
        (Some(erster), Some(letzter)) if erster != letzter => format!("{erster} bis {letzter}"),
        (Some(erster), _) => erster.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gespraeche::Pflegeanlass;
    use crate::name::Channelname;
    use crate::protokoll::zeile::zeile;
    use crate::uhr::Zeit;
    use chrono::NaiveDate;

    /// Die Zeile ab der Stufe, ohne Zeitstempel und Zeilenende.
    fn ab_der_stufe(eintrag: &Eintrag) -> String {
        let zeit = Zeit::parse_from_rfc3339("2026-10-28T12:00:00.000+01:00").expect("Zeit");
        let zeile = zeile(eintrag, zeit);
        zeile
            .trim_end_matches('\n')
            .split_once(' ')
            .expect("Zeitstempel")
            .1
            .to_string()
    }

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    fn beta() -> Kennung {
        Kennung::bilden("beta@127.0.0.1").expect("Kennung")
    }

    fn channel(name: &str) -> Channelname {
        Channelname::bilden(name).expect("Name")
    }

    fn entfallen(tage: &[&str], frist: u32, anlass: Pflegeanlass) -> Pflegebericht {
        Pflegebericht::Entfallen {
            channel: channel("allgemein"),
            tage: tage.iter().map(|text| tag(text)).collect(),
            nachrichten: 5,
            frist,
            anlass,
        }
    }

    /// R00017 „Einträge bilden“, Zeile 1 der Abbildung.
    #[test]
    fn frist_gesetzt_ist_information_mit_kennung_im_kontext() {
        assert_eq!(
            ab_der_stufe(&frist_gesetzt_eintrag("allgemein", 7, 30, &beta())),
            "INFORMATION frist Frist 7 → 30 Tage gesetzt | channel=allgemein kennung=beta@127.0.0.1"
        );
        assert_eq!(
            ab_der_stufe(&frist_gesetzt_eintrag("allgemein", 30, 1, &beta())),
            "INFORMATION frist Frist 30 → 1 Tag gesetzt | channel=allgemein kennung=beta@127.0.0.1"
        );
    }

    /// Zeilen 2 und 3: der Anlass steht in `grund`, die Kennung nur bei `frist_gesetzt`.
    #[test]
    fn entfallen_ist_information_mit_spanne_frist_anzahl_und_grund() {
        assert_eq!(
            ab_der_stufe(&pflege_eintrag(&entfallen(
                &["2026-10-20", "2026-10-21"],
                7,
                Pflegeanlass::Start
            ))),
            "INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) | channel=allgemein anzahl=2 grund=start"
        );
        assert_eq!(
            ab_der_stufe(&pflege_eintrag(&entfallen(
                &["2026-10-22"],
                7,
                Pflegeanlass::Tageswechsel
            ))),
            "INFORMATION frist 1 Tag entfallen (2026-10-22; Frist 7 Tage) | channel=allgemein anzahl=1 grund=tageswechsel"
        );
        assert_eq!(
            ab_der_stufe(&pflege_eintrag(&entfallen(
                &["2026-10-20", "2026-10-21"],
                1,
                Pflegeanlass::FristGesetzt { von: beta() }
            ))),
            "INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 1 Tag) | channel=allgemein anzahl=2 grund=frist_gesetzt kennung=beta@127.0.0.1"
        );
    }

    fn nicht_entfallen(anlass: Pflegeanlass) -> Pflegebericht {
        Pflegebericht::NichtEntfallen {
            channel: channel("allgemein"),
            datei: "channels/allgemein/2026-10-21.log".into(),
            anlass,
            ursache: "Permission denied (os error 13)".into(),
        }
    }

    /// Zeile 4 mit R00019 F0064 „Einträge mit Anlass“ (ADR-0040 Ergänzung R00019, Option B): je nicht entfallenem
    /// Tag ein ERROR, der Anlass in `grund`, bei `frist_gesetzt` die Kennung, der Plattenfehler in `ursache`.
    #[test]
    fn nicht_entfallen_traegt_den_anlass_in_grund_und_den_plattenfehler_in_ursache() {
        let anfang = "ERROR frist Tag nicht entfallen — erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein/2026-10-21.log";
        let ursache = "ursache=\"Permission denied (os error 13)\"";
        for (anlass, kontext) in [
            (Pflegeanlass::Start, "grund=start".to_string()),
            (Pflegeanlass::Tageswechsel, "grund=tageswechsel".to_string()),
            (
                Pflegeanlass::FristGesetzt { von: beta() },
                "grund=frist_gesetzt kennung=beta@127.0.0.1".to_string(),
            ),
        ] {
            assert_eq!(
                ab_der_stufe(&pflege_eintrag(&nicht_entfallen(anlass))),
                format!("{anfang} {kontext} {ursache}")
            );
        }
    }

    /// R00019 F0063 „Pflege“ und F0064 „Einträge mit Anlass“ (ADR-0042 Ergänzung R00019 Punkt 1): ein nicht
    /// auflistbarer Ordner ist ein eigener ERROR mit dem Ordner als `datei`, dem Anlass in `grund` und `ursache`.
    #[test]
    fn tage_nicht_ermittelt_ist_error_mit_ordner_anlass_und_ursache() {
        let anfang = "ERROR frist Tage nicht ermittelt — Ordner nicht lesbar; erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein";
        let ursache = "ursache=\"Permission denied (os error 13)\"";
        for (anlass, kontext) in [
            (Pflegeanlass::Start, "grund=start".to_string()),
            (Pflegeanlass::Tageswechsel, "grund=tageswechsel".to_string()),
            (
                Pflegeanlass::FristGesetzt { von: beta() },
                "grund=frist_gesetzt kennung=beta@127.0.0.1".to_string(),
            ),
        ] {
            let bericht = Pflegebericht::TageNichtErmittelt {
                channel: channel("allgemein"),
                ordner: "channels/allgemein".into(),
                anlass,
                ursache: "Permission denied (os error 13)".into(),
            };
            let eintrag = pflege_eintrag(&bericht);
            assert!(!eintrag.text.contains('@'), "{}", eintrag.text);
            assert_eq!(
                ab_der_stufe(&eintrag),
                format!("{anfang} {kontext} {ursache}")
            );
        }
    }

    /// Zeile 5: defekte Einstellungen als WARNING mit Datei und Zeile; ohne Zeile (verweigert) ohne `zeile=`.
    #[test]
    fn gesperrt_ist_warning_mit_datei_und_zeile() {
        let mit_zeile = Pflegebericht::Gesperrt {
            channel: channel("kaputt"),
            datei: "channels/kaputt/einstellungen.toml".into(),
            zeile: Some(1),
        };
        let ohne_zeile = Pflegebericht::Gesperrt {
            channel: channel("zu"),
            datei: "channels/zu/einstellungen.toml".into(),
            zeile: None,
        };

        assert_eq!(
            ab_der_stufe(&pflege_eintrag(&mit_zeile)),
            "WARNING frist Nichts entfällt — Einstellungen defekt | channel=kaputt datei=channels/kaputt/einstellungen.toml zeile=1"
        );
        assert_eq!(
            ab_der_stufe(&pflege_eintrag(&ohne_zeile)),
            "WARNING frist Nichts entfällt — Einstellungen defekt | channel=zu datei=channels/zu/einstellungen.toml"
        );
    }

    /// Zeile 6 mit R00019 F0064 „Schreibfehler mit `ursache`“ (ADR-0040 Ergänzung R00019, erster Test): der
    /// Schreibfehler beim Setzen als ERROR mit Kennung, Datei und `ursache`, ohne `grund` (kein Anlass).
    #[test]
    fn frist_nicht_gesetzt_ist_error_mit_kennung_datei_und_ursache_ohne_grund() {
        assert_eq!(
            ab_der_stufe(&frist_nicht_gesetzt_eintrag(
                "allgemein",
                &beta(),
                "channels/allgemein/einstellungen.toml",
                "Permission denied (os error 13)"
            )),
            "ERROR frist Frist nicht gesetzt — Einstellungen ließen sich nicht schreiben | channel=allgemein kennung=beta@127.0.0.1 datei=channels/allgemein/einstellungen.toml ursache=\"Permission denied (os error 13)\""
        );
    }

    /// R00019 F0063 „Verkürzen bei nicht lesbarem Ordner“: eigener Text, Ordner als `datei`, ohne `grund`.
    #[test]
    fn tage_nicht_ermittelt_beim_setzen_ist_error_mit_kennung_ordner_und_ursache() {
        assert_eq!(
            ab_der_stufe(&tage_nicht_ermittelt_eintrag(
                "allgemein",
                &beta(),
                "channels/allgemein",
                "Permission denied (os error 13)"
            )),
            "ERROR frist Frist nicht gesetzt — Tage nicht ermittelt | channel=allgemein kennung=beta@127.0.0.1 datei=channels/allgemein ursache=\"Permission denied (os error 13)\""
        );
    }

    /// ADR-0040 Regel 3: die Kennung steht nie im Text.
    #[test]
    fn kein_text_traegt_eine_kennung() {
        let eintraege = [
            frist_gesetzt_eintrag("allgemein", 7, 30, &beta()),
            frist_nicht_gesetzt_eintrag("allgemein", &beta(), "d", "g"),
            tage_nicht_ermittelt_eintrag("allgemein", &beta(), "d", "g"),
            pflege_eintrag(&entfallen(
                &["2026-10-20"],
                7,
                Pflegeanlass::FristGesetzt { von: beta() },
            )),
            pflege_eintrag(&nicht_entfallen(Pflegeanlass::FristGesetzt { von: beta() })),
        ];
        for eintrag in eintraege {
            assert!(!eintrag.text.contains('@'), "{}", eintrag.text);
            assert_eq!(
                eintrag.kontext.wert(Schluessel::Kennung),
                Some("beta@127.0.0.1")
            );
        }
    }
}
