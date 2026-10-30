//! R00029 F0115 „Unbekannte Träger“, „Qualität bleibt“, „Platzhalter“ und „Varianten“ über das echte Programm (ADR-0047
//! Ergänzungen „Karte #9 Security-Review B2“, P1–P10, und „B3“, Proben (a)–(g), P11): je Datei hochladen, senden, `GET /anhaenge/<id>` — in den ausgelieferten Bytes kein
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

impl Lauf {
    /// Hochladen mit erwarteter 422; liefert `grund`.
    async fn abgewiesen(&self, name: &str, inhalt: &[u8]) -> String {
        let basis = self.instanz.adresse.as_str();
        let angemeldet = anmelden(&self.beta, basis, &self.token, name, inhalt.len() as u64).await;
        let antwort = hochladen(
            &self.beta,
            basis,
            &self.token,
            &id(&angemeldet),
            inhalt.to_vec(),
        )
        .await;
        assert_eq!(
            (antwort.status, antwort.json["regel"].as_str()),
            (422, Some("metadaten")),
            "{name}: {}",
            antwort.json
        );
        antwort.json["grund"]
            .as_str()
            .unwrap_or_default()
            .to_string()
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

fn webp_teil(typ: &[u8; 4], daten: &[u8]) -> Vec<u8> {
    let mut bytes = typ.to_vec();
    bytes.extend((daten.len() as u32).to_le_bytes());
    bytes.extend(daten);
    if daten.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

/// WebP 64 × 48 (`VP8X` mit `flaggen`) mit den Chunks dahinter.
fn webp_mit(flaggen: u8, teile: &[Vec<u8>]) -> Vec<u8> {
    let vp8x = webp_teil(b"VP8X", &[flaggen, 0, 0, 0, 63, 0, 0, 47, 0, 0]);
    let inhalt = [&b"WEBP"[..], &vp8x, &teile.concat()].concat();
    [&b"RIFF"[..], &(inhalt.len() as u32).to_le_bytes(), &inhalt].concat()
}

/// GIF 64 × 48 mit den Erweiterungen vor zwei Bildern.
fn gif_mit(erweiterungen: &[u8]) -> Vec<u8> {
    let mut bytes = b"GIF89a\x40\0\x30\0\x80\0\0\0\0\0\xFF\xFF\xFF".to_vec();
    bytes.extend(erweiterungen);
    for _ in 0..2 {
        bytes.extend(b"\x21\xF9\x04\x04\x0a\0\0\0");
        bytes.extend(b"\x2C\0\0\0\0\x40\0\x30\0\0\x02\x02\x44\x01\0");
    }
    bytes.push(0x3B);
    bytes
}

/// ADR-0047 B2 P7 und P8 (WebP-, GIF-Teil) (B0643): WebP mit `C2PA` und unbekanntem `ABCD` → ohne Ort; GIF mit
/// Anwendung `ABCDEFGH123` → ohne Ort; WebP animiert mit `ICCP` und GIF animiert mit `NETSCAPE2.0` und
/// `ICCRGBG1012` → byte-gleich, Art und Maße wie vorher.
#[tokio::test]
async fn webp_und_gif_traeger_ohne_ort_animation_und_farbe_bleiben() {
    let lauf = Lauf::neu().await;
    let vp8 = webp_teil(b"VP8 ", b"Bilddaten");
    let original = webp_mit(
        0,
        &[
            webp_teil(b"C2PA", &ort()),
            webp_teil(b"ABCD", &ort()),
            vp8.clone(),
        ],
    );
    let geliefert = lauf.ausliefern("p7.webp", &original).await;
    ohne_ort("p7.webp", &geliefert.bytes);
    assert!(enthaelt(&geliefert.bytes, &vp8));
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/webp".to_string(), json!(64), json!(48))
    );

    let mut anwendung = b"\x21\xFF\x0bABCDEFGH123".to_vec();
    anwendung.push(ort().len() as u8);
    anwendung.extend(ort());
    anwendung.push(0);
    let original = gif_mit(&anwendung);
    let geliefert = lauf.ausliefern("p7.gif", &original).await;
    ohne_ort("p7.gif", &geliefert.bytes);
    assert!(!enthaelt(&geliefert.bytes, b"ABCDEFGH"));
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/gif".to_string(), json!(64), json!(48))
    );

    let rahmen = |bild: &[u8]| {
        let kopf = [0u8, 0, 0, 0, 0, 0, 63, 0, 0, 47, 0, 0, 100, 0, 0, 0];
        webp_teil(b"ANMF", &[&kopf[..], &webp_teil(b"VP8 ", bild)].concat())
    };
    let original = webp_mit(
        0x22,
        &[
            webp_teil(b"ICCP", b"Farbprofil"),
            webp_teil(b"ANIM", &[0, 0, 0, 0, 0, 0]),
            rahmen(b"Rahmen eins"),
            rahmen(b"Rahmen zwei"),
        ],
    );
    let geliefert = lauf.ausliefern("p8.webp", &original).await;
    assert_eq!(geliefert.bytes, original);
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/webp".to_string(), json!(64), json!(48))
    );

    let original = gif_mit(
        &[
            &b"\x21\xFF\x0bNETSCAPE2.0\x03\x01\0\0\0"[..],
            b"\x21\xFF\x0bICCRGBG1012\x0aFarbprofil\0",
        ]
        .concat(),
    );
    let geliefert = lauf.ausliefern("p8.gif", &original).await;
    assert_eq!(geliefert.bytes, original);
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/gif".to_string(), json!(64), json!(48))
    );
}

fn kiste(typ: &[u8], inhalt: &[u8]) -> Vec<u8> {
    let mut bytes = ((8 + inhalt.len()) as u32).to_be_bytes().to_vec();
    bytes.extend(typ);
    bytes.extend(inhalt);
    bytes
}

/// Typ der C2PA-`uuid`-Box.
const C2PA_UUID: [u8; 16] = [
    0xD8, 0xFE, 0xC3, 0xD6, 0x1B, 0x0E, 0x48, 0x3C, 0x92, 0x97, 0x58, 0x28, 0x87, 0x7E, 0xC4, 0x81,
];

/// MP4: `ftyp`, die Boxen `oben`, `moov` mit `mvhd` und `moov_rest`, `mdat`.
fn mp4_mit(oben: &[Vec<u8>], moov_rest: &[Vec<u8>]) -> Vec<u8> {
    let moov = kiste(
        b"moov",
        &[vec![kiste(b"mvhd", &[0; 100])], moov_rest.to_vec()]
            .concat()
            .concat(),
    );
    [
        vec![kiste(b"ftyp", b"isom\0\0\x02\0isommp41")],
        oben.to_vec(),
        vec![moov, kiste(b"mdat", b"MEDIENDATEN-MEDIENDATEN")],
    ]
    .concat()
    .concat()
}

fn mdat(bytes: &[u8]) -> &[u8] {
    let stelle = bytes.windows(4).rposition(|f| f == b"mdat").expect("mdat");
    &bytes[stelle - 4..]
}

