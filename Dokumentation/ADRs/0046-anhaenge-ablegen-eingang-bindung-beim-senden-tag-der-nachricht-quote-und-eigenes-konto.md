# ADR-0046: Anhänge ablegen — je Datei hochgeladen in einen Eingang, beim Senden in den Tag der Nachricht gebunden, eine Metazeile mit Anhang-ID; Bytes zählen an der Quote und an einem eigenen Konto

**Status**: Vorgeschlagen
**Datum**: 2026-10-29
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #9 I0010) — wartet auf den Betreiber (Fragen B1, B2, B5 unten) und auf die Freigabe zusammen mit [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md)
**Kontext-Anforderung**: — (WBS I0010, Dialog D0003, Karte #9; prägt I0012, I0015, I0016, I0019, I0022, I0023)
**Erprobt durch**: — (überträgt gebaute Mechanik: Anhängen mit Rückbau `src/gespraeche/mod.rs:310–315` und ADR-0011; Quote-Zähler `belegt` und Prüfung unter der Schreibsperre ADR-0044 Regel 2; Schreibkonto `src/schreibkonto/`, ADR-0036; Körperlesen mit Frist `src/http/koerper.rs:215` `roh_lesen`; Löschen von `anhaenge/<tag>/` ohne Symlinks geprobt in ADR-0032)

## Kontext

- **Vision**, Zielbild „Anhänge“: „An eine Nachricht lassen sich eine oder mehrere Dateien hängen.“ Leitplanken: Datenhaltung in täglich rotierenden Logdateien, kein Git, keine Datenbank; Lastprofil „Team-LAN“; **Plattenquote** („an der Grenze wird Senden abgewiesen, deswegen wird nichts gelöscht“). WBS I0010, Fertig-Kriterium: „eine oder mehrere Dateien hängen an einer Nachricht und lassen sich öffnen“; Notiz: Bytes jeder gespeicherten Datei zählen am Zähler `belegt`, geprüft vor dem Ablegen, abgezogen mit dem Tag — „entscheidet die ADR von I0010“.
- **Vorentschieden** in [ADR-0003](0003-ablage-layout-im-ordner.md): Anhänge liegen unter `<gespräch>/anhaenge/<tag>/<zeit>-<n>-<dateiname>` im Tag der Nachricht und entfallen mit ihm; der gespeicherte Name ist bereinigt; erst die Datei (temporär + `rename`), dann der Satz. [ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md): Metazeile `+ anhang: …` (Beispiel mit Pfad, Bytes, Typ), unbekannte Schlüssel bleiben erhalten. [ADR-0030](0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) Regel 8 und [ADR-0032](0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md) Regel 7: `anhaenge/<tag>/` wird nach der Tagesdatei mit `remove_dir_all` gelöscht, Symlinks werden nie verfolgt.
- **Senden heute** ([ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md)): JSON `{text}`, Körper ≤ 131.072 Bytes, Text ≤ 16.384 Bytes, ein weiteres Feld → 422 `feld` („bis I0010/I0013 gibt es keine weiteren“); Option D „`multipart` von Anfang an“ war dort als tote Flexibilität verworfen. Wiederholschlüssel über eine Prüfsumme des Texts.
- **Körper** ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), Ergänzungen): jede Route nennt ihre Körpergrenze mit Herleitung; Lesefrist 10 s als Gesamtfrist — „eine künftige Route mit großem Körper (Anhänge, I0010) braucht eine eigene, aus ihrer Größe hergeleitete Frist oder Mindestrate; 10 s für einen Upload von Megabytes wären falsch“. Verbindungen je Herkunft 160, gesamt 512.
- **Schreibkonto** ([ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md)): Kosten ⌈Bytes / 4.096⌉, Vorrat je Kennung 30, je Herkunft 120; „Anhänge (I0010) entscheiden ihre Kosten selbst (Empfehlung: nach Bytes, eigener Vorrat)“.
- **Quote** ([ADR-0044](0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md)): `belegt` im Baustein Gespräche ist einziger Halter; Invariante `belegt = Σ Verlauf::bytes()`; Anhänge „zählen, sobald gebaut“; Standard 4 GiB mit Herleitung „≥ 11 h von Normalstand bis zur Quote bei einer flutenden Herkunft“; Pre-Mortem 2: jede Interaction, die Gesprächsbytes schreibt, nennt ihren Anteil am Zähler.
- **Große Antworten** ([ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md) Regel 7c): Dateien nie ganz in den Speicher, in 64-KiB-Stücken mit Gegendruck; ein Anhang ist nie Teil einer Liste — die Liste trägt Metadaten.
- **Designsystem** (`Dokumentation/Wireframes/designsystem/components/Anhang/README.md`, `components/Eingabe/README.md`, `1-tastatur.md`): Schalter, `Strg Shift U`, Drag & Drop, Einfügen aus der Zwischenablage; „Hochladen läuft: Balken mit Prozent und ‚Abbrechen‘“, „Fehlgeschlagen … ‚Erneut‘“, „Zu groß: sofort beim Auswählen geprüft … die Nachricht kann ohne die Datei gesendet werden“. `3-offene-fragen.md` Frage 9: „Die 25 MB im Entwurf sind ein Platzhalter; welche Grenze gilt?“
- **Sitzung** ([ADR-0012](0012-identitaet-und-sitzung.md), [ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)): Nachweis nur über den Header `X-LocalchatA-Sitzung`, kein Cookie, kein Token in der URL.

