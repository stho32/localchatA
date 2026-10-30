//! Die Texte der Episoden an einer Stelle (ADR-0042 Regel 5 und Ergänzung Karte #31 Punkt 1). Die Mechanik hängt
//! „ — Episode beginnt“, „ — Episode endet: …“ und „ — läuft weiter …“ an; deshalb endet kein Text auf ein
//! Satzzeichen, und ein Gedankenstrich im Text steht nur vor einer Folge oder einem Ausweg. Ein neuer Episodentext
//! entsteht hier, mit seinem Test. Den Vorlauf bildet `uhr::vorlauf_text` mit seinem Wert (geprüft dort).

pub const CHANNEL_GRENZE: &str = "Anlegen an der Channel-Grenze abgewiesen";
pub const DIREKT_GRENZE: &str = "Neues Direktgespräch an der Grenze abgewiesen";
pub const SITZUNGSGRENZE: &str = "Beitritt an der Sitzungsgrenze abgewiesen";
pub const PLATZGRENZE: &str = "Verbindung an der Platzgrenze abgewiesen";
pub const ABRISSGRENZE: &str = "Verbindung an der Abrissgrenze geschlossen";
pub const VERBINDUNG_NICHT_ANGENOMMEN: &str = "Verbindung nicht angenommen";
pub const KOERPER_ABGEWIESEN: &str = "Körper abgewiesen";
pub const FREMDER_HOST: &str = "Anfrage mit fremdem Host abgewiesen";
pub const FREMDER_URSPRUNG: &str = "Anfrage mit fremdem Ursprung abgewiesen";
pub const NAMENSAUFLOESUNG_ZU_LANGSAM: &str = "Namensauflösung zu langsam";
pub const NICHT_GESPEICHERT: &str = "Nachricht nicht gespeichert";
pub const QUOTE_ERREICHT: &str = "Senden abgewiesen: Quote erreicht";
pub const GEBREMST: &str = "Senden gebremst: zu viele Nachrichten";
pub const BEARBEITEN_GEBREMST: &str = "Bearbeiten gebremst: zu viele Bearbeitungen";
pub const FRIST_GEBREMST: &str = "Frist gebremst: zu viele Änderungen";
pub const ANHAENGE_GEBREMST: &str = "Anhänge gebremst: zu viele Anmeldungen";
pub const METADATEN_NICHT_TILGBAR: &str = "Hochladen abgewiesen: Metadaten nicht tilgbar";
pub const LOESCHEN_GEBREMST: &str = "Löschen gebremst: zu viele Löschanfragen";
pub const GROSSER_ANSPRUCH: &str = "Großer Anspruch";
pub const HOCHLADEN_ZU_LANGSAM: &str = "Hochladen zu langsam abgebrochen";
pub const ANHAENGE_VOLL: &str = "Hochladen abgewiesen: Platz für Anhänge voll";
pub const HOCHLADUNGEN_BELEGEN: &str =
    "Hochladen abgewiesen: laufende Hochladungen belegen den Platz";
pub const PLATTE_FAST_VOLL: &str = "Hochladen abgewiesen: Platte fast voll";
pub const RESERVIERUNG_VERFALLEN: &str = "Reservierung verfallen ohne Hochladen";
pub const EREIGNISSTROM_UEBERHOLT: &str = "Ereignisstrom überholt, neuer Stand gesendet";
pub const FREMD_ABGEWIESEN: &str = "Ändern fremder Nachricht abgewiesen";
pub const AENDERUNG_NICHT_GESCHRIEBEN: &str = "Änderung nicht geschrieben";
pub const SCHLUESSEL_VERDRAENGT: &str =
    "Wiederholschlüssel vor Ablauf verdrängt — eine Wiederholung kann doppelt ankommen";
pub const AKTIVITAET_GEBREMST: &str = "Aktivität gebremst: zu viele Meldungen";
pub const WARTEN_GEBREMST: &str = "Warten gebremst: zu viele gleichzeitige Aufrufe";
pub const NACHHOLEN_GEBREMST: &str = "Nachholen gebremst: zu viele geprüfte Sätze";