/// ADR-0047 B2 P4, P6, P9 (B0644): GPS nur in einer `uuid` oben mit C2PA-Typ bzw. nur in `moov/trak/udta/©xyz` →
/// ausgeliefert ohne Ort, `mdat` byte-gleich; `cmov` in `moov` → 422; `udta` mit 4 Nullbytes am Ende → getilgt;
/// eine Box in `udta` mit `size` über das Ende von `udta` → 422.
#[tokio::test]
async fn mp4_traeger_oben_und_in_trak_ohne_ort_leser_grenzen() {
    let lauf = Lauf::neu().await;
    let p4 = mp4_mit(&[kiste(b"uuid", &[&C2PA_UUID[..], &ort()].concat())], &[]);
    let xyz = kiste(&[0xa9, b'x', b'y', b'z'], &ort());
    let p6 = mp4_mit(&[], &[kiste(b"trak", &kiste(b"udta", &xyz))]);
    for (name, original) in [("p4.mp4", p4), ("p6.mp4", p6)] {
        let geliefert = lauf.ausliefern(name, &original).await;
        ohne_ort(name, &geliefert.bytes);
        assert!(!enthaelt(&geliefert.bytes, &C2PA_UUID), "{name}");
        assert_eq!(mdat(&geliefert.bytes), mdat(&original), "{name}");
    }

    let cmov = mp4_mit(&[], &[kiste(b"cmov", b"gepackter Filmkopf")]);
    assert_eq!(
        lauf.abgewiesen("cmov.mp4", &cmov).await,
        "cmov nicht prüfbar"
    );

    let abschluss = mp4_mit(&[], &[kiste(b"udta", &[xyz.clone(), vec![0; 4]].concat())]);
    let geliefert = lauf.ausliefern("abschluss.mov", &abschluss).await;
    ohne_ort("abschluss.mov", &geliefert.bytes);

    let mut zu_lang = xyz.clone();
    zu_lang[3] += 1;
    let ueber = mp4_mit(&[], &[kiste(b"udta", &zu_lang), kiste(b"free", b"x")]);
    lauf.abgewiesen("ueber.mp4", &ueber).await;
}

/// ADR-0047 B2 P5 (B0645): GPS nur in `moov/udta/meta/ilst/©xyz` („+52.5200+013.4050/“), dazu Hersteller und Titel
/// → ausgeliefert ohne Ort und ohne die anderen Werte, `mdat` byte-gleich.
#[tokio::test]
async fn mp4_ilst_verliert_alle_werte() {
    let lauf = Lauf::neu().await;
    let wert = |typ: &[u8], text: &[u8]| {
        kiste(
            typ,
            &kiste(b"data", &[&[0u8, 0, 0, 1, 0, 0, 0, 0][..], text].concat()),
        )
    };
    let ilst = kiste(
        b"ilst",
        &[
            wert(&[0xa9, b'x', b'y', b'z'], b"+52.5200+013.4050/"),
            wert(&[0xa9, b'm', b'a', b'k'], b"Telefonbauer"),
            wert(&[0xa9, b'n', b'a', b'm'], b"Urlaubsfilm"),
        ]
        .concat(),
    );
    let hdlr = kiste(b"hdlr", &[&[0u8; 8][..], b"mdir", &[0; 13]].concat());
    let meta = kiste(b"meta", &[&[0u8; 4][..], &hdlr, &ilst].concat());
    let original = mp4_mit(&[], &[kiste(b"udta", &meta)]);

    let geliefert = lauf.ausliefern("p5.mp4", &original).await;

    ohne_ort("p5.mp4", &geliefert.bytes);
    assert!(
        !enthaelt(&geliefert.bytes, b"Telefonbauer") && !enthaelt(&geliefert.bytes, b"Urlaubsfilm")
    );
    assert_eq!(mdat(&geliefert.bytes), mdat(&original));
}

fn infe(id: u16, typ: &[u8; 4], rest: &[u8]) -> Vec<u8> {
    kiste(
        b"infe",
        &[
            &[2u8, 0, 0, 0][..],
            &id.to_be_bytes(),
            &[0, 0],
            typ,
            b"\0",
            rest,
        ]
        .concat(),
    )
}

/// HEIC mit Items (Typ, `infe`-Rest, Daten im `mdat`), Eigenschaften in `ipco` und weiteren Boxen im `meta`.
fn heic_mit(
    items: &[(&[u8; 4], &[u8], Vec<u8>)],
    ipco: &[Vec<u8>],
    weitere: &[Vec<u8>],
) -> Vec<u8> {
    heic_mit_iloc(items, ipco, weitere, |anfaenge| {
        let mut iloc = vec![1u8, 0, 0, 0, 0x44, 0x00];
        iloc.extend((items.len() as u16).to_be_bytes());
        for (n, ((_, _, daten), ab)) in items.iter().zip(anfaenge).enumerate() {
            iloc.extend((n as u16 + 1).to_be_bytes());
            iloc.extend([0, 0, 0, 0]);
            iloc.extend(1u16.to_be_bytes());
            iloc.extend((*ab as u32).to_be_bytes());
            iloc.extend((daten.len() as u32).to_be_bytes());
        }
        kiste(b"iloc", &iloc)
    })
}

/// HEIC wie `heic_mit`, das `iloc` baut `iloc` aus den Anfängen der Item-Daten in der Datei (Item n hat ID n + 1).
fn heic_mit_iloc(
    items: &[(&[u8; 4], &[u8], Vec<u8>)],
    ipco: &[Vec<u8>],
    weitere: &[Vec<u8>],
    iloc: impl Fn(&[u64]) -> Vec<u8>,
) -> Vec<u8> {
    let iinf = kiste(
        b"iinf",
        &[
            &[0u8, 0, 0, 0][..],
            &(items.len() as u16).to_be_bytes(),
            &items
                .iter()
                .enumerate()
                .map(|(n, (typ, rest, _))| infe(n as u16 + 1, typ, rest))
                .collect::<Vec<_>>()
                .concat(),
        ]
        .concat(),
    );
    let iprp = kiste(b"iprp", &kiste(b"ipco", &ipco.concat()));
    let bauen = |mdat_anfang: u64| {
        let anfaenge: Vec<u64> = items
            .iter()
            .scan(mdat_anfang, |ab, (_, _, daten)| {
                let anfang = *ab;
                *ab += daten.len() as u64;
                Some(anfang)
            })
            .collect();
        let hdlr = kiste(b"hdlr", &[&[0u8; 8][..], b"pict", &[0; 13]].concat());
        let meta = kiste(
            b"meta",
            &[
                &[0u8; 4][..],
                &hdlr,
                &kiste(b"pitm", b"\0\0\0\0\0\x01"),
                &iinf,
                &iloc(&anfaenge),
                &iprp,
                &weitere.concat(),
            ]
            .concat(),
        );
        [kiste(b"ftyp", b"heic\0\0\0\0mif1heic"), meta].concat()
    };
    let mdat_anfang = bauen(0).len() as u64 + 8;
    let daten: Vec<u8> = items
        .iter()
        .flat_map(|(_, _, daten)| daten.clone())
        .collect();
    [bauen(mdat_anfang), kiste(b"mdat", &daten)].concat()
}

