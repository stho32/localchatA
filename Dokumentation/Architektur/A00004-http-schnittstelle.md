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
- Grenzschutz (äußerste Schicht `grenzschutz`, vor jeder Route): Host-Erlaubnisliste, Origin-/`Sec-Fetch-Site`-Prüfung für jede nicht lesende Methode, Sicherheitsheader auf jeder Antwort ([ADR-0014](../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md)).
- Identität: Header `X-LocalchatA-Sitzung: <token>`, geprüft vom Extraktor `Teilnehmer` gegen [Anwesenheit](A00010-anwesenheit.md); Herkunft aus `ConnectInfo<SocketAddr>`, das die Annahmeschleife kanonisch setzt ([ADR-0012](../ADRs/0012-identitaet-und-sitzung.md)). Für `EventSource` (ab I0005) entscheidet die Anforderung zwischen Query-Parameter und `fetch`-Strom mit Header.
- Ab I0004: `POST /api/beitreten` → 201 | 200 (Wiederkehr) | 409 Name vergeben (mit Vorschlag) | 422 Namensregel ([ADR-0013](../ADRs/0013-namensregel-als-erlaubnisliste.md)) | 429 zu viele Sitzungen; `GET /api/ich` → 200 | 401.
- Ausgehend: [Gespräche](A00006-gespraeche.md) (blockierende Arbeit über `spawn_blocking`), [Anwesenheit](A00010-anwesenheit.md), [Live-Kanal](A00011-live-kanal.md).

## Abhaengigkeiten

[Gespräche](A00006-gespraeche.md), [Anwesenheit](A00010-anwesenheit.md), [Live-Kanal](A00011-live-kanal.md), [Weboberfläche](A00005-weboberflaeche.md) (eingebettet per `include_bytes!`).

## Entscheidungen

- [ADR-0002: Oberfläche ohne Build-Schritt in das Programm eingebettet](../ADRs/0002-oberflaeche-ohne-build-schritt-eingebettet.md) — Akzeptiert, 2026-10-27
- [ADR-0007: Live-Kanal — SSE für Browser, Long-Poll für Agenten, ein Ereignisbus](../ADRs/0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md) — Vorgeschlagen, 2026-10-27
- [ADR-0008: Identität — Herkunft aus der Peer-Adresse, Nachweis über Sitzungs-Token](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md) — Abgelöst durch ADR-0012, 2026-10-27
- [ADR-0010: Eigene Annahmeschleife mit hyper http1 statt `axum::serve`](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md) — Akzeptiert, 2026-10-27
- [ADR-0012: Identität und Sitzung — Herkunft aus der Gegenstelle, Sitzungs-Token im Speicher, eine Sitzung je Browser](../ADRs/0012-identitaet-und-sitzung.md) — Akzeptiert, 2026-10-27
- [ADR-0013: Namensregel als Erlaubnisliste — `[a-z0-9-]`, 1 bis 32 Zeichen, abweisen statt umformen](../ADRs/0013-namensregel-als-erlaubnisliste.md) — Akzeptiert, 2026-10-27
- [ADR-0014: Grenzschutz im LAN — Host-Erlaubnisliste, Origin-Prüfung für Schreibendes, Sicherheitsheader auf jeder Antwort](../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md) — Akzeptiert, 2026-10-27

## Offene Fragen

## Notizen / Quellen

- Grenzregel (`_aspekte.md`): ein Handler ruft nie Logablage oder Index direkt, nur [Gespräche](A00006-gespraeche.md) — prüfbar daran, dass `src/http/` keine `use crate::gespraeche::logablage`/`verlaufsindex` enthält (Review-Punkt, später Architekturtest).
- Erster Test (Integration, `tests/`): echtes Programm auf Port 0; `GET /` liefert `text/html` mit `<title>localchatA</title>`, `GET /web/gibtsnicht.js` → 404; `GET /api/gesundheit` → `{"status":"bereit", …}`.
- **Ist 2026-10-27 (nach I0001, Karte #2 Nachzug):** Routen `/`, `/web/{datei}`, `/api/gesundheit` (`src/http/mod.rs`); Gesundheit meldet die Kennzahlen noch fest mit 0. `ConnectInfo<SocketAddr>` ist in Handlern **nicht** verfügbar, weil das Programm über eine eigene Annahmeschleife ausliefert ([ADR-0010](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md)); vor der ersten Route, die die Herkunft braucht (I0004), setzt die Schleife sie je Request ([ADR-0008](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md), Ergänzung). Zweiter Server-Stack: `testserver` (nur `#[cfg(test)]`) fährt `axum::serve` — für Routing und Serialisierung genügt er, für Herkunft, Ströme und Grenzen nicht.
- **Grenzregel für schreibende Routen (ab 2026-10-27):** eine Route, die in den Ordner schreibt, entsteht erst mit Sitzungsprüfung ([ADR-0008](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md)) und der Host-/Origin-Prüfung samt Sicherheitsheadern (Karte #18, `AI-Team/gelerntes.md` #1) — keine vorläufige Schreibroute mit Absender im Body.
- **Ergänzung 2026-10-27 (Karte #3, I0004):** Die Grenzregel für schreibende Routen gilt ab jetzt für jede Route mit Wirkung, auch ohne Schreiben in den Ordner (`beitreten` belegt einen Namen): sie entsteht nur unter `grenzschutz` ([ADR-0014](../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md)); Sitzungsprüfung nach [ADR-0012](../ADRs/0012-identitaet-und-sitzung.md) (ersetzt ADR-0008) verlangt jede Route außer `beitreten`, `gesundheit` und der Oberfläche. `GET`/`HEAD` sind wirkungsfrei. Der Testserver läuft ab I0004 über `ausliefern` (ein Stack), weil `beitreten` die Herkunft braucht. Erster Test des Grenzschutzes (Integration, echtes Programm): `GET /` mit `Host: evil.example` → 403 mit allen Sicherheitsheadern; `POST /api/beitreten` mit `Origin: http://evil.example` → 403, ohne Origin → 201.
