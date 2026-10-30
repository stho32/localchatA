//! R00029 F0115 — Ortsangaben beim Hochladen tilgen über das echte Programm (B0504): Handy-JPEG ohne Ort im Eingang,
//! Länge und Maße wie angemeldet; beschädigte Datei → 422 `metadaten`; MP4 ohne `©xyz`; andere Formate unverändert;
//! nie Koordinaten oder Kameradaten im Protokoll (M19); Speicher beim Tilgen (M54, L2).

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{
    MIB, anmelden, gesundheit, hochladen, hochladen_abgewiesen, id, im_eingang, starten,
};
use gemeinsam::dateien::{hochkant_jpg, mp4, pdf};
use gemeinsam::protokoll::alle_zeilen;
use gemeinsam::senden::{beitreten, ordner, roh_senden, senden};
use serde_json::json;
use std::path::Path;

fn enthaelt(heu: &[u8], nadel: &[u8]) -> bool {
    heu.windows(nadel.len()).any(|fenster| fenster == nadel)
}

fn eingangsdatei(ordner: &Path, id: &str) -> Vec<u8> {
    std::fs::read(ordner.join(".eingang").join(id)).expect("im Eingang")
}

/// Hochkantfoto mit GPS, Modell, Zeit und XMP → im Eingang ohne diese Spuren, Länge = Anmeldung, die 200 nennt
/// `image/jpeg` mit 3000 × 4000; `DEBUG anhang Metadaten getilgt` mit `art`, `bloecke`, `ort=ja`; nirgends im
/// Protokoll Koordinaten oder Kameradaten.
#[tokio::test]
async fn ein_handyfoto_kommt_ohne_ort_in_den_eingang() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--protokoll-stufe", "debug"], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let foto = hochkant_jpg();
    let angemeldet = id(&anmelden(&beta, basis, &token, "IMG_0001.jpg", foto.len() as u64).await);

    let antwort = hochladen(&beta, basis, &token, &angemeldet, foto.clone()).await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(
        (
            &antwort.json["anhang"]["art"],
            &antwort.json["anhang"]["breite"],
            &antwort.json["anhang"]["hoehe"]
        ),
        (&json!("image/jpeg"), &json!(3000), &json!(4000))
    );
    let gespeichert = eingangsdatei(wurzel.path(), &angemeldet);
    assert_eq!(gespeichert.len(), foto.len());
    for spur in [
        &b"GPS"[..],
        b"52.5200",
        b"Telefon",
        b"2026:10:29",
        b"xmpmeta",
        b"xap/1.0",
    ] {
        assert!(
            !enthaelt(&gespeichert, spur),
            "{}",
            String::from_utf8_lossy(spur)
        );
    }
    assert!(
        enthaelt(&gespeichert, b"BILDDATEN\xff\x00BILDDATEN"),
        "Bilddaten bleiben"
    );
    drop(instanz);
    let zeilen = alle_zeilen(wurzel.path());
    let getilgt: Vec<&String> = zeilen
        .iter()
        .filter(|z| z.contains("DEBUG anhang Metadaten getilgt"))
        .collect();
    assert_eq!(getilgt.len(), 1, "{zeilen:#?}");
    assert!(
        getilgt[0].contains("art=jpeg")
            && getilgt[0].contains("ort=ja")
            && getilgt[0].contains("bloecke=")
    );
    for zeile in &zeilen {
        for spur in [
            "52.52",
            "52,31",
            "Telefon",
            "2026:10:29",
            "IMG_0001",
            angemeldet.as_str(),
        ] {
            assert!(!zeile.contains(spur), "{spur} in {zeile}");
        }
    }
}

/// Ein beschädigtes JPEG → 422 `metadaten` mit dem Text der Anforderung, ohne ID und Name; keine Teildatei, kein
/// Eintrag (ein weiterer `PUT` → 404), `reserviert` 0; `WARNING anhang Hochladen abgewiesen: Metadaten nicht
/// tilgbar` als Episode je Herkunft.
#[tokio::test]
async fn ein_beschaedigtes_jpeg_wird_mit_422_metadaten_abgewiesen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let mut kaputt = vec![0xFF, 0xD8, 0xFF, 0xE1, 0xFF, 0xF0];
    kaputt.extend_from_slice(b"Exif\0\0GPS 52.5200");
    let angemeldet = id(&anmelden(&beta, basis, &token, "kaputt.jpg", kaputt.len() as u64).await);

    let antwort = hochladen(&beta, basis, &token, &angemeldet, kaputt.clone()).await;

    assert_eq!(antwort.status, 422, "{}", antwort.json);
    assert_eq!(antwort.json["regel"], "metadaten");
    assert_eq!(
        antwort.json["fehler"],
        "Nicht hochgeladen: die Datei ist beschädigt — Ortsangaben ließen sich nicht sicher entfernen."
    );
    assert!(antwort.json["grund"].is_string());
    let text = antwort.json.to_string();
    assert!(
        !text.contains(&angemeldet) && !text.contains("kaputt.jpg"),
        "{text}"
    );
    assert!(im_eingang(wurzel.path()).is_empty());
    assert_eq!(gesundheit(basis).await["reserviert"], 0);
    let erneut =
        hochladen_abgewiesen(&instanz, &token, &angemeldet, kaputt.len() as u64, None).await;
    assert_eq!(erneut.status, 404, "{}", erneut.json);
    drop(instanz);
    let zeilen = alle_zeilen(wurzel.path());
    let warnung: Vec<&String> = zeilen
        .iter()
        .filter(|z| z.contains("WARNING anhang Hochladen abgewiesen: Metadaten nicht tilgbar"))
        .collect();
    assert!(!warnung.is_empty(), "{zeilen:#?}");
    assert!(warnung.iter().all(|z| z.contains("herkunft=127.0.0.1")
        && !z.contains(&angemeldet)
        && !z.contains("kaputt")));
}

/// MP4 mit `©xyz` → genullt, `mdat` gleich; PDF bleibt byte-gleich.
#[tokio::test]
async fn ein_video_verliert_nur_den_ort_und_ein_pdf_bleibt_gleich() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let video = mp4(b"isom", Some(b"+52.5200+013.4050/"));
    let dokument = [pdf(), b"GPS 52.5200".to_vec()].concat();

    let mut gespeichert = Vec::new();
    for (name, inhalt) in [("film.mp4", video.clone()), ("brief.pdf", dokument.clone())] {
        let angemeldet = id(&anmelden(&beta, basis, &token, name, inhalt.len() as u64).await);
        let antwort = hochladen(&beta, basis, &token, &angemeldet, inhalt).await;
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        gespeichert.push(eingangsdatei(wurzel.path(), &angemeldet));
    }

    assert_eq!(gespeichert[0].len(), video.len());
    assert!(
        !enthaelt(&gespeichert[0], b"+52.5200")
            && enthaelt(&gespeichert[0], b"MEDIENDATEN-MEDIENDATEN")
    );
    assert_eq!(gespeichert[1], dokument);
}

