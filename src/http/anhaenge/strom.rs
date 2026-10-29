//! Gestreamt senden (B0447, ADR-0039 Regel 7c): Stücke zu 64 KiB ab dem Anfang des Bereichs, eines nach dem anderen
//! mit Gegendruck, genau die Länge des Bereichs. Die Sendefrist ist die Abrissgrenze (`TCP_USER_TIMEOUT` jeder
//! Verbindung). Der Deskriptor lebt so lange wie die Antwort — er hält die Bytes eines entfallenden Tags bis zum
//! Ende. Liest ein Stück weniger als erwartet (Datei gekürzt), endet der Strom mit einem Fehler: die Verbindung
//! schließt ohne Auffüllen, keine Schleife.

use crate::gespraeche::anhangablage::Anhangdatei;
use axum::body::Bytes;
use futures_util::{Stream, StreamExt};
use std::io;
use std::sync::Arc;

pub const STUECK: usize = 64 * 1024;

/// Die Bytes `[von, bis)` der Datei als Strom. Das erste Stück ist schon gelesen, wenn der Strom zurückkommt — so
/// schreibt hyper Kopf und erstes Stück zusammen, statt auf die verzögerte Bestätigung des Kopfs zu warten.
pub async fn bereich_strom(
    datei: Box<dyn Anhangdatei>,
    von: u64,
    bis: u64,
) -> impl Stream<Item = io::Result<Bytes>> + Send + 'static {
    let datei: Arc<dyn Anhangdatei> = Arc::from(datei);
    let erstes = naechstes(Arc::clone(&datei), von, bis).await;
    let weiter = match &erstes {
        Some((Ok(stueck), _)) => von + stueck.len() as u64,
        _ => bis,
    };
    futures_util::stream::iter(erstes.map(|(stueck, _)| stueck)).chain(
        futures_util::stream::unfold((datei, weiter), move |(datei, pos)| async move {
            naechstes(datei, pos, bis).await
        }),
    )
}

type Schritt = (io::Result<Bytes>, (Arc<dyn Anhangdatei>, u64));

/// Das Stück ab `pos` (höchstens 64 KiB bis `bis`); ein kurzes Lesen ist ein Fehler, danach endet der Strom.
async fn naechstes(datei: Arc<dyn Anhangdatei>, pos: u64, bis: u64) -> Option<Schritt> {
    if pos >= bis {
        return None;
    }
    let n = (bis - pos).min(STUECK as u64) as usize;
    let lesend = Arc::clone(&datei);
    let gelesen = tokio::task::spawn_blocking(move || stueck_lesen(lesend.as_ref(), pos, n))
        .await
        .unwrap_or_else(|fehler| Err(io::Error::other(fehler.to_string())));
    match gelesen {
        Ok(puffer) if puffer.len() == n => Some((Ok(Bytes::from(puffer)), (datei, pos + n as u64))),
        Ok(_) => Some((
            Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
            (datei, bis),
        )),
        Err(fehler) => Some((Err(fehler), (datei, bis))),
    }
}

/// Liest bis zu `n` Bytes ab `pos`; weniger nur am Dateiende.
fn stueck_lesen(datei: &dyn Anhangdatei, pos: u64, n: usize) -> io::Result<Vec<u8>> {
    let mut puffer = vec![0u8; n];
    let mut voll = 0;
    while voll < n {
        let gelesen = datei.lesen_an(pos + voll as u64, &mut puffer[voll..])?;
        if gelesen == 0 {
            break;
        }
        voll += gelesen;
    }
    puffer.truncate(voll);
    Ok(puffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Eine Datei im Speicher; nach `leer_ab` Lesevorgängen liefert sie 0 Bytes (gekürzt).
    struct Attrappe {
        daten: Vec<u8>,
        leer_ab: usize,
        gelesen: Mutex<usize>,
    }

    impl Anhangdatei for Attrappe {
        fn lesen_an(&self, pos: u64, puffer: &mut [u8]) -> io::Result<usize> {
            let mut gelesen = self.gelesen.lock().expect("Zähler");
            *gelesen += 1;
            if *gelesen > self.leer_ab {
                return Ok(0);
            }
            let pos = pos as usize;
            let n = puffer.len().min(self.daten.len().saturating_sub(pos));
            puffer[..n].copy_from_slice(&self.daten[pos..pos + n]);
            Ok(n)
        }

        fn laenge(&self) -> io::Result<u64> {
            Ok(self.daten.len() as u64)
        }
    }

    fn attrappe(laenge: usize, leer_ab: usize) -> Box<dyn Anhangdatei> {
        Box::new(Attrappe {
            daten: (0..laenge).map(|i| (i % 251) as u8).collect(),
            leer_ab,
            gelesen: Mutex::new(0),
        })
    }

    #[tokio::test]
    async fn genau_der_bereich_in_stuecken_zu_64_kib() {
        let stuecke: Vec<Bytes> = bereich_strom(attrappe(200_000, usize::MAX), 100, 150_100)
            .await
            .map(|stueck| stueck.expect("Stück"))
            .collect()
            .await;

        assert_eq!(
            stuecke.iter().map(Bytes::len).collect::<Vec<_>>(),
            vec![STUECK, STUECK, 150_000 - 2 * STUECK]
        );
        let alle: Vec<u8> = stuecke.concat();
        let erwartet: Vec<u8> = (100..150_100).map(|i| (i % 251) as u8).collect();
        assert_eq!(alle, erwartet);
    }

    #[tokio::test]
    async fn eine_gekuerzte_datei_endet_mit_einem_fehler_ohne_auffuellen() {
        let ergebnisse: Vec<io::Result<Bytes>> = bereich_strom(attrappe(300_000, 1), 0, 300_000)
            .await
            .collect()
            .await;

        assert_eq!(
            ergebnisse.len(),
            2,
            "ein Stück, dann der Fehler — keine Schleife"
        );
        assert_eq!(ergebnisse[0].as_ref().expect("erstes Stück").len(), STUECK);
        assert!(ergebnisse[1].is_err());
    }

    #[tokio::test]
    async fn ein_leerer_bereich_ist_ein_leerer_strom() {
        let ergebnisse: Vec<io::Result<Bytes>> = bereich_strom(attrappe(10, usize::MAX), 0, 0)
            .await
            .collect()
            .await;
        assert!(ergebnisse.is_empty());
    }
}