Szenarien mit Maß (ergänzen A00001):

- **QA1 Senden**: ein Mensch hängt drei Bildschirmfotos (je ≈ 300 KB) an und sendet → die Nachricht steht mit allen dreien in jeder offenen Sicht < 1 s nach dem Ende des letzten Hochladens (Q3 gilt ab der Bindung); ein Agent sendet einen 5-MB-Bericht mit zwei Aufrufen ohne Browser.
- **QA2 Nichts halb**: bricht das Hochladen ab, scheitert das Anhängen des Satzes oder stürzt die Instanz zwischen Datei und Satz ab → es gibt nie einen Satz, dessen Anhang fehlt; eine verwaiste Datei zählt an der Quote und entfällt mit ihrem Tag; nicht gebundene Hochladungen sind nach dem Ablauf bzw. nach einem Neustart weg.
- **QA3 Flut**: eine Herkunft lädt dauerhaft so schnell hoch, wie sie darf → vom Normalstand (≈ 1,6 GB) bis zur Quote vergehen mindestens **5 h** (heute ≥ 11 h für Text allein; die Hälfte ist der Preis der Anhänge, siehe Frage 4); der Speicher der Instanz wächst je laufendem Hochladen höchstens um ein Stück (≤ 64 KiB) plus Puffer.
- **QA4 Kosten**: Laden eines Ordners mit 1.000 Anhängen kostet ≤ 50 ms zusätzlich (ein `stat` je Datei, Metazeilen aus dem ohnehin gelesenen Rohtext) — **gerechnet, nicht gemessen**.

## Optionen

### Frage 1: Wo und wie liegt ein Anhang?

#### Option O1: Im Ordner des Gesprächs unter dem Tag der Nachricht, Name lesbar, Anhang-ID nur in der Metazeile (gewählt)
`channels/<name>/anhaenge/<JJJJ-MM-TT>/<hhmmssmmm>-<n>-<bereinigter-name>`; der Satz trägt je Datei `+ anhang: <id> <bytes> <art> <maße> <datei> <anzeigename>`.
- Vorteile: genau das Layout von ADR-0003 — der Tag entfällt mit einem Ordner, `ls` zeigt lesbare Namen in Sendefolge, jedes Gespräch bleibt für sich kopierbar und mit `less` lesbar (Q5); die Anhang-ID (128 Bit Zufall, [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md)) steht nur im Log, nicht im Dateinamen — wer den Ordner sieht, hat ohnehin alles.
- Nachteile: zum Ausliefern braucht die Instanz eine Zuordnung ID → Datei im Speicher (beim Laden aus den Metazeilen gebaut); keine Dubletten-Erkennung.

#### Option O2: Zentraler Speicher `anhaenge/<id>` mit Verweiszählung
- Vorteile: kurze Pfade; gleiche Datei in zwei Nachrichten nur einmal (bei Inhalts-Hash).
- Nachteile: Entfallen braucht eine Verweiszählung über alle Gespräche und Tage — eine zweite Löschlogik neben ADR-0030; Archivieren (I0022, „Ordner wandert nach `archiv/<name>/`“) nimmt die Anhänge nicht mit; der Ordner eines Gesprächs ist nicht mehr vollständig.

#### Option O3: Inhalt im Log als Base64-Metazeile
- Vorteile: eine Datei je Tag, atomar mit dem Satz.
- Nachteile: das Log wird unlesbar (Q5), und der Verlaufsindex hält den Rohtext im Speicher (ADR-0006) — jeder Anhang läge 1,33-fach im RAM; Verlaufsblöcke trügen Megabytes (ADR-0039).

#### Option O4: Anhang-ID als Dateiname (`anhaenge/<tag>/<id>`), Anzeigename nur im Satz
- Vorteile: Auslieferung ohne Zuordnung im Speicher (Pfad aus Gespräch, Tag, ID).
- Nachteile: `ls` zeigt Zufallszeichen statt Namen (Q5 für Anhänge verloren); die URL müsste Gespräch und Tag tragen; die ID stünde in jedem Verzeichnislisting.

### Frage 2: Wie kommt die Datei zur Instanz?