fn rss(pid: u32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .expect("status")
        .lines()
        .find_map(|zeile| zeile.strip_prefix("VmRSS:"))
        .and_then(|wert| {
            wert.trim()
                .trim_end_matches(" kB")
                .trim()
                .parse::<u64>()
                .ok()
        })
        .expect("VmRSS")
        * 1024
}

/// MP4 mit `moov` von 64 MiB − 1 (große `trak` aus einer Box — der Tilger steigt in `trak` ab —, `©xyz` am Ende).
fn grosses_mp4() -> Vec<u8> {
    let boxe = |typ: &[u8], inhalt: &[u8]| {
        [&((8 + inhalt.len()) as u32).to_be_bytes()[..], typ, inhalt].concat()
    };
    let xyz = boxe(b"\xa9xyz", b"\0\x12\x15\xc7+52.5200+013.4050/");
    let udta = boxe(b"udta", &xyz);
    let fuellung = 64 * MIB as usize - 1 - 8 - 8 - 8 - udta.len();
    let trak = boxe(b"trak", &boxe(b"edts", &vec![7u8; fuellung]));
    let moov = boxe(b"moov", &[trak, udta].concat());
    assert_eq!(moov.len(), 64 * MIB as usize - 1);
    [
        boxe(b"ftyp", b"isom\0\0\x02\0isommp41"),
        moov,
        boxe(b"mdat", b"MEDIEN"),
    ]
    .concat()
}

/// M54 (L2): 8 gleichzeitige `PUT` einer Herkunft (2 Kennungen × 4) mit `moov` 64 MiB − 1 → RSS-Zuwachs ≤ 8 × 1 MiB
/// + 50 MB, je 200 ohne Ort; Senden von 127.0.0.1 währenddessen 201.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn acht_grosse_videos_gleichzeitig_tilgen_braucht_wenig_speicher() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "1GiB"], &[]);
    let basis = instanz.adresse.clone();
    let pid = instanz.kind.id();
    let video = std::sync::Arc::new(grosses_mp4());
    let mut ids = Vec::new();
    for name in ["eins", "zwei"] {
        let absender = Absender::von("127.0.0.2");
        let token = beitreten(&absender, &basis, name, Some("agent")).await;
        for nummer in 0..4 {
            let angemeldet = id(&anmelden(
                &absender,
                &basis,
                &token,
                &format!("v{nummer}.mp4"),
                video.len() as u64,
            )
            .await);
            ids.push((token.clone(), angemeldet));
        }
    }
    let beta = Absender::von("127.0.0.1");
    let token_beta = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let vorher = rss(pid);
    let messung = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(vorher));
    let messen = {
        let messung = std::sync::Arc::clone(&messung);
        tokio::spawn(async move {
            loop {
                messung.fetch_max(rss(pid), std::sync::atomic::Ordering::Relaxed);
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
    };

    let beginn = std::time::Instant::now();
    let laeufe: Vec<_> = ids
        .iter()
        .map(|(token, angemeldet)| {
            let (token, angemeldet, basis, video) = (
                token.clone(),
                angemeldet.clone(),
                basis.clone(),
                std::sync::Arc::clone(&video),
            );
            tokio::spawn(async move {
                let absender = Absender::von("127.0.0.2");
                hochladen(&absender, &basis, &token, &angemeldet, video.to_vec()).await
            })
        })
        .collect();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let gesendet = senden(&beta, &basis, &token_beta, "allgemein", "währenddessen").await;
    let mut status = Vec::new();
    for lauf in laeufe {
        status.push(lauf.await.expect("Lauf").status);
    }
    messen.abort();

    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    assert_eq!(status, vec![200; 8]);
    let zuwachs = messung
        .load(std::sync::atomic::Ordering::Relaxed)
        .saturating_sub(vorher);
    println!(
        "M54: RSS vorher {vorher} Bytes, Zuwachs höchstens {zuwachs} Bytes, {:?}",
        beginn.elapsed()
    );
    assert!(zuwachs <= 8 * MIB + 50_000_000, "RSS-Zuwachs {zuwachs}");
    for (_, angemeldet) in &ids {
        let datei =
            std::fs::File::open(wurzel.path().join(".eingang").join(angemeldet)).expect("Datei");
        use std::os::unix::fs::FileExt;
        let mut ende = vec![0u8; 200];
        datei
            .read_at(&mut ende, video.len() as u64 - 200)
            .expect("Ende");
        assert!(!enthaelt(&ende, b"+52.5200"), "{angemeldet}");
    }
}

/// Lädt `inhalt` als `name` hoch (200 erwartet) und liefert die Datei im Eingang.
async fn hochgeladen(basis: &str, ordner: &Path, name: &str, inhalt: Vec<u8>) -> Vec<u8> {
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, &name.replace('.', "-"), Some("agent")).await;
    let angemeldet = id(&anmelden(&beta, basis, &token, name, inhalt.len() as u64).await);
    let antwort = hochladen(&beta, basis, &token, &angemeldet, inhalt).await;
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    eingangsdatei(ordner, &angemeldet)
}

/// JPEG mit MP-Index (`APP2` `MPF\0`, zwei Einträge) und einem zweiten Bild mit Exif-GPS dahinter.
fn mpf_jpeg() -> Vec<u8> {
    use gemeinsam::dateien::{jpeg, tiff};
    let mut exif = b"Exif\0\0".to_vec();
    exif.extend(tiff(
        &[(0x0112, 3, 1, 6), (0x8825, 4, 1, 38)],
        b"GPS48.1371N11.5754E",
    ));
    let zweites = jpeg(&[(0xe1, exif)], 160, 120);
    let mpf = |versatz: u32| {
        let mut mpf = b"MPF\0MM\0*\0\0\0\x08\0\x01".to_vec();
        mpf.extend(0xB002u16.to_be_bytes());
        mpf.extend(7u16.to_be_bytes());
        mpf.extend(32u32.to_be_bytes());
        mpf.extend(26u32.to_be_bytes());
        mpf.extend(0u32.to_be_bytes());
        mpf.extend([0; 16]);
        mpf.extend([0, 0, 0, 0]);
        mpf.extend((zweites.len() as u32).to_be_bytes());
        mpf.extend(versatz.to_be_bytes());
        mpf.extend([0, 0, 0, 0]);
        mpf
    };
    let erstes = |versatz| jpeg(&[(0xe2, mpf(versatz))], 4000, 3000);
    // Der TIFF-Kopf des MPF steht hinter SOI, Marke, Länge und `MPF\0`: Versätze zählen ab Byte 10.
    let versatz = (erstes(0).len() - 10) as u32;
    [erstes(versatz), zweites].concat()
}

/// MP4 mit XMP-GPS in der `uuid`-Box (XMP-Typ) oben und in `moov/udta/XMP_`.
fn mp4_mit_xmp() -> Vec<u8> {
    let boxe = |typ: &[u8], inhalt: &[u8]| {
        let mut bytes = ((8 + inhalt.len()) as u32).to_be_bytes().to_vec();
        bytes.extend(typ);
        bytes.extend(inhalt);
        bytes
    };
    let xmp_typ = [
        0xBE, 0x7A, 0xCF, 0xCB, 0x97, 0xA9, 0x42, 0xE8, 0x9C, 0x71, 0x99, 0x94, 0x91, 0xE3, 0xAF,
        0xAC,
    ];
    let uuid = boxe(
        b"uuid",
        &[
            &xmp_typ[..],
            b"<exif:GPSLatitude>52,31.2N</exif:GPSLatitude>",
        ]
        .concat(),
    );
    let udta = boxe(
        b"udta",
        &boxe(b"XMP_", b"<exif:GPSLongitude>13,24.3E</exif:GPSLongitude>"),
    );
    let moov = boxe(b"moov", &[boxe(b"mvhd", &[0; 100]), udta].concat());
    [
        boxe(b"ftyp", b"isom\0\0\x02\0isommp41"),
        uuid,
        moov,
        boxe(b"mdat", b"MEDIENDATEN-MEDIENDATEN"),
    ]
    .concat()
}

/// Befund #51 B2 (b), R00029 „Präparierte Metadaten“ über das Programm (M47, Linie L2): ein JPEG mit MP-Index und
/// Exif-GPS im zweiten Bild und ein MP4 mit XMP-GPS in `uuid` und `udta/XMP_` kommen ohne Ort in den Eingang; Länge
/// gleich, `MPF` und Mediendaten bleiben.
#[tokio::test]
async fn mpf_jpeg_und_mp4_mit_xmp_kommen_ohne_ort_in_den_eingang() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let (foto, video) = (mpf_jpeg(), mp4_mit_xmp());

    let foto_gespeichert =
        hochgeladen(&instanz.adresse, wurzel.path(), "mpf.jpg", foto.clone()).await;
    let video_gespeichert =
        hochgeladen(&instanz.adresse, wurzel.path(), "xmp.mp4", video.clone()).await;

    assert_eq!(foto_gespeichert.len(), foto.len());
    assert!(!enthaelt(&foto_gespeichert, b"GPS") && !enthaelt(&foto_gespeichert, b"48.1371"));
    assert!(enthaelt(&foto_gespeichert, b"MPF\0MM"), "MPF bleibt");
    assert_eq!(video_gespeichert.len(), video.len());
    assert!(!enthaelt(&video_gespeichert, b"GPS") && !enthaelt(&video_gespeichert, b"XMP_"));
    assert!(enthaelt(&video_gespeichert, b"MEDIENDATEN-MEDIENDATEN"));
}

