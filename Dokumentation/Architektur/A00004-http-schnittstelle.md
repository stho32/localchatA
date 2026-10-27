---
id: A00004
name: HTTP-Schnittstelle
code: http
ebene: Komponente
eltern: A00002
repo: localchatA
status: Aktiv
herkunft: dialog
quellen:
  - src/http/
verwandt: [A00005]
domaene: []
blackbox: dünne Integration an der Systemgrenze; verfeinert wird, wenn die API mit I0014/I0015 wächst
---

# HTTP-Schnittstelle

## Verantwortung

Übersetzt jede HTTP-Anfrage an der Systemgrenze in genau einen Aufruf der Fachbausteine samt Identitätsnachweis und Fehlerantwort.

## Schnittstellen

- Eingehend: Router (axum) für `/`, `/web/*` (eingebettete Dateien der [Weboberfläche](A00005-weboberflaeche.md)), `/api/*` JSON, `/api/ereignisse` (SSE), `/api/warten` (Long-Poll), `/api/gesundheit`, `/api/openapi.json`.
- Identität: Header `X-LocalchatA-Sitzung: <token>` (Browser zusätzlich als Query-Parameter für `EventSource`, das keine Header setzen kann); Peer-Adresse über `ConnectInfo<SocketAddr>` ([ADR-0008](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md)).
- Ausgehend: [Gespräche](A00006-gespraeche.md) (blockierende Arbeit über `spawn_blocking`), [Anwesenheit](A00010-anwesenheit.md), [Live-Kanal](A00011-live-kanal.md).

## Abhaengigkeiten

[Gespräche](A00006-gespraeche.md), [Anwesenheit](A00010-anwesenheit.md), [Live-Kanal](A00011-live-kanal.md), [Weboberfläche](A00005-weboberflaeche.md) (eingebettet per `include_bytes!`).

## Entscheidungen

- [ADR-0002: Oberfläche ohne Build-Schritt in das Programm eingebettet](../ADRs/0002-oberflaeche-ohne-build-schritt-eingebettet.md) — Akzeptiert, 2026-10-27
- [ADR-0007: Live-Kanal — SSE für Browser, Long-Poll für Agenten, ein Ereignisbus](../ADRs/0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md) — Vorgeschlagen, 2026-10-27
- [ADR-0008: Identität — Herkunft aus der Peer-Adresse, Nachweis über Sitzungs-Token](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md) — Vorgeschlagen, 2026-10-27
- [ADR-0010: Eigene Annahmeschleife mit hyper http1 statt `axum::serve`](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md) — Akzeptiert, 2026-10-27

## Offene Fragen

## Notizen / Quellen

- Grenzregel (`_aspekte.md`): ein Handler ruft nie Logablage oder Index direkt, nur [Gespräche](A00006-gespraeche.md) — prüfbar daran, dass `src/http/` keine `use crate::gespraeche::logablage`/`verlaufsindex` enthält (Review-Punkt, später Architekturtest).
- Erster Test (Integration, `tests/`): echtes Programm auf Port 0; `GET /` liefert `text/html` mit `<title>localchatA</title>`, `GET /web/gibtsnicht.js` → 404; `GET /api/gesundheit` → `{"status":"bereit", …}`.
- **Ist 2026-10-27 (nach I0001, Karte #2 Nachzug):** Routen `/`, `/web/{datei}`, `/api/gesundheit` (`src/http/mod.rs`); Gesundheit meldet die Kennzahlen noch fest mit 0. `ConnectInfo<SocketAddr>` ist in Handlern **nicht** verfügbar, weil das Programm über eine eigene Annahmeschleife ausliefert ([ADR-0010](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md)); vor der ersten Route, die die Herkunft braucht (I0004), setzt die Schleife sie je Request ([ADR-0008](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md), Ergänzung). Zweiter Server-Stack: `testserver` (nur `#[cfg(test)]`) fährt `axum::serve` — für Routing und Serialisierung genügt er, für Herkunft, Ströme und Grenzen nicht.
- **Grenzregel für schreibende Routen (ab 2026-10-27):** eine Route, die in den Ordner schreibt, entsteht erst mit Sitzungsprüfung ([ADR-0008](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md)) und der Host-/Origin-Prüfung samt Sicherheitsheadern (Karte #18, `AI-Team/gelerntes.md` #1) — keine vorläufige Schreibroute mit Absender im Body.