#### Option U1: Zweistufig — je Datei `POST /api/anhaenge` als Rohkörper in einen Eingang, dann `POST …/nachrichten` mit `{text, anhaenge: [<id>…]}` (gewählt)
- Vorteile: die Senderoute bleibt JSON mit kleiner Grenze (ADR-0034, ADR-0010 bleiben gültig); der Rohkörper wird ohne Parser in Stücken auf die Platte geschrieben (kein `multipart`, keine neue Abhängigkeit); Fortschritt, Abbrechen und „Erneut“ **je Datei** wie im Designsystem; Grenzen (Größe, Quote, Konto) greifen **vor** dem ersten Byte über `Content-Length`; der Agent braucht zwei `curl`-Aufrufe mit denselben Headern wie beim Senden; die Bindung an die Nachricht geschieht unter den Sperren, die das Senden schon hält.
- Nachteile: ein Zwischenzustand (hochgeladen, nicht gebunden) mit Ablauf, Reservierung und Aufräumen; zwei Anfragen statt einer.

#### Option U2: Eine Anfrage `multipart/form-data` mit Text und Dateien
- Vorteile: eine Anfrage, atomar aus Sicht des Senders; Browser-Formulare können es von selbst.
- Nachteile: zweite Körperform der Senderoute (ADR-0034 Option D); Parser `multer` als neue Abhängigkeit; Fortschritt nur für alles zusammen, „Erneut“ lädt alle Dateien neu; die Körpergrenze der Senderoute wächst von 128 KiB auf ein Vielfaches, also auch der Schaden einer Flut ungültiger Körper; `multipart/form-data` ist ein „einfacher“ Cross-Origin-Request — der Schutz hinge allein an Origin-Prüfung und Sitzungs-Header.

#### Option U3: JSON mit Base64-Inhalt in der Senderoute
- Vorteile: eine Anfrage, eine Körperform.
- Nachteile: Körper 1,33-fach, ganz im Speicher und auf der Werkbank geparst (ADR-0010 Ergänzung Regel 2) — 25 MB werden ≈ 33 MB JSON je Anfrage im RAM; bei 16 parallelen Hochladungen ≈ 0,5 GB.

#### Option U4: Nachricht zuerst senden, Dateien danach an die Nachricht hochladen (`PUT …/nachrichten/<zeit>/anhaenge/<n>`)
- Vorteile: die Nachricht erscheint sofort.
- Nachteile: Leser sehen Anhänge, die es noch nicht gibt (oder nie geben wird); ein Satz im Log müsste nachträglich ergänzt werden — widerspricht „Anhängen statt Umschreiben“ (ADR-0004, ADR-0011) und der Reihenfolge „erst Datei, dann Satz“ (ADR-0003).

### Frage 3: Größengrenzen — Richtung beim Betreiber (B2), Mechanik hier

| | je Datei | je Nachricht | Stärke | Schwäche |
|---|---|---|---|---|
| G1 | 25 MiB | 10 Dateien, zusammen 100 MiB (**Empfehlung**) | Platzhalter des Designsystems; Bildschirmfotos, Logs, PDFs, kleine Archive; 25 MiB im LAN (100 Mbit/s) ≈ 2 s; 100 MiB ≈ 2,4 % der Standardquote | ein Container-Abbild oder Video passt nicht |
| G2 | 5 MiB | 10 Dateien, 25 MiB | kaum Quote-Druck, kurze Hochladungen | ein größerer Bericht oder Mitschnitt passt nicht — Agenten teilen Dateien |
| G3 | 100 MiB | 10 Dateien, 250 MiB | fast alles passt | eine einzige Nachricht nimmt 6 % der Standardquote; Konto (Frage 4) müsste so großzügig sein, dass QA3 nicht hält |
| G4 | einstellbar (`--anhang-grenze`) | — | jeder Betrieb wählt | dritte Einstellung neben `--plattenquote`; die Kontowerte (Frage 4) müssten mitwachsen |

Fest unabhängig von der Zahl: **höchstens 10 Dateien je Nachricht** (Liste im JSON-Körper bleibt klein); die Grenze je Datei ist zugleich die Körpergrenze von `POST /api/anhaenge` (ADR-0010 Regel 1 — Herleitung: Grenze der Datei, kein Aufschlag, weil ungeparst).

### Frage 4: Wer bremst die Bytes?

#### Option K1: Eigenes Eimerpaar in Bytes im Schreibkonto — je Kennung und je Herkunft (gewählt; Zahlen hängen an B2)
Empfehlung zu G1: je Kennung Vorrat **100 MiB**, Nachschub **32 KiB/s**; je Herkunft Vorrat **200 MiB**, Nachschub **64 KiB/s**. Gebucht wird beim Hochladen nach `Content-Length`, vor dem ersten Byte, nicht erstattet.
- Vorteile: eine volle Nachricht (100 MiB) geht sofort; eine flutende Herkunft schreibt dauerhaft ≤ 64 KiB/s ≈ 5,3 GiB/Tag — zusammen mit der Textflut (≈ 66 KB/s, ADR-0036 Ergänzung) ≈ 130 KB/s, also vom Normalstand (1,6 GB) bis zur Quote (4 GiB) ≈ (2,4 GiB − 200 MiB) / 130 KB/s ≈ **5 h** (QA3); derselbe Mechanismus wie beim Senden, rein und mit gestellter Uhr prüfbar.
- Nachteile: halbiert die Vorwarnzeit der Quote gegenüber Text allein (≈ 11 h → ≈ 5 h); wer in einer Stunde mehr als ≈ 210 MiB hochlädt, wartet (z. B. die fünfte volle Nachricht ≈ 50 min).

