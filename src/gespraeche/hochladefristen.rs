//! Hochladefristen (B0428, ADR-0046 Regel 2.4 und Ergänzung „Karte #9 Gate“ Regel 1): Stückfrist — kommen 10 s
//! lang keine weiteren 64 KiB (bzw. der Rest) an — und Gesamtfrist `min(beginn + 60 s + Länge / 1 MiB/s,
//! lebensende)`. Rein: Zeiten als Abstand vom Beginn des `PUT`.

use std::time::Duration;

/// So lange darf zwischen zwei Fortschritten vergehen.
pub const STUECKFRIST: Duration = Duration::from_secs(10);
/// So viele Bytes sind ein Fortschritt.
pub const FORTSCHRITT: u64 = 64 * 1024;
/// Grundzeit der Gesamtfrist vor der Übertragung mit der Mindestrate.
const GRUNDZEIT_MS: u128 = 60_000;
/// Mindestrate 1 MiB/s.
const MINDESTRATE: u128 = 1_048_576;

/// Welche Frist gegriffen hat (408).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fristart {
    Stueckfrist,
    Gesamtfrist,
}

/// `60 s + Länge / 1 MiB/s` in ms aufgerundet, gekappt durch den Rest bis zum Lebensende.
pub fn gesamtfrist(laenge: u64, bis_lebensende: Duration) -> Duration {
    let ms = GRUNDZEIT_MS + (u128::from(laenge) * 1_000).div_ceil(MINDESTRATE);
    let eigene = Duration::from_millis(u64::try_from(ms).unwrap_or(u64::MAX));
    eigene.min(bis_lebensende)
}

/// Der Stand der Fristen eines `PUT`.
#[derive(Debug, Clone)]
pub struct Fristen {
    laenge: u64,
    gesamt: Duration,
    gelesen: u64,
    /// Gelesen beim letzten Fortschritt und wann.
    stand_fortschritt: u64,
    zeit_fortschritt: Duration,
}

impl Fristen {
    pub fn neu(laenge: u64, gesamt: Duration) -> Fristen {
        Fristen {
            laenge,
            gesamt,
            gelesen: 0,
            stand_fortschritt: 0,
            zeit_fortschritt: Duration::ZERO,
        }
    }

    /// `bytes` sind zum Zeitpunkt `jetzt` (Abstand vom Beginn) angekommen.
    pub fn gelesen(&mut self, bytes: u64, jetzt: Duration) {
        self.gelesen = self.gelesen.saturating_add(bytes);
        let noetig = FORTSCHRITT.min(self.laenge.saturating_sub(self.stand_fortschritt));
        if self.gelesen - self.stand_fortschritt >= noetig.max(1) {
            self.stand_fortschritt = self.gelesen;
            self.zeit_fortschritt = jetzt;
        }
    }

    /// Wann die nächste Frist greift, wenn bis dahin nichts ankommt.
    pub fn naechste(&self) -> (Duration, Fristart) {
        let stueck = self.zeit_fortschritt + STUECKFRIST;
        if self.gesamt <= stueck {
            (self.gesamt, Fristart::Gesamtfrist)
        } else {
            (stueck, Fristart::Stueckfrist)
        }
    }