/// Befund #51 B2 (c), R00029 „Über das Programm“ (F0115): drei 422 `metadaten` aus derselben Herkunft → genau eine
/// Episode `WARNING anhang Hochladen abgewiesen: Metadaten nicht tilgbar`.
#[tokio::test]
async fn drei_mal_metadaten_nicht_tilgbar_ist_eine_episode() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let mut kaputt = vec![0xFF, 0xD8, 0xFF, 0xE1, 0xFF, 0xF0];
    kaputt.extend_from_slice(b"Exif\0\0GPS 52.5200");

    for nummer in 0..3 {
        let name = format!("kaputt{nummer}.jpg");
        let angemeldet = id(&anmelden(&beta, basis, &token, &name, kaputt.len() as u64).await);
        let antwort = hochladen(&beta, basis, &token, &angemeldet, kaputt.clone()).await;
        assert_eq!(antwort.status, 422, "{}", antwort.json);
    }
    drop(instanz);

    let zeilen = alle_zeilen(wurzel.path());
    let beginn: Vec<&String> = zeilen
        .iter()
        .filter(|z| {
            z.contains("WARNING anhang Hochladen abgewiesen: Metadaten nicht tilgbar")
                && z.contains("Episode beginnt")
        })
        .collect();
    assert_eq!(beginn.len(), 1, "{zeilen:#?}");
    let einzeln = zeilen
        .iter()
        .filter(|z| z.contains("Metadaten nicht tilgbar") && !z.contains("Episode"))
        .count();
    assert_eq!(einzeln, 0, "{zeilen:#?}");
}

/// MP4 von 256 MiB mit `mdat` zuerst und `moov` am Ende (mit `©xyz`).
fn mp4_256_mib_moov_am_ende() -> Vec<u8> {
    let boxe_kopf = |typ: &[u8], laenge: usize| {
        let mut bytes = ((8 + laenge) as u32).to_be_bytes().to_vec();
        bytes.extend(typ);
        bytes
    };
    let ort = b"+52.5200+013.4050/";
    let mut xyz = boxe_kopf(b"\xa9xyz", 4 + ort.len());
    xyz.extend((ort.len() as u16).to_be_bytes());
    xyz.extend([0x15, 0xc7]);
    xyz.extend(ort);
    let mut udta = boxe_kopf(b"udta", xyz.len());
    udta.extend(xyz);
    let mut moov = boxe_kopf(b"moov", 108 + udta.len());
    moov.extend(boxe_kopf(b"mvhd", 100));
    moov.extend([0; 100]);
    moov.extend(udta);
    let mut ftyp = boxe_kopf(b"ftyp", 16);
    ftyp.extend(b"isom\0\0\x02\0isommp41");
    let mdat_inhalt = 256 * MIB as usize - ftyp.len() - moov.len() - 8;
    let mut bytes = ftyp;
    bytes.extend(boxe_kopf(b"mdat", mdat_inhalt));
    bytes.extend((0..mdat_inhalt).map(|n| (n % 251) as u8));
    bytes.extend(moov);
    bytes
}