#### Option K2: Dasselbe Konto wie das Senden, Kosten ⌈Bytes / 4.096⌉
- Vorteile: kein neuer Eimer.
- Nachteile: 25 MiB kosten 6.400 Einheiten bei einem Vorrat von 30 — Anhänge wären praktisch unmöglich; den Vorrat zu heben, öffnet die Textflut.

#### Option K3: Keine Rate — nur Größe je Datei, Quote und Zahl gleichzeitiger Hochladungen
- Vorteile: kein Code im Konto; Menschen warten nie.
- Nachteile: im LAN (≈ 100 MB/s) füllt eine Herkunft die Standardquote in **unter einer Minute** — danach ist die Instanz für alle stumm (ADR-0044 Pre-Mortem 1), ohne Vorwarnung.

#### Option K4: Tageskontingent je Herkunft (z. B. 1 GiB/Tag)
- Vorteile: leicht zu erklären.
- Nachteile: zählt über Mitternacht und Neustart hinweg (Zustand auf der Platte oder Lücke beim Neustart); ein Kontingent für 24 h lässt die ganze Menge in einer Minute zu — dieselbe Flutspitze wie K3, nur kleiner.

### Frage 5: Wie lange lebt ein nicht gebundener Anhang?

- **E1 (gewählt): Eingang `<ordner>/.eingang/`, Ablauf 15 min nach dem Ende des Hochladens, beim Start geleert.** Stärke: ein Abbruch des Tabs oder Agenten hinterlässt nichts Dauerhaftes; die Reservierung an der Quote (Frage 6) ist nach 15 min frei; der Browser lädt beim Senden hoch (nicht schon beim Auswählen), 15 min decken „Erneut senden“ nach einer Störung (ADR-0037). Schwäche: wer länger als 15 min zwischen Hochladen und Senden wartet (nur Agenten mit eigenem Ablauf), bekommt 422 `anhang` und lädt neu.
- E2: Ablauf 24 h. Stärke: Entwürfe mit Anhängen überleben eine Pause. Schwäche: Reservierungen halten die Quote einen Tag lang; der Entwurf je Channel (ADR-0038) liegt im `localStorage` und kann ohnehin keine Dateien halten.
- E3: kein Eingang — Hochladen und Senden in einer Anfrage (U2/U3). Siehe Frage 2.

### Frage 6: Anteil an der Quote (ADR-0044 Pre-Mortem 2)

- **Q-A (gewählt): reservieren beim Hochladen, binden beim Senden, abziehen mit dem Tag.** `belegt` wird zu `gebunden + reserviert`; beide Zähler hält Gespräche. Stärke: geprüft wird, **bevor** ein Byte auf der Platte liegt (WBS-Notiz I0010 und ADR-0044 Folgeentscheidung „vor dem Ablegen der Datei“ — hier ist „Ablegen“ das Schreiben in den Eingang); ein Agent mit vielen offenen Hochladungen kann die Quote höchstens für 15 min binden, und das Konto (Frage 4) begrenzt, wie viel. Schwäche: ein dritter Zustand neben „im Index“ und „entfallen“.
- Q-B: erst beim Senden prüfen (Anhänge + Satz zusammen). Stärke: wörtlich wie die WBS-Notiz. Schwäche: der Eingang füllt die Platte außerhalb der Quote — 16 parallele Hochladungen à 25 MiB je Herkunft bleiben ungezählt, die Quote schützt die Platte nicht mehr.
- Q-C: Eingang nicht zählen, eigene Grenze je Kennung für offene Hochladungen. Stärke: Quote bleibt wie gebaut. Schwäche: zweite Grenze mit eigener Zahl für denselben Zweck.

## Entscheidung

**Gewählt (vorgeschlagen): O1, U1, G1 als Empfehlung an den Betreiber, K1, E1, Q-A.**

Es überwiegt, dass O1 das schon entschiedene Layout erfüllt und den Ordner lesbar hält, dass U1 die Senderoute und alle ihre Zusagen unverändert lässt, jeden Byte vor dem Schreiben gegen Größe, Konto und Quote prüft und das Designsystem (Fortschritt und „Erneut“ je Datei) ohne Umweg trägt, und dass K1 die Vorwarnzeit der Quote in Stunden hält, wo K3 sie auf eine Minute schrumpfen ließe. In Kauf genommen: ein Zwischenzustand mit Ablauf und Reservierung; zwei Anfragen je Datei und Nachricht; die Vorwarnzeit der Quote halbiert sich auf ≈ 5 h; wer viel hochlädt, wartet. **Vorgeschlagen**, weil Größenordnung (B2), erlaubte Dateiarten (B1) und die Nachricht ohne Text (B5) Richtungsfragen des Betreibers sind; die Zahlen unten stehen für die Empfehlung G1 und ändern sich mit der Antwort (die Mechanik nicht).

