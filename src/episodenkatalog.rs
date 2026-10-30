//! Der ausführbare Katalog der Episoden (ADR-0050 Regel 6): eine Zeile je Zeile der Tabelle in ADR-0042, Ergänzung
//! „Karte #70“ (Quelle, Beginn, Ende). Die Erwartung kommt aus dieser Tabelle, nicht aus dem Code: wer eine
//! Episode baut oder umbaut, schreibt die Tabelle und diese Zeile — sonst ist ein Test rot.
//!
//! Je Zeile bauen drei Aufrufe dieselbe Bauweise: A und B mit **gleichen Quellargumenten** und **paarweise
//! verschiedenen** übrigen Argumenten, C mit einer anderen Quelle. Geprüft wird über `Episoden` mit gestellter Uhr:
//!
//! - E1: A und B ergeben genau eine Beginn-Zeile, C eine zweite (nicht bei Episoden der Instanz ohne Quellwerte);
//! - E2: Zwischen- und Ende-Zeile enthalten keinen Einzelwert von A oder B und jeden Quellwert;
//! - E3: die Schlüssel der Beginn-Zeile sind Quelle plus Beginn-Spalte, die der Ende-Zeile Quelle plus Ende-Spalte;
//! - E4: Fortlaufendes trägt im Ende den Wert von B.
//!
//! Dazu die Vollständigkeit: die Zahl der Stellen `Auftreten::neu(` im Quelltext ist gleich der Zahl der
//! verschiedenen Bauweise-Funktionen in diesem Katalog.

use crate::aktivitaet::Grenze as AktivitaetGrenze;
use crate::gespraeche::anhangeingang::Verfallen;
use crate::gespraeche::anhangid::Anhangid;
use crate::gespraeche::anhangquote::{Platzfehler, Quotestand};
use crate::gespraeche::wiederholung::{Grenze as Wiederholgrenze, Verdraengt};
use crate::gespraeche::{Anmeldefehler, Kennung};
use crate::http::aktivitaet::gebremst_auftreten as aktivitaet_gebremst_auftreten;
use crate::http::anhaenge::anmelden::{abweisung_auftreten, anspruch_auftreten};
use crate::http::anhaenge::hochladen::{metadaten_auftreten, zu_langsam_auftreten};
use crate::http::auskunft::{Abfrage, zu_langsam_auftreten as namensaufloesung_auftreten};
use crate::http::bearbeiten::{
    fremd_auftreten, schreibfehler_auftreten as aenderung_nicht_geschrieben_auftreten,
};
use crate::http::bremse::{Gegenstand, gebremst_auftreten};
use crate::http::ereignisse::ueberholt_auftreten;
use crate::http::grenzschutz::abweisung_auftreten as grenzschutz_auftreten;
use crate::http::koerper::{Koerperort, koerper_auftreten};
use crate::http::nachholplatz::gebremst_auftreten as nachholen_gebremst_auftreten;
use crate::http::senden::{quote_auftreten, schreibfehler_auftreten};
use crate::http::teilnehmer::Teilnehmer;
use crate::http::warten::gebremst_auftreten as warten_gebremst_auftreten;
use crate::http::ziel::Ziel;
use crate::name::Name;
use crate::protokoll::episoden::{Auftreten, Episoden, RUHE};
use crate::protokoll::episodentexte as texte;
use crate::protokoll::grenze::{Gegriffen, Grenzart, grenzauftreten};
use crate::protokoll::testhilfe::{im_speicher, zeit};
use crate::protokoll::vorlauf::vorlauf_auftreten;
use crate::protokoll::{Bereich, Eintrag, Schluessel, Stufe};
use crate::schreibkonto::Grenze;
use crate::start::{abriss, annahmefehler, verdraengt_auftreten, verfall_auftreten};
use std::net::IpAddr;
use std::time::Duration;

/// Welcher der drei Aufrufe einer Zeile gemeint ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Aufruf {
    /// Der erste Aufruf der Quelle.
    A,
    /// Der zweite Aufruf derselben Quelle mit anderen übrigen Argumenten.
    B,
    /// Ein Aufruf einer anderen Quelle.
    C,
}

impl Aufruf {
    /// Welche Quelle: A und B teilen sie, C hat eine andere.
    fn quelle(self) -> u64 {
        match self {
            Aufruf::A | Aufruf::B => 1,
            Aufruf::C => 2,
        }
    }

    /// Welche übrigen Argumente: in jedem Aufruf andere.
    fn einzeln(self) -> u64 {
        match self {
            Aufruf::A => 1,
            Aufruf::B => 2,
            Aufruf::C => 3,
        }
    }

    fn ip(self) -> IpAddr {
        format!("10.0.{}.9", self.quelle()).parse().expect("IP")
    }

    /// Eine Kennung der Quelle.
    fn kennung_der_quelle(self) -> String {
        format!("beta{}@10.0.{}.9", self.quelle(), self.quelle())
    }

    /// Eine Kennung, die je Aufruf wechselt.
    fn kennung_je_aufruf(self) -> String {
        format!("wer{}@10.0.0.9", self.einzeln())
    }

    /// Eine Zahl, die je Aufruf wechselt (nie gleich einer Zahl der Quelle).
    fn zahl(self) -> u64 {
        1000 * self.einzeln() + 17
    }

    /// Ein Text, der je Aufruf wechselt.
    fn text(self, stamm: &str) -> String {
        format!("{stamm}-{}", self.einzeln())
    }