/// Befund #51 B2 (a), R00029 „Gleichzeitig“ (M50): Hochladen und Tilgen einer MP4 von 256 MiB mit `moov` am Ende,
/// während `alpha` jede Sekunde sendet → jedes Senden 201 (Gate ohne Zeitmaß); Länge = Anmeldung, Ort genullt; nach
/// dem Senden stimmt `Content-Range` eines Teilabrufs, die Bytes sind die des Originals.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grosses_video_tilgen_waehrend_gesendet_wird() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "1GiB"], &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.1");
    let alpha = Absender::von("127.0.0.2");
    let token = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let token_alpha = beitreten(&alpha, &basis, "alpha", Some("mensch")).await;
    let video = mp4_256_mib_moov_am_ende();
    let laenge = video.len() as u64;
    assert_eq!(laenge, 256 * MIB);
    let ausschnitt = video[200_000_000..200_000_100].to_vec();
    let angemeldet = id(&anmelden(&beta, &basis, &token, "lang.mp4", laenge).await);

    let hochladung = {
        let (basis, token, angemeldet) = (basis.clone(), token.clone(), angemeldet.clone());
        tokio::spawn(async move {
            let beta = Absender::von("127.0.0.1");
            hochladen(&beta, &basis, &token, &angemeldet, video).await
        })
    };
    let mut gesendet = Vec::new();
    while !hochladung.is_finished() {
        gesendet.push(
            senden(&alpha, &basis, &token_alpha, "allgemein", "währenddessen")
                .await
                .status,
        );
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    let antwort = hochladung.await.expect("Hochladung");

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert!(!gesendet.is_empty());
    assert!(gesendet.iter().all(|status| *status == 201), "{gesendet:?}");
    assert_eq!(antwort.json["anhang"]["bytes"], laenge);
    let eingang = wurzel.path().join(".eingang").join(&angemeldet);
    assert_eq!(std::fs::metadata(&eingang).expect("Eingang").len(), laenge);
    let ende = {
        use std::os::unix::fs::FileExt;
        let mut ende = vec![0u8; 300];
        std::fs::File::open(&eingang)
            .expect("Eingang")
            .read_exact_at(&mut ende, laenge - 300)
            .expect("Ende");
        ende
    };
    assert!(!enthaelt(&ende, b"+52.5200"), "Ort genullt");
    let mit = roh_senden(
        &beta,
        &basis,
        Some(&token),
        "allgemein",
        "application/json",
        json!({"text": "", "anhaenge": [angemeldet]})
            .to_string()
            .into_bytes(),
    )
    .await;
    assert_eq!(mit.status, 201, "{}", mit.json);
    let teil = alpha
        .anfrage(
            reqwest::Method::GET,
            format!("{basis}anhaenge/{angemeldet}"),
        )
        .header("Range", "bytes=200000000-200000099")
        .send()
        .await
        .expect("Teilabruf");
    assert_eq!(teil.status(), 206);
    assert_eq!(
        teil.headers()
            .get("content-range")
            .and_then(|wert| wert.to_str().ok()),
        Some(format!("bytes 200000000-200000099/{laenge}").as_str())
    );
    assert_eq!(teil.bytes().await.expect("Bytes").to_vec(), ausschnitt);
}

fn vmhwm(pid: u32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .expect("status")
        .lines()
        .find_map(|zeile| zeile.strip_prefix("VmHWM:"))
        .and_then(|wert| {
            wert.trim()
                .trim_end_matches(" kB")
                .trim()
                .parse::<u64>()
                .ok()
        })
        .expect("VmHWM")
        * 1024
}

/// Ein JPEG, dessen `APP2` `MPF` (≈ 64 KiB) 2.730 gleiche `MP_ENTRY`-Tags trägt, je über denselben Bereich von 2.047
/// Einträgen — 5.585.580 Nennungen desselben zweiten Bildes (Exif-GPS); `flut` 0 → ein einziger Tag mit zwei
/// Einträgen (dasselbe Bild, gleich groß aufgefüllt).
fn mpf_flut(tags: usize) -> Vec<u8> {
    use gemeinsam::dateien::{jpeg, tiff};
    let mut exif = b"Exif\0\0".to_vec();
    exif.extend(tiff(
        &[(0x0112, 3, 1, 6), (0x8825, 4, 1, 38)],
        b"GPS48.1371N11.5754E",
    ));
    let zweites = jpeg(&[(0xe1, exif)], 160, 120);
    const EINTRAEGE: usize = 2_047;
    let mpf = |versatz: u32| {
        let daten_ab = (8 + 2 + tags.max(1) * 12 + 4) as u32;
        let mut mpf = b"MPF\0MM\0*\0\0\0\x08".to_vec();
        mpf.extend((tags.max(1) as u16).to_be_bytes());
        let zahl = if tags == 0 { 32 } else { EINTRAEGE * 16 };
        for _ in 0..tags.max(1) {
            mpf.extend(0xB002u16.to_be_bytes());
            mpf.extend(7u16.to_be_bytes());
            mpf.extend((zahl as u32).to_be_bytes());
            mpf.extend(daten_ab.to_be_bytes());
        }
        mpf.extend(0u32.to_be_bytes());
        mpf.extend([0; 16]);
        for _ in 1..zahl / 16 {
            mpf.extend([0, 0, 0, 0]);
            mpf.extend((zweites.len() as u32).to_be_bytes());
            mpf.extend(versatz.to_be_bytes());
            mpf.extend([0, 0, 0, 0]);
        }
        mpf.resize(65_530, 0);
        mpf
    };
    let erstes = |versatz| jpeg(&[(0xe2, mpf(versatz))], 4000, 3000);
    let versatz = (erstes(0).len() - 10) as u32;
    [erstes(versatz), zweites].concat()
}

/// Befund aus Teil A der Nacharbeit zu #9 (`tilger/mpf.rs`, `weitere_bilder`): viele gleiche `MP_ENTRY`-Tags ließen
/// die Liste der weiteren Bilder auf Millionen Einträge wachsen. Probe: das präparierte JPEG → Speicherzuwachs
/// (VmHWM) ≤ 1 MiB je Hochladung, Ergebnis 422 `metadaten` (seit ADR-0047 B1 Regel 2 „Verweise doppelt“, vorher
/// „über 100.000 Blöcke“); Kontrolle: dasselbe Bild mit einem Tag → 200 ohne GPS.
#[tokio::test]
async fn viele_gleiche_mp_eintraege_brauchen_keinen_speicher() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let pid = instanz.kind.id();
    for name in ["einer1.jpg", "einer2.jpg", "einer3.jpg"] {
        let getilgt = hochgeladen(basis, wurzel.path(), name, mpf_flut(0)).await;
        assert!(!enthaelt(&getilgt, b"GPS48"), "Kontrolle");
    }
    let flut = mpf_flut(2_730);
    assert!(flut.len() < 70_000, "{}", flut.len());
    let vorher = vmhwm(pid);

    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "flut", Some("agent")).await;
    let angemeldet = id(&anmelden(&beta, basis, &token, "flut.jpg", flut.len() as u64).await);
    let antwort = hochladen(&beta, basis, &token, &angemeldet, flut).await;

    let zuwachs = vmhwm(pid).saturating_sub(vorher);
    assert_eq!(
        (
            antwort.status,
            &antwort.json["regel"],
            &antwort.json["grund"]
        ),
        (
            422,
            &serde_json::json!("metadaten"),
            &serde_json::json!("Verweise doppelt")
        ),
        "{}",
        antwort.json
    );
    assert!(zuwachs <= 1_048_576, "VmHWM +{zuwachs} Bytes");
}

