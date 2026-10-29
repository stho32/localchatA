---
id: A00016
name: Anhangverzeichnis
code: anhangverzeichnis
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: vermutung
quellen:
  - src/gespraeche/anhangverzeichnis.rs
verwandt: [A00009, A00008, A00004]
domaene: []
---

# Anhangverzeichnis

## Verantwortung

Findet zu einer Anhang-ID Ort, Größe, Art und Anzeigename der gebundenen Datei, solange ihr Tag in der Frist liegt.

## Schnittstellen

- `laden(gespraech, tag, saetze)` — Einträge aus den Metazeilen `+ anhang:` des Rohtexts eines Tages ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 6); dazu `bytes_je_tag(gespraech, tag)` aus der Größe der regulären Dateien in `anhaenge/<tag>/` (gezählt wird die Platte, nicht die Metazeile).
- `eintragen(id, AnhangOrt)` — beim Binden, unter der Schreibsperre ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 5 e).
- `finden(id) -> Option<AnhangOrt { gespraech, tag, datei, bytes, art, masse, name }>` — für die Auslieferung ([ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Regel 1).
- `tag_entfernen(gespraech, tag) -> u64` — nimmt alle Einträge des Tages und liefert seine Anhangbytes für den Quote-Zähler ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 8).

## Abhaengigkeiten

Keine nach außen; Elternbaustein [Gespräche](A00006-gespraeche.md) hält es unter denselben Sperren wie den [Verlaufsindex](A00009-verlaufsindex.md).

## Entscheidungen

- [ADR-0046: Anhänge ablegen — je Datei hochgeladen in einen Eingang, beim Senden in den Tag der Nachricht gebunden, eine Metazeile mit Anhang-ID; Bytes zählen an der Quote und an einem eigenen Konto](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) — Vorgeschlagen, 2026-10-29
- [ADR-0047: Anhänge ausliefern — eine Route ohne Sitzung über die Anhang-ID, die Art bestimmt der Server, inline nur Gefahrloses unter `CSP: sandbox`, alles andere als Download; Bildvorschau aus dem Original](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) — Vorgeschlagen, 2026-10-29

## Offene Fragen

- Wird der Schnitt so gebaut? Hängt an der Freigabe von ADR-0046/ADR-0047 durch den Betreiber (Fragen B1–B5, Bericht Karte #9); bis dahin `herkunft: vermutung`.

## Notizen / Quellen

- Datenhoheit: Gespräche (über dieses Kind) ist einziger Schreiber der Einträge; die HTTP-Schnittstelle liest nur über `finden`.
- Erster Test (Unit, rein): Rohtext eines Tages mit zwei Sätzen, einer mit zwei `+ anhang:`-Zeilen, einer mit einer kaputten (`+ anhang: zu kurz`) → zwei Einträge, ein Befund für `WARNING laden`; `finden` liefert Gespräch, Tag, Datei; `tag_entfernen` → `finden` = `None`, Rückgabe = Σ der übergebenen Dateigrößen. Ersetzte Provider: keine (die Dateigrößen übergibt Gespräche aus der Logablage).
- Speicher: ≈ 150 Bytes je Anhang — 10.000 Anhänge ≈ 1,5 MB (gerechnet).
