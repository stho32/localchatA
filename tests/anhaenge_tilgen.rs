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
use gemeinsam::senden::{beitreten, ordner, senden};
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

/// MP4 mit `moov` von 64 MiB − 1 (große `trak`, `©xyz` am Ende).
fn grosses_mp4() -> Vec<u8> {
    let boxe = |typ: &[u8], inhalt: &[u8]| {
        [&((8 + inhalt.len()) as u32).to_be_bytes()[..], typ, inhalt].concat()
    };
    let xyz = boxe(b"\xa9xyz", b"\0\x12\x15\xc7+52.5200+013.4050/");
    let udta = boxe(b"udta", &xyz);
    let fuellung = 64 * MIB as usize - 1 - 8 - 8 - udta.len();
    let trak = boxe(b"trak", &vec![7u8; fuellung]);
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