/// Erstes JPEG mit `APP2` `MPF` aus `tags` gleichen `MP_ENTRY`-Feldern, dahinter `bilder`; der Index nennt die Bilder
/// in der Reihenfolge `nennungen` (Versätze ab dem TIFF-Kopf bei Byte 10).
fn mpf_index(tags: usize, bilder: &[Vec<u8>], nennungen: &[usize]) -> Vec<u8> {
    use gemeinsam::dateien::jpeg;
    let mpf = |weitere: &[(u32, u32)]| {
        let mut mpf = b"MPF\0MM\0*\0\0\0\x08".to_vec();
        mpf.extend((tags as u16).to_be_bytes());
        for _ in 0..tags {
            mpf.extend(0xB002u16.to_be_bytes());
            mpf.extend(7u16.to_be_bytes());
            mpf.extend((((weitere.len() + 1) * 16) as u32).to_be_bytes());
            mpf.extend(((8 + 2 + tags * 12 + 4) as u32).to_be_bytes());
        }
        mpf.extend(0u32.to_be_bytes());
        mpf.extend([0; 16]);
        for (groesse, versatz) in weitere {
            mpf.extend([0; 4]);
            mpf.extend(groesse.to_be_bytes());
            mpf.extend(versatz.to_be_bytes());
            mpf.extend([0; 4]);
        }
        mpf
    };
    let laenge = jpeg(&[(0xe2, mpf(&vec![(0, 0); nennungen.len()]))], 4000, 3000).len();
    let mut anfaenge = Vec::new();
    let mut stelle = laenge;
    for bild in bilder {
        anfaenge.push(stelle);
        stelle += bild.len();
    }
    let weitere: Vec<(u32, u32)> = nennungen
        .iter()
        .map(|&nummer| (bilder[nummer].len() as u32, (anfaenge[nummer] - 10) as u32))
        .collect();
    [
        vec![jpeg(&[(0xe2, mpf(&weitere))], 4000, 3000)],
        bilder.to_vec(),
    ]
    .concat()
    .concat()
}

/// Ein JPEG von 64 Bytes mit Ortstext im `COM`.
fn winziges_jpeg() -> Vec<u8> {
    let mut kommentar = b"GPS 48.1371 11.5754".to_vec();
    kommentar.resize(50, b'.');
    let mut bild = vec![0xFF, 0xD8, 0xFF, 0xFE, 0, 52];
    bild.extend(kommentar);
    bild.extend([0xFF, 0xDA, 0, 3, 1, 0xAB, 0xFF, 0xD9]);
    assert_eq!(bild.len(), 64);
    bild
}

/// Lädt `inhalt` hoch und misst den `PUT`.
async fn hochladen_gemessen(
    basis: &str,
    name: &str,
    inhalt: Vec<u8>,
) -> (gemeinsam::ApiAntwort, std::time::Duration) {
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, &name.replace('.', "-"), Some("agent")).await;
    let angemeldet = id(&anmelden(&beta, basis, &token, name, inhalt.len() as u64).await);
    let beginn = std::time::Instant::now();
    let antwort = hochladen(&beta, basis, &token, &angemeldet, inhalt).await;
    (antwort, beginn.elapsed())
}

/// ADR-0047 B1 P3 über das Programm (B0639): MP-Index mit 4.000 Einträgen auf dasselbe Bild → 422 binnen 2 s;
/// 4.000 Einträge auf verschiedene Bilder je 64 Bytes → getilgt binnen 2 s; zwei `MP_ENTRY`-Felder → 422.
#[tokio::test]
async fn mehrfachverweise_im_mp_index_binnen_2_s() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();

    let (gleich, dauer) = hochladen_gemessen(
        basis,
        "gleich.jpg",
        mpf_index(1, &[winziges_jpeg()], &[0; 4_000]),
    )
    .await;
    assert_eq!(
        (
            gleich.status,
            gleich.json["regel"].as_str(),
            gleich.json["grund"].as_str()
        ),
        (422, Some("metadaten"), Some("Verweise überschneiden sich")),
        "{}",
        gleich.json
    );
    assert!(dauer.as_secs_f64() <= 2.0, "{dauer:?}");

    let nennungen: Vec<usize> = (0..4_000).collect();
    let (verschieden, dauer) = hochladen_gemessen(
        basis,
        "verschieden.jpg",
        mpf_index(1, &vec![winziges_jpeg(); 4_000], &nennungen),
    )
    .await;
    assert_eq!(verschieden.status, 200, "{}", verschieden.json);
    assert!(dauer.as_secs_f64() <= 2.0, "{dauer:?}");
    let getilgt = eingangsdatei(
        wurzel.path(),
        verschieden.json["anhang"]["id"].as_str().expect("id"),
    );
    assert!(!enthaelt(&getilgt, b"GPS"));

    let (doppelt, _) =
        hochladen_gemessen(basis, "doppelt.jpg", mpf_index(2, &[winziges_jpeg()], &[0])).await;
    assert_eq!(
        (doppelt.status, doppelt.json["grund"].as_str()),
        (422, Some("Verweise doppelt")),
        "{}",
        doppelt.json
    );
}

fn kiste(typ: &[u8], inhalt: &[u8]) -> Vec<u8> {
    let mut bytes = ((8 + inhalt.len()) as u32).to_be_bytes().to_vec();
    bytes.extend(typ);
    bytes.extend(inhalt);
    bytes
}

fn infe(id: u16, typ: &[u8; 4]) -> Vec<u8> {
    kiste(
        b"infe",
        &[&[2u8, 0, 0, 0][..], &id.to_be_bytes(), &[0, 0], typ, b"\0"].concat(),
    )
}

/// HEIC aus `infe`, einem `iloc` (Version 1, Breiten 4/4) mit (ID, Versatz im `mdat`, Länge) und `mdat`-Inhalt.
fn heic(infe: &[Vec<u8>], verweise: &[(u16, u64, u64)], mdat: &[u8]) -> Vec<u8> {
    let iinf = kiste(
        b"iinf",
        &[&[0u8, 0, 0, 0, 0, 0][..], &infe.concat()].concat(),
    );
    let bauen = |mdat_anfang: u64| {
        let mut iloc = vec![1u8, 0, 0, 0, 0x44, 0x00];
        iloc.extend((verweise.len() as u16).to_be_bytes());
        for (id, ab, lang) in verweise {
            iloc.extend(id.to_be_bytes());
            iloc.extend([0, 0, 0, 0]);
            iloc.extend(1u16.to_be_bytes());
            iloc.extend(((mdat_anfang + ab) as u32).to_be_bytes());
            iloc.extend((*lang as u32).to_be_bytes());
        }
        let meta = kiste(
            b"meta",
            &[
                &[0u8, 0, 0, 0][..],
                &kiste(b"hdlr", &[&[0u8; 8][..], b"pict", &[0; 13]].concat()),
                &iinf,
                &kiste(b"iloc", &iloc),
            ]
            .concat(),
        );
        [kiste(b"ftyp", b"heic\0\0\0\0mif1heic"), meta].concat()
    };
    let mdat_anfang = bauen(0).len() as u64 + 8;
    [bauen(mdat_anfang), kiste(b"mdat", mdat)].concat()
}

/// Exif-Item mit Ort, das `mdat` dahinter auf `bis` Bytes aufgefüllt.
fn exif_item_mit_ort(bis: usize) -> (Vec<u8>, u64) {
    use gemeinsam::dateien::tiff;
    let mut exif = 0u32.to_be_bytes().to_vec();
    exif.extend(tiff(
        &[(0x0112, 3, 1, 6), (0x8825, 4, 1, 38)],
        b"GPS52.5200N13.4050E",
    ));
    let lang = exif.len() as u64;
    exif.resize(bis, 0x77);
    (exif, lang)
}