/// ADR-0047 B2 P7 und P8 (HEIC-Teil) (B0646): `mime`-Item `application/c2pa`, `xml ` im obersten `meta`, `udes` in
/// `ipco` → ohne Ort; Kachel (`hvc1`), `grid`, Gain-Map (`tmap`), `colr`, `irot`, `ispe` byte-gleich, Art wie vorher.
/// Kontrolle: ein echtes AVIF (ffmpeg) kommt byte-gleich an.
#[tokio::test]
async fn heic_traeger_ohne_ort_bild_items_und_eigenschaften_bleiben() {
    let lauf = Lauf::neu().await;
    let mut exif = 0u32.to_be_bytes().to_vec();
    exif.extend(b"MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01\0\x06\0\0\0\0\0\0");
    exif.extend(ort());
    let kachel = vec![0x5a; 64];
    let grid = vec![0, 0, 0, 0, 0, 0x40, 0, 0x30];
    let tmap = vec![0x7b; 32];
    let colr = kiste(b"colr", b"nclx\0\x01\0\x0d\0\x01\x80");
    let irot = kiste(b"irot", b"\x01");
    let ispe = kiste(b"ispe", b"\0\0\0\0\0\0\0\x40\0\0\0\x30");
    let original = heic_mit(
        &[
            (b"grid", b"", grid.clone()),
            (b"hvc1", b"", kachel.clone()),
            (b"tmap", b"", tmap.clone()),
            (b"Exif", b"", exif),
            (
                b"mime",
                b"application/c2pa\0",
                [&b"c2pa "[..], &ort()].concat(),
            ),
        ],
        &[
            colr.clone(),
            irot.clone(),
            ispe.clone(),
            kiste(b"udes", &[&b"\0\0\0\0de\0"[..], &ort()].concat()),
        ],
        &[kiste(b"xml ", &[&b"\0\0\0\0<x>"[..], &ort()].concat())],
    );

    let geliefert = lauf.ausliefern("p7.heic", &original).await;

    ohne_ort("p7.heic", &geliefert.bytes);
    for bleibt in [&kachel, &grid, &tmap, &colr, &irot, &ispe] {
        assert!(enthaelt(&geliefert.bytes, bleibt));
    }
    assert_eq!(geliefert.anhang["art"], "image/heic");

    let avif = gemeinsam::dateien::avif_foto();
    let geliefert = lauf.ausliefern("foto.avif", &avif).await;
    assert_eq!(geliefert.bytes, avif, "echtes AVIF byte-gleich");
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/avif".to_string(), json!(4032), json!(3024))
    );
}

/// 2026-10-29T12:34:56Z in Sekunden seit 1904-01-01.
const ZEIT_1904: u32 = 3_876_122_096;

/// Ein Film aus `ffmpeg` (Testbild und Ton, 1 s, `creation_time` 2026-10-29T12:34:56Z, Ort) mit weiteren Optionen.
fn film(ordner: &std::path::Path, name: &str, optionen: &[&str]) -> Vec<u8> {
    let pfad = ordner.join(name);
    let ausgabe = std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=d=1:s=64x48:r=10",
        ])
        .args(["-f", "lavfi", "-i", "sine=d=1", "-shortest"])
        .args(["-metadata", "creation_time=2026-10-29T12:34:56Z"])
        .args(["-metadata", "location=+52.5200+013.4050/"])
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"])
        .args(optionen)
        .arg(&pfad)
        .output()
        .expect("Vorbedingung fehlt: ffmpeg");
    assert!(
        ausgabe.status.success(),
        "{}",
        String::from_utf8_lossy(&ausgabe.stderr)
    );
    std::fs::read(&pfad).expect("Film")
}

/// Die Boxen einer Ebene: (Typ, Anfang des Inhalts, Ende).
fn boxen(bytes: &[u8], anfang: usize, ende: usize) -> Vec<([u8; 4], usize, usize)> {
    let mut liste = Vec::new();
    let mut pos = anfang;
    while pos + 8 <= ende {
        let groesse = u32::from_be_bytes(bytes[pos..pos + 4].try_into().expect("Größe")) as usize;
        let typ: [u8; 4] = bytes[pos + 4..pos + 8].try_into().expect("Typ");
        let (inhalt, bis) = match groesse {
            1 => (
                pos + 16,
                pos + u64::from_be_bytes(bytes[pos + 8..pos + 16].try_into().expect("lang"))
                    as usize,
            ),
            0 => (pos + 8, ende),
            _ => (pos + 8, pos + groesse),
        };
        liste.push((typ, inhalt, bis));
        pos = bis;
    }
    liste
}

/// Alle Boxen `typ` unter den Behältern `moov`, `trak`, `mdia`, `moof`, `traf` (Inhalt).
fn finden<'a>(bytes: &'a [u8], typ: &[u8; 4]) -> Vec<&'a [u8]> {
    fn suchen<'a>(
        bytes: &'a [u8],
        anfang: usize,
        ende: usize,
        typ: &[u8; 4],
        funde: &mut Vec<&'a [u8]>,
    ) {
        for (art, inhalt, bis) in boxen(bytes, anfang, ende) {
            if &art == typ {
                funde.push(&bytes[inhalt..bis]);
            } else if [b"moov", b"trak", b"mdia", b"moof", b"traf"].contains(&&art) {
                suchen(bytes, inhalt, bis, typ, funde);
            }
        }
    }
    let mut funde = Vec::new();
    suchen(bytes, 0, bytes.len(), typ, &mut funde);
    funde
}

/// Baut `mvhd`, `tkhd` und `mdhd` in `moov` auf Version 1 (64 Bit) um; `moov` muss hinter `mdat` stehen, damit die
/// Versätze in `stco` gelten.
fn version_1(bytes: &[u8]) -> Vec<u8> {
    fn umbauen(bytes: &[u8], anfang: usize, ende: usize) -> Vec<u8> {
        let mut neu = Vec::new();
        for (typ, inhalt, bis) in boxen(bytes, anfang, ende) {
            let teil = match &typ {
                b"moov" | b"trak" | b"mdia" => umbauen(bytes, inhalt, bis),
                b"mvhd" | b"tkhd" | b"mdhd" if bytes[inhalt] == 0 => {
                    let alt = &bytes[inhalt..bis];
                    let zahl = |stelle: usize| {
                        u64::from(u32::from_be_bytes(
                            alt[stelle..stelle + 4].try_into().expect("u32"),
                        ))
                    };
                    let mut v1 = vec![1, alt[1], alt[2], alt[3]];
                    v1.extend(zahl(4).to_be_bytes());
                    v1.extend(zahl(8).to_be_bytes());
                    v1.extend(&alt[12..16]);
                    let dauer_ab = if &typ == b"tkhd" { 20 } else { 16 };
                    v1.extend(&alt[16..dauer_ab]);
                    v1.extend(zahl(dauer_ab).to_be_bytes());
                    v1.extend(&alt[dauer_ab + 4..]);
                    v1
                }
                _ => bytes[inhalt..bis].to_vec(),
            };
            neu.extend(((teil.len() + 8) as u32).to_be_bytes());
            neu.extend(typ);
            neu.extend(teil);
        }
        neu
    }
    let liste = boxen(bytes, 0, bytes.len());
    let moov = liste
        .iter()
        .position(|(typ, _, _)| typ == b"moov")
        .expect("moov");
    let mdat = liste
        .iter()
        .position(|(typ, _, _)| typ == b"mdat")
        .expect("mdat");
    assert!(mdat < moov, "moov hinter mdat");
    umbauen(bytes, 0, bytes.len())
}

