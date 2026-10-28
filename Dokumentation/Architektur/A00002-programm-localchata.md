---
id: A00002
name: Programm localchatA
code: localchatA
ebene: Container
eltern: A00001
repo: localchatA
status: Aktiv
herkunft: dialog
quellen:
  - Cargo.toml
  - src/
  - web/
ausgenommen:
  - src/lib.rs
  - src/fehler.rs
  - src/uhr.rs
verwandt: []
domaene: []
---

# Programm localchatA

## Verantwortung

Liefert aus einem einzigen Prozess Weboberfläche, JSON-API und Live-Kanal für genau einen Ordner aus.

## Schnittstellen

- **Eingehend:** Aufruf `localchatA [<ordner>] [--port <n>] [--host <adresse>] [--kein-browser]`; Standard-Host `0.0.0.0` (LAN), Standard-Port 7430, belegt → nächster freier (höchstens 50 Versuche), `--port 0` wählt das System. Startzeile auf stdout: `localchatA läuft für <ordner> unter http://<host>:<port>/` — Tests lesen die Adresse daraus.
- **Eingehend (HTTP):** `/` und `/web/*` (Oberfläche), `/api/*` (JSON), `/api/ereignisse` (SSE), `/api/warten` (Long-Poll), `/api/gesundheit` (Health-Check).
- **Ausgehend:** Dateisystem des Ordners (nur über [Gespräche](A00006-gespraeche.md)).
- Exit-Codes: 0 geordnet beendet oder `--hilfe`, 1 Ordner/Port/Sperre scheitert, 2 falscher Aufruf. SIGTERM und Strg+C beenden geordnet; offene Ströme und Warte-Aufrufe schließen.

## Abhaengigkeiten

Kinder:

- [Start](A00003-start.md) (aktiv)
- [HTTP-Schnittstelle](A00004-http-schnittstelle.md) (aktiv)
- [Weboberfläche](A00005-weboberflaeche.md) (aktiv)
- [Gespräche](A00006-gespraeche.md) (geplant)
- [Anwesenheit](A00010-anwesenheit.md) (geplant)
- [Live-Kanal](A00011-live-kanal.md) (geplant)
- [Schreibkonto](A00014-schreibkonto.md) (geplant, I0009)

Crates (ADR-0001): `axum` 0.8 (mit `multipart`), `tokio`, `tokio-stream`, `futures-util`, `serde`, `serde_json`, `chrono` (Lokalzeit mit Versatz), `toml` (Einstellungen je Gespräch), `utoipa`/`utoipa-axum` (API-Beschreibung für Agenten); Test: `tempfile`, `reqwest`.

## Entscheidungen

- [ADR-0001: Ein Programm in Rust mit axum und tokio, Bibliothek plus Binärprogramm](../ADRs/0001-ein-programm-rust-axum-tokio.md) — Akzeptiert, 2026-10-27
- [ADR-0009: Teststrategie, Gate und Health-Check](../ADRs/0009-teststrategie-gate-und-health-check.md) — Akzeptiert, 2026-10-27
- [ADR-0010: Eigene Annahmeschleife mit hyper http1 statt `axum::serve`](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md) — Akzeptiert, 2026-10-27
- [ADR-0036: Schreibkonto gegen Flut — Token-Eimer je Kennung und je Herkunft](../ADRs/0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md) — Akzeptiert, 2026-10-28

## Offene Fragen

- Standard-Port 7430: genügt er dem Betreiber, oder soll der Port fest sein (kein Ausweichen), damit Agenten auf anderen Rechnern ihn kennen? Empfehlung: Ausweichen wie kanbanD, Startzeile nennt den echten Port.

## Notizen / Quellen

- Entworfen am 2026-10-27 per `/architektur brainstorming --still`.
- `ausgenommen`: `src/lib.rs` (nur Moduldeklarationen), `src/fehler.rs` (Aspekt [Fehlerbehandlung](Aspekte/X00004-fehlerbehandlung.md)), `src/uhr.rs` (Aspekt [Zeit](Aspekte/X00007-zeit.md)) — querschnittlich, kein eigener Baustein.
- Vorbild kanbanD (Q04): `src/main.rs` (Start, geordnetes Beenden), `src/start/` (Aufruf, freier Port), `src/web/mod.rs` (Router, `spawn_blocking`-Muster `blockierend`).
- **Stand 2026-10-27 (Ist nach I0001, Karte #2 Nachzug):** gebaut sind Start und HTTP-Schnittstelle mit `/`, `/web/{datei}`, `/api/gesundheit`; Auslieferung über eine eigene Annahmeschleife statt `axum::serve` ([ADR-0010](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md)): Kopf-/Leerlaufgrenze 10 s, höchstens 64 Verbindungen je Gegenstelle, gesamt `min(512, weiche Deskriptorgrenze / 2)`, beim Ende Frist 5 s, nur HTTP/1.1 ohne Upgrades. Direkte Abhängigkeiten heute: `axum`, `tokio`, `serde`, `serde_json`, `hyper`, `hyper-util`, unter Unix `rustix`; die übrigen Crates der Liste oben kommen mit ihrer Interaction (Nachtrag in [ADR-0001](../ADRs/0001-ein-programm-rust-axum-tokio.md)). `src/fehler.rs` und `src/uhr.rs` (unter `ausgenommen`) existieren noch nicht — sie entstehen mit I0002. Gespräche, Anwesenheit und Live-Kanal sind geplant.