/// ADR-0047 B1 P1 und P2 über das Programm (B0638): HEIC 32 MiB, ein Exif-Item, dessen `infe` 2.000-mal mit derselben
/// `item_ID` steht → 422 `metadaten` binnen 2 s, `reserviert` 0, keine Datei im Eingang; 2.000 Exif-Items auf
/// denselben Bereich → 422 binnen 2 s; ein Exif-Item über eine Bildkachel um ein Byte → 422.
#[tokio::test]
async fn mehrfachverweise_in_heif_binnen_2_s() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "1GiB"], &[]);
    let basis = instanz.adresse.as_str();
    let (mdat, lang) = exif_item_mit_ort(32 * MIB as usize);
    let doppelt = heic(&vec![infe(1, b"Exif"); 2_000], &[(1, 0, lang)], &mdat);
    let (mdat_klein, _) = exif_item_mit_ort(4_096);
    let alle: Vec<Vec<u8>> = (1..=2_000).map(|id| infe(id, b"Exif")).collect();
    let verweise: Vec<(u16, u64, u64)> = (1..=2_000).map(|id| (id, 0, lang)).collect();
    let gleich = heic(&alle, &verweise, &mdat_klein);
    let kachel = heic(
        &[infe(1, b"hvc1"), infe(2, b"Exif")],
        &[(1, 0, 50), (2, 49, lang)],
        &[vec![0x5A; 50], mdat_klein.clone()].concat(),
    );

    for (name, inhalt, grund) in [
        ("doppelt.heic", doppelt, "Verweise doppelt"),
        ("gleich.heic", gleich, "Verweise überschneiden sich"),
        ("kachel.heic", kachel, "Verweise überschneiden sich"),
    ] {
        let (antwort, dauer) = hochladen_gemessen(basis, name, inhalt).await;
        assert_eq!(
            (
                antwort.status,
                antwort.json["regel"].as_str(),
                antwort.json["grund"].as_str()
            ),
            (422, Some("metadaten"), Some(grund)),
            "{name}: {}",
            antwort.json
        );
        assert!(dauer.as_secs_f64() <= 2.0, "{name}: {dauer:?}");
        assert_eq!(gesundheit(basis).await["reserviert"], 0, "{name}");
        assert!(im_eingang(wurzel.path()).is_empty(), "{name}");
    }
}

/// Die `arbeit` der DEBUG-Zeilen „Metadaten getilgt“ in der Reihenfolge der Hochladungen.
fn arbeit_je_hochladung(ordner: &Path) -> Vec<u64> {
    alle_zeilen(ordner)
        .iter()
        .filter(|zeile| zeile.contains("DEBUG anhang Metadaten getilgt"))
        .map(|zeile| {
            let wert = zeile.split("arbeit=").nth(1).expect("arbeit=");
            wert.split(|c: char| !c.is_ascii_digit())
                .next()
                .expect("Zahl")
                .parse()
                .expect("Zahl")
        })
        .collect()
}

/// JPEG mit `APP1` Exif samt Ort, den Segmenten `davor` und einem `APP2` `MPF`, dessen Index auf die `weiteren`
/// Bilder dahinter zeigt (Größe, Anfang); `weitere` hängen an.
fn jpeg_mit_mpf(davor: &[(u8, Vec<u8>)], weitere: &[Vec<u8>]) -> Vec<u8> {
    use gemeinsam::dateien::{jpeg, tiff};
    let mut exif = b"Exif\0\0".to_vec();
    exif.extend(tiff(
        &[(0x0112, 3, 1, 1), (0x8825, 4, 1, 38)],
        b"GPS52.5200N13.4050E",
    ));
    let index = |anfaenge: &[u32]| {
        let mut mpf = b"MPF\0MM\0*\0\0\0\x08\0\x01".to_vec();
        mpf.extend(0xB002u16.to_be_bytes());
        mpf.extend(7u16.to_be_bytes());
        mpf.extend((((weitere.len() + 1) * 16) as u32).to_be_bytes());
        mpf.extend(26u32.to_be_bytes());
        mpf.extend(0u32.to_be_bytes());
        mpf.extend([0; 16]);
        for (bild, anfang) in weitere.iter().zip(anfaenge) {
            mpf.extend([0; 4]);
            mpf.extend((bild.len() as u32).to_be_bytes());
            mpf.extend(anfang.to_be_bytes());
            mpf.extend([0; 4]);
        }
        mpf
    };
    let segmente = |mpf: Vec<u8>| {
        let mut alle = vec![(0xe1, exif.clone())];
        alle.extend(davor.iter().cloned());
        alle.push((0xe2, mpf));
        jpeg(&alle, 4000, 3000)
    };
    let probe = segmente(index(&vec![0; weitere.len()]));
    let tiff = probe.windows(4).position(|f| f == b"MPF\0").expect("MPF") + 4;
    let mut stelle = probe.len();
    let anfaenge: Vec<u32> = weitere
        .iter()
        .map(|bild| {
            let anfang = (stelle - tiff) as u32;
            stelle += bild.len();
            anfang
        })
        .collect();
    [vec![segmente(index(&anfaenge))], weitere.to_vec()]
        .concat()
        .concat()
}

