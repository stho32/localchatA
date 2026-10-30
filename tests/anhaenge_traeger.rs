//! R00029 F0115 „Unbekannte Träger“ und „Qualität bleibt“ über das echte Programm (ADR-0047 Ergänzung „Karte #9
//! Security-Review B2“, P1–P9): je Datei hochladen, senden, `GET /anhaenge/<id>` — in den ausgelieferten Bytes kein
//! Ort, Länge wie angemeldet, Bilddaten byte-gleich; was die Darstellung braucht, bleibt byte-gleich.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{anmelden, hochladen, id, starten};
use gemeinsam::dateien::{jpeg, jpeg_segment, png_chunk};
use gemeinsam::senden::{beitreten, ordner, roh_senden};
use serde_json::{Value, json};

/// GPSLatitude 52° 31′ 12″ als drei RATIONAL (Motorola) — der binäre Wert von 52.52.
const ORT_BINAER: [u8; 24] = [
    0, 0, 0, 52, 0, 0, 0, 1, 0, 0, 0, 31, 0, 0, 0, 1, 0, 0, 0, 12, 0, 0, 0, 1,
];

/// Der Ort der Proben (`GPSLatitude` 52.52, `GPSLongitude` 13.405) als Text und binär.
fn ort() -> Vec<u8> {
    [
        &b"GPSLatitude 52.52 +52.5200+013.4050/ GPSLongitude 13.405 "[..],
        &ORT_BINAER[..],
    ]
    .concat()
}

fn enthaelt(heu: &[u8], nadel: &[u8]) -> bool {
    heu.windows(nadel.len()).any(|fenster| fenster == nadel)
}

/// Kein Ort: weder `GPS` noch „52.52“, „+52.5200“, noch der binäre Wert; `exiftool`, wo installiert, meldet kein
/// `GPS*`.
fn ohne_ort(name: &str, bytes: &[u8]) {
    for spur in [&b"GPS"[..], b"52.52", b"+52.5200", &ORT_BINAER] {
        assert!(
            !enthaelt(bytes, spur),
            "{name}: {}",
            String::from_utf8_lossy(spur)
        );
    }
    exiftool_ohne_gps(name, bytes);
}

fn exiftool_ohne_gps(name: &str, bytes: &[u8]) {
    let ordner = ordner();
    let pfad = ordner.path().join(name);
    std::fs::write(&pfad, bytes).expect("geschrieben");
    let Ok(ausgabe) = std::process::Command::new("exiftool")
        .args(["-a", "-ee", "-G1", "-n"])
        .arg(&pfad)
        .output()
    else {
        return;
    };
    let text = String::from_utf8_lossy(&ausgabe.stdout);
    assert!(!text.contains("GPS"), "{name}: {text}");
}

/// Eine Instanz, in der `beta` Dateien hochlädt, sendet und ausgeliefert holt.
struct Lauf {
    _wurzel: tempfile::TempDir,
    instanz: gemeinsam::Instanz,
    beta: Absender,
    token: String,
}

/// Was nach dem Hochladen ankommt: die Angaben der 200 und die ausgelieferten Bytes.
struct Ausgeliefert {
    anhang: Value,
    bytes: Vec<u8>,
}

impl Lauf {
    async fn neu() -> Lauf {
        let wurzel = ordner();
        let instanz = starten(wurzel.path(), &[], &[]);
        let beta = Absender::von("127.0.0.1");
        let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
        Lauf {
            _wurzel: wurzel,
            instanz,
            beta,
            token,
        }
    }

