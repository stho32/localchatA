---
id: R00044
status: Neu
datum: 2026-10-30
---

# R00044: Tilger gehärtet — Umbau ohne Verhaltensänderung, benannte Abbruchgründe, `exif_ort`, Rest, Kapitel, Träger unter `trak`/`traf`, Feldbreite und `meta`

## Beschreibung

Härtung von R00029 F0115 („Ortsangaben beim Hochladen tilgen“) nach dem Wiederholungs-Review von Karte #9 (code-reviewer S1–S4 und Optional, security-reviewer N1–N3). Das Verhalten steht; es hat zwei Mängel der Wartbarkeit und fünf Lücken in der Zusage „Videos und Bilder verlieren alle Nutzerdaten“ (PO Zeile 130):

- **Umbau ohne Verhaltensänderung** (F0150): eine Datei mit 2.357 Zeilen, neun Ebenen-Funktionen mit je eigenem Platzhalter-Arm (ein fehlender Arm in `iprp` war Befund B1). Künftig drei Dateien, eine Funktion „Kinder einer Ebene“, die Platzhalter-Regel an einer Stelle, Golden-Test zuerst.
- **Abbruchgründe** (F0151): der `grund` im 422 `metadaten` heißt je Ursache anders; 21-mal „Längen widersprechen sich“ und 3-mal „Exif-Item nicht als Bytebereich der Datei“ werden zu 12 Wortlauten. **Das ist eine Änderung des API-Felds `grund`** (Tabelle unten).
- **Protokollfeld `exif_ort`** (F0152): das Feld `ort` sagte bei Videos Falsches („nein“ trotz getilgtem Ort); es heißt `exif_ort` und erscheint nur, wenn ein Exif gelesen wurde.
- **Rest unter 8 Bytes** (F0153), **Kapitelnamen `chpl`** (F0154), **Träger unter `trak` und `traf`** (F0155), **Feldbreite und mehrdeutiges `meta`** (F0156): Stellen, an denen freier Text die Tilgung überlebte oder ein Leser anders deutete als der Tilger.

Zahlt ein auf: [Vision](R00000-vision.md) — Anhänge, ohne dass ein Ort mitreist (Leitplanke „kein Ort im Anhang“, QS7), und die Betreiberin liest im Protokoll nur Wahres.

Herkunft: Karte #72 — code-reviewer #9 (Wiederholung Teil C/D) S1–S4 und Optional O1–O6, security-reviewer #9 (Wiederholung Teil D) N1–N3; entschieden vom architekt in [ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #72“ (Bausteine B72-0 bis B72-9, Proben wörtlich), [ADR-0042](../Dokumentation/ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Ergänzung „Karte #72“ (`exif_ort`) und [A00018](../Dokumentation/Architektur/A00018-anhangpruefung.md) Soll-Zuschnitt; product-owner Zeile 130 (`AI-Team/po-entscheidungen.md`: „alle Nutzerdaten“, eine Regel für Bilder und Videos) angewandt.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0010): **F0150** Tilger-Umbau ohne Verhaltensänderung (B0674–B0677), **F0151** Jede Abweisung der Metadaten nennt ihre Ursache (B0678), **F0152** Das Protokoll meldet den Ort nur, wo ein Exif gelesen wurde (B0679), **F0153** Der Rest unter 8 Bytes wird genullt (B0680), **F0154** Kapitelnamen fallen (B0681), **F0155** Nutzerdaten unter `trak` und `traf` werden unkenntlich (B0682), **F0156** Mehrdeutige `meta` und ungültige Feldbreiten werden abgewiesen (B0683).

**Eigene Anforderung statt Nachtrag zu R00029** (planer): R00029 ist `Neu`, 515 Zeilen lang und wurde in zwei Tagen viermal über den Nachtragsweg angeglichen; dieses Dokument bündelt eine Karte mit eigener Abnahme (Golden-Datei, Wortlauttabelle, Wiedergabe-Kontrollen, Wiederholungs-Review) und bricht an drei Stellen eine Zusage von R00029 (`chpl` bleibt; Wortlaut „Längen widersprechen sich“; Feld `ort`). Ein Nachtrag hätte diese Brüche in einem Dokument vergraben, das als Ganzes die Zusage von F0115 trägt — Muster R00038 → R00042, R00040 → R00043. R00029 bekommt an den überholten Stellen nur einen Verweis (Zeilen 90, 94, 95, 216, 219 und Notizen), keine Neufassung.

