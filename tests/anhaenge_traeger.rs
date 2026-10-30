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
        let mut iloc = vec![1u8, 0, 0, 0, 0x44, 0x00];
        iloc.extend((items.len() as u16).to_be_bytes());
        let mut ab = mdat_anfang;
        for (n, (_, _, daten)) in items.iter().enumerate() {
            iloc.extend((n as u16 + 1).to_be_bytes());
            iloc.extend([0, 0, 0, 0]);
            iloc.extend(1u16.to_be_bytes());
            iloc.extend((ab as u32).to_be_bytes());
            iloc.extend((daten.len() as u32).to_be_bytes());
            ab += daten.len() as u64;
        }
        let hdlr = kiste(b"hdlr", &[&[0u8; 8][..], b"pict", &[0; 13]].concat());
        let meta = kiste(
            b"meta",
            &[
                &[0u8; 4][..],
                &hdlr,
                &kiste(b"pitm", b"\0\0\0\0\0\x01"),
                &iinf,
                &kiste(b"iloc", &iloc),
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