    /// Ein Text der Quelle.
    fn text_der_quelle(self, stamm: &str) -> String {
        format!("{stamm}-quelle-{}", self.quelle())
    }
}

/// Ein fortlaufender Schlüssel und der Wert, den das Ende von ihm trägt.
type EndeWert = (Schluessel, fn(Aufruf) -> String);

/// Eine Zeile der Tabelle.
struct Zeile {
    /// Bereich und Text der Tabellenzeile, wie sie in ADR-0042 steht.
    name: &'static str,
    /// Die Bauweise-Funktion; gleiche Funktion in mehreren Zeilen zählt für die Vollständigkeit einmal.
    bauweise: &'static str,
    /// Wie viele Texte die Bauweise hat (Grenzschutz 2, Schreibkonto 5); jeder wird geprüft.
    texte: usize,
    /// Spalte „Quelle“: die Schlüssel, aus denen die Episode gebildet ist. Leer: eine Episode der Instanz.
    quelle: &'static [Schluessel],
    /// Spalte „Beginn: zusätzlich“.
    beginn: &'static [Schluessel],
    /// Spalte „Ende und Zwischenzeile: zusätzlich“ (nur Schlüssel im Kontext; eine Zählung im Text steht nicht hier).
    ende: &'static [Schluessel],
    /// Was ein fortlaufender Schlüssel im Ende trägt, aus den Argumenten von B gerechnet.
    ende_wert: &'static [EndeWert],
    /// Baut das Auftreten für Aufruf und Text.
    bauen: fn(Aufruf, usize) -> Auftreten,
}

use Schluessel as S;

fn teilnehmer(aufruf: Aufruf) -> Teilnehmer {
    Teilnehmer {
        kennung: Kennung::bilden(&aufruf.kennung_der_quelle()).expect("Kennung"),
        name: Name::bilden("beta").expect("Name"),
        herkunft: aufruf.ip(),
        art: crate::anwesenheit::Art::Agent,
    }
}

fn quotestand(aufruf: Aufruf) -> Quotestand {
    Quotestand {
        belegt: 100 * aufruf.einzeln(),
        gebunden: 1,
        reserviert: 10 * aufruf.einzeln(),
        hoechstens: 5000 * aufruf.quelle(),
        frei: 7 * aufruf.einzeln(),
        braucht: 1,
    }
}

fn gegriffen(aufruf: Aufruf) -> Gegriffen {
    Gegriffen {
        art: if aufruf.einzeln() % 2 == 1 {
            Grenzart::Herkunft
        } else {
            Grenzart::Gesamt
        },
        hoechstens: usize::try_from(100 * aufruf.einzeln()).expect("klein"),
    }
}

fn platzgrenze(aufruf: Aufruf, bereich: Bereich, text: &'static str) -> Auftreten {
    grenzauftreten(bereich, text, aufruf.ip(), gegriffen(aufruf))
}

fn koerper(aufruf: Aufruf) -> Auftreten {
    let (protokoll, _, _, _) = im_speicher("2026-10-30T12:00:00+01:00", Stufe::Warning);
    let kennung = aufruf.kennung_je_aufruf();
    let ort = Koerperort {
        protokoll: &protokoll,
        route: if aufruf.quelle() == 1 {
            "POST /api/senden"
        } else {
            "POST /api/beitreten"
        },
        herkunft: aufruf.ip(),
        kennung: Some(&kennung),
    };
    let art = if aufruf.einzeln() == 1 {
        "zu_gross"
    } else {
        "kaputt"
    };
    koerper_auftreten(&ort, art, Some("vollständig"), aufruf.zahl())
}

fn verfallen(aufruf: Aufruf) -> Verfallen {
    Verfallen {
        id: Anhangid::lesen("abcdefghijklmnopqrstuvwxyz").expect("ID"),
        kennung: Kennung::bilden(&aufruf.kennung_je_aufruf()).expect("Kennung"),
        herkunft: aufruf.ip(),
        bytes: aufruf.zahl(),
        ohne_hochladen: true,
        laufend: false,
        fertig: false,
    }
}

fn verdraengt(aufruf: Aufruf) -> Verdraengt {
    Verdraengt {
        kennung: Kennung::bilden(&aufruf.kennung_der_quelle()).expect("Kennung"),
        grenze: if aufruf.einzeln() % 2 == 1 {
            Wiederholgrenze::Kennung
        } else {
            Wiederholgrenze::Gesamt
        },
        hoechstens: usize::try_from(100 * aufruf.einzeln()).expect("klein"),
        alter_ms: aufruf.zahl(),
    }
}

fn abfrage(aufruf: Aufruf) -> Abfrage {
    if aufruf.einzeln() % 2 == 1 {
        Abfrage::Vorwaerts(format!("name{}.example", aufruf.einzeln()))
    } else {
        Abfrage::Rueckwaerts(format!("10.9.{}.1", aufruf.einzeln()).parse().expect("IP"))
    }
}

fn platzabweisung(aufruf: Aufruf, fehler: Platzfehler) -> Auftreten {
    abweisung_auftreten(
        &teilnehmer(aufruf),
        &Anmeldefehler::Platz(fehler),
        aufruf.zahl(),
    )
    .expect("Episode")
}