fn ffprobe(pfad: &std::path::Path, eintraege: &str) -> Value {
    let ausgabe = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", eintraege, "-of", "json"])
        .arg(pfad)
        .output()
        .expect("Vorbedingung fehlt: ffprobe");
    assert!(
        ausgabe.status.success(),
        "{}",
        String::from_utf8_lossy(&ausgabe.stderr)
    );
    serde_json::from_slice(&ausgabe.stdout).expect("JSON")
}

/// ADR-0047 B2 P10 (B0647): MP4 (Android-Art), MOV (iPhone-Art), fragmentiertes MP4 und eines mit Version-1-Köpfen,
/// je mit `creation_time` 2026-10-29T12:34:56Z → ausgeliefert zeigt `ffprobe` nirgends `creation_time`, die Zeitfelder
/// in `mvhd`, `tkhd`, `mdhd` sind Nullen, die Zeit kommt in keiner Box vor; Dauer, `time_base`, Spuren wie vorher,
/// `ffmpeg` dekodiert ohne Fehler; Länge, `mdat`, `tfdt` byte-gleich; der Ort ist weg.
#[tokio::test]
async fn zeitfelder_der_struktur_sind_getilgt() {
    let lauf = Lauf::neu().await;
    let ordner = ordner();
    let android = film(ordner.path(), "android.mp4", &[]);
    let faelle = [
        ("android.mp4", android.clone()),
        ("iphone.mov", film(ordner.path(), "iphone.mov", &[])),
        (
            "fragmentiert.mp4",
            film(
                ordner.path(),
                "fragmentiert.mp4",
                &["-movflags", "frag_keyframe+empty_moov+default_base_moof"],
            ),
        ),
        ("version1.mp4", version_1(&android)),
    ];
    let zeit = ZEIT_1904.to_be_bytes();
    let dauer = "format=duration:stream=index,codec_type,time_base,duration";
    let zeittags = "format_tags=creation_time:stream_tags=creation_time";
    for (name, original) in faelle {
        let vorher_pfad = ordner.path().join(format!("vorher-{name}"));
        std::fs::write(&vorher_pfad, &original).expect("geschrieben");
        assert!(
            ffprobe(&vorher_pfad, zeittags)
                .to_string()
                .contains("2026-10-29T12:34:56"),
            "{name}: Probe ohne Zeit"
        );
        assert!(enthaelt(&original, &zeit), "{name}");

        let geliefert = lauf.ausliefern(name, &original).await;

        let pfad = ordner.path().join(format!("nachher-{name}"));
        std::fs::write(&pfad, &geliefert.bytes).expect("geschrieben");
        let tags = ffprobe(&pfad, zeittags).to_string();
        assert!(!tags.contains("creation_time"), "{name}: {tags}");
        assert_eq!(
            ffprobe(&pfad, dauer),
            ffprobe(&vorher_pfad, dauer),
            "{name}"
        );
        let dekodiert = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(&pfad)
            .args(["-f", "null", "-"])
            .output()
            .expect("ffmpeg");
        assert!(
            dekodiert.status.success() && dekodiert.stderr.is_empty(),
            "{name}: {}",
            String::from_utf8_lossy(&dekodiert.stderr)
        );
        for typ in [b"mvhd", b"tkhd", b"mdhd"] {
            let funde = finden(&geliefert.bytes, typ);
            assert!(!funde.is_empty(), "{name}");
            for kiste in funde {
                let breite = if kiste[0] == 1 { 8 } else { 4 };
                assert!(
                    kiste[4..4 + 2 * breite].iter().all(|b| *b == 0),
                    "{name}: {}",
                    String::from_utf8_lossy(typ)
                );
            }
        }
        assert!(!enthaelt(&geliefert.bytes, &zeit), "{name}: Zeit");
        assert_eq!(
            finden(&geliefert.bytes, b"mdat"),
            finden(&original, b"mdat"),
            "{name}"
        );
        assert_eq!(
            finden(&geliefert.bytes, b"tfdt"),
            finden(&original, b"tfdt"),
            "{name}"
        );
        assert!(!enthaelt(&geliefert.bytes, b"+52.5200"), "{name}: Ort");
    }
}

/// Eine Ortsspur wie in Dashcams (BlackVue, LIGO): NMEA-Satz, Koordinate als Text und binär.
fn nmea() -> Vec<u8> {
    [
        &b"$GPRMC,123456.00,A,5231.2000,N,01324.3000,E,0.0,0.0,291026,,,A*6C "[..],
        &ort(),
    ]
    .concat()
}

/// Hängt `neu` ans Ende der Box, die `pfad` von oben nennt (leer: ans Ende der Datei); die Größen der Behälter
/// wachsen mit. `moov` muss hinter `mdat` stehen, damit die Versätze in `stco` gelten.
fn einfuegen(bytes: &[u8], pfad: &[&[u8; 4]], neu: &[u8]) -> Vec<u8> {
    fn bauen(bytes: &[u8], anfang: usize, ende: usize, pfad: &[&[u8; 4]], neu: &[u8]) -> Vec<u8> {
        let Some((ziel, rest)) = pfad.split_first() else {
            return [&bytes[anfang..ende], neu].concat();
        };
        let mut aus = Vec::new();
        let mut gefunden = false;
        for (typ, inhalt, bis) in boxen(bytes, anfang, ende) {
            if gefunden || &typ != *ziel {
                aus.extend(&bytes[inhalt - 8..bis]);
                continue;
            }
            gefunden = true;
            let teil = bauen(bytes, inhalt, bis, rest, neu);
            aus.extend(((teil.len() + 8) as u32).to_be_bytes());
            aus.extend(typ);
            aus.extend(teil);
        }
        assert!(gefunden, "{}", String::from_utf8_lossy(*ziel));
        aus
    }
    let liste = boxen(bytes, 0, bytes.len());
    let stelle = |gesucht: &[u8; 4]| liste.iter().position(|(typ, _, _)| typ == gesucht);
    assert!(stelle(b"mdat") < stelle(b"moov"), "moov hinter mdat");
    bauen(bytes, 0, bytes.len(), pfad, neu)
}

/// Der Platzhalter `alt` (aus dem Original) steht in `bytes` an derselben Stelle mit gleichem Kopf und Nullen.
fn platzhalter_genullt(name: &str, original: &[u8], bytes: &[u8], alt: &[u8]) {
    let stelle = original
        .windows(alt.len())
        .position(|fenster| fenster == alt)
        .expect("Platzhalter im Original");
    assert_eq!(bytes[stelle..stelle + 8], alt[..8], "{name}: Typ und Größe");
    assert!(
        bytes[stelle + 8..stelle + alt.len()]
            .iter()
            .all(|b| *b == 0),
        "{name}: Inhalt"
    );
}

/// `ffprobe -v error` ohne Ausgabe und `ffmpeg -v error -i … -f null -` mit Code 0 ohne Ausgabe.
fn spielt_fehlerfrei(name: &str, bytes: &[u8]) {
    let ordner = ordner();
    let pfad = ordner.path().join(name);
    std::fs::write(&pfad, bytes).expect("geschrieben");
    let geprobt = std::process::Command::new("ffprobe")
        .args(["-v", "error"])
        .arg(&pfad)
        .output()
        .expect("Vorbedingung fehlt: ffprobe");
    assert!(
        geprobt.status.success() && geprobt.stdout.is_empty() && geprobt.stderr.is_empty(),
        "{name}: {}",
        String::from_utf8_lossy(&geprobt.stderr)
    );
    let dekodiert = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(&pfad)
        .args(["-f", "null", "-"])
        .output()
        .expect("Vorbedingung fehlt: ffmpeg");
    assert!(
        dekodiert.status.success() && dekodiert.stderr.is_empty(),
        "{name}: {}",
        String::from_utf8_lossy(&dekodiert.stderr)
    );
}

