//! Art einer Datei, wie die Instanz sie bestimmt (ADR-0047 Frage 3): Inhaltstyp, Disposition und — nur bei
//! Bildern — die angezeigten Maße.

/// Inhaltstyp für alles, was nicht dargestellt wird.
pub const DATEI: &str = "application/octet-stream";

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
