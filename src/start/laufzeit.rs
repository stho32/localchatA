//! Laufzeit mit Rückhalt (B0255, ADR-0039 Regel 5): der Runtime-Builder statt `#[tokio::main]`, mit höchstens
//! `BLOCKING_HOECHSTENS` Blocking-Threads statt 512 — so viele Plätze, wie eine Herkunft Verbindungen hat.
//! Leichte Blocking-Arbeit (Channel-Stand, Plattenschreiben) läuft ohne Werkbank; dieser Deckel begrenzt sie mit.

use tokio::runtime::{Builder, Runtime};

pub const BLOCKING_HOECHSTENS: usize = 64;

/// Die Laufzeit des Programms: mehrere Worker, alle Treiber, höchstens 64 Blocking-Threads.
pub fn laufzeit() -> std::io::Result<Runtime> {
    Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(BLOCKING_HOECHSTENS)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    /// 80 blockierende Aufgaben, die warten, bis der Test sie freigibt: zugleich laufen genau 64 — auch 300 ms
    /// nach dem Erreichen der 64 nicht mehr; nach der Freigabe laufen alle 80 zu Ende.
    #[test]
    fn hoechstens_64_blocking_threads_zugleich() {
        let laufzeit = laufzeit().expect("Laufzeit");
        let laufend = Arc::new(AtomicUsize::new(0));
        let fertig = Arc::new(AtomicUsize::new(0));
        let frei = Arc::new(AtomicBool::new(false));
        for _ in 0..80 {
            let (laufend, fertig, frei) = (laufend.clone(), fertig.clone(), frei.clone());
            laufzeit.spawn_blocking(move || {
                laufend.fetch_add(1, Ordering::SeqCst);
                while !frei.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(5));
                }
                fertig.fetch_add(1, Ordering::SeqCst);
            });
        }
        let warten_bis = |bedingung: &dyn Fn() -> bool| {
            let beginn = Instant::now();
            while !bedingung() && beginn.elapsed() < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_millis(5));
            }
        };

        warten_bis(&|| laufend.load(Ordering::SeqCst) >= 64);
        std::thread::sleep(Duration::from_millis(300));
        let zugleich = laufend.load(Ordering::SeqCst);
        frei.store(true, Ordering::SeqCst);
        warten_bis(&|| fertig.load(Ordering::SeqCst) == 80);

        assert_eq!(zugleich, 64);
        assert_eq!(fertig.load(Ordering::SeqCst), 80);
    }
}
