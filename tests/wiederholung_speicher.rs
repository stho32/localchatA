//! R00025 F0087 B0493 Nacharbeit „Speicher des Wiederholspeichers“ (ADR-0034 Ergänzung „Karte #36 Speicher“, S1 und
//! S2): ein zählender Allokator misst, was `Wiederholung::default()` plus 36.000 × `merken` im ungünstigsten Fall
//! allokiert hält — je Eintrag eine eigene Kennung (Name 32 + `@` + IPv6-Herkunft 39 Zeichen), Schlüssel 64 Zeichen,
//! längster Channelname (32). S1 verlangt höchstens 1.200 Bytes je Eintrag; gemessen wird die Netto-Allokation
//! (Kapazität, nicht berührte Seiten), nicht der RSS. Der zweite Fall (57 Kennungen zu je ≈ 632 Einträgen) ist nur
//! Messwert. Ausgabe der Werte: `cargo test --test wiederholung_speicher -- --nocapture`.
//!
//! Eigenes Test-Crate, weil `GlobalAlloc` `unsafe` verlangt und die Bibliothek `#![forbid(unsafe_code)]` trägt
//! (ADR-0010 Ergänzung Karte #35, Regel 5); der Allokator gilt so nur für dieses Testprogramm. Die Zähler stehen je
//! Thread, damit parallel laufende Tests nicht mitzählen (ADR-0034 S2, Pre-Mortem 1).
#![allow(unsafe_code)]

use localchata::gespraeche::wiederholung::{Pruefung, Wiederholung, pruefsumme};
use localchata::gespraeche::{Gespraech, Kennung};
use localchata::uhr::Zeit;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// Zahlen aus ADR-0034 Ergänzung „Karte #36 Speicher“, S1/S2 — bewusst nicht aus den Konstanten des Codes.
const EINTRAEGE: usize = 36_000;
const HOECHSTENS_BYTES_JE_EINTRAG: usize = 1_200;
const KENNUNGEN_IM_ZWEITEN_FALL: usize = 57;
/// Untergrenze als Probe des Zählers: Schlüssel (64) und Kennung (72) muss der Speicher je Eintrag mindestens einmal
/// halten — misst er weniger, zählt der Allokator nicht, und S2 wäre grün ohne Aussage.
const MINDESTENS_TEXT_JE_EINTRAG: usize = 64 + 72;

struct Zaehlend;

thread_local! {
    /// Netto allokierte Bytes dieses Threads (Allokationen minus Freigaben, auf diesem Thread ausgeführt).
    static NETTO: Cell<isize> = const { Cell::new(0) };
}

fn buchen(bytes: isize) {
    // `try_with`: beim Abbau eines Threads ist die Ablage schon fort — dann wird nicht gezählt.
    let _ = NETTO.try_with(|netto| netto.set(netto.get() + bytes));
}

fn als_zahl(bytes: usize) -> isize {
    isize::try_from(bytes).unwrap_or(isize::MAX)
}

// SAFETY: reicht jeden Aufruf unverändert an `System` weiter und zählt nur; Layout und Zeiger bleiben die des
// Aufrufers, die Zählung allokiert nichts (Thread-Ablage mit `const`-Anfangswert ohne `Drop`).
unsafe impl GlobalAlloc for Zaehlend {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: dieselben Zusicherungen wie der Aufrufer von `alloc`.
        let zeiger = unsafe { System.alloc(layout) };
        if !zeiger.is_null() {
            buchen(als_zahl(layout.size()));
        }
        zeiger
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: dieselben Zusicherungen wie der Aufrufer von `alloc_zeroed`.
        let zeiger = unsafe { System.alloc_zeroed(layout) };
        if !zeiger.is_null() {
            buchen(als_zahl(layout.size()));
        }
        zeiger
    }

    unsafe fn dealloc(&self, zeiger: *mut u8, layout: Layout) {
        // SAFETY: `zeiger` kommt aus diesem Allokator mit genau diesem `layout`.
        unsafe { System.dealloc(zeiger, layout) };
        buchen(-als_zahl(layout.size()));
    }

    unsafe fn realloc(&self, zeiger: *mut u8, layout: Layout, neue_groesse: usize) -> *mut u8 {
        // SAFETY: dieselben Zusicherungen wie der Aufrufer von `realloc`.
        let neu = unsafe { System.realloc(zeiger, layout, neue_groesse) };
        if !neu.is_null() {
            buchen(als_zahl(neue_groesse) - als_zahl(layout.size()));
        }
        neu
    }
}

#[global_allocator]
static ALLOKATOR: Zaehlend = Zaehlend;

fn netto() -> isize {
    NETTO.with(Cell::get)
}

fn jetzt() -> Zeit {
    Zeit::parse_from_rfc3339("2026-10-29T12:00:00+01:00").expect("Zeit")
}