### Domänenbezug

Kein Glossar im Projekt. Begriffe wie in R00029: Anhang, Tilger, Erlaubnisliste, Box (ISOBMFF), unkenntlich (Typ `free`, Inhalt genullt), Platzhalter, Positionsport, Budget, `grund`, Episode „Metadaten nicht tilgbar“.

## Geschäftlicher Nutzen

Wer ein Urlaubsvideo mit GPS hochlädt, will, dass der Ort nicht mitreist; wer es nur halb tut, bekommt Vertrauen ohne Grundlage. Jede der sieben Lücken war ein Ort, an dem freier Text in einer Datei lag, die der Tilger für „getilgt“ hielt (Kapitel „Hamburg Hafen“, sieben Bytes Rest, ein `meco` unter `trak`). Die Betreiberin, die im Protokoll liest, warum ein Upload abgewiesen wurde, sieht heute bei 21 verschiedenen Ursachen denselben Satz; ein Agent, der den `grund` anzeigt, kann ihm nicht entnehmen, was an der Datei falsch ist. Und die Wartbarkeit: ein neuer Container-Typ darf keine Ebene mehr „vergessen“ können — das war der Befund B1 des ersten Reviews.

## Funktionale Anforderungen

### F0150 Tilger-Umbau ohne Verhaltensänderung ([ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #72“ Baustein 0, S2, S3, O2, O4)

- **Reihenfolge**: erst der Golden-Test (B0674), allein committet und grün gegen den heutigen Code; dann Verschieben (B0675, `git diff -M` zeigt es), dann `ebene` (B0676), dann `ausdehnungen` (B0677). Ein geänderter Test oder eine geänderte `golden.txt` in einem Umbau-Commit ist ein Befund.
- **Drei Dateien**: `kisten.rs` (Grundlagen und `ebene`), `isobmff.rs` (Video- und Audio-Familie), `heif.rs` (Bildsammlungen); Pfeile nur `isobmff → heif → kisten` und `isobmff → kisten`.
- **Eine Funktion `ebene`** liest die Kinder; der Platzhalter-Fall steht dort, einmal; jede Ebene liefert nur ihre `wahl`. Keine Liste enthält einen Typ, den ihr `match` vorher abfängt.
- **Hilfen einmal**: `enthaelt`, `kiste`, `ftyp`, `tilgen` je Baum an genau einer Stelle (`anhangpruefung/testhilfe.rs`, `tests/gemeinsam/dateien.rs`).

### F0151 Jede Abweisung der Metadaten nennt ihre Ursache ([ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #72“ S4)

- **`enum Grund`** (`tilger/grund.rs`), `Abbruch::Metadaten(Grund)`, `Grund::text` mit einem `match` ohne Platzhalterarm, Liste `Grund::ALLE`. Der Wortlaut ist der Wert von `grund` im 422 `{regel: "metadaten", grund}` und von `grund=` in der Episode „Metadaten nicht tilgbar“; `regel: "metadaten"` bleibt die Maschinenkennung.
- **Wortlaute, die schon eine einzige Ursache nennen, bleiben wörtlich** (API-Vertrag): „Leseziel über 1 MiB“, „Schreibziel über 1 MiB“, „über 100.000 Blöcke“, „moov über 64 MiB“, „meta über 64 MiB“, „cmov nicht prüfbar“, „Zeitfelder unlesbar“, „HEIF-Kind doppelt“, „Verweise doppelt“, „Verweise überschneiden sich“, „Ziel ohne Länge“, „Arbeit über dem Budget“, „MPF-Eintrag nicht ganz in der Datei“, „MPF ohne TIFF-Kopf“, „MPF-Index unlesbar“, „JPEG ohne FF D8 FF“, „JPEG-Segment ohne Marker“, „JPEG-Marker an falscher Stelle“, „PNG ohne Signatur“, „WebP ohne RIFF-Kopf“, „GIF ohne Kopf“, „GIF-Block unbekannt“ (22).
- **Geändert werden nur die beiden Sammelwortlaute** — das ist der Vertragsbruch des Felds `grund`:

| bisher (Stellen) | neu, je Ursache | Ort heute |
|---|---|---|
| „Längen widersprechen sich“ (21) | „Zugriff außerhalb der Datei“ | `Port::lesen`, `schreiben`, `nullen`, Puffer-`nullen`, `laden` (5) |
| | „JPEG-Segmentlänge ungültig“ | `bild::jpeg` (Länge < 2 oder über das Ende) |
| | „PNG-Chunk ragt über die Datei“ | `bild::png` |
| | „WebP-Länge über die Datei“ / „WebP-Chunk ragt über seinen Rahmen“ | `bild::webp` (RIFF-Länge) / `webp_chunk` |
| | „Box kürzer als ihr Kopf“ / „Box länger als ihre Elternbox“ | `kiste` (zwei Stellen) |
| | „Box ohne Fortschritt“ | `kinder`, `isobmff` (Ende ≤ Anfang) |
| | „Feld außerhalb seiner Box“ | `lies_u32`, `lies_zahl`, `items` (zwei), `ausdehnungen` |
| | „iloc-Ausdehnung überläuft“ | `ausdehnungen` (`checked_add`, zwei) |
| „Exif-Item nicht als Bytebereich der Datei“ (3) | „Ziel-Item ohne Ausdehnung in iloc“ / „Ziel-Item außerhalb der Datei“ | `ziele` — Ziele sind auch `mime`, `uri `, unbekannte Items, das Wort „Exif“ stimmte nicht |

  Dazu kommen mit F0156 zwei Wortlaute: „Feldbreite ungültig“ und „meta nicht eindeutig“ (dann 36 Varianten).
- Wer `grund` liest, findet in `web/` keinen Leser (die Oberfläche lädt noch nicht hoch); `e2e/` prüft der Entwickler vor dem Umbau.

### F0152 Das Protokoll meldet den Ort nur, wo ein Exif gelesen wurde ([ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #72“ S1, ADR-0042)

- Das Feld `exif_ort` (`ja`/`nein`) der Zeile `DEBUG anhang Metadaten getilgt` steht nur, wenn ein Exif gelesen wurde (JPEG, PNG, WebP, HEIF-Exif-Item): `ja` = GPS-Verweis im Exif, `nein` = Exif ohne GPS. Bei Videos, Audio und Bildern ohne Exif fehlt es. `ort` kommt in dieser Zeile nicht mehr vor.
- Verworfen: `ort` für `©xyz`/`loci` behalten (bleibt falsch bei QuickTime-Schlüsseln und XMP), `keys` wieder lesen (Fuzz-Fläche für ein Protokollfeld), Feld streichen (bei Bildern misst es etwas Wahres).

### F0153 Der Rest unter 8 Bytes wird genullt ([ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #72“ N2)

- Bleiben am Ende einer Ebene 1 bis 7 Bytes, werden sie in `ebene` zum Nullziel (nie gelesen, kein Block, 7 Bytes in der Budgetzählung); die Länge ändert sich nie. Gilt für `udta`, `moov`, `ilst`, die oberste Ebene und das HEIC-`meta`. Der Abschluss von 4 Nullbytes, den QuickTime schreibt, bleibt byte-gleich.

### F0154 Kapitelnamen fallen ([ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #72“ N3, PO Zeile 130)

