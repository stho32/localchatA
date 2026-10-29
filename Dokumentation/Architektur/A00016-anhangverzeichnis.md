---
id: A00016
name: Anhangverzeichnis
code: anhangverzeichnis
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/anhangverzeichnis.rs
verwandt: [A00009, A00008, A00004]
domaene: []
---

# Anhangverzeichnis

## Verantwortung

Findet zu einer Anhang-ID Ort, Größe, Art und Anzeigename der gebundenen Datei, solange ihr Tag in der Frist liegt.

## Schnittstellen

- `laden(gespraech, tag, saetze)` — Einträge aus den Metazeilen `+ anhang:` des Rohtexts eines Tages ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 6); dazu `bytes_je_tag(gespraech, tag)` aus der Größe der regulären Dateien in `anhaenge/<tag>/` im Blockmaß ⌈Bytes / 4.096⌉ × 4.096 (gezählt wird die Platte, nicht die Metazeile; [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 8).
- `eintragen(id, AnhangOrt)` — beim Binden, unter der Schreibsperre ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 5 e).
- `finden(id) -> Option<AnhangOrt { gespraech, tag, datei, bytes, art, masse, name }>` — für die Auslieferung ([ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Regel 1).
- `tag_entfernen(gespraech, tag) -> u64` — nimmt alle Einträge des Tages und liefert seine Anhangbytes für den Quote-Zähler ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 8).

## Abhaengigkeiten

Keine nach außen; Elternbaustein [Gespräche](A00006-gespraeche.md) hält es unter denselben Sperren wie den [Verlaufsindex](A00009-verlaufsindex.md).

## Entscheidungen

- [ADR-0046: Anhänge ablegen — anmelden, gestreamt in einen Eingang hochladen, beim Senden in den Tag der Nachricht binden, eine Metazeile mit Anhang-ID; ohne Grenze je Datei unter der Quote, eine Reserve bleibt dem Text](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) — Akzeptiert, 2026-10-29
- [ADR-0047: Anhänge ausliefern — eine Route ohne Sitzung über die Anhang-ID, die Art bestimmt der Server, inline nur Gefahrloses unter `CSP: sandbox`, alles andere als Download; Bildvorschau aus dem Original, Video und Audio im Player des Browsers mit Range](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) — Akzeptiert, 2026-10-29
- [ADR-0048: Eigene Nachrichten bearbeiten und löschen — Änderungssatz im Tageslog der Änderung, Fassung je Nachricht im Index, zwei Routen, Thema `aenderung`, der Löschsatz ist der Commit-Punkt für die Anhänge](../ADRs/0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md)

## Offene Fragen

