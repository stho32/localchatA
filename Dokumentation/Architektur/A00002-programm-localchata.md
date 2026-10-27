---
id: A00002
name: Programm localchatA
code: localchatA
ebene: Container
eltern: A00001
repo: localchatA
status: Geplant
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

Kinder (alle geplant):

- [Start](A00003-start.md) (geplant)
- [HTTP-Schnittstelle](A00004-http-schnittstelle.md) (geplant)
- [Weboberfläche](A00005-weboberflaeche.md) (geplant)
- [Gespräche](A00006-gespraeche.md) (geplant)
- [Anwesenheit](A00010-anwesenheit.md) (geplant)
- [Live-Kanal](A00011-live-kanal.md) (geplant)

Crates (ADR-0001): `axum` 0.8 (mit `multipart`), `tokio`, `tokio-stream`, `futures-util`, `serde`, `serde_json`, `chrono` (Lokalzeit mit Versatz), `toml` (Einstellungen je Gespräch), `utoipa`/`utoipa-axum` (API-Beschreibung für Agenten); Test: `tempfile`, `reqwest`.

## Entscheidungen

- [ADR-0001: Ein Programm in Rust mit axum und tokio, Bibliothek plus Binärprogramm](../ADRs/0001-ein-programm-rust-axum-tokio.md) — Akzeptiert, 2026-10-27
- [ADR-0009: Teststrategie, Gate und Health-Check](../ADRs/0009-teststrategie-gate-und-health-check.md) — Akzeptiert, 2026-10-27

## Offene Fragen

- Standard-Port 7430: genügt er dem Betreiber, oder soll der Port fest sein (kein Ausweichen), damit Agenten auf anderen Rechnern ihn kennen? Empfehlung: Ausweichen wie kanbanD, Startzeile nennt den echten Port.

## Notizen / Quellen

- Entworfen am 2026-10-27 per `/architektur brainstorming --still`.
- `ausgenommen`: `src/lib.rs` (nur Moduldeklarationen), `src/fehler.rs` (Aspekt [Fehlerbehandlung](Aspekte/X00004-fehlerbehandlung.md)), `src/uhr.rs` (Aspekt [Zeit](Aspekte/X00007-zeit.md)) — querschnittlich, kein eigener Baustein.
- Vorbild kanbanD (Q04): `src/main.rs` (Start, geordnetes Beenden), `src/start/` (Aufruf, freier Port), `src/web/mod.rs` (Router, `spawn_blocking`-Muster `blockierend`).