    /// Ob zum Zeitpunkt `jetzt` eine Frist gegriffen hat.
    pub fn pruefen(&self, jetzt: Duration) -> Option<Fristart> {
        let (frist, art) = self.naechste();
        (jetzt >= frist).then_some(art)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1_048_576;

    fn ms(wert: u64) -> Duration {
        Duration::from_millis(wert)
    }

    /// 64 KiB, dann 9,999 s Stille → läuft; 10,000 s → Stückfrist.
    #[test]
    fn ohne_64_kib_in_10_s_greift_die_stueckfrist() {
        let mut fristen = Fristen::neu(MIB, gesamtfrist(MIB, Duration::MAX));
        fristen.gelesen(FORTSCHRITT, ms(0));
        assert_eq!(fristen.pruefen(ms(9_999)), None);
        assert_eq!(fristen.pruefen(ms(10_000)), Some(Fristart::Stueckfrist));
    }

    /// 1 Byte je 9 s ist kein Fortschritt — nach 10 s ohne 64 KiB Stückfrist.
    #[test]
    fn troepfeln_ist_kein_fortschritt() {
        let mut fristen = Fristen::neu(MIB, gesamtfrist(MIB, Duration::MAX));
        fristen.gelesen(1, ms(0));
        fristen.gelesen(1, ms(9_000));
        assert_eq!(fristen.pruefen(ms(9_999)), None);
        assert_eq!(fristen.pruefen(ms(10_000)), Some(Fristart::Stueckfrist));
    }

    /// Fehlt weniger als 64 KiB, ist der Rest der Fortschritt.
    #[test]
    fn der_rest_unter_64_kib_ist_ein_fortschritt() {
        let mut fristen = Fristen::neu(FORTSCHRITT + 10, Duration::MAX);
        fristen.gelesen(FORTSCHRITT, ms(1_000));
        fristen.gelesen(10, ms(10_500));
        assert_eq!(fristen.pruefen(ms(20_000)), None);
    }

    /// Gesamtfrist für 10 MiB = 70 s; 10 MiB gleichmäßig mit 1 MiB/s → gelesen. Lesart (R00029 „Fristen“ nennt
    /// 0,9 MiB/s, die 10 MiB aber schon nach 11,1 s ankommen ließen): 90 % der Rate, mit der die 10 MiB genau 70 s
    /// brauchen → Gesamtfrist bei 70 s; mit genau dieser Rate → gelesen.
    #[test]
    fn die_gesamtfrist_fuer_10_mib_ist_70_s() {
        let laenge = 10 * MIB;
        let gesamt = gesamtfrist(laenge, Duration::MAX);
        assert_eq!(gesamt, ms(70_000));

        let mut schnell = Fristen::neu(laenge, gesamt);
        for sekunde in 1..=10 {
            schnell.gelesen(MIB, ms(sekunde * 1_000));
            assert_eq!(schnell.pruefen(ms(sekunde * 1_000)), None);
        }

        let gleichmaessig = |prozent: u64| {
            let mut fristen = Fristen::neu(laenge, gesamt);
            let mut gelesen = 0;
            for zehntel in 1..=800 {
                let jetzt = ms(zehntel * 100);
                let ziel = (laenge * prozent / 100 * zehntel / 700).min(laenge);
                fristen.gelesen(ziel - gelesen, jetzt);
                gelesen = ziel;
                if gelesen == laenge {
                    return (jetzt, None);
                }
                if let Some(art) = fristen.pruefen(jetzt) {
                    return (jetzt, Some(art));
                }
            }
            (ms(80_000), None)
        };
        assert_eq!(gleichmaessig(100), (ms(70_000), None));
        assert_eq!(gleichmaessig(90), (ms(70_000), Some(Fristart::Gesamtfrist)));
    }

    /// Die Gesamtfrist ist höchstens das Lebensende: Beginn t0 + 50 s, 9 MiB, Lebensende t0 + 78 s → 28 s nach dem
    /// Beginn (nicht 69 s; Mutante „Gesamtfrist ohne `min`“).
    #[test]
    fn die_gesamtfrist_endet_spaetestens_am_lebensende() {
        assert_eq!(gesamtfrist(9 * MIB, ms(28_000)), ms(28_000));
        assert_eq!(gesamtfrist(9 * MIB, ms(100_000)), ms(69_000));
        assert_eq!(gesamtfrist(1, Duration::MAX), ms(60_001));
        let fristen = Fristen::neu(9 * MIB, gesamtfrist(9 * MIB, ms(28_000)));
        assert_eq!(fristen.naechste(), (ms(10_000), Fristart::Stueckfrist));
        let mut weiter = fristen.clone();
        weiter.gelesen(FORTSCHRITT, ms(25_000));
        assert_eq!(weiter.naechste(), (ms(28_000), Fristart::Gesamtfrist));
    }
}
