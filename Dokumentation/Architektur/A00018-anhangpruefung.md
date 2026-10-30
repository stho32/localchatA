---
id: A00018
name: Anhangprüfung
code: anhangpruefung
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/anhangpruefung.rs
verwandt: [A00017, A00004]
domaene: []
blackbox: reine Funktionen; Regeln und Tabelle vollständig in ADR-0046 Regel 4 und ADR-0047 Frage 3
---

# Anhangprüfung

## Verantwortung

Leitet aus Anzeigename und Inhalt einer hochgeladenen Datei ab, unter welchem Namen sie gespeichert und als welche Art sie ausgeliefert und im Verlauf gezeigt wird.

## Schnittstellen

- `name_pruefen(anzeigename) -> Result<GespeicherterName, Namensfehler>` — Namensregel ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 4).
- `Erkenner::neu()`, `Erkenner::stueck(&[u8])`, `Erkenner::ergebnis(bytes) -> Art { typ, disposition, masse, anzeige: Vorschau | Player(Video | Audio) | Zeile }` — Signaturen aus den ersten 64 KiB (Bildköpfe, `%PDF-`, MP4 `ftyp`, WebM EBML, `OggS`, MP3 `ID3` bzw. zwei Rahmenköpfe), Text-Prüfung (UTF-8 mit Übertrag über Stückgrenzen, kein NUL) über die ganze Datei, Text über 16 MiB als `attachment`; Tabelle in [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Frage 3.

## Abhaengigkeiten

keine (rein).

## Entscheidungen

- [ADR-0046: Anhänge ablegen — anmelden, gestreamt in einen Eingang hochladen, beim Senden in den Tag der Nachricht binden, eine Metazeile mit Anhang-ID; ohne Grenze je Datei unter der Quote, eine Reserve bleibt dem Text](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) — Akzeptiert, 2026-10-29
- [ADR-0047: Anhänge ausliefern — eine Route ohne Sitzung über die Anhang-ID, die Art bestimmt der Server, inline nur Gefahrloses unter `CSP: sandbox`, alles andere als Download; Bildvorschau aus dem Original, Video und Audio im Player des Browsers mit Range](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) — Akzeptiert, 2026-10-29

## Offene Fragen

- F3 an den Betreiber (Bericht Karte #9): Text über 16 MiB nur als Download — einverstanden? Bis zur Antwort gilt die Grenze aus ADR-0047.

## Notizen / Quellen

- Angenommen wird jede Datei (Betreiber B1); die Prüfung entscheidet nur Art, Disposition und Anzeige, nie Annahme oder Abweisung des Inhalts.
- Erster Test (Unit, rein): Tabelle in [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md), „Folgeentscheidungen“ (Bilder, PDF, MP4 Video/Audio/`qt  `, WebM/Matroska, Ogg Opus/Vorbis/Theora, MP3 `ID3` und Rahmenköpfe, Text, NUL, 16 MiB + 1 Byte Text); dieselbe Datei in Stücken zu 1, 3 und 65.536 Bytes → gleiches Ergebnis (Mehrbytezeichen über einer Grenze); ersetzte Provider: keine.
- Sicherheitskern: die Tabelle ist die erste Linie gegen aktive Inhalte (QS1); jede Änderung ist eine Ergänzung von ADR-0047 und ein Review-Punkt des `security-reviewer`. Fuzz der Köpfe: Angriffsmatrix M3 in ADR-0047.
- **Ergänzung 2026-10-29 (Karte #9 Gate, [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung Regel 4), Soll:** Maße nur aus der Struktur, die der Erkenner über die Längenkette erreicht (PNG `IHDR` als erster Chunk, JPEG erstes über Segmentlängen erreichtes `SOFn`, nie Suche nach Markerbytes; WebP aus dem Chunk, den der Kopf nennt); Breite oder Höhe 0 → keine Maße. Matrix M27.
- **Ergänzung 2026-10-29 (Karte #9 Betreiber F4, [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #9 Betreiber F4“), Soll:** zweite reine Aufgabe neben dem Erkenner — **Metadaten tilgen** an Ort und Stelle bei gleicher Länge über einen Port mit Positionslesen und -schreiben: JPEG (`APP1` Exif → minimales EXIF nur mit `Orientation`, XMP, `APP13`, `COM` genullt), PNG (`eXIf` minimal, Textchunks und `tIME` als `lcAx` genullt, CRC neu), WebP (`EXIF` minimal, `XMP ` → `JUNK`), GIF (Kommentar, XMP-Erweiterung), ISOBMFF jeder Marke (Ortsboxen in `moov`, Exif-/XMP-Items über `iloc`). Grenzen: Tiefe 16, `moov`/`meta` ≤ 64 MiB, 100.000 Blöcke, Längen mit Überlaufprüfung; Verstoß → `metadaten`. Keine neue Abhängigkeit. Erster Test: ADR-0047 Ergänzung „Karte #9 Betreiber F4“, „Erste Tests“.
- **Ergänzung 2026-10-29 (Karte #9 Betreiber F1′):** unberührt — Erkenner, Namensregel und Tilgen gelten für Anhänge von Nachrichten und für Dateien ohne Nachricht (Karte #45) gleich; 422 `metadaten` gibt den Anspruch frei wie bisher ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Betreiber F1′“ Regel 3).
- **Ergänzung 2026-10-29 (Karte #9 Security-Gate, [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #9 Security-Gate“ Regeln 1–5), Soll:** der Tilger liest über Blockköpfe per Position; kein Lese- oder Schreibpuffer über 1 MiB (Leseziele wie `keys`, `ilst`-Köpfe, `iinf`, `iloc`, TIFF-Kopf und IFD0 in Stücken ≤ 1 MiB, größer → 422 `metadaten`; Nullziele werden nur geschrieben); `moov`/`meta` ≤ 64 MiB nur als Längengrenze; gezählt werden nur gedeutete Blöcke (GIF-Unterblöcke und PNG `IDAT`/`fdAT` nie), sequentielle Ketten gepuffert; MPF-Index im JPEG wird gefolgt (Tiefe 1); XMP-`uuid` und `XMP_` werden zu `free` mit Nullen; Telemetrie (`gpmd`, `camm`) und von MPF nicht genannte Anhänge hinter dem ersten JPEG-Bild sind benannte Grenzen. Erster Test mit einer Attrappe des Positions-Ports, die jede Lese- und Schreiblänge festhält. Matrix M47, M48, M54.
- **Ergänzung 2026-10-29 (Karte #9 Nacharbeit, product-owner F3/F4, [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #9 Nacharbeit F3/F4“), Soll:** der Erkenner liest alle Marken der `ftyp`-Box und ordnet in fester Reihenfolge zu — `avif`/`avis` → `image/avif` inline, `heic`/`heix`/`heim`/`heis`/`hevc`/`hevx` → `image/heic` attachment, `mif1`/`msf1` → `image/heif` attachment, sonst MP4 wie bisher; für AVIF Maße aus dem `ispe` des Primärbilds (`pitm`, `ipma`), getauscht bei `irot` 90°/270°, Tiefe ≤ 4, Schleifen mit Fortschritt, Fehler ergibt kein Maß. Eine Datei ohne `ftyp` am Anfang ist kein ISOBMFF (MOV ohne `ftyp`: nicht getilgt, `application/octet-stream`, benannte Grenze).
- **Ergänzung 2026-10-30 (Karte #9 Security-Review B1, [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #9 Security-Review B1“), Soll:** der Tilger arbeitet je Hochladung höchstens 2 × Länge + 8 MiB (gelesene und geschriebene Bytes, gezählt im Positions-Port; darüber 422 `metadaten`); Verweisziele (HEIF-Items über `iinf`/`iloc`, MPF-Bilder) sind eindeutig und überschneidungsfrei, sonst 422, geprüft vor dem ersten Schreiben; kein gepufferter Leser liest über den Bereich hinaus, den sein Verweis nennt. Ist: Budget und Eindeutigkeit fehlen (geprobt: 2.000 Verweise auf ein Exif-Item 7,5 s).
- **Ergänzung 2026-10-30 (Karte #9 Security-Review B2, [ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #9 Security-Review B2“), Soll:** der Tilger arbeitet mit **Erlaubnislisten** je Format — bleiben nur Struktur-, Farb-, Orientierungs-, HDR- und Animationsblöcke, jeder andere Block wird an Ort und Stelle unkenntlich (JPEG-`APPn`, PNG-Chunks, WebP-Chunks, GIF-Erweiterungen, ISOBMFF-Boxen oben, in `moov`, `udta`, `meta`, HEIF-Items, `udes`); Videos verlieren alle Nutzerdaten; `keys` ist kein Leseziel mehr. Ist: Sperrliste, Orte in `APP3`/`APP11`, `caBX`, `uuid`, `ilst/©xyz`, `trak/udta` bleiben stehen (geprobt). Offen: Frage an den `product-owner` zu Nutzerdaten in Videos und C2PA.