/// Kein Ort, keine NMEA-Spur, gleiche Länge, `mdat` byte-gleich, der Platzhalter `alt` mit Kopf und Nullen.
fn platzhalter_ohne_ort(name: &str, original: &[u8], bytes: &[u8], alt: &[u8]) {
    ohne_ort(name, bytes);
    for spur in [&b"5231.2000"[..], b"$GPRMC", b"LIGOGPSINFO", b"ptrh"] {
        assert!(
            !enthaelt(bytes, spur),
            "{name}: {}",
            String::from_utf8_lossy(spur)
        );
    }
    assert_eq!(bytes.len(), original.len(), "{name}");
    assert_eq!(finden(bytes, b"mdat"), finden(original, b"mdat"), "{name}");
    platzhalter_genullt(name, original, bytes, alt);
}

/// Ein JPEG (Vorschau einer Drohne) mit `APP1` Exif und GPS-IFD: `GPSLatitude` 52° 31′ 12″ als RATIONAL.
fn vorschau_mit_gps() -> Vec<u8> {
    let mut gps_ifd = 1u16.to_be_bytes().to_vec();
    gps_ifd.extend([0, 2, 0, 5, 0, 0, 0, 3, 0, 0, 0, 56]);
    gps_ifd.extend(0u32.to_be_bytes());
    gps_ifd.extend(ORT_BINAER);
    let tiff = gemeinsam::dateien::tiff(&[(0x0112, 3, 1, 1), (0x8825, 4, 1, 38)], &gps_ifd);
    jpeg(&[(0xe1, [&b"Exif\0\0"[..], &tiff].concat())], 64, 48)
}

/// ADR-0047 B3 Regel 5h, Proben (a)–(g) des `security-reviewer` (B0649): Ortsspur nur in (a) oberstem `free` mit
/// `ptrh`/`gps`, (b) oberstem `skip` mit `LIGOGPSINFO`, (c) oberstem `free` mit `mdat` und JPEG mit Exif-GPS,
/// (d) `free` in `moov`, (e) `free` in `udta` und in `udta/meta`, (f) oberstem `wide`, (g) `free` im obersten
/// HEIF-`meta` → ausgeliefert ohne Ort, Typ und Größe gleich, Inhalt Nullen, Länge und `mdat` gleich; die Filme
/// spielen fehlerfrei (`ffprobe`, `ffmpeg`).
#[tokio::test]
async fn platzhalter_verlieren_ihre_ortsspur() {
    let lauf = Lauf::neu().await;
    let ordner = ordner();
    let grund = film(ordner.path(), "grund.mp4", &[]);
    let spur = |typ: &[u8; 4], marke: &[u8]| kiste(typ, &[marke, &nmea()].concat());
    let mit_jpeg = kiste(b"free", &kiste(b"mdat", &vorschau_mit_gps()));
    /// Name, Pfad der Elternbox (leer: oben), Platzhalter.
    type Fall<'a> = (&'a str, &'a [&'a [u8; 4]], Vec<u8>);
    let filme: Vec<Fall> = vec![
        ("a.mp4", &[], spur(b"free", b"ptrh\0\0\0\x40gps ")),
        ("b.mp4", &[], spur(b"skip", b"LIGOGPSINFO\0")),
        ("c.mp4", &[], mit_jpeg),
        ("d.mp4", &[b"moov"], spur(b"free", b"im moov ")),
        ("e1.mp4", &[b"moov", b"udta"], spur(b"free", b"in udta ")),
        (
            "e2.mp4",
            &[b"moov", b"udta", b"meta"],
            spur(b"free", b"in meta "),
        ),
        ("f.mp4", &[], spur(b"wide", b"wide ")),
    ];
    for (name, pfad, platzhalter) in filme {
        let original = einfuegen(&grund, pfad, &platzhalter);
        spielt_fehlerfrei(&format!("vorher-{name}"), &original);

        let geliefert = lauf.ausliefern(name, &original).await;

        platzhalter_ohne_ort(name, &original, &geliefert.bytes, &platzhalter);
        spielt_fehlerfrei(name, &geliefert.bytes);
    }

    let im_heif = spur(b"free", b"HEIF meta ");
    let kachel = vec![0x5a; 64];
    let heic = heic_mit(
        &[(b"hvc1", b"", kachel.clone())],
        &[],
        std::slice::from_ref(&im_heif),
    );
    let geliefert = lauf.ausliefern("g.heic", &heic).await;
    platzhalter_ohne_ort("g.heic", &heic, &geliefert.bytes, &im_heif);
    assert!(enthaelt(&geliefert.bytes, &kachel));
    assert_eq!(geliefert.anhang["art"], "image/heic");
}

/// Ein `meta` mit `ilst/©xyz` (Ort) und Hersteller.
fn ilst_meta() -> Vec<u8> {
    let wert = |typ: &[u8], text: &[u8]| {
        kiste(
            typ,
            &kiste(b"data", &[&[0u8, 0, 0, 1, 0, 0, 0, 0][..], text].concat()),
        )
    };
    let hdlr = kiste(b"hdlr", &[&[0u8; 8][..], b"mdir", &[0; 13]].concat());
    let ilst = kiste(b"ilst", &wert(&[0xa9, b'x', b'y', b'z'], &ort()));
    kiste(b"meta", &[&[0u8; 4][..], &hdlr, &ilst].concat())
}

/// Fügt `neu` oben vor `mfra` ein (ohne `mfra` ans Ende) — `mfra` findet ein Leser über `mfro` vom Ende her, die
/// Versätze davor bleiben gültig.
fn vor_mfra(bytes: &[u8], neu: &[u8]) -> Vec<u8> {
    let stelle = boxen(bytes, 0, bytes.len())
        .iter()
        .find(|(typ, _, _)| typ == b"mfra")
        .map_or(bytes.len(), |(_, inhalt, _)| inhalt - 8);
    [&bytes[..stelle], neu, &bytes[stelle..]].concat()
}

/// Hängt `neu` ans Ende des letzten `moof` und verschiebt `data_offset` jedes `trun` darin um dessen Länge (Versatz
/// ab dem `moof`, `default-base-is-moof`); `mfra` dahinter findet ein Leser über `mfro`.
fn in_letztes_moof(bytes: &[u8], neu: &[u8]) -> Vec<u8> {
    let liste = boxen(bytes, 0, bytes.len());
    let &(_, inhalt, ende) = liste
        .iter()
        .rfind(|(typ, _, _)| typ == b"moof")
        .expect("moof");
    let mut moof = bytes[inhalt - 8..ende].to_vec();
    for (typ, traf_inhalt, traf_ende) in boxen(bytes, inhalt, ende) {
        if &typ != b"traf" {
            continue;
        }
        for (typ, trun, _) in boxen(bytes, traf_inhalt, traf_ende) {
            if &typ == b"trun" && bytes[trun + 3] & 1 == 1 {
                let stelle = trun + 8 - (inhalt - 8);
                let alt = i32::from_be_bytes(moof[stelle..stelle + 4].try_into().expect("i32"));
                moof[stelle..stelle + 4].copy_from_slice(&(alt + neu.len() as i32).to_be_bytes());
            }
        }
    }
    moof.extend(neu);
    let groesse = moof.len() as u32;
    moof[..4].copy_from_slice(&groesse.to_be_bytes());
    [&bytes[..inhalt - 8], &moof, &bytes[ende..]].concat()
}