    /// Hochladen (200), senden (201), `GET /anhaenge/<id>` (200); Länge = angemeldete `bytes`.
    async fn ausliefern(&self, name: &str, inhalt: &[u8]) -> Ausgeliefert {
        let basis = self.instanz.adresse.as_str();
        let angemeldet = anmelden(&self.beta, basis, &self.token, name, inhalt.len() as u64).await;
        let anhang_id = id(&angemeldet);
        let antwort = hochladen(&self.beta, basis, &self.token, &anhang_id, inhalt.to_vec()).await;
        assert_eq!(antwort.status, 200, "{name}: {}", antwort.json);
        let koerper = json!({"text": "", "anhaenge": [anhang_id]}).to_string();
        let gesendet = roh_senden(
            &self.beta,
            basis,
            Some(&self.token),
            "allgemein",
            "application/json",
            koerper.into_bytes(),
        )
        .await;
        assert_eq!(gesendet.status, 201, "{name}: {}", gesendet.json);
        let geholt = self
            .beta
            .anfrage(reqwest::Method::GET, format!("{basis}anhaenge/{anhang_id}"))
            .send()
            .await
            .expect("Antwort");
        assert_eq!(geholt.status(), 200, "{name}");
        let bytes = geholt.bytes().await.expect("Körper").to_vec();
        assert_eq!(bytes.len(), inhalt.len(), "{name}: Länge = bytes");
        Ausgeliefert {
            anhang: antwort.json["anhang"].clone(),
            bytes,
        }
    }
}

/// `(Art, Breite, Höhe)` aus der 200 des Hochladens.
fn art_und_masse(anhang: &Value) -> (String, Value, Value) {
    (
        anhang["art"].as_str().unwrap_or_default().to_string(),
        anhang["breite"].clone(),
        anhang["hoehe"].clone(),
    )
}

fn ab_sos(bytes: &[u8]) -> &[u8] {
    let sos = bytes
        .windows(2)
        .rposition(|paar| paar == [0xFF, 0xDA])
        .expect("SOS");
    &bytes[sos..]
}

fn segment(marke: u8, nutzlast: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    jpeg_segment(&mut bytes, marke, nutzlast);
    bytes
}

/// ADR-0047 B2 P1, P2, P7 (JPEG-Teil), P8 (JPEG-Teil) (B0641): GPS nur in `APP11` JUMBF/C2PA (`stds.exif`), nur in
/// `APP3`, nur im Vorschau-JPEG eines `APP0` `JFXX` → ausgeliefert ohne Ort, JPEG ab `SOS` byte-gleich; ICC über drei
/// `APP2`, `APP14` Adobe, Orientierung 6, ISO 21496-1 und MPF bleiben byte-gleich, Art und Maße wie vorher.
#[tokio::test]
async fn jpeg_traeger_ohne_ort_farbe_und_hdr_bleiben() {
    let lauf = Lauf::neu().await;
    let jumbf = [
        &b"JP\0\x01\0\0\0\x01\0\0\0\x80jumb\0\0\0\x1Ejumdc2pa\0\x11\0\x10\x80\0\xAA\08\x9Bq\x03c2pa\0"[..],
        b"\0\0\0\x40jumb\0\0\0\x1Djumdjson\0\x11\0\x10\x80\0\xAA\0stds.exif\0",
        &ort(),
    ]
    .concat();
    let mut exif_gps = b"Exif\0\0".to_vec();
    exif_gps.extend(ort());
    let mut jfxx = b"JFXX\0\x10".to_vec();
    jfxx.extend(jpeg(&[(0xe1, exif_gps)], 16, 16));
    let faelle = [
        ("p1-app11.jpg", vec![(0xeb, jumbf)]),
        (
            "p2-app3.jpg",
            vec![(0xe3, [&b"META\0\0"[..], &ort()].concat())],
        ),
        ("p7-jfxx.jpg", vec![(0xe0, jfxx)]),
    ];
    for (name, segmente) in faelle {
        let original = jpeg(&segmente, 4000, 3000);
        let geliefert = lauf.ausliefern(name, &original).await;
        ohne_ort(name, &geliefert.bytes);
        assert_eq!(ab_sos(&geliefert.bytes), ab_sos(&original), "{name}");
        assert_eq!(
            art_und_masse(&geliefert.anhang),
            ("image/jpeg".to_string(), json!(4000), json!(3000)),
            "{name}"
        );
    }

    let bleiben = [
        segment(0xe0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"),
        segment(0xe2, b"ICC_PROFILE\0\x01\x03Teil eins"),
        segment(0xe2, b"ICC_PROFILE\0\x02\x03Teil zwei"),
        segment(0xe2, b"ICC_PROFILE\0\x03\x03Teil drei"),
        segment(0xee, b"Adobe\0\x64\0\0\0\0\x02"),
        segment(0xe2, b"urn:iso:std:iso:ts:21496:-1\0\0\0\0\0"),
        segment(0xe2, b"MPF\0MM\0*\0\0\0\x08\0\0"),
    ]
    .concat();
    let mut orientierung =
        b"Exif\0\0MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01\0\x06\0\0\0\0\0\0".to_vec();
    orientierung.extend(ort());
    let original = [
        vec![0xff, 0xd8],
        bleiben.clone(),
        jpeg(&[(0xe1, orientierung)], 4000, 3000)[2..].to_vec(),
    ]
    .concat();
    let geliefert = lauf.ausliefern("p8-qualitaet.jpg", &original).await;
    ohne_ort("p8-qualitaet.jpg", &geliefert.bytes);
    assert_eq!(
        geliefert.bytes[..2 + bleiben.len()],
        original[..2 + bleiben.len()]
    );
    assert_eq!(ab_sos(&geliefert.bytes), ab_sos(&original));
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/jpeg".to_string(), json!(3000), json!(4000)),
        "Orientierung 6 bleibt"
    );
}

