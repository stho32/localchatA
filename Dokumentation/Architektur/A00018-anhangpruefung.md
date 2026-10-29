---
id: A00018
name: Anhangprüfung
code: anhangpruefung
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: vermutung
quellen:
  - src/gespraeche/anhangpruefung.rs
verwandt: [A00017, A00004]
domaene: []
blackbox: reine Funktionen; Regeln und Tabelle vollständig in ADR-0046 Regel 3 und ADR-0047 Frage 3
---

# Anhangprüfung

## Verantwortung

Leitet aus Anzeigename und Inhalt einer hochgeladenen Datei ab, unter welchem Namen sie gespeichert und als welche Art sie ausgeliefert wird.

## Schnittstellen

- `name_pruefen(anzeigename) -> Result<GespeicherterName, Namensfehler>` — Namensregel ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 3).
- `Erkenner::neu()`, `Erkenner::stueck(&[u8])`, `Erkenner::ergebnis() -> Art { typ, disposition, masse, vorschau }` — Art aus den ersten 64 KiB (Bildköpfe, `%PDF-`), Text-Prüfung (UTF-8, kein NUL) über die ganze Datei; Tabelle in [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Frage 3.

## Abhaengigkeiten

keine (rein).

## Entscheidungen

- [ADR-0046: Anhänge ablegen — je Datei hochgeladen in einen Eingang, beim Senden in den Tag der Nachricht gebunden, eine Metazeile mit Anhang-ID; Bytes zählen an der Quote und an einem eigenen Konto](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) — Vorgeschlagen, 2026-10-29
- [ADR-0047: Anhänge ausliefern — eine Route ohne Sitzung über die Anhang-ID, die Art bestimmt der Server, inline nur Gefahrloses unter `CSP: sandbox`, alles andere als Download; Bildvorschau aus dem Original](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) — Vorgeschlagen, 2026-10-29

## Offene Fragen

- Welche Arten überhaupt angenommen werden (alle oder Erlaubnisliste) — Frage B1 an den Betreiber (Bericht Karte #9); bis zur Freigabe `herkunft: vermutung`.

## Notizen / Quellen

- Erster Test (Unit, rein): Tabelle in [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md), „Folgeentscheidungen“; ersetzte Provider: keine.
- Sicherheitskern: die Tabelle ist die erste Linie gegen aktive Inhalte (QS1); jede Änderung ist eine Ergänzung von ADR-0047 und ein Review-Punkt des `security-reviewer`. Fuzz-Probe der Bildköpfe vor dem Bau (Angriffsmatrix).