/// Die Tabelle aus ADR-0042, Ergänzung „Karte #70“, Zeile für Zeile.
fn katalog() -> Vec<Zeile> {
    vec![
        Zeile {
            name: "verbindung · Verbindung an der Platzgrenze abgewiesen",
            bauweise: "protokoll::grenze::grenzauftreten",
            texte: 1,
            quelle: &[S::Herkunft],
            beginn: &[S::Grenze, S::Hoechstens],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| platzgrenze(a, Bereich::Verbindung, texte::PLATZGRENZE),
        },
        Zeile {
            name: "verbindung · Verbindung an der Abrissgrenze geschlossen",
            bauweise: "start::abriss::abriss_auftreten",
            texte: 1,
            quelle: &[S::Herkunft, S::Hoechstens],
            beginn: &[],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                let grenze = Duration::from_millis(4000 * a.quelle());
                abriss::abriss_auftreten(a.ip(), Some(grenze))
            },
        },
        Zeile {
            name: "verbindung · Verbindung nicht angenommen",
            bauweise: "start::annahmefehler::annahmefehler_auftreten",
            texte: 1,
            quelle: &[S::Grund],
            beginn: &[],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                let nummer = if a.quelle() == 1 { 24 } else { 23 };
                annahmefehler::annahmefehler_auftreten(&std::io::Error::from_raw_os_error(nummer))
            },
        },
        Zeile {
            name: "grenzschutz · Anfrage mit fremdem Host abgewiesen · Anfrage mit fremdem Ursprung abgewiesen",
            bauweise: "http::grenzschutz::abweisung_auftreten",
            texte: 2,
            quelle: &[S::Herkunft],
            beginn: &[S::Grund],
            ende: &[],
            ende_wert: &[],
            bauen: |a, t| {
                let ereignis = [texte::FREMDER_HOST, texte::FREMDER_URSPRUNG][t];
                grenzschutz_auftreten(ereignis, a.ip().to_string(), &a.text("Host boese.example"))
            },
        },
        Zeile {
            name: "grenzschutz · Namensauflösung zu langsam",
            bauweise: "http::auskunft::zu_langsam_auftreten",
            texte: 1,
            quelle: &[],
            beginn: &[S::Art, S::Schluessel],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| namensaufloesung_auftreten(&abfrage(a)),
        },
        Zeile {
            name: "sitzung · Beitritt an der Sitzungsgrenze abgewiesen",
            bauweise: "protokoll::grenze::grenzauftreten",
            texte: 1,
            quelle: &[S::Herkunft],
            beginn: &[S::Grenze, S::Hoechstens],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| platzgrenze(a, Bereich::Sitzung, texte::SITZUNGSGRENZE),
        },
        Zeile {
            name: "channels · Anlegen an der Channel-Grenze abgewiesen",
            bauweise: "protokoll::grenze::grenzauftreten",
            texte: 1,
            quelle: &[S::Herkunft],
            beginn: &[S::Grenze, S::Hoechstens],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| platzgrenze(a, Bereich::Channels, texte::CHANNEL_GRENZE),
        },
        Zeile {
            name: "körper · Körper abgewiesen",
            bauweise: "http::koerper::koerper_auftreten",
            texte: 1,
            quelle: &[S::Herkunft, S::Route],
            beginn: &[S::Kennung],
            ende: &[S::Bytes],
            ende_wert: &[(S::Bytes, |a| a.zahl().to_string())],
            bauen: |a, _| koerper(a),
        },
        Zeile {
            name: "senden · Nachricht nicht gespeichert",
            bauweise: "http::senden::schreibfehler_auftreten",
            texte: 1,
            quelle: &[S::Channel, S::Datei, S::Grund],
            beginn: &[S::Kennung],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                schreibfehler_auftreten(
                    &Ziel::Channel(a.text_der_quelle("channel")),
                    &a.kennung_je_aufruf(),
                    &a.text_der_quelle("datei.log"),
                    &a.text_der_quelle("Platte voll"),
                )
            },
        },
        Zeile {
            name: "senden · Wiederholschlüssel vor Ablauf verdrängt",
            bauweise: "start::verdraengt_auftreten",
            texte: 1,
            quelle: &[S::Kennung],
            beginn: &[S::Grenze, S::Hoechstens, S::AlterMs],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| verdraengt_auftreten(&verdraengt(a)),
        },
        Zeile {
            name: "schreibkonto · Senden gebremst · Bearbeiten gebremst · Frist gebremst · Anhänge gebremst · Löschen gebremst",
            bauweise: "http::bremse::gebremst_auftreten",
            texte: 5,
            quelle: &[S::Kennung],
            beginn: &[S::Grenze],
            ende: &[],
            ende_wert: &[],
            bauen: |a, t| {
                let gegenstand = [
                    Gegenstand::Nachrichten,
                    Gegenstand::Bearbeitungen,
                    Gegenstand::Aenderungen,
                    Gegenstand::Anmeldungen,
                    Gegenstand::Loeschungen,
                ][t];
                let grenze = if a.einzeln() % 2 == 1 {
                    Grenze::Kennung
                } else {
                    Grenze::Herkunft
                };
                gebremst_auftreten(&a.kennung_der_quelle(), grenze, gegenstand)
            },
        },
        Zeile {
            name: "zeit · Nachrichtenzeit läuft der Systemuhr voraus",
            bauweise: "protokoll::vorlauf::vorlauf_auftreten",
            texte: 1,
            quelle: &[],
            beginn: &[S::DauerMs],
            ende: &[S::DauerMs],
            ende_wert: &[(S::DauerMs, |a| (60_000 + a.zahl()).to_string())],
            bauen: |a, _| vorlauf_auftreten(60_000 + a.zahl()),
        },
        Zeile {
            name: "live · Ereignisstrom überholt, neuer Stand gesendet",
            bauweise: "http::ereignisse::ueberholt_auftreten",
            texte: 1,
            quelle: &[S::Kennung],
            beginn: &[S::Anzahl],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| ueberholt_auftreten(&a.kennung_der_quelle(), a.zahl()),
        },
        Zeile {
            name: "quote · Senden abgewiesen: Quote erreicht (beim Bearbeiten route=bearbeiten, beim Löschen route=loeschen)",
            bauweise: "http::senden::quote_auftreten",
            texte: 1,
            quelle: &[S::Kennung, S::Hoechstens],
            beginn: &[S::Channel, S::Belegt, S::Route],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                let route = if a.einzeln() % 2 == 1 {
                    "bearbeiten"
                } else {
                    "loeschen"
                };
                quote_auftreten(
                    &Ziel::Channel(a.text("channel")),
                    &a.kennung_der_quelle(),
                    a.zahl(),
                    5000 * a.quelle(),
                )
                .einzeln(S::Route, route)
            },
        },
        Zeile {
            name: "anhang · Hochladen abgewiesen: Platz für Anhänge voll",
            bauweise: "http::anhaenge::anmelden::abweisung_auftreten",
            texte: 1,
            quelle: &[S::Kennung, S::Hoechstens],
            beginn: &[S::Bytes, S::Belegt],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| platzabweisung(a, Platzfehler::Voll(quotestand(a))),
        },
        Zeile {
            name: "anhang · Hochladen abgewiesen: laufende Hochladungen belegen den Platz",
            bauweise: "http::anhaenge::anmelden::abweisung_auftreten",
            texte: 1,
            quelle: &[S::Kennung, S::Hoechstens],
            beginn: &[S::Reserviert],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| platzabweisung(a, Platzfehler::Hochladungen(quotestand(a))),
        },
        Zeile {
            name: "anhang · Hochladen abgewiesen: Platte fast voll",
            bauweise: "http::anhaenge::anmelden::abweisung_auftreten",
            texte: 1,
            quelle: &[S::Herkunft],
            beginn: &[S::Bytes, S::Frei],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                platzabweisung(
                    a,
                    Platzfehler::Platte {
                        frei: 7 * a.einzeln(),
                    },
                )
            },
        },
        Zeile {
            name: "anhang · Hochladen abgewiesen: Metadaten nicht tilgbar (art nach ADR-0042 Z. 354, vom Director für B0657 entschieden)",
            bauweise: "http::anhaenge::hochladen::metadaten_auftreten",
            texte: 1,
            quelle: &[S::Herkunft],
            beginn: &[S::Art, S::Grund],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                let art = if a.einzeln() % 2 == 1 {
                    "image/jpeg"
                } else {
                    "image/png"
                };
                metadaten_auftreten(a.ip(), art, &a.text("Längen widersprechen sich"))
            },
        },
        Zeile {
            name: "anhang · Hochladen zu langsam abgebrochen",
            bauweise: "http::anhaenge::hochladen::zu_langsam_auftreten",
            texte: 1,
            quelle: &[S::Herkunft],
            beginn: &[],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| zu_langsam_auftreten(a.ip()),
        },
        Zeile {
            name: "anhang · Reservierung verfallen ohne Hochladen",
            bauweise: "start::verfall_auftreten",
            texte: 1,
            quelle: &[S::Herkunft],
            beginn: &[S::Kennung, S::Bytes],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| verfall_auftreten(&verfallen(a)),
        },
        Zeile {
            name: "anhang · Großer Anspruch",
            bauweise: "http::anhaenge::anmelden::anspruch_auftreten",
            texte: 1,
            quelle: &[S::Kennung, S::Herkunft],
            beginn: &[S::Bytes, S::HochladenBis],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                anspruch_auftreten(
                    &a.kennung_der_quelle(),
                    a.ip(),
                    a.zahl(),
                    &format!("2026-10-30T0{}:00:00+01:00", a.einzeln()),
                )
            },
        },
        Zeile {
            name: "aendern · Ändern fremder Nachricht abgewiesen",
            bauweise: "http::bearbeiten::fremd_auftreten",
            texte: 1,
            quelle: &[S::Kennung],
            beginn: &[S::Channel],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                fremd_auftreten(&Ziel::Channel(a.text("channel")), &a.kennung_der_quelle())
            },
        },
        Zeile {
            name: "aendern · Änderung nicht geschrieben",
            bauweise: "http::bearbeiten::schreibfehler_auftreten",
            texte: 1,
            quelle: &[S::Channel, S::Datei, S::Grund],
            beginn: &[S::Kennung],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                aenderung_nicht_geschrieben_auftreten(
                    &Ziel::Channel(a.text_der_quelle("channel")),
                    &a.kennung_je_aufruf(),
                    &a.text_der_quelle("datei.log"),
                    &a.text_der_quelle("Platte voll"),
                )
            },
        },
        Zeile {
            name: "aktivitaet · Aktivität gebremst: zu viele Meldungen",
            bauweise: "http::aktivitaet::gebremst_auftreten",
            texte: 1,
            quelle: &[S::Kennung],
            beginn: &[S::Channel, S::Grenze],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                let grenze = if a.einzeln() % 2 == 1 {
                    AktivitaetGrenze::Gespraech
                } else {
                    AktivitaetGrenze::Herkunft
                };
                aktivitaet_gebremst_auftreten(
                    &a.kennung_der_quelle(),
                    &Ziel::Channel(a.text("channel")),
                    grenze,
                )
            },
        },
        Zeile {
            name: "warten · Warten gebremst: zu viele gleichzeitige Aufrufe",
            bauweise: "http::warten::gebremst_auftreten",
            texte: 1,
            quelle: &[S::Kennung],
            beginn: &[S::Hoechstens],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                warten_gebremst_auftreten(
                    &a.kennung_der_quelle(),
                    usize::try_from(a.zahl()).expect("Zahl"),
                )
            },
        },
        Zeile {
            name: "nachholen · Nachholen gebremst: zu viele geprüfte Sätze",
            bauweise: "http::nachholplatz::gebremst_auftreten",
            texte: 1,
            quelle: &[S::Kennung],
            beginn: &[S::Grenze],
            ende: &[],
            ende_wert: &[],
            bauen: |a, _| {
                let grenze = if a.einzeln() % 2 == 1 {
                    Grenze::Kennung
                } else {
                    Grenze::Herkunft
                };
                nachholen_gebremst_auftreten(&a.kennung_der_quelle(), grenze)
            },
        },
    ]
}