/// Längster Channelname: 32 Zeichen (Namensregel ADR-0013).
fn laengster_channel() -> Gespraech {
    Gespraech::channel(&"c".repeat(32))
}

/// Kennung der Form aus S1: Name mit 32 Zeichen, `@`, IPv6-Herkunft in der längsten Textform (39 Zeichen).
fn kennung(nummer: usize) -> Kennung {
    Kennung::bilden(&format!(
        "{nummer:0>32}@ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"
    ))
    .expect("Kennung")
}

fn schluessel(nummer: usize) -> String {
    format!("{nummer:0>64}")
}

/// Füllt einen frischen Speicher mit `EINTRAEGE` Einträgen; Eintrag `i` gehört der Kennung `i % kennungen`. Gibt
/// den Speicher und die netto allokierten Bytes zurück, die er danach hält (Eingaben sind bis dahin freigegeben).
fn gefuellt(kennungen: usize) -> (Wiederholung, isize) {
    let channel = laengster_channel();
    let zeit = jetzt();
    let vorher = netto();
    let mut speicher = Wiederholung::default();
    let mut verdraengt = 0;
    for nummer in 0..EINTRAEGE {
        let wer = kennung(nummer % kennungen);
        let schluessel = schluessel(nummer);
        let summe = pruefsumme(&schluessel);
        if speicher
            .merken(&wer, &schluessel, &channel, summe, zeit, zeit)
            .is_some()
        {
            verdraengt += 1;
        }
    }
    let gehalten = netto() - vorher;
    assert_eq!(verdraengt, 0, "der Speicher muss alle Einträge halten");
    (speicher, gehalten)
}

fn je_eintrag(gehalten: isize) -> f64 {
    gehalten as f64 / EINTRAEGE as f64
}

/// Alle Einträge sind noch da: der erste und der letzte gelten als Wiederholung.
fn haelt_alle(speicher: &Wiederholung, kennungen: usize) -> bool {
    [0, EINTRAEGE - 1].iter().all(|&nummer| {
        let schluessel = schluessel(nummer);
        matches!(
            speicher.pruefen(
                &kennung(nummer % kennungen),
                &schluessel,
                &laengster_channel(),
                pruefsumme(&schluessel),
                jetzt(),
            ),
            Pruefung::Wiederholt { .. }
        )
    })
}

/// ADR-0034 S2: im ungünstigsten Fall (je Eintrag eine eigene Kennung) höchstens 1.200 Bytes je Eintrag, also
/// höchstens 43,2 MB für 36.000.
#[test]
fn der_speicher_haelt_im_unguenstigsten_fall_hoechstens_1200_bytes_je_eintrag() {
    let (speicher, gehalten) = gefuellt(EINTRAEGE);

    println!(
        "Wiederholspeicher S2: {EINTRAEGE} Kennungen je 1 Eintrag → {gehalten} B allokiert, {:.0} B je Eintrag \
         (höchstens {HOECHSTENS_BYTES_JE_EINTRAG})",
        je_eintrag(gehalten)
    );
    assert!(haelt_alle(&speicher, EINTRAEGE));
    assert!(
        gehalten >= als_zahl(EINTRAEGE * MINDESTENS_TEXT_JE_EINTRAG),
        "der Zähler sieht nicht einmal Schlüssel und Kennung je einmal ({gehalten} B) — er zählt nicht"
    );
    assert!(
        gehalten <= als_zahl(EINTRAEGE * HOECHSTENS_BYTES_JE_EINTRAG),
        "Wiederholspeicher hält {gehalten} B = {:.0} B je Eintrag, erlaubt höchstens {HOECHSTENS_BYTES_JE_EINTRAG} \
         (ADR-0034 Ergänzung „Karte #36 Speicher“, S1) — Befund an den architekt (Option S-B)",
        je_eintrag(gehalten)
    );
}

/// ADR-0034 S2, zweiter Fall: 57 Kennungen zu je ≈ 632 Einträgen — der Speicher hält alle 36.000 (unter 640 je
/// Kennung); der Speicher je Eintrag wird nur ausgegeben, nicht geprüft. Läuft parallel zum ersten Fall und belegt
/// so, dass die Zähler je Thread einander nicht sehen.
#[test]
fn siebenundfuenfzig_kennungen_halten_alle_eintraege_und_der_speicher_wird_ausgegeben() {
    let (speicher, gehalten) = gefuellt(KENNUNGEN_IM_ZWEITEN_FALL);

    println!(
        "Wiederholspeicher S2 (Messwert): {KENNUNGEN_IM_ZWEITEN_FALL} Kennungen → {gehalten} B allokiert, {:.0} B \
         je Eintrag",
        je_eintrag(gehalten)
    );
    assert!(haelt_alle(&speicher, KENNUNGEN_IM_ZWEITEN_FALL));
}