/// NTP-Zeit 2026-10-29T12:34:56Z (Sekunden seit 1900, Bruchteil 0).
const NTP: [u8; 8] = [0xee, 0x8d, 0xbb, 0xf0, 0, 0, 0, 0];

/// ADR-0047 B3 Regel 5i, P11 (h)–(k) (B0650): ein fragmentiertes MP4 mit (h) `meco` oben samt `meta`/`ilst`/`©xyz`,
/// (i) `meta`/`ilst`/`©xyz` im `moof`, (j) `emsg` mit Ort in `message_data`, (k) `prft` mit `ntp_timestamp`
/// 2026-10-29T12:34:56Z → ausgeliefert ohne Ort, die Box ist `free` mit Nullen, die NTP-Bytefolge kommt nicht vor;
/// `mfhd`, `traf`, `mfra`, `mdat` byte-gleich; `ffprobe`/`ffmpeg` fehlerfrei.
#[tokio::test]
async fn meco_moof_emsg_und_prft_verlieren_ihre_ortsspur() {
    let lauf = Lauf::neu().await;
    let ordner = ordner();
    let grund = film(
        ordner.path(),
        "fragmentiert.mp4",
        &["-movflags", "frag_keyframe+empty_moov+default_base_moof"],
    );
    let meco = kiste(b"meco", &ilst_meta());
    let emsg = kiste(
        b"emsg",
        &[&[0u8; 4][..], b"urn:ort\0\0", &[0; 16], &ort()].concat(),
    );
    let prft = kiste(
        b"prft",
        &[&[0u8; 4][..], &[0, 0, 0, 1], &NTP, &[0; 4]].concat(),
    );
    let faelle = [
        ("h.mp4", vor_mfra(&grund, &meco), meco),
        ("i.mp4", in_letztes_moof(&grund, &ilst_meta()), ilst_meta()),
        ("j.mp4", vor_mfra(&grund, &emsg), emsg),
        ("k.mp4", vor_mfra(&grund, &prft), prft),
    ];
    for (name, original, kiste) in faelle {
        spielt_fehlerfrei(&format!("vorher-{name}"), &original);

        let geliefert = lauf.ausliefern(name, &original).await;

        let bytes = &geliefert.bytes;
        ohne_ort(name, bytes);
        assert!(
            !enthaelt(bytes, &NTP) && !enthaelt(bytes, b"urn:ort"),
            "{name}"
        );
        let stelle = original
            .windows(kiste.len())
            .position(|fenster| fenster == kiste)
            .expect("Box im Original");
        assert_eq!(bytes[stelle..stelle + 4], kiste[..4], "{name}: Größe");
        assert_eq!(&bytes[stelle + 4..stelle + 8], b"free", "{name}: Typ");
        assert!(
            bytes[stelle + 8..stelle + kiste.len()]
                .iter()
                .all(|b| *b == 0),
            "{name}: Inhalt"
        );
        for typ in [b"mdat", b"mfhd", b"traf", b"mfra"] {
            assert_eq!(finden(bytes, typ), finden(&original, typ), "{name}");
        }
        assert_eq!(bytes.len(), original.len(), "{name}");
        spielt_fehlerfrei(name, bytes);
    }
}

/// ADR-0047 B3 Regel 5j, P11 (l) (B0651): HEIC mit zweitem `iinf`, zweitem `iloc`, zweitem `hdlr` → je 422
/// `metadaten` „HEIF-Kind doppelt“. Kontrolle: HEIC nach Art des iPhone (`grid`, Kacheln, Gain-Map, `colr`, `irot`)
/// mit mehreren `free`/`skip` im obersten `meta` → getilgt, Art wie vorher, Bilddaten byte-gleich; ein echtes AVIF
/// (ffmpeg) byte-gleich, Maße wie vorher.
#[tokio::test]
async fn heif_kinder_hoechstens_einmal() {
    let lauf = Lauf::neu().await;
    let kachel = vec![0x5a; 64];
    let items = [
        (b"grid", &b""[..], vec![0, 0, 0, 0, 0, 0x40, 0, 0x30]),
        (b"hvc1", b"", kachel.clone()),
        (b"tmap", b"", vec![0x7b; 32]),
    ];
    let ipco = [
        kiste(b"colr", b"nclx\0\x01\0\x0d\0\x01\x80"),
        kiste(b"irot", b"\x01"),
    ];
    let hdlr = kiste(b"hdlr", &[&[0u8; 8][..], b"pict", &[0; 13]].concat());
    for (name, zweites) in [
        ("iinf.heic", kiste(b"iinf", &[0u8, 0, 0, 0, 0, 0])),
        ("iloc.heic", kiste(b"iloc", &[1u8, 0, 0, 0, 0x44, 0, 0, 0])),
        ("hdlr.heic", hdlr),
    ] {
        let doppelt = heic_mit(&items, &ipco, &[zweites]);
        assert_eq!(
            lauf.abgewiesen(name, &doppelt).await,
            "HEIF-Kind doppelt",
            "{name}"
        );
    }

    let platzhalter = [
        kiste(b"free", &nmea()),
        kiste(b"skip", &nmea()),
        kiste(b"free", b"zwei"),
        kiste(b"skip", b"zwei"),
    ];
    let original = heic_mit(&items, &ipco, &platzhalter);
    let geliefert = lauf.ausliefern("platzhalter.heic", &original).await;
    ohne_ort("platzhalter.heic", &geliefert.bytes);
    assert!(enthaelt(&geliefert.bytes, &kachel) && enthaelt(&geliefert.bytes, &ipco.concat()));
    assert_eq!(geliefert.anhang["art"], "image/heic");

    let avif = gemeinsam::dateien::avif_foto();
    let geliefert = lauf.ausliefern("foto.avif", &avif).await;
    assert_eq!(geliefert.bytes, avif, "echtes AVIF byte-gleich");
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/avif".to_string(), json!(4032), json!(3024))
    );
}

/// Ein `iloc`-Eintrag: ID, `construction_method`, `base_offset`, Ausdehnungen (Versatz, Länge).
type IlocEintrag = (u16, u16, u64, Vec<(u64, u64)>);

/// `iloc` Version 1 mit 8-Byte-Feldern für Versatz, Länge und Basis.
fn iloc_breit(eintraege: &[IlocEintrag]) -> Vec<u8> {
    let mut inhalt = vec![1u8, 0, 0, 0, 0x88, 0x80];
    inhalt.extend((eintraege.len() as u16).to_be_bytes());
    for (id, methode, basis, ausdehnungen) in eintraege {
        inhalt.extend(id.to_be_bytes());
        inhalt.extend(methode.to_be_bytes());
        inhalt.extend([0, 0]);
        inhalt.extend(basis.to_be_bytes());
        inhalt.extend((ausdehnungen.len() as u16).to_be_bytes());
        for (ab, lang) in ausdehnungen {
            inhalt.extend(ab.to_be_bytes());
            inhalt.extend(lang.to_be_bytes());
        }
    }
    kiste(b"iloc", &inhalt)
}