Regeln:

1. **Hochladen** `POST /api/anhaenge?name=<anzeigename, prozentkodiert UTF-8>` unter `grenzschutz`, Extraktor `Teilnehmer` (Mensch und Agent gleich), Körper roh. Prüfreihenfolge (abgewiesen, nie umgeformt):
   1. fremder Origin/Host 403 · ohne Sitzung 401 · `Content-Type` nicht `application/octet-stream` → 415 · ohne `Content-Length` (auch `chunked`) → 411 · `Content-Length` 0 → 422 `{regel: "leer"}` · über der Grenze je Datei → **413** ohne Lesen, `Connection: close`;
   2. `name` fehlt oder verletzt die Namensregel (Regel 3) → 422 `{fehler, regel: "name", grund}`;
   3. mehr als 4 offene Hochladungen dieser Kennung bzw. 16 dieser Herkunft (laufend oder im Eingang) → 429 `{fehler, grenze: "offen_kennung" | "offen_herkunft", hoechstens}` mit `Retry-After: 5` (keine Eimerrechnung — frei wird ein Platz, wenn eine Hochladung endet oder gebunden wird);
   4. Konto: Buchung von `Content-Length` am Anhang-Eimerpaar (Regel 7), sonst 429 wie [ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md) Regel 4 mit Text „Zu viele Anhänge von … — wieder möglich in n s.“;
   5. Quote: `gebunden + reserviert + Content-Length > hoechstens` → **507** `{fehler, regel: "quote", belegt, hoechstens}` („Nicht hochgeladen: …“, sonst Wortlaut von ADR-0044 Regel 4), sonst `reserviert += Content-Length`;
   6. Körper lesen und in Stücken (≤ 64 KiB) nach `.eingang/<id>.teil` schreiben; dabei Art und Maße bestimmen (A00018); **Frist** = 10 s + `Content-Length` / 256 KiB/s (25 MiB → 110 s; Mindestrate, keine Stückfrist) → sonst 408 mit dem gemeinsamen Text ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), Ergänzung Karte #26); weniger oder mehr Bytes als angekündigt → 400; jeder Abbruch löscht die Teildatei und gibt die Reservierung frei, die Buchung am Konto bleibt;
   7. Erfolg: `rename` nach `.eingang/<id>` → **201** `{anhang: {id, name, bytes, art, breite, hoehe, gueltig_bis}}`.
