//! Kopfzeilen der Anhang-Antwort (B0445, ADR-0047 Regel 3, Ergänzung Karte #9 Gate Regeln 1, 6): die Route setzt
//! keine CSP — die setzt die Weiche in `sicherheitsheader_setzen` (B0467). Rein.

use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
    ETAG, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue};

/// Kopfzeilen jeder Antwort der Route (auch 304, 404, 410, 416).
pub fn route(kopf: &mut HeaderMap) {
    kopf.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    kopf.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    kopf.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    kopf.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
}

/// Was eine Dateiantwort (200, 206) beschreibt.
pub struct Datei<'a> {
    pub id: &'a str,
    pub typ: &'a str,
    pub inline: bool,
    pub anzeigename: &'a str,
    /// ASCII-Teil des gespeicherten Namens.
    pub ascii: &'a str,
    pub laenge: u64,
}

/// Kopfzeilen einer Dateiantwort; `teil` ist der gelieferte Bereich (erstes, letztes Byte) bei 206.
pub fn datei(kopf: &mut HeaderMap, datei: &Datei<'_>, teil: Option<(u64, u64)>) {
    route(kopf);
    let wert = |text: String| HeaderValue::from_str(&text).unwrap_or(HeaderValue::from_static(""));
    kopf.insert(CONTENT_TYPE, wert(datei.typ.to_string()));
    kopf.insert(
        CONTENT_DISPOSITION,
        wert(disposition(datei.inline, datei.ascii, datei.anzeigename)),
    );
    kopf.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    kopf.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-cache"));
    kopf.insert(ETAG, wert(etag(datei.id)));
    let gesendet = match teil {
        Some((von, bis)) => {
            kopf.insert(
                CONTENT_RANGE,
                wert(format!("bytes {von}-{bis}/{}", datei.laenge)),
            );
            bis - von + 1
        }
        None => datei.laenge,
    };
    kopf.insert(CONTENT_LENGTH, wert(gesendet.to_string()));
}

/// Kopfzeilen einer 416: die Route dazu `Content-Range: bytes */<länge>`.
pub fn unerfuellbar(kopf: &mut HeaderMap, laenge: u64) {
    route(kopf);
    if let Ok(wert) = HeaderValue::from_str(&format!("bytes */{laenge}")) {
        kopf.insert(CONTENT_RANGE, wert);
    }
}

pub fn etag(id: &str) -> String {
    format!("\"{id}\"")
}

/// `inline|attachment; filename="<ascii>"; filename*=UTF-8''<anzeigename nach RFC 8187>`.
pub fn disposition(inline: bool, ascii: &str, anzeigename: &str) -> String {
    let art = if inline { "inline" } else { "attachment" };
    let ersatz: String = ascii
        .chars()
        .map(|zeichen| {
            if zeichen.is_ascii_graphic() && zeichen != '"' && zeichen != '\\' {
                zeichen
            } else {
                '_'
            }
        })
        .collect();
    format!(
        "{art}; filename=\"{ersatz}\"; filename*=UTF-8''{}",
        rfc8187(anzeigename)
    )
}