/// HEIC aus Items (Typ, `infe`-Rest, Daten), Eigenschaften in `ipco` und `iref`.
fn heic_voll(items: &[(&[u8; 4], &[u8], Vec<u8>)], ipco: &[Vec<u8>], iref: &[u8]) -> Vec<u8> {
    let iinf = kiste(
        b"iinf",
        &[
            &[0u8, 0, 0, 0][..],
            &(items.len() as u16).to_be_bytes(),
            &items
                .iter()
                .enumerate()
                .map(|(n, (typ, rest, _))| {
                    kiste(
                        b"infe",
                        &[
                            &[2u8, 0, 0, 0][..],
                            &(n as u16 + 1).to_be_bytes(),
                            &[0, 0],
                            &typ[..],
                            b"\0",
                            rest,
                        ]
                        .concat(),
                    )
                })
                .collect::<Vec<_>>()
                .concat(),
        ]
        .concat(),
    );
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
        let meta = kiste(
            b"meta",
            &[
                &[0u8; 4][..],
                &kiste(b"hdlr", &[&[0u8; 8][..], b"pict", &[0; 13]].concat()),
                &kiste(b"pitm", b"\0\0\0\0\0\x01"),
                &iinf,
                &kiste(b"iref", &[&[0u8; 4][..], iref].concat()),
                &kiste(b"iloc", &iloc),
                &kiste(
                    b"iprp",
                    &[kiste(b"ipco", &ipco.concat()), kiste(b"ipma", &[0; 8])].concat(),
                ),
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

/// Die Kontrollen aus ADR-0047 B1 als Nachbauten mit dem Aufbau der Telefondateien (echte Dateien im Entwicklertest).
fn kontrollen(ordner: &Path) -> Vec<(&'static str, Vec<u8>)> {
    use gemeinsam::dateien::jpeg;
    let (mut exif, laenge) = exif_item_mit_ort(4_096);
    exif.truncate(laenge as usize);
    let iso = (0xe2, b"urn:iso:std:iso:ts:21496:-1\0\0\0\0\0".to_vec());
    let xmp_hdr = (
        0xe1,
        b"http://ns.adobe.com/xap/1.0/\0<hdrgm:Version>1.0</hdrgm:Version>".to_vec(),
    );
    let gainmap = jpeg(&[xmp_hdr.clone(), iso.clone()], 1000, 750);
    let ultra_hdr = jpeg_mit_mpf(&[xmp_hdr, iso], &[gainmap]);
    let mut exif_rechts = b"Exif\0\0".to_vec();
    exif_rechts.extend(gemeinsam::dateien::tiff(
        &[(0x8825, 4, 1, 26)],
        b"GPS52.5200N13.4050E",
    ));
    let mpo = jpeg_mit_mpf(&[], &[jpeg(&[(0xe1, exif_rechts)], 4000, 3000)]);
    let kachel = |n: u8| vec![n; 4_096];
    let iphone = heic_voll(
        &[
            (b"grid", b"", vec![0, 0, 0x01, 0x01, 0x0f, 0xc0, 0x0b, 0xd0]),
            (b"hvc1", b"", kachel(1)),
            (b"hvc1", b"", kachel(2)),
            (b"hvc1", b"", kachel(3)),
            (b"hvc1", b"", kachel(4)),
            (b"Exif", b"", exif.clone()),
            (
                b"mime",
                b"application/rdf+xml\0",
                b"<x:xmpmeta>GPS 52.5200</x:xmpmeta>".to_vec(),
            ),
            (b"hvc1", b"", kachel(8)),
            (b"tmap", b"", vec![0x7b; 64]),
        ],
        &[
            kiste(b"hvcC", &[0x11; 32]),
            kiste(b"ispe", b"\0\0\0\0\0\0\x0f\xc0\0\0\x0b\xd0"),
            kiste(b"colr", b"prof\0\0\x02\x24Profil"),
            kiste(b"irot", b"\x03"),
            kiste(
                b"auxC",
                b"\0\0\0\0urn:com:apple:photo:2020:aux:hdrgainmap\0",
            ),
        ],
        &[
            kiste(b"dimg", b"\0\x01\0\x04\0\x02\0\x03\0\x04\0\x05"),
            kiste(b"cdsc", b"\0\x06\0\x01\0\x01"),
            kiste(b"auxl", b"\0\x08\0\x01\0\x01"),
        ]
        .concat(),
    );
    let sammlung = heic_voll(
        &[
            (b"hvc1", b"", kachel(1)),
            (b"hvc1", b"", kachel(2)),
            (b"Exif", b"", exif.clone()),
            (b"Exif", b"", exif),
        ],
        &[kiste(b"ispe", b"\0\0\0\0\0\0\x10\0\0\0\x10\0")],
        &[
            kiste(b"cdsc", b"\0\x03\0\x01\0\x01"),
            kiste(b"cdsc", b"\0\x04\0\x01\0\x02"),
        ]
        .concat(),
    );
    let pfad = ordner.join("telefon.mp4");
    let erzeugt = std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=d=2:s=320x240:r=30",
        ])
        .args([
            "-metadata",
            "location=+52.5200+013.4050/",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&pfad)
        .status()
        .expect("Vorbedingung fehlt: ffmpeg");
    assert!(erzeugt.success());
    vec![
        ("ultra-hdr.jpg", ultra_hdr),
        ("stereo.mpo", mpo),
        ("iphone.heic", iphone),
        ("sammlung.heic", sammlung),
        ("telefon.mp4", std::fs::read(&pfad).expect("Film")),
    ]
}

/// ADR-0047 B1 Kontrollen als Nachbauten (B0640): Ultra-HDR-JPEG (MPF mit Gain-Map, ISO 21496-1), MPO-Stereobild,
/// HEIC nach Art des iPhone (`grid` aus Kacheln, Exif, XMP, Hilfsbild, `tmap`), HEIF-Sammlung mit zwei Bildern und je
/// einem Exif-Item, MP4 eines Telefons → je 200, Ort getilgt, gezählte Bytes ≤ 1,5 × Länge + 4 MiB.
#[tokio::test]
async fn kontrollen_bleiben_tilgbar_und_im_budget() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--protokoll-stufe", "debug"], &[]);
    let basis = instanz.adresse.as_str();
    let werkstatt = ordner();
    let faelle = kontrollen(werkstatt.path());

    let mut laengen = Vec::new();
    for (name, inhalt) in &faelle {
        let (antwort, _) = hochladen_gemessen(basis, name, inhalt.clone()).await;
        assert_eq!(antwort.status, 200, "{name}: {}", antwort.json);
        let getilgt = eingangsdatei(
            wurzel.path(),
            antwort.json["anhang"]["id"].as_str().expect("id"),
        );
        for spur in [&b"GPS"[..], b"52.5200"] {
            assert!(
                !enthaelt(&getilgt, spur),
                "{name}: {}",
                String::from_utf8_lossy(spur)
            );
        }
        laengen.push(inhalt.len() as u64);
    }
    drop(instanz);

    let arbeit = arbeit_je_hochladung(wurzel.path());
    assert_eq!(arbeit.len(), faelle.len(), "{arbeit:?}");
    for ((name, _), (arbeit, laenge)) in faelle.iter().zip(arbeit.iter().zip(&laengen)) {
        assert!(
            *arbeit <= laenge * 3 / 2 + 4 * MIB,
            "{name}: {arbeit} Bytes bei {laenge}"
        );
    }
}

/// K1 (R00029 „Matrix“): `alpha@127.0.0.1` meldet ein PNG von 300 KiB an → 201, `PUT` → 200 `image/png`, Senden → 201,
/// `GET /anhaenge/<id>` → 200 mit `sandbox`-CSP, `Range: bytes=0-99` → 206.
async fn k1(basis: &str, name: &str) {
    let alpha = Absender::von("127.0.0.1");
    let token = beitreten(&alpha, basis, name, Some("agent")).await;
    let mut bild = gemeinsam::dateien::png(64, 64);
    bild.resize(307_200, 0);
    let angemeldet = id(&anmelden(&alpha, basis, &token, "k1.png", bild.len() as u64).await);
    let antwort = hochladen(&alpha, basis, &token, &angemeldet, bild).await;
    assert_eq!(
        (antwort.status, antwort.json["anhang"]["art"].as_str()),
        (200, Some("image/png")),
        "K1"
    );
    let koerper = json!({"text": "", "anhaenge": [angemeldet]})
        .to_string()
        .into_bytes();
    let gesendet = roh_senden(
        &alpha,
        basis,
        Some(&token),
        "allgemein",
        "application/json",
        koerper,
    )
    .await;
    assert_eq!(gesendet.status, 201, "K1: {}", gesendet.json);
    let url = format!("{basis}anhaenge/{angemeldet}");
    let ganz = alpha
        .anfrage(reqwest::Method::GET, url.clone())
        .send()
        .await
        .expect("GET");
    assert_eq!(ganz.status(), 200, "K1");
    assert!(
        ganz.headers()["content-security-policy"]
            .to_str()
            .expect("CSP")
            .starts_with("sandbox")
    );
    let teil = alpha
        .anfrage(reqwest::Method::GET, url)
        .header("Range", "bytes=0-99")
        .send()
        .await
        .expect("Range");
    assert_eq!(teil.status(), 206, "K1");
}

/// K2 nach den Grenzen aus Teil B (ADR-0046 Ergänzung „Karte #9 Nacharbeit F1“): eine Kennung von 127.0.0.3 mit 10
/// offenen Anmeldungen, die 11. → 429 `offen_kennung`.
async fn k2(basis: &str) {
    let gamma = Absender::von("127.0.0.3");
    let token = beitreten(&gamma, basis, "gamma", Some("agent")).await;
    for _ in 0..10 {
        id(&anmelden(&gamma, basis, &token, "x", 1).await);
    }
    let elfte = anmelden(&gamma, basis, &token, "x", 1).await;
    assert_eq!(
        (elfte.status, elfte.json["grenze"].as_str()),
        (429, Some("offen_kennung")),
        "K2"
    );
}

/// MP4 von 1 GiB: `moov` mit `mvhd`, `trak` (`tkhd`, `udta/©xyz`), dahinter `mdat` aus Nullen.
fn mp4_1_gib() -> Vec<u8> {
    let xyz = kiste(b"\xa9xyz", b"\0\x12\x15\xc7+52.5200+013.4050/");
    let mut tkhd = vec![0u8; 84];
    tkhd[4..8].copy_from_slice(&3_876_122_096u32.to_be_bytes());
    let trak = kiste(
        b"trak",
        &[kiste(b"tkhd", &tkhd), kiste(b"udta", &xyz)].concat(),
    );
    let kopf = [
        kiste(b"ftyp", b"isom\0\0\x02\0isommp41"),
        kiste(b"moov", &[kiste(b"mvhd", &[0; 100]), trak].concat()),
    ]
    .concat();
    let mdat = 1_073_741_824 - kopf.len() - 8;
    let mut video = kopf;
    video.extend(((mdat + 8) as u32).to_be_bytes());
    video.extend(b"mdat");
    video.resize(1_073_741_824, 0);
    video
}

/// ADR-0047 B1 P5 und die Kontrolle MP4 1 GiB im Release-Build (B0640, Linie L2): 8 Hochladungen nach P1 (HEIC
/// 32 MiB, 2.000 gleiche `infe`) gleichzeitig von 127.0.0.2 (2 Kennungen × 4), währenddessen ein Upload von 4 MiB von
/// 127.0.0.1 → dessen `PUT` 200; alle 8 → 422 binnen 2 s. MP4 1 GiB → 200, Ort getilgt, gezählte Bytes
/// ≤ 1,5 × Länge + 4 MiB. Kontrollpaar K1, K2 grün.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Systemprobe im Release-Build: cargo test --release --test anhaenge_tilgen -- --ignored systemprobe"]
async fn systemprobe_arbeit_des_tilgers() {
    let wurzel = ordner();
    let instanz = starten(
        wurzel.path(),
        &["--anhangquote", "2GiB", "--protokoll-stufe", "debug"],
        &[],
    );
    let basis = instanz.adresse.clone();
    k1(&basis, "alpha").await;
    k2(&basis).await;

    let (mdat, lang) = exif_item_mit_ort(32 * MIB as usize);
    let p1 = std::sync::Arc::new(heic(&vec![infe(1, b"Exif"); 2_000], &[(1, 0, lang)], &mdat));
    let mut laeufe = Vec::new();
    for name in ["eins", "zwei"] {
        let absender = Absender::von("127.0.0.2");
        let token = beitreten(&absender, &basis, name, Some("agent")).await;
        for nummer in 0..4 {
            let angemeldet = id(&anmelden(
                &absender,
                &basis,
                &token,
                &format!("p1-{nummer}.heic"),
                p1.len() as u64,
            )
            .await);
            laeufe.push((token.clone(), angemeldet));
        }
    }
    let beta = Absender::von("127.0.0.1");
    let token_beta = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let opfer = id(&anmelden(&beta, &basis, &token_beta, "opfer.bin", 4 * MIB).await);

    let angriffe: Vec<_> = laeufe
        .into_iter()
        .map(|(token, angemeldet)| {
            let (basis, p1) = (basis.clone(), std::sync::Arc::clone(&p1));
            tokio::spawn(async move {
                let absender = Absender::von("127.0.0.2");
                let beginn = std::time::Instant::now();
                let antwort = hochladen(&absender, &basis, &token, &angemeldet, p1.to_vec()).await;
                (
                    antwort.status,
                    antwort.json["grund"].clone(),
                    beginn.elapsed(),
                )
            })
        })
        .collect();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let beginn = std::time::Instant::now();
    let antwort = hochladen(
        &beta,
        &basis,
        &token_beta,
        &opfer,
        vec![0x42; 4 * MIB as usize],
    )
    .await;
    println!("Opfer: {} nach {:?}", antwort.status, beginn.elapsed());
    assert_eq!(antwort.status, 200, "Opfer: {}", antwort.json);
    for angriff in angriffe {
        let (status, grund, dauer) = angriff.await.expect("Lauf");
        println!("P1: {status} {grund} nach {dauer:?}");
        assert_eq!((status, grund), (422, json!("Verweise doppelt")));
        assert!(dauer.as_secs_f64() <= 2.0, "{dauer:?}");
    }

    let video = mp4_1_gib();
    let laenge = video.len() as u64;
    let angemeldet = id(&anmelden(&beta, &basis, &token_beta, "gross.mp4", laenge).await);
    let antwort = hochladen(&beta, &basis, &token_beta, &angemeldet, video).await;
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    let mut kopf = vec![0u8; 4_096];
    use std::os::unix::fs::FileExt;
    std::fs::File::open(wurzel.path().join(".eingang").join(&angemeldet))
        .expect("im Eingang")
        .read_at(&mut kopf, 0)
        .expect("Kopf");
    assert!(!enthaelt(&kopf, b"+52.5200") && !enthaelt(&kopf, &3_876_122_096u32.to_be_bytes()));
    k1(&basis, "alpha2").await;
    drop(instanz);
    let arbeit = *arbeit_je_hochladung(wurzel.path())
        .last()
        .expect("DEBUG-Zeile");
    println!("MP4 1 GiB: arbeit {arbeit} Bytes");
    assert!(arbeit <= laenge * 3 / 2 + 4 * MIB, "{arbeit}");
}