2. **Eingang** `<ordner>/.eingang/` ([A00017 Anhangeingang](../Architektur/A00017-anhangeingang.md)): je Eintrag Kennung, Name, Bytes, Art, Maße, Ablaufzeit; nur im Speicher verzeichnet, die Datei liegt auf der Platte. Ablauf **15 min** nach Erfolg, geprüft im Sekundentakt der Pflege (ADR-0030) — dann Datei löschen, `reserviert −= bytes`. Beim Start wird `.eingang/` vor dem Laden geleert (nur reguläre Dateien, kein Symlink wird verfolgt, ADR-0032-Muster); ein Unterordner oder Symlink dort ist ein `WARNING` und bleibt liegen. `.eingang` ist kein gültiger Channelname (ADR-0013: kein Punkt) und liegt neben `channels/`, also auf demselben Dateisystem — `rename` ist O(1).
3. **Namensregel** (rein, [A00018 Anhangprüfung](../Architektur/A00018-anhangpruefung.md)): Anzeigename 1–255 Bytes UTF-8; **abgewiesen**: die verbotenen Zeichen aus ADR-0034 Regel 3.6 sowie Tab, Zeilenumbruch, `\r`; Richtungszeichen U+200E, U+200F, U+202A–U+202E, U+2066–U+2069; `/`, `\`; die Namen `.` und `..`. Gespeicherter Dateiname: `<hhmmssmmm>-<n>-<ascii>` mit `<ascii>` = Anzeigename, jedes Zeichen außerhalb `[A-Za-z0-9._-]` zu `_`, höchstens 64 Zeichen (Endung bleibt, wenn sie ≤ 16 Zeichen hat), nie leer (`datei`). Der Anzeigename erscheint nur in der Metazeile, in der API und in `Content-Disposition` (ADR-0047).
4. **Senden mit Anhängen** — [ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) wächst um ein Feld: Körper `{text, anhaenge?: [<id>, …]}`, 1–10 IDs, keine doppelt. Neue Abweisungen nach Schritt 3.6 von ADR-0034: unbekannte, abgelaufene, fremde (anderer Kennung) oder doppelte ID → 422 `{fehler, regel: "anhang", id, grund: "unbekannt" | "abgelaufen" | "fremd" | "doppelt"}`; mehr als 10 → 422 `{regel: "anhaenge", hoechstens: 10}`. **Text leer**: heute 422 `leer` — mit mindestens einem Anhang erlaubt, **falls B5 so entschieden wird** (Empfehlung), sonst bleibt 422. Die Kosten am Text-Konto bleiben ⌈Bytes des Texts / 4.096⌉, mindestens 1. Der Wiederholschlüssel prüft Text **und** Anhang-IDs (Prüfsumme über beide); eine Wiederholung mit bekanntem Schlüssel antwortet 200 ohne Bindung.
5. **Bindung** in `Gespraeche::senden` unter Gesprächs- und Schreibsperre ([ADR-0035](0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md)), nach der Prüfung des Wiederholschlüssels: (a) Zeit vergeben, Tag bestimmen; (b) Quote nur noch für den Satz: `gebunden + reserviert + Bytes(Satz) > hoechstens` → 507 (die Anhänge sind schon reserviert); (c) je Anhang `rename(.eingang/<id>, <gespräch>/anhaenge/<tag>/<datei>)` — der Ordner des Tages wird bei Bedarf angelegt; (d) Satz mit den Metazeilen anhängen (ADR-0011); (e) erst danach `gebunden += Bytes(Satz) + Σ Anhangbytes`, `reserviert −= Σ Anhangbytes`, Einträge ins [Anhangverzeichnis](../Architektur/A00016-anhangverzeichnis.md), Eintrag aus dem Eingang, Ereignis. Scheitert (c) oder (d): bereits verschobene Dateien zurück nach `.eingang/` (`rename`), Satz zurückgebaut (ADR-0011), 500, nichts im Index, kein Ereignis, Reservierung bleibt (die Anhänge sind weiter sendbar bis zum Ablauf). Scheitert das Zurückverschieben, bleibt die Datei als Waise im Tag — sie zählt nach dem nächsten Start an der Quote und entfällt mit dem Tag (QA2); `ERROR senden` mit Datei und Grund.
6. **Metazeile** (Präzisierung von [ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md); eine je Anhang, in Sendefolge, nach `+ art:`):
   `+ anhang: <id> <bytes> <art> <breite>x<hoehe>|- <datei> <anzeigename>`
   — `<id>` 26 Zeichen `[a-z2-7]`, `<bytes>` dezimal, `<art>` aus der Tabelle von ADR-0047 (`image/png`, …, `application/octet-stream`), Maße nur für Bilder, `<datei>` der gespeicherte Name (ohne Leerzeichen, Regel 3), `<anzeigename>` der Rest der Zeile (darf Leerzeichen enthalten, nie Zeilenumbrüche — Regel 3). Beispiel:
   `+ anhang: k3v7q2m5xpc4h6n6tewaz2b5yr 2457600 application/pdf - 140712345-1-Bericht_M_rz.pdf Bericht März.pdf`
   Eine Metazeile, die dieser Grammatik nicht folgt, ist kein Defekt des Satzes (ADR-0004: unbekannte Metazeilen bleiben erhalten), aber der Anhang erscheint nicht im Verzeichnis; `WARNING laden` mit Datei und Zeile.
7. **Konto** ([A00014 Schreibkonto](../Architektur/A00014-schreibkonto.md)) — zweites Eimerpaar in **Bytes**, getrennt vom Text-Konto, dieselbe Mechanik (alles oder nichts, Uhr-Provider, volle Eimer werden entfernt): je Kennung Vorrat 100 MiB, Nachschub 32 KiB/s; je Herkunft Vorrat 200 MiB, Nachschub 64 KiB/s (Werte für G1; Regel: Vorrat je Kennung = Grenze je Nachricht, Vorrat je Herkunft = 2 × Vorrat je Kennung, Nachschub je Herkunft so, dass QA3 ≥ 5 h hält). `gebremst` im Health-Check zählt auch diese 429; Protokoll eigene Episode „Anhänge gebremst“ je Kennung ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regel 5).
8. **Quote** (Ergänzt [ADR-0044](0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) Regeln 1 und 2): `belegt = gebunden + reserviert`. **Laden**: `gebunden` = Σ Rohtext der Tage im Index + Σ Größe der regulären Dateien in `anhaenge/<tag>/` dieser Tage (`symlink_metadata`, Symlinks und Unterordner zählen nicht) — gezählt wird, was auf der Platte liegt, auch Waisen und Dateien ohne Metazeile. **Entfallen**: je Tag `gebunden −= Bytes(Tagesdatei) + Anhangbytes(Tag)`; die Anhangbytes je Tag hält das Anhangverzeichnis. **Invariante**: `gebunden = Σ Verlauf::bytes() + Σ Anhangverzeichnis::bytes_je_tag()` und `reserviert = Σ Eingang::bytes()` zu jedem Zeitpunkt ohne laufende Operation. Prüft ADR-0044 Regel 7 die 90-%-Schwelle, dann gegen `belegt`. **Health-Check** ([ADR-0043](0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md)): neues Feld `belegt` (Bytes, gegen `plattenquote` zu vergleichen); `log_bytes` behält seine Bedeutung (Σ Tageslogs im Index, der Speicher) — die Monitoring-Empfehlung aus ADR-0044 Regel 9 liest sich ab dem Bau mit `belegt` statt `log_bytes`.
9. **Entfallen**: unverändert ADR-0030 Regel 8 und ADR-0032 Regel 7 — nach der Tagesdatei `anhaenge/<tag>/` mit `remove_dir_all` (folgt keinen Symlinks), dann Einträge des Tages aus dem Anhangverzeichnis, dann der Zähler. Scheitert das Löschen des Anhang-Ordners, bleibt der Tag mit seinen Anhängen im Verzeichnis und im Zähler (wie ADR-0030 Regel 4 für die Tagesdatei).
10. **Archivieren** (I0022): Anhänge wandern mit dem Ordner des Channels; ihre Bytes bleiben im Zähler (ADR-0044 Regel 1); die Auslieferung archivierter Anhänge entscheidet I0022.
11. **Direktgespräche** (I0012): dieselbe Ablage unter `direkt/<…>/anhaenge/<tag>/`, dieselbe Bindung; die Sichtbarkeit regelt ADR-0047 Regel 2.

## Konsequenzen

**Positiv:**
- Kein Satz ohne Datei, keine Datei außerhalb der Quote; nicht gebundene Hochladungen räumen sich selbst weg.
- Die Senderoute bleibt klein und JSON; alle Zusagen von ADR-0034 gelten weiter, ein Feld kommt dazu.
- Der Ordner bleibt lesbar: `ls channels/allgemein/anhaenge/2026-10-29/` zeigt Zeit, Reihenfolge und Namen; `grep '+ anhang:'` findet jeden Anhang mit Größe und Art.
- Agent und Mensch nutzen dieselben zwei Routen (`curl --data-binary @bericht.pdf -H 'Content-Type: application/octet-stream' -H 'X-LocalchatA-Sitzung: …' '…/api/anhaenge?name=bericht.pdf'`).
- Jede Grenze (Größe, Zahl, Konto, Quote) greift vor dem ersten geschriebenen Byte.

**Negativ / Inkauf zu nehmen:**
- Zwei Anfragen je Nachricht mit Anhang; ein Zwischenzustand mit 15 min Ablauf.
- Die Vorwarnzeit der Quote sinkt bei Flut von ≈ 11 h auf ≈ 5 h.
- Wer viel hochlädt, wartet am Konto (G1: die fünfte volle Nachricht in einer Stunde ≈ 50 min).
- Der Start stat-et jede Anhangdatei in der Frist (QA4, gerechnet).
- Dateinamen auf der Platte verlieren Umlaute und Sonderzeichen (`Bericht_M_rz.pdf`); der richtige Name steht im Log.

**Pre-Mortem (ein Monat nach dem Bau gescheitert, weil …):**
1. … die Quote durch Anhänge in wenigen Stunden voll war und morgens niemand senden kann. Früh-Indikator: `WARNING quote … 90 %`; Episode „Anhänge gebremst“ derselben Kennung; `belegt` steigt in Sprüngen. Gegenmaßnahme: Konto K1 (QA3 ≥ 5 h); Frist des Channels verkürzen; anheben der Quote ist ein Neustart.
2. … `gebunden` vom Bestand abdriftet, weil ein Weg (Rückbau, Waise, Löschfehler, Archiv) die Anhangbytes vergisst. Früh-Indikator: Unit-Invariante (Regel 8) nach jeder Operation; `belegt` springt nach einem Neustart. Gegenmaßnahme: erster Unit-Test unten; Review-Punkt `code-reviewer`: jede Stelle, die unter `anhaenge/` oder `.eingang/` schreibt, verschiebt oder löscht, ändert genau einen der beiden Zähler.
3. … `.eingang/` voll mit Waisen liegt, weil ein Fehlerpfad Teildateien nicht löscht und die Reservierung nie frei wird. Früh-Indikator: `belegt` minus Σ Index minus Σ Anhänge wächst ohne offene Hochladungen; `du .eingang`. Gegenmaßnahme: Start leert den Eingang; Ablauf im Sekundentakt; Mutationsprobe „Abbruch löscht Teildatei nicht“.
4. … Agenten den Ablauf von 15 min nicht kennen und Anhänge „verloren“ melden. Früh-Indikator: 422 `anhang` mit `abgelaufen`. Gegenmaßnahme: `gueltig_bis` in der 201; Anleitung I0019 „hochladen direkt vor dem Senden“.

**Folgeentscheidungen / Aufgaben:**
- [ ] **Betreiber** (über den Director): B1 Dateiarten, B2 Größen, B5 Nachricht nur aus Anhängen — Fragen im Bericht Karte #9; danach Status `Akzeptiert` oder neue Zahlen in Regeln 1, 4, 7.
- [ ] **Erster Unit-Test** (Gespräche mit Eingang, Speicherablage, gestellte Uhr; ersetzt: Logablage, Uhr, Zufall): Quote 1.000 Bytes, geladen ein Tag mit 600 Bytes Log und einem Anhang zu 100 Bytes → `gebunden = 700`, `reserviert = 0`. Reservieren 200 → ok, `belegt = 900`; Reservieren 101 → `Quote`, `reserviert` unverändert. Senden mit dem 200er-Anhang und einem Satz von 100 Bytes → `gebunden = 1.000`, `reserviert = 0`, Speicherablage hat die Datei unter `anhaenge/<tag>/`, das Verzeichnis kennt die ID. Satz von 1 Byte mehr → `Quote`, Datei bleibt im Eingang. Anhängen scheitert (Attrappe „Fehler nach n Bytes“) → Datei zurück im Eingang, Satz nicht im Index, `reserviert` unverändert. Ablauf nach 15 min (Uhr + 15 min + 1 s) → Eingang leer, `reserviert −=`. `pflegen` nimmt den Tag → `gebunden −= Log + Anhänge`. Nach jedem Schritt die Invariante aus Regel 8. Mutanten „Bindung zieht `reserviert` nicht ab“, „Entfallen vergisst Anhangbytes“, „Prüfung `>=`“ werden rot.
- [ ] **Unit Namensregel** (rein): `Bericht März.pdf` → gespeichert `…-Bericht_M_rz.pdf`, Anzeige gleich; `../x`, `a/b`, `a\b`, `.`, `..`, leer, 256 Bytes, `rechnung‮fdp.exe`, `a\tb`, `a\nb`, `\u{1b}[31m` → 422 `name` mit `grund`; 255 Bytes → ok; Endung `.tar.gz` bleibt `.gz`, Endung länger als 16 Zeichen entfällt.
- [ ] **Erster Integrationstest** (echtes Programm, Wegwerf-Ordner, `--plattenquote 1MiB`): Hochladen 300 KiB → 201 mit `id`, Datei unter `.eingang/`; Senden `{text:"x", anhaenge:[id]}` → 201, `channels/allgemein/anhaenge/<heute>/…` hat 307.200 Bytes, Satz trägt die Metazeile, Ereignis `nachricht` trägt `anhaenge` mit derselben ID; zweites Senden mit derselben ID → 422 `anhang`/`unbekannt`; Hochladen 800 KiB → 507 `quote` ohne Lesen (Server liest nachweislich keine Körperbytes: Client sendet nur den Kopf und wartet auf die Antwort); ohne `Content-Length` → 411; `Content-Length` 26.214.401 → 413; Neustart → `.eingang/` leer, Health `belegt` = Log + 307.200.
- [ ] **Messwert** (Entwicklertest, Release-Build, zweites Gerät im LAN): 4 parallele Hochladungen à 25 MiB, dabei `GET /api/gesundheit` p90 ≤ 0,15 s (wie ADR-0010 Ergänzung R00010) und RSS-Zuwachs ≤ 4 × 64 KiB + 50 MB.
- [ ] **Ergänzungen bei Annahme** (architekt): ADR-0003 (Eingang, Benennung), ADR-0004 (Metazeile), ADR-0010 (Körpergrenze und Frist der Hochlade-Route), ADR-0034 (Feld `anhaenge`, Wiederholschlüssel), ADR-0036 (Anhang-Konto), ADR-0042 (Bereich `anhang`: Episode „Anhänge gebremst“, `ERROR senden` beim Binden, `WARNING laden` für Metazeilen und Eingang, Ablauf als `DEBUG`), ADR-0043 (Feld `belegt`), ADR-0044 (Regel 8 hier).
- [ ] I0019 (Anleitung, `lc`): zwei Schritte, `gueltig_bis`, 507 und 413 nicht wiederholen, 429 nach `Retry-After`.

## Quellen / Bezuege

- [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00008 Logablage](../Architektur/A00008-logablage.md), [A00014 Schreibkonto](../Architektur/A00014-schreibkonto.md), [A00016 Anhangverzeichnis](../Architektur/A00016-anhangverzeichnis.md), [A00017 Anhangeingang](../Architektur/A00017-anhangeingang.md), [A00018 Anhangprüfung](../Architektur/A00018-anhangpruefung.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md)
- ADR-0003, ADR-0004, ADR-0010, ADR-0011, ADR-0013, ADR-0030, ADR-0032, ADR-0034, ADR-0035, ADR-0036, ADR-0037, ADR-0039, ADR-0042, ADR-0043, ADR-0044, ADR-0047
- Vision R00000 (Zielbild „Anhänge“, Leitplanken Plattenquote und Lastprofil); WBS I0010 (Notiz Plattenquote); Designsystem `components/Anhang/README.md`, `components/Eingabe/README.md`, `1-tastatur.md`, `3-offene-fragen.md` Frage 9
- `411 Length Required`, `413 Content Too Large`: RFC 9110 Abschnitte 15.5.12 und 15.5.14 — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet
