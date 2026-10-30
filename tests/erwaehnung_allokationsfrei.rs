//! R00049 Nacharbeit security-reviewer #17 Befund 2 (Erwähnungsregel): die Trefferregel prüft die Erwähnung eines
//! Wartenden ohne Allokation — auch erwähnungsdicht (16 KiB „@a “ × 5.461), wo vorher je `@` ein String entstand. Ein
//! zählender Allokator zählt die Allokationen eines Aufrufs; das Zeitmaß dazu steht als Release-Test in
//! `src/gespraeche/treffer.rs` (`erwaehnungsdicht_bleibt_die_trefferregel_unter_20_us`), dieser Test ist sein
//! deterministischer Wächter im Debug-Gate.
//!
//! Eigenes Test-Crate, weil `GlobalAlloc` `unsafe` verlangt und die Bibliothek `unsafe_code` verbietet (wie
//! `tests/wiederholung_speicher.rs`); die Zähler stehen je Thread, damit parallel laufende Tests nicht mitzählen.
#![allow(unsafe_code)]

use localchata::gespraeche::treffer::{Anlass, Filter, treffer};
use localchata::gespraeche::{Gespraech, Kennung, Nachricht};
use localchata::uhr::Zeit;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Zaehlend;

thread_local! {
    /// Zahl der Allokationen dieses Threads.
    static ANLAGEN: Cell<usize> = const { Cell::new(0) };
}

fn buchen() {
    // `try_with`: beim Abbau eines Threads ist die Ablage schon fort — dann wird nicht gezählt.
    let _ = ANLAGEN.try_with(|anlagen| anlagen.set(anlagen.get() + 1));
}

// SAFETY: reicht jeden Aufruf unverändert an `System` weiter und zählt nur; Layout und Zeiger bleiben die des
// Aufrufers, die Zählung allokiert nichts (Thread-Ablage mit `const`-Anfangswert ohne `Drop`).
unsafe impl GlobalAlloc for Zaehlend {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        buchen();
        // SAFETY: dieselben Zusicherungen wie der Aufrufer von `alloc`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        buchen();
        // SAFETY: dieselben Zusicherungen wie der Aufrufer von `alloc_zeroed`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, zeiger: *mut u8, layout: Layout) {
        // SAFETY: `zeiger` kommt aus diesem Allokator mit genau diesem `layout`.
        unsafe { System.dealloc(zeiger, layout) };
    }

    unsafe fn realloc(&self, zeiger: *mut u8, layout: Layout, neue_groesse: usize) -> *mut u8 {
        buchen();
        // SAFETY: dieselben Zusicherungen wie der Aufrufer von `realloc`.
        unsafe { System.realloc(zeiger, layout, neue_groesse) }
    }
}

#[global_allocator]
static ALLOKATOR: Zaehlend = Zaehlend;

fn anlagen() -> usize {
    ANLAGEN.with(Cell::get)
}

fn nachricht(text: String) -> Nachricht {
    Nachricht {
        zeit: Zeit::parse_from_rfc3339("2026-10-30T12:00:00.000+01:00").expect("Zeit"),
        von: Kennung::bilden("anna@127.0.0.1").expect("Kennung"),
        art: None,
        text,
        anhaenge: Vec::new(),
        geaendert: None,
        bezug: None,
        antwort_auf: None,
    }
}

/// Allokationen eines Aufrufs der Trefferregel für `claude` mit `erwaehnung` und das Ergebnis.
fn ein_aufruf(text: String) -> (usize, Option<Vec<Anlass>>) {
    let gespraech = Gespraech::channel("b");
    let nachricht = nachricht(text);
    let wartender = Kennung::bilden("claude@127.0.0.1").expect("Kennung");
    let filter = Filter {
        erwaehnung: true,
        ..Filter::default()
    };
    let vorher = anlagen();
    let ergebnis = treffer(&gespraech, &nachricht, &wartender, &filter);
    (anlagen() - vorher, ergebnis)
}

/// Erwähnungsdicht ohne den Namen des Wartenden: kein Treffer und keine Allokation (vorher 5.461, je `@` eine).
/// Kontrolle des Zählers: trifft die Erwähnung, legt die Antwort den Anlass an — mindestens eine Allokation.
#[test]
fn die_erwaehnung_wird_ohne_allokation_geprueft() {
    let (dicht, ohne_treffer) = ein_aufruf("@a ".repeat(5_461));
    let (ohne_at, kein_at) = ein_aufruf("x".repeat(16_384));
    let (mit_namen, getroffen) = ein_aufruf(format!("{}@claude", "@a ".repeat(5_000)));

    assert_eq!((dicht, ohne_treffer), (0, None), "erwähnungsdicht");
    assert_eq!((ohne_at, kein_at), (0, None), "ohne @");
    assert_eq!(getroffen, Some(vec![Anlass::Erwaehnung]));
    assert!(mit_namen >= 1, "der Zähler zählt: {mit_namen}");
}
