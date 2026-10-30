//! Frist-Meldungen als Einträge des Betriebsprotokolls (R00017, ADR-0030 Ergänzung „wie I0018 an das
//! Betriebsprotokoll anbindet“): rein, ohne Uhr und Protokoll. Kennungen nur im Kontext (ADR-0040 Regel 3).

use super::Kennung;
use super::direktgespraeche::ordner_fuer_protokoll;
use super::pflege::{Pflegeanlass, Pflegebericht, Pflegegespraech};
use crate::protokoll::{Bereich, Eintrag, Kontext, Schluessel, Stufe};
use chrono::NaiveDate;

/// Text der Zeile, wenn die Anhänge eines Tags nicht entfernt wurden (ADR-0046 Ergänzung „Review #53 B1“ Regel 4).
pub const ANHAENGE_NICHT_ENTFERNT: &str =
    "Anhänge eines Tags nicht entfernt — erneuter Versuch bei der nächsten Pflege";

/// Text der Zeile, wenn ein Tag wegen eines nicht entfernten Ziel-Tags bleibt (ADR-0042 Ergänzung „Karte #73“ (3)).
pub const TAG_ZURUECKGESTELLT: &str = "Tag zurückgestellt — erneuter Versuch beim nächsten Anlass";

/// Der Eintrag zu einem Pflegebericht: Entfallen als INFORMATION, ein nicht entfallener Tag und ein nicht
/// auflistbarer Ordner als ERROR, defekte Einstellungen als WARNING (R00017, Abbildung Zeilen 2–5; R00019). Im
/// Bereich `frist` ist `grund` immer der Anlass, ein Plattenfehler steht in `ursache` (R00019).
pub fn pflege_eintrag(bericht: &Pflegebericht) -> Eintrag {
    match bericht {
        Pflegebericht::Entfallen {
            gespraech,
            tage,
            frist,
            anlass,
            ..
        } => {
            let kontext = mit_anlass(
                bezug(Kontext::neu(), gespraech).mit(Schluessel::Anzahl, tage.len()),
                anlass,
            );
            let stufe = match gespraech {
                Pflegegespraech::Channel(_) => Stufe::Information,
                Pflegegespraech::Direkt(_) => Stufe::Debug,
            };
            Eintrag::neu(
                stufe,
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
            gespraech,
            datei,
            anlass,
            ursache,
        } => Eintrag::neu(
            Stufe::Error,
            Bereich::Frist,
            "Tag nicht entfallen — erneuter Versuch beim nächsten Anlass",
            mit_anlass(
                bezug(Kontext::neu(), gespraech).mit(Schluessel::Datei, datei),
                anlass,
            )
            .mit(Schluessel::Ursache, ursache),
        ),
        Pflegebericht::Zurueckgestellt {
            gespraech,
            datei,
            ziel,
        } => Eintrag::neu(
            Stufe::Debug,
            Bereich::Frist,
            TAG_ZURUECKGESTELLT,
            bezug(Kontext::neu(), gespraech)
                .mit(Schluessel::Datei, datei)
                .mit(Schluessel::Ziel, ziel.format("%Y-%m-%d")),
        ),
        Pflegebericht::AnhaengeNichtEntfernt {
            gespraech,
            tag,
            anlass,
            ursache,
        } => Eintrag::neu(
            Stufe::Error,
            Bereich::Frist,
            ANHAENGE_NICHT_ENTFERNT,
            mit_anlass(
                bezug(Kontext::neu(), gespraech).mit(Schluessel::Tag, tag.format("%Y-%m-%d")),
                anlass,
            )
            .mit(Schluessel::Ursache, ursache),
        ),
        Pflegebericht::TageNichtErmittelt {
            gespraech,
            ordner,
            anlass,
            ursache,
        } => Eintrag::neu(
            Stufe::Error,
            Bereich::Frist,
            "Tage nicht ermittelt — Ordner nicht lesbar; erneuter Versuch beim nächsten Anlass",
            mit_anlass(
                bezug(Kontext::neu(), gespraech).mit(Schluessel::Datei, ordner),
                anlass,
            )
            .mit(Schluessel::Ursache, ursache),
        ),
        Pflegebericht::OrdnerEntfernt { gespraech, anlass } => Eintrag::neu(
            Stufe::Debug,
            Bereich::Frist,
            "Leerer Ordner entfernt",
            mit_anlass(bezug(Kontext::neu(), gespraech), anlass),
        ),
        Pflegebericht::OrdnerNichtEntfernt {
            gespraech,
            anlass,
            ursache,
        } => Eintrag::neu(
            Stufe::Error,
            Bereich::Frist,
            "Leerer Ordner nicht entfernt — erneuter Versuch beim nächsten Anlass",
            mit_anlass(bezug(Kontext::neu(), gespraech), anlass).mit(Schluessel::Ursache, ursache),
        ),
        Pflegebericht::Gesperrt {
            gespraech,
            datei,
            zeile,
        } => {
            let kontext = bezug(Kontext::neu(), gespraech).mit(Schluessel::Datei, datei);
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

/// Das Gespräch einer Pflegezeile: `channel=` für einen Channel, `ordner=direkt/<paar>` für ein Direktgespräch
/// (ADR-0055 Regel 11; der Ordnername ist gekürzt und ohne Steuerzeichen).
fn bezug(kontext: Kontext, gespraech: &Pflegegespraech) -> Kontext {
    match gespraech {
        Pflegegespraech::Channel(name) => kontext.mit(Schluessel::Channel, name),
        Pflegegespraech::Direkt(paar) => kontext.mit(
            Schluessel::Ordner,
            ordner_fuer_protokoll(&paar.ordnername()),
        ),
    }
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
            gespraech: channel("allgemein").into(),
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

    /// ADR-0055 Regel 11: eine Pflegezeile eines Direktgesprächs nennt den Ordner, nie `channel=`.
    #[test]
    fn ein_direktgespraech_steht_mit_ordner_statt_channel_im_kontext() {
        let anna = Kennung::bilden("anna@10.0.0.7").expect("Kennung");
        let paar = crate::gespraeche::Direktpaar::bilden(&anna, &beta()).expect("Paar");
        let bericht = Pflegebericht::NichtEntfallen {
            gespraech: Pflegegespraech::Direkt(paar),
            datei: "direkt/anna@10.0.0.7+beta@127.0.0.1/2026-10-21.log".into(),
            anlass: Pflegeanlass::Start,
            ursache: "Permission denied (os error 13)".into(),
        };

        let zeile = ab_der_stufe(&pflege_eintrag(&bericht));

        assert!(
            zeile.contains(" | ordner=direkt/anna@10.0.0.7+beta@127.0.0.1 datei="),
            "{zeile}"
        );
        assert!(!zeile.contains("channel="), "{zeile}");
    }

    /// ADR-0055 Regel 11: das Entfallen von Tagen eines Direktgesprächs steht auf DEBUG (beim Channel INFORMATION) und
    /// nennt den Ordner — sonst läse jeder ohne Sitzung am Wachstum von `protokoll_bytes_heute` ab, wann ein Paar
    /// entfiel. Mutante: INFORMATION-Zeile beim Entfallen.
    #[test]
    fn das_entfallen_im_direktgespraech_ist_debug_mit_ordner() {
        let anna = Kennung::bilden("anna@10.0.0.7").expect("Kennung");
        let paar = crate::gespraeche::Direktpaar::bilden(&anna, &beta()).expect("Paar");
        let bericht = Pflegebericht::Entfallen {
            gespraech: Pflegegespraech::Direkt(paar),
            tage: vec![tag("2026-10-20"), tag("2026-10-21")],
            nachrichten: 5,
            frist: 7,
            anlass: Pflegeanlass::Tageswechsel,
        };

        assert_eq!(
            ab_der_stufe(&pflege_eintrag(&bericht)),
            "DEBUG frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) | ordner=direkt/anna@10.0.0.7+beta@127.0.0.1 anzahl=2 grund=tageswechsel"
        );
    }

    /// ADR-0055 Regel 11: das Entfernen eines leeren Paarordners steht auf DEBUG, ein gescheitertes als ERROR mit
    /// `ursache`; beide nennen den Ordner und den Anlass, nie einen Channel.
    #[test]
    fn das_entfernen_eines_leeren_paarordners_ist_debug_und_ein_fehler_ist_error() {
        let paar = || {
            let anna = Kennung::bilden("anna@10.0.0.7").expect("Kennung");
            crate::gespraeche::Direktpaar::bilden(&anna, &beta()).expect("Paar")
        };

        let entfernt = ab_der_stufe(&pflege_eintrag(&Pflegebericht::OrdnerEntfernt {
            gespraech: Pflegegespraech::Direkt(paar()),
            anlass: Pflegeanlass::Tageswechsel,
        }));
        let nicht = ab_der_stufe(&pflege_eintrag(&Pflegebericht::OrdnerNichtEntfernt {
            gespraech: Pflegegespraech::Direkt(paar()),
            anlass: Pflegeanlass::Start,
            ursache: "Permission denied (os error 13)".into(),
        }));

        assert_eq!(
            entfernt,
            "DEBUG frist Leerer Ordner entfernt | ordner=direkt/anna@10.0.0.7+beta@127.0.0.1 grund=tageswechsel"
        );
        assert_eq!(
            nicht,
            "ERROR frist Leerer Ordner nicht entfernt — erneuter Versuch beim nächsten Anlass | ordner=direkt/anna@10.0.0.7+beta@127.0.0.1 grund=start ursache=\"Permission denied (os error 13)\""
        );
    }

    fn nicht_entfallen(anlass: Pflegeanlass) -> Pflegebericht {
        Pflegebericht::NichtEntfallen {
            gespraech: channel("allgemein").into(),
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

    /// R00043 F0147 „Protokoll“ (ADR-0042 Ergänzung „Karte #73“ (3)): ein zurückgestellter Tag ist eine DEBUG-Zeile im
    /// Bereich `frist` mit `channel`, `datei` (der zurückgestellte Tag) und `ziel` (der Tag, der ihn hält), ohne Anlass.
    #[test]
    fn zurueckgestellt_ist_debug_mit_channel_datei_und_ziel() {
        let bericht = Pflegebericht::Zurueckgestellt {
            gespraech: channel("allgemein").into(),
            datei: "channels/allgemein/2026-10-27.log".into(),
            ziel: NaiveDate::from_ymd_opt(2026, 10, 26).expect("Tag"),
        };

        assert_eq!(
            ab_der_stufe(&pflege_eintrag(&bericht)),
            "DEBUG frist Tag zurückgestellt — erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein/2026-10-27.log ziel=2026-10-26"
        );
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
                gespraech: channel("allgemein").into(),
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
            gespraech: channel("kaputt").into(),
            datei: "channels/kaputt/einstellungen.toml".into(),
            zeile: Some(1),
        };
        let ohne_zeile = Pflegebericht::Gesperrt {
            gespraech: channel("zu").into(),
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