/// Exif-Item mit Ort (`exif_tiff_header_offset` 0, TIFF mit GPS-IFD).
fn exif_item_mit_ort() -> Vec<u8> {
    let mut exif = 0u32.to_be_bytes().to_vec();
    exif.extend(b"MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01\0\x06\0\0\0\0\0\0");
    exif.extend(ort());
    exif
}

/// ADR-0047 B3 Regel 5l, P11 (n) (B0652): `iloc`-Länge 0 bei einem Exif-Item → 422 „Ziel ohne Länge“; ein Bild-Item
/// (Nicht-Ziel) mit Länge 0 und ein Exif-Item dahinter → 422 „Verweise überschneiden sich“; `base_offset` +
/// `extent_offset` mit Überlauf → 422. Kontrolle: ein Bild-Item mit Länge 0 als letztes Item ohne Ziel dahinter →
/// getilgt, Bilddaten byte-gleich.
#[tokio::test]
async fn iloc_mit_laenge_0_und_ueberlauf() {
    let lauf = Lauf::neu().await;
    let kachel = vec![0x5a; 64];
    let exif = exif_item_mit_ort();
    let lang = exif.len() as u64;
    let kachel_vorn = [
        (b"hvc1", &b""[..], kachel.clone()),
        (b"Exif", b"", exif.clone()),
    ];
    let ohne_laenge = heic_mit_iloc(&kachel_vorn, &[], &[], |ab| {
        iloc_breit(&[(1, 0, 0, vec![(ab[0], 64)]), (2, 0, 0, vec![(ab[1], 0)])])
    });
    assert_eq!(
        lauf.abgewiesen("exif-0.heic", &ohne_laenge).await,
        "Ziel ohne Länge"
    );
    let bis_ende = heic_mit_iloc(&kachel_vorn, &[], &[], |ab| {
        iloc_breit(&[(1, 0, 0, vec![(ab[0], 0)]), (2, 0, 0, vec![(ab[1], lang)])])
    });
    assert_eq!(
        lauf.abgewiesen("bild-0.heic", &bis_ende).await,
        "Verweise überschneiden sich"
    );
    let ueberlauf = heic_mit_iloc(&kachel_vorn, &[], &[], |ab| {
        iloc_breit(&[
            (1, 0, u64::MAX, vec![(ab[0], 64)]),
            (2, 0, 0, vec![(ab[1], lang)]),
        ])
    });
    assert_eq!(
        lauf.abgewiesen("ueberlauf.heic", &ueberlauf).await,
        "Längen widersprechen sich"
    );

    let kachel_hinten = [
        (b"Exif", &b""[..], exif.clone()),
        (b"hvc1", b"", kachel.clone()),
    ];
    let am_ende = heic_mit_iloc(&kachel_hinten, &[], &[], |ab| {
        iloc_breit(&[(1, 0, 0, vec![(ab[0], lang)]), (2, 0, 0, vec![(ab[1], 0)])])
    });
    let geliefert = lauf.ausliefern("bild-0-am-ende.heic", &am_ende).await;
    ohne_ort("bild-0-am-ende.heic", &geliefert.bytes);
    assert!(geliefert.bytes.ends_with(&kachel));
}

/// Ein Vorschau-JPEG mit `APP1` Exif samt GPS-IFD (Ort binär), `APP2` ICC und Bilddaten; mit `zweites` davor ein
/// `APP2` `MPF`, dessen zweiter Eintrag ein Bild von `zweites` Bytes direkt hinter dem JPEG nennt.
fn vorschau_im_heif(zweites: Option<u32>) -> Vec<u8> {
    let icc = (0xe2, [&b"ICC_PROFILE\0\x01\x01"[..], &[0x11; 40]].concat());
    let ohne_mpf = vorschau_mit_gps();
    let mut segmente = vec![icc];
    let Some(groesse) = zweites else {
        let mut bytes = ohne_mpf[..2].to_vec();
        jpeg_segment(&mut bytes, segmente[0].0, &segmente[0].1);
        bytes.extend(&ohne_mpf[2..]);
        return bytes;
    };
    // SOI (2), Marker und Länge (4), `MPF\0` (4): der TIFF-Kopf steht bei 10; das MPF-Segment ist 66 Bytes lang.
    let ganz = (ohne_mpf.len() + 66 + 4 + segmente[0].1.len()) as u32;
    let mut index = b"MM\0*\0\0\0\x08\0\x01\xb0\x02\0\x07\0\0\0\x20\0\0\0\x1a\0\0\0\0".to_vec();
    for (laenge, ab) in [(ganz, 0), (groesse, ganz - 10)] {
        index.extend([0; 4]);
        index.extend(laenge.to_be_bytes());
        index.extend(ab.to_be_bytes());
        index.extend([0; 4]);
    }
    segmente.insert(0, (0xe2, [&b"MPF\0"[..], &index].concat()));
    let mut bytes = ohne_mpf[..2].to_vec();
    for (marke, nutzlast) in &segmente {
        jpeg_segment(&mut bytes, *marke, nutzlast);
    }
    bytes.extend(&ohne_mpf[2..]);
    assert_eq!(bytes.len() as u32, ganz);
    bytes
}

/// ADR-0047 B3 Regel 5k, P11 (m) (B0653): HEIC mit `jpeg`-Item (Vorschau) mit `APP1` Exif-GPS → ausgeliefert ohne
/// Ort, `APP2` ICC und Bilddaten ab `SOS` byte-gleich, die Kachel byte-gleich, Art wie vorher; ein `jpeg`-Item, dessen
/// MPF-Index ein Bild hinter dem Item nennt → 422; mit zwei Ausdehnungen oder in `idat` → 422.
#[tokio::test]
async fn jpeg_item_im_heif_verliert_den_ort() {
    let lauf = Lauf::neu().await;
    let kachel = vec![0x5a; 64];
    let jpeg = vorschau_im_heif(None);
    let original = heic_mit(
        &[(b"hvc1", b"", kachel.clone()), (b"jpeg", b"", jpeg.clone())],
        &[],
        &[],
    );

    let geliefert = lauf.ausliefern("vorschau.heic", &original).await;

    ohne_ort("vorschau.heic", &geliefert.bytes);
    let icc = &jpeg[2..2 + 4 + 54];
    assert!(enthaelt(icc, b"ICC_PROFILE"));
    assert!(enthaelt(&geliefert.bytes, icc) && enthaelt(&geliefert.bytes, &kachel));
    assert!(geliefert.bytes.ends_with(ab_sos(&jpeg)));
    assert_eq!(geliefert.anhang["art"], "image/heic");

    let zweites = jpeg_mit_ort_im_com();
    let zweites_lang = zweites.len() as u64;
    let mit_mpf = vorschau_im_heif(Some(zweites_lang as u32));
    let ueber = heic_mit_iloc(
        &[(b"jpeg", b"", mit_mpf.clone()), (b"hvc1", b"", zweites)],
        &[],
        &[],
        |ab| {
            iloc_breit(&[
                (1, 0, 0, vec![(ab[0], mit_mpf.len() as u64)]),
                (2, 0, 0, vec![(ab[1], zweites_lang)]),
            ])
        },
    );
    lauf.abgewiesen("mpf-ueber.heic", &ueber).await;
    let lang = jpeg.len() as u64;
    let zwei = heic_mit_iloc(&[(b"jpeg", b"", jpeg.clone())], &[], &[], |ab| {
        iloc_breit(&[(1, 0, 0, vec![(ab[0], 10), (ab[0] + 10, lang - 10)])])
    });
    lauf.abgewiesen("zwei.heic", &zwei).await;
    let idat = heic_mit_iloc(&[(b"jpeg", b"", jpeg.clone())], &[], &[], |_| {
        iloc_breit(&[(1, 1, 0, vec![(0, lang)])])
    });
    lauf.abgewiesen("idat.heic", &idat).await;
}