/// Der Kontext einer Zeile als Paare aus Schlüsselwort und Wert.
fn paare(eintrag: &Eintrag) -> Vec<(&'static str, String)> {
    eintrag
        .kontext
        .paare()
        .into_iter()
        .map(|(schluessel, wert)| (schluessel, wert.to_string()))
        .collect()
}

fn namen(schluessel: &[Schluessel]) -> Vec<&'static str> {
    schluessel.iter().map(|s| s.als_text()).collect()
}

fn schluessel_von(paare: &[(&'static str, String)]) -> Vec<&'static str> {
    paare.iter().map(|(schluessel, _)| *schluessel).collect()
}

/// Quelle, dann die weiteren Schlüssel einer Spalte (ADR-0050 Regel 3: in dieser Reihenfolge).
fn erwartete_schluessel(quelle: &[Schluessel], spalte: &[Schluessel]) -> Vec<&'static str> {
    let mut alle = namen(quelle);
    alle.extend(namen(spalte));
    alle
}

/// Die Zeilen eines Laufs A, B: Beginn, Zwischenzeile (Uhr + 10 min 1 s) und Ende (Uhr + Ruhe).
struct Lauf {
    beginn: Eintrag,
    zwischen: Eintrag,
    ende: Eintrag,
}

fn fahren(zeile: &Zeile, text: usize) -> Lauf {
    let start = zeit("2026-10-30T12:00:00+01:00");
    let mut episoden = Episoden::neu(RUHE);
    let mut beginn = episoden.auftreten((zeile.bauen)(Aufruf::A, text), start);
    assert_eq!(beginn.len(), 1, "{}: Beginn von A", zeile.name);
    let spaeter = start + chrono::Duration::seconds(601);
    let mut zwischen = episoden.auftreten((zeile.bauen)(Aufruf::B, text), spaeter);
    assert_eq!(zwischen.len(), 1, "{}: Zwischenzeile bei B", zeile.name);
    let mut ende = episoden.schliessen(spaeter + chrono::Duration::from_std(RUHE).expect("Ruhe"));
    assert_eq!(ende.len(), 1, "{}: Ende", zeile.name);
    Lauf {
        beginn: beginn.remove(0),
        zwischen: zwischen.remove(0),
        ende: ende.remove(0),
    }
}

fn je_zeile_und_text(pruefung: impl Fn(&Zeile, usize)) {
    for zeile in katalog() {
        for text in 0..zeile.texte {
            pruefung(&zeile, text);
        }
    }
}

/// E1: A und B mit gleicher Quelle sind eine Episode, C mit anderer Quelle eine zweite. Episoden der Instanz
/// (Quelle ohne Werte) haben keine andere Quelle.
#[test]
fn e1_gleiche_quelle_ist_eine_episode_andere_quelle_eine_zweite() {
    je_zeile_und_text(|zeile, text| {
        let start = zeit("2026-10-30T12:00:00+01:00");
        let mut episoden = Episoden::neu(RUHE);
        let a = episoden.auftreten((zeile.bauen)(Aufruf::A, text), start);
        let b = episoden.auftreten((zeile.bauen)(Aufruf::B, text), start);
        assert_eq!(a.len(), 1, "{}: A beginnt", zeile.name);
        assert!(b.is_empty(), "{}: B zählt in die Episode von A", zeile.name);
        if zeile.quelle.is_empty() {
            return;
        }
        let c = episoden.auftreten((zeile.bauen)(Aufruf::C, text), start);
        assert_eq!(c.len(), 1, "{}: C ist eine zweite Episode", zeile.name);
    });
}

/// E2: Zwischen- und Ende-Zeile enthalten keinen Einzelwert von A oder B und jeden Quellwert.
#[test]
fn e2_zwischen_und_ende_nennen_keinen_einzelwert_und_jeden_quellwert() {
    je_zeile_und_text(|zeile, text| {
        let lauf = fahren(zeile, text);
        let a = paare(&lauf.beginn);
        let quellwerte = &a[..zeile.quelle.len()];
        let b = (zeile.bauen)(Aufruf::B, text).beginn_kontext();
        for (name, eintrag) in [("Zwischenzeile", &lauf.zwischen), ("Ende", &lauf.ende)] {
            let zusammenfassung = paare(eintrag);
            for quellwert in quellwerte {
                assert!(
                    zusammenfassung.contains(quellwert),
                    "{} {name}: Quellwert {quellwert:?} fehlt in {zusammenfassung:?}",
                    zeile.name
                );
            }
            for schluessel in zeile.beginn {
                if zeile.ende.contains(schluessel) {
                    continue;
                }
                let aus_a = a.iter().find(|(s, _)| *s == schluessel.als_text());
                let aus_b = b.wert(*schluessel);
                assert!(
                    zusammenfassung
                        .iter()
                        .all(|(s, _)| *s != schluessel.als_text()),
                    "{} {name}: Einzelschlüssel {} steht in {zusammenfassung:?}",
                    zeile.name,
                    schluessel.als_text()
                );
                for einzelwert in aus_a.map(|(_, w)| w.as_str()).into_iter().chain(aus_b) {
                    assert!(
                        zusammenfassung.iter().all(|(_, wert)| wert != einzelwert),
                        "{} {name}: Einzelwert {einzelwert:?} steht in {zusammenfassung:?}",
                        zeile.name
                    );
                }
            }
        }
    });
}

/// E3: Beginn-Zeile = Quelle plus Beginn-Spalte, Ende- und Zwischenzeile = Quelle plus Ende-Spalte.
#[test]
fn e3_die_schluessel_der_zeilen_sind_quelle_plus_spalte() {
    je_zeile_und_text(|zeile, text| {
        let lauf = fahren(zeile, text);
        assert_eq!(
            schluessel_von(&paare(&lauf.beginn)),
            erwartete_schluessel(zeile.quelle, zeile.beginn),
            "{}: Beginn",
            zeile.name
        );
        let erwartet = erwartete_schluessel(zeile.quelle, zeile.ende);
        assert_eq!(
            schluessel_von(&paare(&lauf.zwischen)),
            erwartet,
            "{}: Zwischenzeile",
            zeile.name
        );
        assert_eq!(
            schluessel_von(&paare(&lauf.ende)),
            erwartet,
            "{}: Ende",
            zeile.name
        );
    });
}

/// E4: Fortlaufendes trägt im Ende (und in der Zwischenzeile) den Wert von B, des letzten Auftretens.
#[test]
fn e4_fortlaufendes_traegt_im_ende_den_wert_des_letzten_auftretens() {
    je_zeile_und_text(|zeile, text| {
        let lauf = fahren(zeile, text);
        for (schluessel, wert) in zeile.ende_wert {
            for (name, eintrag) in [("Zwischenzeile", &lauf.zwischen), ("Ende", &lauf.ende)] {
                assert_eq!(
                    eintrag.kontext.wert(*schluessel),
                    Some(wert(Aufruf::B).as_str()),
                    "{} {name}: {}",
                    zeile.name,
                    schluessel.als_text()
                );
            }
        }
    });
}

/// Alle Quelltextdateien unter `src/` (Pfad relativ zu `src/`), ohne diesen Katalog.
fn quelltexte() -> Vec<(std::path::PathBuf, String)> {
    fn sammeln(ordner: &std::path::Path, alle: &mut Vec<std::path::PathBuf>) {
        for eintrag in std::fs::read_dir(ordner).expect("src lesbar") {
            let pfad = eintrag.expect("Eintrag").path();
            if pfad.is_dir() {
                sammeln(&pfad, alle);
            } else if pfad.extension().is_some_and(|endung| endung == "rs") {
                alle.push(pfad);
            }
        }
    }
    let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut pfade = Vec::new();
    sammeln(&wurzel, &mut pfade);
    pfade
        .into_iter()
        .filter(|pfad| !pfad.ends_with("episodenkatalog.rs"))
        .map(|pfad| {
            let inhalt = std::fs::read_to_string(&pfad).expect("Quelltext");
            let relativ = pfad.strip_prefix(&wurzel).expect("unter src").to_path_buf();
            (relativ, inhalt)
        })
        .collect()
}

/// Die Bauweisen (Modulpfad `::` Funktionsname) an den Stellen `Auftreten::neu(` außerhalb von Testcode und ohne
/// Kommentarzeilen, eine je Stelle. Testcode ist jedes Item, das mit `#[cfg(test)]` bzw. `#[cfg(all(test` in
/// Spalte 0 beginnt: es endet an der ersten Zeile in Spalte 0, die mit `;` schließt (`mod x;`, `use …;`) oder mit
/// `}` beginnt bzw. auf `}` endet (rustfmt-Form) — danach zählt der Produktivcode wieder.
fn stellen_im_quelltext(quelltexte: &[(std::path::PathBuf, String)]) -> Vec<String> {
    quelltexte
        .iter()
        .flat_map(|(pfad, inhalt)| {
            let modul = modulpfad(pfad);
            stellen_ausserhalb_von_testitems(inhalt)
                .into_iter()
                .map(move |funktion| format!("{modul}::{funktion}"))
        })
        .collect()
}

/// `http/senden.rs` → `http::senden`; `start/mod.rs` → `start`.
fn modulpfad(pfad: &std::path::Path) -> String {
    let teile: Vec<String> = pfad
        .with_extension("")
        .components()
        .map(|teil| teil.as_os_str().to_string_lossy().into_owned())
        .collect();
    let modul = teile.join("::");
    modul.strip_suffix("::mod").unwrap_or(&modul).to_string()
}

/// Der Name der Funktion, die eine Zeile einführt (`pub(crate) async fn name(`), wenn sie eine einführt.
fn funktionsname(zeile: &str) -> Option<&str> {
    let (davor, danach) = zeile.split_once("fn ")?;
    let am_wortanfang = davor.is_empty() || davor.ends_with(char::is_whitespace);
    let name_ende = danach
        .find(|zeichen: char| !(zeichen.is_alphanumeric() || zeichen == '_'))
        .unwrap_or(danach.len());
    (am_wortanfang && name_ende > 0).then(|| &danach[..name_ende])
}

/// Die Funktionsnamen der Stellen `Auftreten::neu(` in einem Quelltext, eine je Stelle.
fn stellen_ausserhalb_von_testitems(inhalt: &str) -> Vec<String> {
    let mut im_testitem = false;
    let mut funktion = "?";
    let mut stellen = Vec::new();
    for zeile in inhalt.lines() {
        if !im_testitem
            && (zeile.starts_with("#[cfg(test)]") || zeile.starts_with("#[cfg(all(test"))
        {
            im_testitem = true;
            continue;
        }
        if im_testitem {
            im_testitem = !schliesst_item(zeile);
            continue;
        }
        if zeile.trim_start().starts_with("//") {
            continue;
        }
        if let Some(name) = funktionsname(zeile) {
            funktion = name;
        }
        for _ in 0..zeile.matches("Auftreten::neu(").count() {
            stellen.push(funktion.to_string());
        }
    }
    stellen
}

/// Schließt die Zeile ein Item der obersten Ebene ab?
fn schliesst_item(zeile: &str) -> bool {
    let oberste_ebene = !zeile.starts_with(char::is_whitespace);
    let ende = zeile.trim_end();
    oberste_ebene && (ende.ends_with(';') || ende.starts_with('}') || ende.ends_with('}'))
}

/// Was in `soll` steht und in `ist` fehlt, sortiert und ohne Doppelte.
fn differenz<T: AsRef<str>, U: AsRef<str>>(soll: &[T], ist: &[U]) -> Vec<String> {
    let mut fehlt: Vec<String> = soll
        .iter()
        .map(AsRef::as_ref)
        .filter(|eintrag| !ist.iter().any(|vorhanden| vorhanden.as_ref() == *eintrag))
        .map(str::to_string)
        .collect();
    fehlt.sort_unstable();
    fehlt.dedup();
    fehlt
}

/// Vollständigkeit: jede Stelle, die ein `Auftreten` baut, hat eine Bauweise-Funktion im Katalog — eine neue
/// Bauweise ohne Katalogzeile ist rot, und die Meldung nennt sie beim Namen.
#[test]
fn jede_stelle_im_quelltext_hat_eine_bauweise_im_katalog() {
    let stellen = stellen_im_quelltext(&quelltexte());
    let zeilen = katalog();
    let im_katalog: Vec<&str> = zeilen.iter().map(|zeile| zeile.bauweise).collect();

    let ohne_zeile = differenz(&stellen, &im_katalog);
    let ohne_stelle = differenz(&im_katalog, &stellen);
    assert!(
        ohne_zeile.is_empty(),
        "Bauweisen im Quelltext ohne Katalogzeile: {ohne_zeile:#?}"
    );
    assert!(
        ohne_stelle.is_empty(),
        "Bauweisen im Katalog ohne Stelle im Quelltext: {ohne_stelle:#?}"
    );
    assert_eq!(
        stellen.len(),
        differenz(&im_katalog, &Vec::<&str>::new()).len(),
        "Stellen `Auftreten::neu(` im Quelltext gegen verschiedene Bauweisen im Katalog"
    );
}

/// Die Zeilen der Tabelle in ADR-0042, Ergänzung „Karte #70“, in ihrer Reihenfolge, und die Zeile der Ergänzung „Karte #77“.
const ZEILEN_DER_TABELLE: [&str; 26] = [
    "verbindung · Verbindung an der Platzgrenze abgewiesen",
    "verbindung · Verbindung an der Abrissgrenze geschlossen",
    "verbindung · Verbindung nicht angenommen",
    "grenzschutz · Anfrage mit fremdem Host abgewiesen · Anfrage mit fremdem Ursprung abgewiesen",
    "grenzschutz · Namensauflösung zu langsam",
    "sitzung · Beitritt an der Sitzungsgrenze abgewiesen",
    "channels · Anlegen an der Channel-Grenze abgewiesen",
    "körper · Körper abgewiesen",
    "senden · Nachricht nicht gespeichert",
    "senden · Wiederholschlüssel vor Ablauf verdrängt",
    "schreibkonto · Senden gebremst · Bearbeiten gebremst · Frist gebremst · Anhänge gebremst · Löschen gebremst",
    "zeit · Nachrichtenzeit läuft der Systemuhr voraus",
    "live · Ereignisstrom überholt, neuer Stand gesendet",
    "quote · Senden abgewiesen: Quote erreicht (beim Bearbeiten route=bearbeiten, beim Löschen route=loeschen)",
    "anhang · Hochladen abgewiesen: Platz für Anhänge voll",
    "anhang · Hochladen abgewiesen: laufende Hochladungen belegen den Platz",
    "anhang · Hochladen abgewiesen: Platte fast voll",
    "anhang · Hochladen abgewiesen: Metadaten nicht tilgbar (art nach ADR-0042 Z. 354, vom Director für B0657 entschieden)",
    "anhang · Hochladen zu langsam abgebrochen",
    "anhang · Reservierung verfallen ohne Hochladen",
    "anhang · Großer Anspruch",
    "aendern · Ändern fremder Nachricht abgewiesen",
    "aendern · Änderung nicht geschrieben",
    "aktivitaet · Aktivität gebremst: zu viele Meldungen",
    "warten · Warten gebremst: zu viele gleichzeitige Aufrufe",
    "nachholen · Nachholen gebremst: zu viele geprüfte Sätze",
];

/// Jede Zeile der Tabelle hat ihre Katalogzeile und umgekehrt — auch wenn ihre Bauweise noch in einer anderen
/// Zeile steht; die Meldung nennt die Zeile.
#[test]
fn jede_zeile_der_tabelle_hat_eine_katalogzeile() {
    let zeilen = katalog();
    let im_katalog: Vec<&str> = zeilen.iter().map(|zeile| zeile.name).collect();

    let fehlt = differenz(&ZEILEN_DER_TABELLE, &im_katalog);
    let zuviel = differenz(&im_katalog, &ZEILEN_DER_TABELLE);
    assert!(
        fehlt.is_empty(),
        "Zeilen der Tabelle ohne Katalogzeile: {fehlt:#?}"
    );
    assert!(
        zuviel.is_empty(),
        "Katalogzeilen ohne Zeile in der Tabelle: {zuviel:#?}"
    );
    assert_eq!(im_katalog.len(), ZEILEN_DER_TABELLE.len());
}

/// Die Differenz nennt, was fehlt: Bauweisen des Quelltexts ohne Katalogzeile beim Namen, und eine Zeile mit
/// mehrfach genutzter Bauweise fehlt in der Zeilenliste, obwohl ihre Bauweise bleibt.
#[test]
fn die_differenz_nennt_die_fehlende_bauweise_und_die_fehlende_zeile() {
    let quelltext = [
        "start::abriss::abriss_auftreten",
        "http::senden::quote_auftreten",
    ];
    let katalog = ["http::senden::quote_auftreten"];
    assert_eq!(
        differenz(&quelltext, &katalog),
        ["start::abriss::abriss_auftreten"]
    );

    let tabelle = ["verbindung · a", "sitzung · b"];
    let zeilen = ["sitzung · b"];
    assert_eq!(differenz(&tabelle, &zeilen), ["verbindung · a"]);
    assert!(differenz(&zeilen, &tabelle).is_empty());
}

/// Die Bauweise einer Stelle ist Modulpfad und Funktionsname, auch für `mod.rs` und Dateien in Ordnern.
#[test]
fn die_stelle_nennt_modulpfad_und_funktion() {
    let quelltexte = vec![
        (
            std::path::PathBuf::from("start/abriss.rs"),
            "pub(crate) fn abriss_auftreten(x: u8) -> Auftreten {\n    Auftreten::neu(1)\n}\n"
                .to_string(),
        ),
        (
            std::path::PathBuf::from("start/mod.rs"),
            "pub(crate) async fn verfall_auftreten() -> Auftreten {\n    Auftreten::neu(2)\n}\n"
                .to_string(),
        ),
    ];

    assert_eq!(
        stellen_im_quelltext(&quelltexte),
        [
            "start::abriss::abriss_auftreten",
            "start::verfall_auftreten"
        ]
    );
}

/// Die Zählung der Vollständigkeit erkennt Kommentare und Testmodule: nur die Stelle davor zählt.
#[test]
fn die_zaehlung_der_stellen_ignoriert_kommentare_und_testmodule() {
    let quelltexte = vec![(
        std::path::PathBuf::from("beispiel.rs"),
        "fn a() { Auftreten::neu(1) }\n// Auftreten::neu(2)\nfn b() { Auftreten::neu(3) }\n#[cfg(test)]\nmod tests { fn c() { Auftreten::neu(4) } }\n"
            .to_string(),
    )];

    assert_eq!(stellen_im_quelltext(&quelltexte).len(), 2);
}

/// Produktivcode hinter einem `#[cfg(test)]`-Einzelitem (Funktion, `mod x;`, `impl`) zählt; nur das Item selbst
/// und ein `#[cfg(test)] mod tests { … }` (auch mitten in der Datei) zählen nicht.
#[test]
fn die_zaehlung_der_stellen_zaehlt_produktivcode_nach_einem_testeinzelitem() {
    let quelltext = [
        "fn a() { Auftreten::neu(1) }",
        "#[cfg(test)]",
        "async fn nur_im_test(kopf: &Kopf) -> Result<Wert, Fehler> {",
        "    Auftreten::neu(2)",
        "}",
        "#[cfg(test)]",
        "mod nur_im_test;",
        "#[cfg(test)]",
        "impl Hilfe {",
        "    fn x() { Auftreten::neu(3) }",
        "}",
        "fn b() { Auftreten::neu(4) }",
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    fn c() { Auftreten::neu(5) }",
        "}",
        "fn d() { Auftreten::neu(6) }",
        "#[cfg(test)]",
        "mod tests_am_ende {",
        "    fn e() { Auftreten::neu(7) }",
        "}",
    ]
    .join("\n");
    let quelltexte = vec![(
        std::path::PathBuf::from("beispiel.rs"),
        quelltext.to_string(),
    )];

    assert_eq!(stellen_im_quelltext(&quelltexte).len(), 3);
}