/// Alles außer `attr-char` (RFC 8187: ALPHA, DIGIT, `!#$&+-.^_`|~`) prozentkodiert.
pub fn rfc8187(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dekodieren(kodiert: &str) -> String {
        let mut bytes = Vec::new();
        let mut rest = kodiert.as_bytes();
        while let Some((&erstes, weiter)) = rest.split_first() {
            if erstes == b'%' {
                let hex = std::str::from_utf8(&weiter[..2]).expect("hex");
                bytes.push(u8::from_str_radix(hex, 16).expect("hex"));
                rest = &weiter[2..];
            } else {
                bytes.push(erstes);
                rest = weiter;
            }
        }
        String::from_utf8(bytes).expect("UTF-8")
    }

    #[test]
    fn download_unter_dem_anzeigenamen() {
        assert_eq!(
            disposition(false, "Bericht_M_rz.bin", "Bericht März.bin"),
            "attachment; filename=\"Bericht_M_rz.bin\"; filename*=UTF-8''Bericht%20M%C3%A4rz.bin"
        );
        assert_eq!(
            disposition(true, "x.html", "x.html"),
            "inline; filename=\"x.html\"; filename*=UTF-8''x.html"
        );
    }

    #[test]
    fn sonderzeichen_werden_kodiert_und_dekodieren_zum_anzeigenamen() {
        assert_eq!(rfc8187("a;b'c%d*e f.txt"), "a%3Bb%27c%25d%2Ae%20f.txt");
        for name in ["a;b'c%d*e f.txt", "Grüße 😀.pdf", "x\"y\\z.txt", "日本.txt"] {
            let kopf = disposition(false, "datei", name);
            assert!(HeaderValue::from_str(&kopf).is_ok(), "{kopf}");
            let kodiert = kopf.split("filename*=UTF-8''").nth(1).expect("filename*");
            assert_eq!(dekodieren(kodiert), name);
            assert!(kodiert.bytes().all(|byte| byte.is_ascii_graphic()));
        }
    }

    #[test]
    fn der_ascii_ersatz_steht_sicher_in_anfuehrungszeichen() {
        assert_eq!(
            disposition(false, "a\"b\\c d", "x"),
            "attachment; filename=\"a_b_c_d\"; filename*=UTF-8''x"
        );
    }

    #[test]
    fn jede_kopfzeile_einer_dateiantwort() {
        let mut kopf = HeaderMap::new();
        let beschrieben = Datei {
            id: "aaaaaaaaaaaaaaaaaaaaaaaaaa",
            typ: "image/png",
            inline: true,
            anzeigename: "Foto.png",
            ascii: "Foto.png",
            laenge: 1_000,
        };

        datei(&mut kopf, &beschrieben, Some((100, 199)));

        let erwartet = [
            ("x-content-type-options", "nosniff"),
            ("cross-origin-resource-policy", "same-origin"),
            ("referrer-policy", "no-referrer"),
            ("x-frame-options", "DENY"),
            ("content-type", "image/png"),
            (
                "content-disposition",
                "inline; filename=\"Foto.png\"; filename*=UTF-8''Foto.png",
            ),
            ("accept-ranges", "bytes"),
            ("cache-control", "private, no-cache"),
            ("etag", "\"aaaaaaaaaaaaaaaaaaaaaaaaaa\""),
            ("content-range", "bytes 100-199/1000"),
            ("content-length", "100"),
        ];
        for (name, wert) in erwartet {
            assert_eq!(
                kopf.get(name).and_then(|w| w.to_str().ok()),
                Some(wert),
                "{name}"
            );
        }
        assert_eq!(kopf.len(), erwartet.len());
        assert!(
            kopf.get("content-security-policy").is_none(),
            "die CSP setzt die Weiche"
        );
    }

    #[test]
    fn eine_ganze_datei_ohne_content_range_und_416_mit_stern() {
        let mut kopf = HeaderMap::new();
        datei(
            &mut kopf,
            &Datei {
                id: "a",
                typ: "application/octet-stream",
                inline: false,
                anzeigename: "a",
                ascii: "a",
                laenge: 7,
            },
            None,
        );
        assert_eq!(
            kopf.get("content-length").map(|w| w.to_str().ok()),
            Some(Some("7"))
        );
        assert!(kopf.get("content-range").is_none());
        let mut kopf = HeaderMap::new();
        unerfuellbar(&mut kopf, 7);
        assert_eq!(
            kopf.get("content-range").map(|w| w.to_str().ok()),
            Some(Some("bytes */7"))
        );
        assert_eq!(
            kopf.get("x-content-type-options").map(|w| w.to_str().ok()),
            Some(Some("nosniff"))
        );
    }
}