- F6 (Rückgängig nach dem Löschen) und F7 (sehen Teilnehmer, wer gelöscht hat) an den Betreiber — [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Betreiber F1′“; bis zur Antwort je Option 1 (kein Papierkorb, Löschender nur im Protokoll).
- F8–F14 (Laufwerk: Frist für Dateien ohne Nachricht, Unterordner, Umbenennen, Verschieben, Ersetzen, Ordner hochladen, Archiv) — [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #9 Betreiber F1′“, Karte #45.

## Notizen / Quellen

- Datenhoheit: Gespräche (über dieses Kind) ist einziger Schreiber der Einträge; die HTTP-Schnittstelle liest nur über `finden`.
- Erster Test (Unit, rein): Rohtext eines Tages mit zwei Sätzen, einer mit zwei `+ anhang:`-Zeilen, einer mit einer kaputten (`+ anhang: zu kurz`) → zwei Einträge, ein Befund für `WARNING laden`; `finden` liefert Gespräch, Tag, Datei; `tag_entfernen` → `finden` = `None`, Rückgabe = Σ der übergebenen Dateigrößen. Ersetzte Provider: keine (die Dateigrößen übergibt Gespräche aus der Logablage).
- Speicher: ≈ 150 Bytes je Anhang — 10.000 Anhänge ≈ 1,5 MB (gerechnet).
- Ergänzung 2026-10-29 (Karte #9, entschieden): `finden` liefert `bytes` für `Content-Length` und Teilabrufe (Range, [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Regel 4); Einträge auch für Video- und Audioarten (B1). Erster Test zusätzlich: Anhang zu 1 Byte → `tag_entfernen` liefert 4.096.
- **Ergänzung 2026-10-29 (Karte #9 Betreiber F1, [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Betreiber F1“), Soll:** hält zusätzlich die **Verdrängungsfolge** aller gebundenen Anhänge über alle Gespräche, geordnet nach (Nachrichtenzeit, Sendefolge); jeder Eintrag trägt einen atomaren Zustand `gebunden | verdraengt | entfallen`, Übergänge nur aus `gebunden` per Vergleichen-und-Tauschen (wer gewinnt, zieht ab und räumt). Neu: `verdraengen(bedarf) -> Vec<Opfer>` unter der Schreibsperre (vom ältesten an, ≤ 1.024 Opfer je Abschnitt dank Zählmaß 64 KiB); `finden` liefert den Zustand (Auslieferung 410, [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #9 Betreiber F1“); `laden` erkennt eine Datei der Länge 0 bei `bytes` > 0 als `verdraengt`; `tag_entfernen` zieht nur gewonnene Einträge ab. Zählmaß `max(65.536, ⌈n / 4.096⌉ × 4.096)` statt Blockmaß — höchstens 327.680 Einträge bei 20 GiB (≈ 50 MB). Erster Test: ADR-0046 Ergänzung „Karte #9 Betreiber F1“, „Erste Tests“.
- **Ergänzung 2026-10-29 (Karte #9 Betreiber F1′), Soll — ersetzt die Verdrängungsfolge der Ergänzung „F1“:** Zustand je Eintrag `vorhanden | geloescht | entfallen`, Übergänge nur aus `vorhanden` per Vergleichen-und-Tauschen (wer gewinnt, zieht ab): `loeschen_beginnen(ids, sichtbar_fuer) -> (Vorschau | Opfer)` unter der Schreibsperre, `loeschen_abschliessen(ergebnisse)` nach dem Kürzen (Fehler → zurück auf `vorhanden`); `finden` liefert den Zustand (Abruf 410); `laden` erkennt Länge 0 bei `bytes` > 0 als `geloescht`; `tag_entfernen` zieht nur `vorhanden` ab. Je Eintrag zusätzlich Satzzeit, Sendefolge und Absender (für `fremde`, den Bezug zur Nachricht und Dateien ohne Nachricht). **Für Karte #9** genügt ein Schlüssel ID → Eintrag; für Karte #45 (Vorschlag) kommen je Gespräch drei geordnete Indizes (Name, Größe, Alter, je mit ID) und Summen (Anzahl, Bytes, jüngster) dazu — ≈ 120 Bytes mehr je Anhang, bei 327.680 Einträgen ≈ 40 MB; eine Seite kostet O(log n + k). Erster Test: [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Betreiber F1′“, „Erste Tests“; Sortierung (Karte #45): drei Anhänge `b.png` 3 KB t1, `A.pdf` 1 MB t2, `c.txt` 10 B t3 → `name auf` A, b, c; `groesse ab` A, b, c; `alter ab` c, A, b; Seite n = 2 mit Cursor → dritter Eintrag; Löschen von `b` zwischen zwei Seiten → keine Lücke, kein Doppel.
- **Ergänzung 2026-10-29 (Karte #48, I0032/I0033), Soll:** Löschen einer Nachricht tauscht ihre Anhänge `vorhanden → geloescht` wie das Laufwerk; scheitert das Kürzen, bleibt der Anhang `geloescht` (gezählt); beim Laden sind Anhänge einer gelöschten Nachricht `geloescht` gleich welcher Dateilänge (ADR-0048 Regel 9).
- **Ergänzung 2026-10-29 (Karte #9 Nacharbeit, [ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Anhang-Tagordner unabhängig von der Tagesdatei“), Soll:** hält je Gespräch die Liste der Anhangtage ohne Tagesdatei mit `W` je Tag (Waisen); Invariante `G = Σ z(vorhandene Einträge) + Σ W(tag)`.
