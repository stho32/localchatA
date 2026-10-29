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
pub const GEBREMST: &str = "Zu viele Nachrichten — gebremst";
