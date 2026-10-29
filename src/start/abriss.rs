//! Abriss an der Abrissgrenze erkennen (R00018 F0060, ADR-0042 Ergänzung #24 Punkt 7): eine Verbindung, die der
//! Kernel nach der Sendefrist oder bei stillem Abriss schließt, liefert beim Lesen oder Schreiben `ETIMEDOUT`.
//! Probe aus dem Gate K5 (2026-10-29): hyper reicht diesen Fehler nicht durch — `serve_connection` endet mit
//! `Ok(())`. Deshalb beobachtet `Beobachtet` den Strom selbst und merkt sich den ersten Ein-/Ausgabefehler;
//! eingeordnet wird er wie ein Fehler aus `serve_connection` (die Einordnung folgt der Probe).

use crate::protokoll::episoden::Auftreten;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use std::error::Error;
use std::io;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Ein Strom, der seinen ersten Ein-/Ausgabefehler festhält, ohne etwas daran zu ändern.
pub struct Beobachtet<S> {
    strom: S,
    erster_fehler: Arc<OnceLock<io::Error>>,
}

impl<S> Beobachtet<S> {
    /// Der beobachtete Strom und der Platz, an dem sein erster Fehler steht.
    pub fn neu(strom: S) -> (Beobachtet<S>, Arc<OnceLock<io::Error>>) {
        let erster_fehler = Arc::new(OnceLock::new());
        let beobachtet = Beobachtet {
            strom,
            erster_fehler: Arc::clone(&erster_fehler),
        };
        (beobachtet, erster_fehler)
    }

    fn merken<T>(&self, ergebnis: Poll<io::Result<T>>) -> Poll<io::Result<T>> {
        if let Poll::Ready(Err(fehler)) = &ergebnis {
            let kopie = match fehler.raw_os_error() {
                Some(nummer) => io::Error::from_raw_os_error(nummer),
                None => io::Error::from(fehler.kind()),
            };
            let _ = self.erster_fehler.set(kopie);
        }
        ergebnis
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Beobachtet<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        puffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let ergebnis = Pin::new(&mut self.strom).poll_read(cx, puffer);
        self.merken(ergebnis)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Beobachtet<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        daten: &[u8],
    ) -> Poll<io::Result<usize>> {
        let ergebnis = Pin::new(&mut self.strom).poll_write(cx, daten);
        self.merken(ergebnis)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        teile: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let ergebnis = Pin::new(&mut self.strom).poll_write_vectored(cx, teile);
        self.merken(ergebnis)
    }

    fn is_write_vectored(&self) -> bool {
        self.strom.is_write_vectored()
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let ergebnis = Pin::new(&mut self.strom).poll_flush(cx);
        self.merken(ergebnis)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let ergebnis = Pin::new(&mut self.strom).poll_shutdown(cx);
        self.merken(ergebnis)
    }
}

/// Was eine Verbindung braucht, um ihren Abriss einzutragen: Protokoll, Herkunft, Abrissgrenze (falls gepflegt).
pub struct Abrissspur<'a> {
    pub protokoll: &'a Protokoll,
    pub herkunft: IpAddr,
    pub grenze: Option<Duration>,
}

impl Abrissspur<'_> {
    /// Endete die Verbindung mit einem Abriss — im Ergebnis oder im ersten Fehler des Stroms —, zählt er in
    /// `abgerissen` (immer) und geht als WARNING-Episode je Herkunft ins Protokoll: `herkunft`, `hoechstens` =
    /// Abrissgrenze in ms, ohne `grenze` (es gibt nur eine Art).
    pub fn verbindung_endete<E: Error + 'static>(
        &self,
        ergebnis: &Result<(), E>,
        erster_fehler: Option<&io::Error>,
    ) {
        let im_ergebnis = ergebnis
            .as_ref()
            .err()
            .is_some_and(|fehler| ist_abriss(fehler));
        let im_strom = erster_fehler.is_some_and(|fehler| ist_abriss(fehler));
        if !(im_ergebnis || im_strom) {
            return;
        }
        self.protokoll.abriss_zaehlen();
        let mut kontext = Kontext::neu().mit(Schluessel::Herkunft, self.herkunft);
        if let Some(grenze) = self.grenze {
            kontext = kontext.mit(Schluessel::Hoechstens, grenze.as_millis());
        }
        self.protokoll.episode(Auftreten {
            stufe: Stufe::Warning,
            bereich: Bereich::Verbindung,
            text: crate::protokoll::episodentexte::ABRISSGRENZE,
            quelle: self.herkunft.to_string(),
            kontext,
            art: None,
            verworfen: None,
            bytes: None,
            anzeige: None,
        });
    }
}