/// Ein Episodentext endet ohne Satzzeichen und trägt höchstens einen Gedankenstrich (vor Folge oder Ausweg);
/// gilt auch für den Vorlauftext aus `uhr`.
#[cfg(test)]
pub fn regelgerecht(text: &str) -> bool {
    let endet_ohne = !text.ends_with(['.', '!', '?', ':', ';']);
    endet_ohne && text.matches(" — ").count() <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R00020 F0070 „Episodentexte ohne Satzzeichen“: je Text ein Test auf den Wortlaut und die Regel.
    macro_rules! wortlaut {
        ($($name:ident: $text:ident = $erwartet:literal;)*) => {$(
            #[test]
            fn $name() {
                assert_eq!($text, $erwartet);
                assert!(regelgerecht($text), "{}", $text);
            }
        )*};
    }

    wortlaut! {
        channel_grenze: CHANNEL_GRENZE = "Anlegen an der Channel-Grenze abgewiesen";
        direkt_grenze: DIREKT_GRENZE = "Neues Direktgespräch an der Grenze abgewiesen";
        sitzungsgrenze: SITZUNGSGRENZE = "Beitritt an der Sitzungsgrenze abgewiesen";
        platzgrenze: PLATZGRENZE = "Verbindung an der Platzgrenze abgewiesen";
        abrissgrenze: ABRISSGRENZE = "Verbindung an der Abrissgrenze geschlossen";
        verbindung_nicht_angenommen: VERBINDUNG_NICHT_ANGENOMMEN = "Verbindung nicht angenommen";
        koerper_abgewiesen: KOERPER_ABGEWIESEN = "Körper abgewiesen";
        fremder_host: FREMDER_HOST = "Anfrage mit fremdem Host abgewiesen";
        fremder_ursprung: FREMDER_URSPRUNG = "Anfrage mit fremdem Ursprung abgewiesen";
        namensaufloesung_zu_langsam: NAMENSAUFLOESUNG_ZU_LANGSAM = "Namensauflösung zu langsam";
        nicht_gespeichert: NICHT_GESPEICHERT = "Nachricht nicht gespeichert";
        quote_erreicht: QUOTE_ERREICHT = "Senden abgewiesen: Quote erreicht";
        gebremst: GEBREMST = "Senden gebremst: zu viele Nachrichten";
        bearbeiten_gebremst: BEARBEITEN_GEBREMST = "Bearbeiten gebremst: zu viele Bearbeitungen";
        frist_gebremst: FRIST_GEBREMST = "Frist gebremst: zu viele Änderungen";
        anhaenge_gebremst: ANHAENGE_GEBREMST = "Anhänge gebremst: zu viele Anmeldungen";
        metadaten_nicht_tilgbar: METADATEN_NICHT_TILGBAR = "Hochladen abgewiesen: Metadaten nicht tilgbar";
        loeschen_gebremst: LOESCHEN_GEBREMST = "Löschen gebremst: zu viele Löschanfragen";
        grosser_anspruch: GROSSER_ANSPRUCH = "Großer Anspruch";
        hochladen_zu_langsam: HOCHLADEN_ZU_LANGSAM = "Hochladen zu langsam abgebrochen";
        anhaenge_voll: ANHAENGE_VOLL = "Hochladen abgewiesen: Platz für Anhänge voll";
        hochladungen_belegen: HOCHLADUNGEN_BELEGEN = "Hochladen abgewiesen: laufende Hochladungen belegen den Platz";
        platte_fast_voll: PLATTE_FAST_VOLL = "Hochladen abgewiesen: Platte fast voll";
        reservierung_verfallen: RESERVIERUNG_VERFALLEN = "Reservierung verfallen ohne Hochladen";
        ereignisstrom_ueberholt: EREIGNISSTROM_UEBERHOLT =
            "Ereignisstrom überholt, neuer Stand gesendet";
        fremd_abgewiesen: FREMD_ABGEWIESEN = "Ändern fremder Nachricht abgewiesen";
        aenderung_nicht_geschrieben: AENDERUNG_NICHT_GESCHRIEBEN = "Änderung nicht geschrieben";
        schluessel_verdraengt: SCHLUESSEL_VERDRAENGT =
            "Wiederholschlüssel vor Ablauf verdrängt — eine Wiederholung kann doppelt ankommen";
        aktivitaet_gebremst: AKTIVITAET_GEBREMST = "Aktivität gebremst: zu viele Meldungen";
        warten_gebremst: WARTEN_GEBREMST = "Warten gebremst: zu viele gleichzeitige Aufrufe";
        nachholen_gebremst: NACHHOLEN_GEBREMST = "Nachholen gebremst: zu viele geprüfte Sätze";
    }

    /// Die Regel selbst: jedes verbotene Satzzeichen am Ende und ein zweiter Gedankenstrich fallen auf.
    #[test]
    fn die_regel_erkennt_satzzeichen_am_ende_und_zwei_gedankenstriche() {
        for text in ["a.", "a!", "a?", "a:", "a;", "Zu viele — gebremst — mehr"] {
            assert!(!regelgerecht(text), "{text}");
        }
        assert!(regelgerecht(
            "Zeit (zurückgestellt?) — bleibt, bis sie aufholt"
        ));
    }
}