- `chpl` fällt aus der Erlaubnisliste von `udta`: unkenntlich (Typ `free`, Inhalt genullt); `tsel` und `meta` bleiben. **Kapitel gehen verloren** (VLC u. ä. zeigen sie nicht mehr). Nachrüstbar als Ausbaustufe C2 (Namen nullen, Zeitpunkte behalten), wenn der Betreiber Kapitel behalten will; dann ist Zeile 130 zu ändern.
- Überholt in R00029: „bleiben nur `chpl`, `tsel`, `meta`“ (Beschreibung „Was, nach Struktur“), „`chpl` byte-gleich“ (Kriterium „Qualität bleibt“) und B2 Probe P8.

### F0155 Nutzerdaten unter `trak` und `traf` werden unkenntlich ([ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #72“ N1)

Keine Erlaubnisliste in diesem Bereich (die Menge der Strukturboxen ist offen, ein vergessener Eintrag bricht die Wiedergabe), sondern eine kurze **Sperrliste** der Boxarten, die freien Text tragen können:

| Ort | Box | Entscheidung |
|---|---|---|
| `trak` | `meco` | unkenntlich |
| `trak` | `uuid` | nur der Typ `FFCC8263-F855-4A93-8814-587A02521FDD` (Spherical Video V1, 360-Grad-Projektion) bleibt, jede andere unkenntlich |
| `trak` | `tref` | bleibt (benannte Grenze: nur Listen von Spurkennungen; Kapitelspuren und Untertitelzuordnung hängen daran) |
| `traf` | `meta`, `meco`, `udta` | unkenntlich; Abstieg in `traf` nur, um diese drei zu finden |
| `traf` | `uuid` | bleibt (benannte Grenze: Smooth-Streaming- und PIFF-Verschlüsselung) |
| `trak/mdia` | `hdlr` (Name), `minf/stbl` | bleiben (benannte Grenze wie in R00029) |

- **Die UUID von Spherical Video V1 ist aus dem Gedächtnis der ADR, nicht geöffnet**; der Beleg ist die handgebaute 360-Datei der Probe (b). Passt sie nicht, ist das eine Korrektur der Konstante, keine neue Richtung.

### F0156 Mehrdeutige `meta` und ungültige Feldbreiten werden abgewiesen ([ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #72“ O1, O6)

- **O1**: `lies_zahl` mit `breite` größer als 8 → 422 „Feldbreite ungültig“; 0 bis 8 bleiben erlaubt (heute schneidet der Linksschub bei 9 bis 15 Bytes still ab).
- **O6**: QuickTime-Deutung eines `meta` nur, wenn die ersten vier Bytes ungleich 0 sind **und** `hdlr` an Stelle 4 steht; ISO-Deutung nur, wenn die ersten vier Bytes 0 sind; beides oder keines von beidem → 422 „meta nicht eindeutig“. Risiko: echte Dateien mit Version ungleich 0 würden abgewiesen (nicht bekannt, die Kontrollen sind der Beleg).

## Nicht-funktionale Anforderungen

- **Zeitkriterien** nach `AI-Team/planer.md`: keine neue Zeitgrenze. Bestehende Grenzen von R00029 (je Fall ≤ 2 s, Budget B = 2 × Länge + 8 MiB, Lesepuffer ≤ 1 MiB) gelten unverändert; die Golden-Datei zählt die Zugriffe und hält sie fest (Zugriffsfolge gleich).
- **Konsole**: berührt keine Oberfläche; „keine Konsolenfehler“ entfällt.
- **Sicherheit**: das Protokoll nennt nie Inhalte, Koordinaten oder Anhangnamen (R00029); `exif_ort` ist ein Wahrheitswert. Die Wiedergabe echter Dateien darf nicht schlechter werden: die Kontrollen aus R00029 („Qualität bleibt“, „Kontrollen Platzhalter“: iPhone, Android, fMP4/DASH, 360-Datei) laufen nach F0153, F0154, F0155 und F0156 mit. Review-Punkt für den `security-reviewer`: Wiederholung nach F0155.
- **Länge**: die Länge der Datei ändert sich nie (R00029), auch nicht durch die Nullung des Rests und von `chpl`.
- **Umgebung**: `ffmpeg` und `ffprobe` sind Vorbedingung der Integrationstests (`AI-Team/umgebung.md`); `exiftool` nur für die Proben der Reviewer, nicht im Gate.