/// Wahr, wenn in der Quellenkette ein `io::Error` der Art `TimedOut` steht (rein).
pub fn ist_abriss(fehler: &(dyn Error + 'static)) -> bool {
    let mut glied = Some(fehler);
    while let Some(fehler) = glied {
        let abgelaufen = fehler
            .downcast_ref::<io::Error>()
            .is_some_and(|io| io.kind() == io::ErrorKind::TimedOut);
        if abgelaufen {
            return true;
        }
        glied = fehler.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    /// Ein Fehler, der einen anderen einpackt — wie hyper den `io::Error` der Verbindung.
    #[derive(Debug)]
    struct Eingepackt(io::Error);

    impl fmt::Display for Eingepackt {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Verbindung gescheitert")
        }
    }

    impl Error for Eingepackt {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn timed_out_ist_ein_abriss_auch_eingepackt() {
        let direkt = io::Error::from(io::ErrorKind::TimedOut);
        let eingepackt = Eingepackt(io::Error::from(io::ErrorKind::TimedOut));

        assert!(ist_abriss(&direkt));
        assert!(ist_abriss(&eingepackt));
    }

    #[test]
    fn andere_enden_sind_kein_abriss() {
        for art in [
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::UnexpectedEof,
        ] {
            assert!(!ist_abriss(&io::Error::from(art)), "{art:?}");
            assert!(!ist_abriss(&Eingepackt(io::Error::from(art))), "{art:?}");
        }
    }

    /// R00018 F0060 (ADR-0042 Ergänzung #24 Punkt 7): zwei Abrisse derselben Herkunft → eine Episode, zwei
    /// gezählt; ein anderes Ende zählt nicht und schreibt nichts.
    #[test]
    fn ein_abriss_ist_eine_episode_je_herkunft_und_zaehlt_jedes_mal() {
        use crate::protokoll::testhilfe::im_speicher;
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let spur = Abrissspur {
            protokoll: &protokoll,
            herkunft: "127.0.0.2".parse().expect("IP"),
            grenze: Some(Duration::from_millis(4000)),
        };

        let abgelaufen = io::Error::from(io::ErrorKind::TimedOut);
        let zurueckgesetzt = io::Error::from(io::ErrorKind::ConnectionReset);
        spur.verbindung_endete(
            &Err(Eingepackt(io::Error::from(io::ErrorKind::TimedOut))),
            None,
        );
        spur.verbindung_endete::<io::Error>(&Ok(()), Some(&abgelaufen));
        spur.verbindung_endete(&Err(zurueckgesetzt), None);
        spur.verbindung_endete::<io::Error>(
            &Ok(()),
            Some(&io::Error::from(io::ErrorKind::BrokenPipe)),
        );
        spur.verbindung_endete::<io::Error>(&Ok(()), None);

        assert_eq!(
            speicher.zeilen(),
            [
                "2026-10-28T12:00:00.000+01:00 WARNING verbindung Verbindung an der Abrissgrenze geschlossen — Episode beginnt | herkunft=127.0.0.2 hoechstens=4000"
            ]
        );
        assert_eq!(protokoll.kennzahlen().abgerissen, 2);
        assert_eq!(protokoll.zaehlerstand().warnings, 2);
    }

    /// Ein Strom, dessen Schreiben scheitert wie eine abgerissene Verbindung (ETIMEDOUT).
    struct Abgerissen;

    impl AsyncRead for Abgerissen {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _puffer: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for Abgerissen {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _daten: &[u8],
        ) -> Poll<io::Result<usize>> {
            Poll::Ready(Err(io::Error::from_raw_os_error(110)))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Err(io::Error::from(io::ErrorKind::NotConnected)))
        }
    }

    /// Der Beobachter hält den ersten Fehler fest, auch wenn der Aufrufer ihn verschluckt; spätere ändern ihn nicht.
    #[tokio::test]
    async fn der_erste_fehler_des_stroms_bleibt_stehen() {
        use tokio::io::AsyncWriteExt;
        let (mut strom, erster_fehler) = Beobachtet::neu(Abgerissen);

        let _ = strom.write_all(b"Antwort").await;
        let _ = strom.shutdown().await;

        let fehler = erster_fehler.get().expect("erster Fehler");
        assert_eq!(fehler.kind(), io::ErrorKind::TimedOut);
        assert!(ist_abriss(fehler));
    }
}