/// Ein JPEG mit Ort im `COM`.
fn jpeg_mit_ort_im_com() -> Vec<u8> {
    jpeg(&[(0xfe, ort())], 8, 8)
}

/// Die obersten Platzhalter (`free`, `skip`, `wide`) als (Anfang des Inhalts, Ende).
fn oberste_platzhalter(bytes: &[u8]) -> Vec<(usize, usize)> {
    boxen(bytes, 0, bytes.len())
        .into_iter()
        .filter(|(typ, _, _)| [b"free", b"skip", b"wide"].contains(&typ))
        .map(|(_, inhalt, bis)| (inhalt, bis))
        .collect()
}

/// Lage und Größe jeder obersten `mdat`.
fn lage_mdat(bytes: &[u8]) -> Vec<(usize, usize)> {
    boxen(bytes, 0, bytes.len())
        .into_iter()
        .filter(|(typ, _, _)| typ == b"mdat")
        .map(|(_, inhalt, bis)| (inhalt, bis))
        .collect()
}

/// ADR-0047 B3 P12, Kontrollen als Nachbauten (B0654; die echten Telefon- und Kameradateien im Entwicklertest): MP4
/// nach Art eines Android-Telefons (`free` vor und hinter `mdat`), MOV nach Art des iPhone (`wide` vor `mdat`), MP4
/// mit Faststart-Reserve in `free` (≥ 64 KiB), ein DASH-MP4 mit `sidx`, `mfra`, `prft` und `emsg`, ein `free` mit
/// `size` = 1 (64 Bit) und eines mit `size` 0 am Dateiende → je 200, `ffprobe` gleiche Dauer, Spuren, Zeitbasis,
/// fehlerfrei dekodiert, Lage und Größe von `mdat` gleich, `mdat` byte-gleich, jeder oberste Platzhalter genullt;
/// `sidx`/`mfra` byte-gleich. HEIC nach Art des iPhone und ein AVIF aus ffmpeg ohne `jpeg`-Item → Art und Maße wie
/// vorher, Bildboxen byte-gleich. `free` mit `size` 0 in `moov` → 422.
#[tokio::test]
async fn kontrollen_platzhalter_und_varianten() {
    let lauf = Lauf::neu().await;
    let ordner = ordner();
    let android = film(ordner.path(), "android.mp4", &[]);
    let mut gross = 1u32.to_be_bytes().to_vec();
    gross.extend(b"free");
    gross.extend((16u64 + 4_096).to_be_bytes());
    gross.extend(vec![0x37; 4_096]);
    let bis_ende = [&0u32.to_be_bytes()[..], b"free", &[0x38; 4_096]].concat();
    let dash = film(
        ordner.path(),
        "dash.mp4",
        &["-movflags", "dash+global_sidx", "-write_prft", "wallclock"],
    );
    let emsg = kiste(
        b"emsg",
        &[&[0u8; 4][..], b"urn:dash:event\0\0", &[0; 16], b"Nutzlast"].concat(),
    );
    let filme = [
        (
            "android.mp4",
            [android.clone(), kiste(b"free", &[0x36; 1_024])].concat(),
        ),
        ("iphone.mov", film(ordner.path(), "iphone.mov", &[])),
        (
            "reserve.mp4",
            film(ordner.path(), "reserve.mp4", &["-moov_size", "70000"]),
        ),
        ("dash.mp4", vor_mfra(&dash, &emsg)),
        ("free64.mp4", [android.clone(), gross].concat()),
        ("free0.mp4", [android.clone(), bis_ende].concat()),
    ];
    let dauer = "format=duration:stream=index,codec_type,time_base,duration";
    for (name, original) in filme {
        let vorher = ordner.path().join(format!("vorher-{name}"));
        std::fs::write(&vorher, &original).expect("geschrieben");
        spielt_fehlerfrei(&format!("vorher-{name}"), &original);
        // Der DASH-Film trägt keinen Platzhalter, aber `prft` und `emsg`, die danach Platzhalter sind (B0650).
        assert!(
            name == "dash.mp4" || !oberste_platzhalter(&original).is_empty(),
            "{name}"
        );

        let geliefert = lauf.ausliefern(name, &original).await;

        let bytes = &geliefert.bytes;
        spielt_fehlerfrei(name, bytes);
        let nachher = ordner.path().join(format!("nachher-{name}"));
        std::fs::write(&nachher, bytes).expect("geschrieben");
        assert_eq!(ffprobe(&nachher, dauer), ffprobe(&vorher, dauer), "{name}");
        assert_eq!(lage_mdat(bytes), lage_mdat(&original), "{name}");
        for typ in [b"mdat", b"sidx", b"mfra", b"moof"] {
            assert_eq!(finden(bytes, typ), finden(&original, typ), "{name}");
        }
        for (inhalt, bis) in oberste_platzhalter(bytes) {
            assert!(
                bytes[inhalt..bis].iter().all(|b| *b == 0),
                "{name}: {inhalt}"
            );
        }
    }

    let kachel = vec![0x5a; 64];
    let ipco = [
        kiste(b"colr", b"nclx\0\x01\0\x0d\0\x01\x80"),
        kiste(b"irot", b"\x01"),
        kiste(b"ispe", b"\0\0\0\0\0\0\0\x40\0\0\0\x30"),
    ];
    let iphone = heic_mit(
        &[
            (b"grid", b"", vec![0, 0, 0, 0, 0, 0x40, 0, 0x30]),
            (b"hvc1", b"", kachel.clone()),
            (b"tmap", b"", vec![0x7b; 32]),
            (b"Exif", b"", exif_item_mit_ort()),
        ],
        &ipco,
        &[kiste(b"free", &[0x39; 64])],
    );
    let geliefert = lauf.ausliefern("iphone.heic", &iphone).await;
    ohne_ort("iphone.heic", &geliefert.bytes);
    assert!(enthaelt(&geliefert.bytes, &kachel) && enthaelt(&geliefert.bytes, &ipco.concat()));
    assert_eq!(geliefert.anhang["art"], "image/heic");
    let avif = gemeinsam::dateien::avif_foto();
    let geliefert = lauf.ausliefern("foto.avif", &avif).await;
    assert_eq!(geliefert.bytes, avif, "echtes AVIF byte-gleich");
    assert_eq!(
        art_und_masse(&geliefert.anhang),
        ("image/avif".to_string(), json!(4032), json!(3024))
    );

    let ohne_groesse = [&0u32.to_be_bytes()[..], b"free", &[0x3a; 64]].concat();
    let in_moov = einfuegen(&android, &[b"moov"], &ohne_groesse);
    lauf.abgewiesen("free0-in-moov.mp4", &in_moov).await;
}