## Akzeptanzkriterien

Gemeinsam, wenn nicht anders genannt: Start mit `TZ=Europe/Berlin localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser --plattenquote 10MiB --anhangquote 10MiB --protokoll-stufe debug` auf einem Wegwerf-Ordner; Unit-Tests wie in `tilger/`; „Mutante“ heißt: eine Testkopie mit genau dieser Änderung, der genannte Test wird rot.

### F0150 Tilger-Umbau ohne Verhaltensänderung

- [ ] **Golden-Test (B0674)**: Mutante „ein Byte des Ergebnisses verändert“ und Mutante „ein Zugriff mehr“ färben `golden.txt` rot (der Test sieht beides). Heute nicht vorhanden. `golden.txt` und alle bestehenden Tests sind die Gleichwertigkeitsprobe für B0675 bis B0677 (ein geänderter Test in einem Umbau-Commit ist ein Befund)
- [ ] **Verschieben (B0675)**: `cargo test` ohne Teständerung grün und `golden.txt` unverändert; Zahl der Tests gleich (`cargo test -- --list` vor/nach, Namen gleich außer dem Umzug); `git diff -M --stat` zeigt Verschiebungen, keine Änderung am Funktionsrumpf (Prüfung durch den `code-reviewer`: Rümpfe zeilengleich); `grep -rn "fn enthaelt" src tests` zeigt je Baum genau eine Stelle
- [ ] **`ebene` (B0676)**: `golden.txt` und alle bestehenden Tests unverändert grün. Mutantenpflicht: für jede der neun Ebenen ist die Mutante „Platzhalter-Fall dieser Ebene deaktiviert“ nach dem Umbau mit **einer** Änderung (der einen Stelle in `ebene`) für alle Ebenen zugleich erreichbar, und die Platzhalter-Proben (B3 (a)–(g), `platzhalter_*`) färben sich rot — mindestens 9 Tests; Mutante „Eintrag aus der Liste gestrichen“ je Ebene → ein Test rot (Proben von B2 P8/P9). Keine Liste enthält mehr einen Typ, den ihr `match` vorher abfängt
- [ ] **`ausdehnungen` (B0677)**: `cargo test` ohne Teständerung grün und `golden.txt` unverändert; die Regeln B1 Regel 1 und B3 Regel 5l ändern sich nicht
- [ ] **Richtwert Größe** (kein Gate): keine Quelldatei des Tilgers hat mehr als 1.000 Zeilen einschließlich ihrer Tests.

### F0151 Jede Abweisung der Metadaten nennt ihre Ursache

- [ ] **Wortlaute (B0678)**: (1) Unit je Variante: eine Eingabe, die genau diese Ursache auslöst, liefert genau diesen Wortlaut — 22 unveränderte plus die 12 neuen der Tabelle; (2) `Grund::ALLE` hat paarweise verschiedene Texte, keine Variante ohne Test; (3) Integration: `PUT` mit abgeschnittenem JPEG → 422 `{regel: "metadaten", grund: "JPEG-Segmentlänge ungültig"}` (heute „Längen widersprechen sich“, rot); `iloc` mit Überlauf → „iloc-Ausdehnung überläuft“ (rot); (4) `golden.txt` ändert sich nur in den Grund-Spalten der Tabelle; (5) Mutante „zwei Ursachen mit einem Wortlaut“ → (2) rot

### F0152 Das Protokoll meldet den Ort nur, wo ein Exif gelesen wurde

- [ ] **`exif_ort` (B0679)**: (1) JPEG mit Exif-GPS → Zeile enthält `exif_ort=ja` und **kein** ` ort=`; Exif ohne GPS → `exif_ort=nein`; JPEG ohne Exif → weder `exif_ort` noch `ort`; (2) MOV mit `keys`/`ilst` `com.apple.quicktime.location.ISO6709` („Test wieder scharf“: der Test `quicktime_schluessel_in_ilst_alle_werte_genullt` prüft jetzt `zaehler.exif_ort == None` **und** dass kein Byte des Ortes bleibt) und MP4 mit `udta/©xyz` → getilgt, Zeile ohne `exif_ort`; (3) HEIC mit Exif-GPS → `exif_ort=ja`; (4) Mutanten „Video meldet `exif_ort=nein`“ und „Exif-GPS wird nicht gemeldet“ je rot. Heute rot: (1) dritter Fall (`ort=nein`), (2)

