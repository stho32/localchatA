//! R00050 F0177 B0783 — jedes Feld des Health-Checks hat ein Urteil (ADR-0055 Regel 9, B2; Matrix H3; Linie L2, echtes
//! Programm): die Schlüssel der Antwort werden gegen das Inventar der ADR verglichen — ein Feld ohne Zeile macht den
//! Test rot —, und ein Direktgespräch mit Anhang bewegt nur die als Restorakel (f) benannten Felder.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, starten as starten_anhaenge};
use gemeinsam::direkt::{BEN, Runde, runde_mit};
use gemeinsam::gesundheit;
use gemeinsam::senden::ordner;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Das Urteil der ADR je Feld (Tabelle in Regel 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Urteil {
    /// Konfiguration, feste Werte, Zustand ohne Gesprächsbezug.
    Unveraendert,
    /// `nachrichten`, `log_bytes`, `defekte_zeilen`, `nachholen.saetze`/`kosten`: nur Channels (neu).
    NurChannels,
    /// `direkt_mib`: neu, abgerundet auf ganze MiB (Restorakel (d)).
    Neu,
    /// Restorakel (b): die globale Ordnung der Zeiten.
    ZeitOrdnung,
    /// Restorakel (a), (g), (c), (i): „dass“, nie Gespräch, Beteiligte oder Zahl der Nachrichten.
    Restorakel,
    /// Restorakel (f): die Anhangzähler — Zählmaß und Zeitpunkt von Anmeldung, Löschen und Entfallen.
    Anhangzaehler,
    /// Startwert, nach dem Start konstant.
    Startwert,
}

use Urteil::{Anhangzaehler, Neu, NurChannels, Restorakel, Startwert, Unveraendert, ZeitOrdnung};

/// Das Inventar aus ADR-0055 Regel 9, ein Eintrag je Blatt der Antwort (Objekte als `a.b`).
const INVENTAR: &[(&str, Urteil)] = &[
    ("status", Unveraendert),
    ("version", Unveraendert),
    ("ordner", Unveraendert),
    ("alle_namen", Unveraendert),
    ("herzschlag_ms", Unveraendert),
    ("abrissgrenze_ms", Unveraendert),
    ("protokoll_stufe", Unveraendert),
    ("plattenquote", Unveraendert),
    ("anhangquote", Unveraendert),
    ("werkbank_plaetze", Unveraendert),
    ("nachholen.bank_plaetze", Unveraendert),
    ("nachrichten", NurChannels),
    ("log_bytes", NurChannels),
    ("direkt_mib", Neu),
    ("ladezeit_ms", Startwert),
    ("defekte_zeilen", NurChannels),
    ("channels", Unveraendert),
    ("sitzungen", Unveraendert),
    ("halter", Unveraendert),
    ("wartend", Unveraendert),
    ("zeit_vorlauf_ms", ZeitOrdnung),
    ("gebremst", Restorakel),
    ("schluessel_verdraengt", Restorakel),
    ("quote_abgewiesen", Restorakel),
    ("abgewiesen.zu_gross", Restorakel),
    ("abgewiesen.kein_json", Restorakel),
    ("abgewiesen.kaputt", Restorakel),
    ("abgewiesen.abgebrochen", Restorakel),
    ("abgewiesen.zu_langsam", Restorakel),
    ("abgerissen", Restorakel),
    ("nachholen.gebremst", Restorakel),
    ("nachholen.saetze", NurChannels),
    ("nachholen.kosten", NurChannels),
    ("werkbank_belegt", Restorakel),
    ("nachholen.bank_belegt", Restorakel),
    ("protokoll_ok", Unveraendert),
    ("protokoll_verworfen", Unveraendert),
    ("aufgaben_ausgefallen", Unveraendert),
    ("protokoll_bytes_heute", Restorakel),
    ("warnings", Restorakel),
    ("errors", Restorakel),
    ("exceptions", Restorakel),
    ("letzter_error", Restorakel),
    ("letzte_exception", Restorakel),
    ("anhang_belegt", Anhangzaehler),
    ("reserviert", Anhangzaehler),
    ("hochladungen", Anhangzaehler),
    ("hochladebank_belegt", Anhangzaehler),
    ("anhang_abgewiesen", Anhangzaehler),
    ("anhaenge_geloescht", Anhangzaehler),
    ("anhaenge_ungekuerzt", Anhangzaehler),
];

/// Alle Blätter der Antwort mit Wert; Objekte als `a.b`.
fn blaetter(wert: &Value, vorsatz: &str, ziel: &mut Vec<(String, Value)>) {
    match wert {
        Value::Object(felder) => {
            for (name, inhalt) in felder {
                let pfad = if vorsatz.is_empty() {
                    name.clone()
                } else {
                    format!("{vorsatz}.{name}")
                };
                blaetter(inhalt, &pfad, ziel);
            }
        }
        _ => ziel.push((vorsatz.to_string(), wert.clone())),
    }
}

fn flach(gesund: &Value) -> Vec<(String, Value)> {
    let mut ziel = Vec::new();
    blaetter(gesund, "", &mut ziel);
    ziel
}

async fn health(basis: &str) -> Value {
    let (status, json) = gesundheit(basis).await;
    assert_eq!(status, 200);
    json
}

/// Die Blätter, deren Wert sich zwischen zwei Schnappschüssen ändert.
fn geaendert(vorher: &Value, nachher: &Value) -> BTreeSet<String> {
    let vorher = flach(vorher);
    flach(nachher)
        .into_iter()
        .filter(|eintrag| !vorher.contains(eintrag))
        .map(|(name, _)| name)
        .collect()
}

/// B0783 erster Test, Schlüsselliste: die Blätter der Antwort sind genau die Zeilen des Inventars — ein neues Feld
/// ohne Urteil macht den Test rot, ebenso eine Zeile ohne Feld.
#[tokio::test]
async fn jedes_feld_der_antwort_hat_im_inventar_ein_urteil() {
    let wurzel = ordner();
    let instanz = gemeinsam::starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("TZ", "Europe/Berlin")],
    );

    let gesund = health(&instanz.adresse).await;

    let antwort: BTreeSet<String> = flach(&gesund).into_iter().map(|(name, _)| name).collect();
    let inventar: BTreeSet<String> = INVENTAR.iter().map(|(name, _)| name.to_string()).collect();
    let ohne_urteil: Vec<_> = antwort.difference(&inventar).collect();
    let ohne_feld: Vec<_> = inventar.difference(&antwort).collect();
    assert!(
        ohne_urteil.is_empty() && ohne_feld.is_empty(),
        "Felder ohne Urteil in ADR-0055 Regel 9: {ohne_urteil:?}; Zeilen ohne Feld: {ohne_feld:?}"
    );
    assert_eq!(inventar.len(), INVENTAR.len(), "keine doppelte Zeile");
}

