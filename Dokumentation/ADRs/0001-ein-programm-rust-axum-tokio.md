# ADR-0001: Ein Programm in Rust mit axum und tokio, Bibliothek plus Binärprogramm

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #1 I0001)
**Kontext-Anforderung**: — (WBS I0001 Instanz starten; Vision R00000, Leitplanken)
**Erprobt durch**: bestehende Systemantwort kanbanD (Q04: `Cargo.toml`, `src/main.rs`, `src/web/mod.rs`); Wegwerf-Probe axum 0.8 + tokio im Scratchpad (Abbruch-Erkennung, siehe ADR-0007)

## Kontext

Die Vision setzt Rust und „ein einziges Programm, das Weboberfläche und WebAPI zugleich ausliefert — wie kanbanD“ (Leitplanken). Gebraucht werden HTTP mit langen Verbindungen (Live-Kanal, wartende Agenten), Datei-Uploads (Anhänge) und nebenläufige Schreiber. Qualitätsziel Q7 verlangt, dass Log-Ablage, Index, Live-Kanal und HTTP getrennt testbar sind; der Auftrag verlangt den kleinsten Schnitt (ein Container). kanbanD löst dieselbe Lage mit `axum` 0.8 auf `tokio`, einer Bibliothek `src/lib.rs` und einem dünnen `main.rs`.

## Optionen

### Option A: axum 0.8 + tokio, Crate mit `[lib]` und `[[bin]]`, fachliche Module (wie kanbanD)
- Vorteile: bewährt im Geschwister (Muster `blockierend` = `spawn_blocking`, SSE, Multipart, geordnetes Beenden); Integrationstests rufen die Bibliothek oder das Binärprogramm; eine Bauform für beide Programme des Betreibers.
- Nachteile: async-Laufzeit für ein kleines Programm; Modulgrenzen nur per Review erzwungen.

### Option B: actix-web
- Vorteile: reif, schnell, eingebaute WebSocket-Aktoren.
- Nachteile: zweiter Webrahmen neben kanbanD — kein übertragbares Wissen, andere Testmuster; eigene Laufzeit-Eigenheiten.

### Option C: synchron, nur Standardbibliothek plus `tiny_http`, ein Thread je Verbindung
- Vorteile: kaum Abhängigkeiten, leicht zu durchschauen.
- Nachteile: jede offene SSE-Verbindung und jeder wartende Agent bindet einen Thread; Multipart, SSE und geordnetes Beenden selbst bauen.

### Option D: Cargo-Workspace mit je einer Crate je Komponente
- Vorteile: Grenzen vom Compiler erzwungen.
- Nachteile: Schnittstellen erstarren, bevor die Fachsprache steht; mehr Bau- und Pflegeaufwand.

## Entscheidung

**Gewählt: Option A**

Ein Container „Programm localchatA“, eine Crate `localchata` mit Bibliothek und Binärprogramm `localchatA`, Edition 2024, Module nach Fachlichkeit: `start`, `http`, `gespraeche` (mit `logformat`, `logablage`, `verlaufsindex`), `anwesenheit`, `live`, dazu `fehler` und `uhr`. Es überwiegt, dass kanbanD dieselbe Frage schon beantwortet und erprobt hat und das Bedienkonzept ohnehin gleich sein soll. In Kauf genommen: Modulgrenzen hängen an Review-Disziplin (Grenzregel in A00004), nicht am Compiler — Option D bleibt als späterer Schritt offen.

## Konsequenzen

**Positiv:**
- Wissen und Testmuster aus kanbanD gelten 1:1 (ohne Code zu kopieren).
- Jeder Baustein ist ein Modul mit eigenem `#[cfg(test)]`; das Ganze über `tests/` und `CARGO_BIN_EXE_localchatA`.

**Negativ / Inkauf zu nehmen:**
- Dateizugriffe sind blockierend und laufen über `spawn_blocking`.
- Mindestversion Rust 1.89 (für `File::try_lock`, ADR-0005).

**Folgeentscheidungen / Aufgaben:**
- [ ] Crates: `axum` (`multipart`), `tokio` (`rt-multi-thread`, `macros`, `net`, `sync`, `time`, `signal`, `fs`), `tokio-stream`, `futures-util`, `serde`, `serde_json`, `chrono` (`clock`, `std`), `toml`, `utoipa` + `utoipa-axum`; dev: `tempfile`, `reqwest`. Kein `serde_yaml` (nicht mehr gepflegt).
- [ ] Gerüst in I0001 (Anforderung durch den `planer`).

## Quellen / Bezuege

- [A00002 Programm localchatA](../Architektur/A00002-programm-localchata.md), [A00001 localchatA](../Architektur/A00001-localchata.md) (Makroarchitektur)
- Vision `Anforderungen/R00000-vision.md`, Leitplanken; kanbanD `Cargo.toml`, `src/main.rs`, `src/web/mod.rs` (Q04, nur gelesen)