### F0153 Der Rest unter 8 Bytes wird genullt

- [ ] **Rest (B0680)**: MP4 mit 7 Bytes Text am Ende von `udta`, von `moov`, von `ilst` und hinter der letzten obersten Box (je eine Datei) → die Bytefolge kommt in der Ausgabe nicht vor, Länge gleich, `ffprobe -v error` ohne Ausgabe; `udta` mit 4 Nullbytes → Ausgabe byte-gleich zur Eingabe, kein 422; HEIC-`meta` mit Rest → genullt; Mutante „Rest bleibt“ in jeder der vier Ebenen rot (heute rot). Die sieben Bytes der Probe sind „52.52,1“ (die ADR nennt „52.52,13“ als Beispiel; maßgeblich ist die Länge 1 bis 7).

### F0154 Kapitelnamen fallen

- [ ] **`chpl` (B0681)**: MP4 mit `udta/chpl` (Eintrag „Hamburg Hafen“) → die Bytefolge kommt nicht vor, Box als `free` mit Nullen, Länge gleich, `ffprobe -v error -show_chapters` ohne Kapitel und ohne Fehler, Wiedergabe unverändert; `tsel` byte-gleich; Mutante „`chpl` bleibt“ rot (heute rot); die B2-Probe P8 wird auf „`chpl` genullt“ umgestellt

### F0155 Nutzerdaten unter `trak` und `traf` werden unkenntlich

- [ ] **Sperrliste (B0682)**: (a) `trak/meco` mit `meta/ilst/©xyz` → `free`, Inhalt genullt; (b) `trak/uuid` mit XMP-Text und GPS → unkenntlich; `trak/uuid` Typ Spherical V1 mit dem XML `<rdf:SphericalVideo>` → byte-gleich, `ffprobe` zeigt die Spur unverändert; (c) `moof/traf/meta/ilst/©xyz`, `traf/meco`, `traf/udta/©xyz` → unkenntlich, `traf/tfhd`, `tfdt`, `trun`, `senc` byte-gleich, DASH-Kontrolle aus B2 P8/P10 weiter grün, `ffmpeg -v error -i <fMP4> -f null -` Code 0; (d) `trak/tref` und `traf/uuid` byte-gleich (Kontrolle der Grenzen); (e) Mutanten „`trak/meco` bleibt“, „fremde `trak/uuid` bleibt“, „Spherical-`uuid` wird genullt“, „`traf/meta` bleibt“, „`tfhd` genullt“ je rot. Heute rot: (a), (b) erster Teil, (c)

### F0156 Mehrdeutige `meta` und ungültige Feldbreiten werden abgewiesen

- [ ] **Feldbreite und `meta` (B0683)**: O1: `iloc` mit `offset_size` 9 → 422, mit 0/4/8 unverändert; Mutante „Breite ungeprüft“ rot. O6: `meta` mit Version 1 und `hdlr` an Stelle 8 → 422; `meta` ohne FullBox-Kopf, `hdlr` an Stelle 4 → wie heute getilgt; Kontrollen byte-gleich; Mutante „unklar → ISO“ rot

### Text-Baustein Betreiber-Anleitung (I0019)

- [ ] **Der Text steht unten wörtlich und ist in der Notiz von I0019 verzeichnet** (grep `Karte #72` in der Notiz von I0019 = 1 Treffer). Wird die Anleitung für I0019 geschrieben, enthält sie jeden der fünf Punkte sinngemäß und mit dem genannten Wert; Prüfung durch den `planer` bei der Anforderung zu I0019.