async fn runde_anhaenge() -> Runde {
    let wurzel = ordner();
    let instanz = starten_anhaenge(wurzel.path(), &[], &[]);
    runde_mit(wurzel, instanz).await
}

fn erlaubt_bei_anhang() -> BTreeSet<String> {
    INVENTAR
        .iter()
        .filter(|(_, urteil)| *urteil == Anhangzaehler)
        .map(|(name, _)| name.to_string())
        .collect()
}

/// H3: Direktnachricht mit Anhang, Löschen einer Direktnachricht mit Anhang, Löschen eines Anhangs über das
/// Laufwerk → von Schritt zu Schritt ändern sich nur die Felder des Restorakels (f); `direkt_mib` bleibt 0 und
/// `nachrichten` zählt nur den einen Channel-Satz.
#[tokio::test]
async fn ein_direktanhang_bewegt_nur_die_anhangzaehler() {
    let r = runde_anhaenge().await;
    let basis = r.instanz.adresse.clone();
    let channel =
        gemeinsam::senden::senden(&r.cora.absender, &basis, &r.cora.token, "allgemein", "c").await;
    assert_eq!(channel.status, 201, "{}", channel.json);
    let mut staende = vec![health(&basis).await];
    let schritt = |gesund: Value, staende: &mut Vec<Value>| staende.push(gesund);

    let a = ablegen(
        &r.anna.absender,
        &basis,
        &r.anna.token,
        "a.bin",
        vec![1; 10],
    )
    .await;
    schritt(health(&basis).await, &mut staende);
    let gesendet = r
        .anna
        .senden(&basis, BEN, &json!({"text": "mit", "anhaenge": [a]}))
        .await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    schritt(health(&basis).await, &mut staende);
    let zeit = gesendet.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();
    let weg = r
        .anna
        .post(&basis, BEN, "nachrichten/loeschen", &json!({"zeit": zeit}))
        .await;
    assert_eq!(weg.status, 200, "{}", weg.json);
    schritt(health(&basis).await, &mut staende);
    let b = ablegen(
        &r.anna.absender,
        &basis,
        &r.anna.token,
        "b.bin",
        vec![2; 10],
    )
    .await;
    r.anna
        .senden(&basis, BEN, &json!({"text": "noch", "anhaenge": [b]}))
        .await;
    schritt(health(&basis).await, &mut staende);
    let laufwerk = gemeinsam::api_antwort(
        r.anna
            .absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/anhaenge/loeschen"),
            )
            .header("X-LocalchatA-Sitzung", &r.anna.token)
            .json(&json!({"ids": [b], "bestaetigt": true})),
    )
    .await;
    assert_eq!(laufwerk.status, 200, "{}", laufwerk.json);
    schritt(health(&basis).await, &mut staende);

    let erlaubt = erlaubt_bei_anhang();
    let mut bewegt = BTreeSet::new();
    for paar in staende.windows(2) {
        let diff = geaendert(&paar[0], &paar[1]);
        // Löscht ein Schritt einen Anhang, schreibt er die Zeile `INFORMATION anhang Anhänge gelöscht` (ohne
        // Gespräch, ohne Ordner) — `protokoll_bytes_heute` wächst dann zugleich mit `anhaenge_geloescht` und sagt
        // nicht mehr als dieser Zähler. Ohne gelöschten Anhang darf es sich nicht bewegen. Die Inventarzeile der ADR
        // nennt diese Zeile nicht (Befund im Bericht).
        let mut erlaubt = erlaubt.clone();
        if diff.contains("anhaenge_geloescht") {
            erlaubt.insert("protokoll_bytes_heute".to_string());
        }
        let fremd: Vec<_> = diff.difference(&erlaubt).collect();
        assert!(fremd.is_empty(), "außerhalb des Restorakels (f): {fremd:?}");
        bewegt.extend(diff);
    }
    assert!(
        bewegt.contains("anhaenge_geloescht"),
        "Kontrolle: {bewegt:?}"
    );
    let letzter = staende.last().expect("Stand");
    assert_eq!(letzter["direkt_mib"], 0, "{letzter}");
    assert_eq!(letzter["nachrichten"], 1, "{letzter}");
}
