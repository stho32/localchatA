//! Die Texte der Episoden an einer Stelle (ADR-0042 Regel 5 und Ergänzung Karte #31 Punkt 1). Die Mechanik hängt
//! „ — Episode beginnt“, „ — Episode endet: …“ und „ — läuft weiter …“ an; deshalb endet kein Text auf ein
//! Satzzeichen, und ein Gedankenstrich im Text steht nur vor einer Folge oder einem Ausweg. Ein neuer Episodentext
//! entsteht hier, mit seinem Test. Den Vorlauf bildet `uhr::vorlauf_text` mit seinem Wert (geprüft dort).

pub const CHANNEL_GRENZE: &str = "Anlegen an der Channel-Grenze abgewiesen";
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
pub const FRIST_GEBREMST: &str = "Frist gebremst: zu viele Änderungen";
pub const ANHAENGE_GEBREMST: &str = "Anhänge gebremst: zu viele Anmeldungen";
pub const SCHLUESSEL_VERDRAENGT: &str =
    "Wiederholschlüssel vor Ablauf verdrängt — eine Wiederholung kann doppelt ankommen";

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
        frist_gebremst: FRIST_GEBREMST = "Frist gebremst: zu viele Änderungen";
        anhaenge_gebremst: ANHAENGE_GEBREMST = "Anhänge gebremst: zu viele Anmeldungen";
        schluessel_verdraengt: SCHLUESSEL_VERDRAENGT =
            "Wiederholschlüssel vor Ablauf verdrängt — eine Wiederholung kann doppelt ankommen";
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