> **Text-Baustein.** Was der Tilger beim Hochladen ändert:
> 1. Der `grund` im 422 `metadaten` nennt die Ursache. Zwei Sammelwortlaute früherer Fassungen gibt es nicht mehr: „Längen widersprechen sich“ ist nun eines von „Zugriff außerhalb der Datei“, „JPEG-Segmentlänge ungültig“, „PNG-Chunk ragt über die Datei“, „WebP-Länge über die Datei“, „WebP-Chunk ragt über seinen Rahmen“, „Box kürzer als ihr Kopf“, „Box länger als ihre Elternbox“, „Box ohne Fortschritt“, „Feld außerhalb seiner Box“, „iloc-Ausdehnung überläuft“; „Exif-Item nicht als Bytebereich der Datei“ ist nun „Ziel-Item ohne Ausdehnung in iloc“ oder „Ziel-Item außerhalb der Datei“. Wer den Text vergleicht, vergleicht `regel: "metadaten"` und zeigt `grund` nur an.
> 2. Bei Videos, Audio und Bildern ohne Exif fehlt das Feld `exif_ort` im Protokoll; bei Videos sieht die Betreiberin also nicht, ob ein Ort getilgt wurde — es fallen alle Nutzerdaten.
> 3. **Kapitel gehen verloren**: die Kapitelnamen eines MP4 (`chpl`) werden mit den übrigen Nutzerdaten getilgt.
> 4. Ein Rest von 1 bis 7 Bytes am Ende einer Box wird genullt.
> 5. Bleiben dürfen die Spherical-Video-V1-Projektion (`trak/uuid`), die Spurverweise `trak/tref` und die Verschlüsselungsangaben `traf/uuid`; 360-Grad-Videos spielen weiter als 360 Grad. Eine Datei mit nicht eindeutigem `meta` oder einer Feldbreite über 8 Bytes wird abgewiesen.

## Betroffene Verzeichnisstruktur

- `src/gespraeche/anhangpruefung/tilger/` (`mod.rs`, `kisten.rs` neu, `isobmff.rs`, `heif.rs` neu, `grund.rs` neu, `bild.rs`, `mpf.rs`, `port.rs`, `golden.txt` neu), `src/gespraeche/anhangpruefung/testhilfe.rs` neu und die Kopien der Testhelfer in `avif.rs`, `erkenner.rs`, `medien.rs`.
- `src/http/anhaenge/hochladen.rs`, `src/protokoll/zeile.rs` (`Schluessel::ExifOrt`), `src/episodenkatalog.rs` (ein Beispieltext).
- Tests: `tests/gemeinsam/dateien.rs` neu, `tests/anhaenge_tilgen.rs`, `tests/anhaenge_traeger.rs`, `tests/episoden.rs`.
- Dokumentation nach dem Bau (architekt): A00018 (Soll wird Ist), ADR-0042 Katalog.

## Technische Überlegungen

### Ablauf (Vorplanung — welche Bubbles es am Ende werden, entscheidet der Entwickler)

1. **Karte A** (`entwickler`): B0674 → B0675 → B0676 → B0677, je Baustein ein Commit.
2. **Karte B** (Mini-Entwickler): B0678 → B0679 → B0680 → B0681.
3. **Karte C** (`entwickler`): B0682 → B0683; danach Wiederholungs-Review `security-reviewer`.

### Grobentwurf

- `ebene(port, eltern, wahl, zaehler)` mit `Behandlung::{Bleibt, Unkenntlich, Zeitfelder, Abstieg(fn)}`; `Abbruch::Metadaten(Grund)`; `Getilgt.ort: Option<bool>`.

## Tests

- Golden-Datei plus Mutanten (Ergebnis, Zugriff), Unit je Variante von `Grund`, Integration gegen das Programm mit `ffprobe`/`ffmpeg`, Mutanten je Regel (siehe Kriterien).

## Aufwand (Zählung)

Gezählt aus der WBS (2026-10-30): 7 Slices, 10 Bubbles (B0674–B0683); B0675, B0679, B0680, B0681 je 2 h (8 h), B0674, B0676, B0677, B0678, B0682, B0683 je 2-4 h (12–24 h) → 20 bis 32 h, Richtwert, ungemessen für diese Bubble-Typen. Karte A 8–14 h, Karte B 8–10 h, Karte C 4–8 h.

## Abhängigkeiten

- Abhängig von: R00029 (F0115 grün: B0501–B0654), Karte #70 (gemergt), `ffmpeg` und `ffprobe` im Gate.
- Reihenfolge: F0150 → F0151 → F0152 → F0153 → F0154 → F0155 → F0156 (Baureihenfolge der ADR).
- Blockiert: die Anleitung I0019 (Text-Baustein).

## Offene Fragen

- **An den Betreiber** (nicht blockierend): Kapitel gehen mit F0154 verloren (PO Zeile 130 angewandt); soll später ein Kapitel-Erhalt (Namen nullen, Zeitpunkte behalten) kommen, ist das eine neue Entscheidung mit neuem Leser.
- **Für den architekt** (nicht blockierend): ADR-0047 nennt N1 im Fließtext vor N3 und in der Bausteintabelle N3 (B72-7) vor N1 (B72-8); die WBS folgt der Tabelle.
- **An den Director**: die Anleitung I0019 braucht den Text-Baustein; die UUID von Spherical Video V1 ist ungeprüft.

## Missing-Docs

- Glossar fehlt; Begriffe aus ADR-0047, R00029.
- Die Zahlen „21 Stellen“ und „2.357 Zeilen“ stammen aus der ADR (Stand `25b7303`); der Entwickler zählt vor dem Umbau nach.

## Warum löst diese Anforderung das Problem? (Pflicht)

- `chpl` ist vom Nutzer frei gesetzter Text, und „alle Nutzerdaten“ gilt (Zeile 130): wenn F0154 die Box unkenntlich macht, steht „Hamburg Hafen“ nicht mehr in den ausgelieferten Bytes, und `ffprobe -show_chapters` bestätigt es.
- Ein Rest von 7 Bytes hinter der letzten Box ist für jeden Boxleser unsichtbar und für `strings` lesbar; wenn `ebene` ihn nullt, bleibt kein Platz für Text, ohne dass eine Box gelesen werden müsste.
- `meco`, fremde `uuid` und `traf/meta` unter `trak` und `traf` sind dieselbe Klasse wie die oben bereits unkenntlichen; die Sperrliste (statt einer Erlaubnisliste) schließt sie, ohne die offene Menge der Strukturboxen zu raten, und die Kontrollen mit `ffmpeg` belegen, dass die Wiedergabe bleibt.
- Mit einem Enum, dessen Wortlaute ein Test paarweise vergleicht, ist „21-mal derselbe Satz“ nicht mehr möglich, und der Agent, der den `grund` anzeigt, nennt dem Menschen die Ursache.
- Mit der Golden-Datei und der einen Stelle für den Platzhalter-Fall kann der nächste Umbau das Verhalten nicht unbemerkt ändern (der Mutant „Platzhalter-Fall fehlt“ färbt mindestens neun Tests rot) und eine neue Ebene nicht mehr „vergessen“ werden.

## Notizen

- Verworfen (ADR-0047 Ergänzung „Karte #72“): S3-B/C (Zyklus, Verzeichnis `isobmff/`), S2-B/C (Trait, nur Hilfsfunktion), S4-B/C (nur Konstanten, Maschinenkennung neben dem Text), O2/O3/O4 für `ort`, R2/R3 für den Rest (benannte Grenze, Abweisung), C2/C3 für `chpl`, alle `trak/uuid` unkenntlich oder alle bleiben, S4 zuerst, alles in einem Commit.
- Nicht Teil dieser Anforderung: Telemetriespuren in `mdat`, Handlername in `mdia/hdlr`, Fotos in Archiven (benannte Grenzen wie bisher).
- Zusagen aus R00029 bleiben, soweit nicht oben als überholt benannt; Tests, die den alten Wortlaut „Längen widersprechen sich“ oder `ort=ja` erwarten, ziehen mit F0151 und F0152 mit.