/// PNG 64 × 48 mit den Chunks `davor` hinter `IHDR` und einem `IDAT`.
fn png_mit(davor: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    png_chunk(
        &mut bytes,
        b"IHDR",
        &[0, 0, 0, 64, 0, 0, 0, 48, 8, 2, 0, 0, 0],
    );
    for (typ, daten) in davor {
        png_chunk(&mut bytes, typ, daten);
    }
    png_chunk(&mut bytes, b"IDAT", &[0x78, 0x9c, 0x63, 0, 0, 0, 1, 0, 1]);
    png_chunk(&mut bytes, b"IEND", &[]);
    bytes
}

/// ADR-0047 B2 P3 und P8 (PNG-Teil) (B0642): GPS nur in `caBX` → ausgeliefert ohne Ort, `IDAT` byte-gleich;
/// `iCCP`, `cICP`, `mDCV`, `cLLI`, `pHYs`, APNG (`acTL`/`fcTL`/`fdAT`) → byte-gleich, Art und Maße wie vorher.
#[tokio::test]
async fn png_traeger_ohne_ort_farbe_hdr_und_animation_bleiben() {
    let lauf = Lauf::neu().await;
    let original = png_mit(&[(b"caBX", [&b"jumb c2pa stds.exif "[..], &ort()].concat())]);

    let geliefert = lauf.ausliefern("p3-cabx.png", &original).await;

    ohne_ort("p3-cabx.png", &geliefert.bytes);
    let idat = original.len() - 12 - 21;
    assert_eq!(geliefert.bytes[idat..], original[idat..], "IDAT und IEND");
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/png".to_string(), json!(64), json!(48))
    );

    let mut fctl = 0u32.to_be_bytes().to_vec();
    fctl.extend([
        0, 0, 0, 64, 0, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 10, 0, 0,
    ]);
    let original = png_mit(&[
        (b"iCCP", b"Profil\0\0Farbe".to_vec()),
        (b"cICP", vec![9, 16, 0, 1]),
        (b"mDCV", vec![1; 24]),
        (b"cLLI", vec![0, 0, 3, 232, 0, 0, 1, 144]),
        (b"pHYs", vec![0, 0, 11, 19, 0, 0, 11, 19, 1]),
        (b"acTL", vec![0, 0, 0, 2, 0, 0, 0, 0]),
        (b"fcTL", fctl),
        (b"fdAT", vec![0, 0, 0, 1, 0x78, 0x9c]),
    ]);

    let geliefert = lauf.ausliefern("p8-qualitaet.png", &original).await;

    assert_eq!(geliefert.bytes, original);
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/png".to_string(), json!(64), json!(48))
    );
}
