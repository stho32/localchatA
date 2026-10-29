# ADR-0046: Anhänge ablegen — anmelden, gestreamt in einen Eingang hochladen, beim Senden in den Tag der Nachricht binden, eine Metazeile mit Anhang-ID; ohne Grenze je Datei unter der Quote, eine Reserve bleibt dem Text

**Status**: Akzeptiert
**Datum**: 2026-10-29 (Entwurf), 2026-10-29 (akzeptiert nach den Antworten des Betreibers B1, B2, B5)
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #9 I0010); Richtung vom Betreiber (Kommentar auf Karte #9, 2026-10-29 07:40)
**Kontext-Anforderung**: — (WBS I0010, Dialog D0003, Karte #9; prägt I0012, I0015, I0016, I0019, I0022, I0023)
**Erprobt durch**: Mechanik übertragen aus Gebautem (Anhängen mit Rückbau `src/gespraeche/mod.rs:310–315` und ADR-0011; Quote-Zähler `belegt` unter der Schreibsperre ADR-0044 Regel 2; Vorab-Buchung am Schreibkonto `src/schreibkonto/`, ADR-0036; Körperlesen mit Frist `src/http/koerper.rs:215` `roh_lesen`; Löschen von `anhaenge/<tag>/` ohne Symlinks ADR-0032). **Plattenprobe** 2026-10-29 (Python, NVMe, 31 GiB RAM, `vm.dirty_ratio` 20, Probeordner unter `target/`, danach gelöscht): kleine Sätze anhängen wie die Logablage (`open(append)`, `write`, `close`, **ohne** `fsync` — im Code gibt es kein `sync_all`/`sync_data`) alle 10 ms, daneben zwei Schreibströme in 64-KiB-Stücken aus einem anderen Thread desselben Prozesses: ohne Strom p90 0,15–0,17 ms; während 2 × 1 GiB ungebremst (2.429 MiB/s) p90 0,16 ms, max 2,2 ms; während 2 × 4 GiB (über der Dirty-Grenze, 3.155 MiB/s) p90 0,13 ms, max 0,5 ms; während 2 × 1 GiB mit 105 MiB/s (Gigabit-LAN) p90 0,07 ms, max 13,2 ms. Ergebnis: großes Hochladen bremst den Schreibpfad der Sätze auf diesem Rechner nicht; Festplatten (HDD) und Rechner mit wenig RAM sind nicht geprobt (Messwert QA3).

## Kontext

- **Vision**, Zielbild „Anhänge“: „An eine Nachricht lassen sich eine oder mehrere Dateien hängen.“ Leitplanken: Datenhaltung in täglich rotierenden Logdateien, kein Git, keine Datenbank; Lastprofil „Team-LAN“; **Plattenquote** („an der Grenze wird Senden abgewiesen, deswegen wird nichts gelöscht“). WBS I0010, Fertig-Kriterium: „eine oder mehrere Dateien hängen an einer Nachricht und lassen sich öffnen“; Notiz: Bytes jeder gespeicherten Datei zählen am Zähler `belegt`, geprüft vor dem Ablegen, abgezogen mit dem Tag.
- **Antworten des Betreibers** (Karte #9, 2026-10-29) auf die Fragen des Entwurfs:
  - **B1** „jede Datei annehmen; im Browser öffnen Bilder, PDF, Text und Video/Audio (MP4, WebM, MP3, Ogg — Player des Browsers); alles andere nur Laden.“ → keine Erlaubnisliste beim Annehmen; die Arten-Tabelle steht in [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md).
  - **B2** „Im LAN darf die Dateigröße fast unbegrenzt sein. Die Übermittlung sollte aber non-blocking erfolgen und die Festplattenquota darf darüber nicht verletzt werden.“ → **keine feste Grenze je Datei** unterhalb der Quote; Hochladen blockiert weder den Chat noch andere Teilnehmer noch andere Hochladungen; Reservierung gegen die Quote **vor** dem Schreiben.
  - **B5** „Nachricht nur aus Anhängen ohne Text: erlaubt.“
- **Vorentschieden** in [ADR-0003](0003-ablage-layout-im-ordner.md): Anhänge liegen unter `<gespräch>/anhaenge/<tag>/<zeit>-<n>-<dateiname>` im Tag der Nachricht und entfallen mit ihm; erst die Datei (temporär + `rename`), dann der Satz. [ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md): Metazeile `+ anhang: …`, unbekannte Schlüssel bleiben erhalten. [ADR-0030](0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) Regel 8 und [ADR-0032](0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md) Regel 7: `anhaenge/<tag>/` wird nach der Tagesdatei gelöscht, Symlinks werden nie verfolgt.
- **Senden heute** ([ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md)): JSON `{text}`, Körper ≤ 131.072 Bytes, Text 1–16.384 Bytes, ein weiteres Feld → 422 `feld`; Option „`multipart` von Anfang an“ dort als tote Flexibilität verworfen.
- **Körper** ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), Ergänzungen): jede Route nennt ihre Körpergrenze mit Herleitung; Lesefrist 10 s als Gesamtfrist — „eine künftige Route mit großem Körper (Anhänge, I0010) braucht eine eigene, aus ihrer Größe hergeleitete Frist oder Mindestrate“. Wird ein Körper abgewiesen und ungelesen geschlossen, setzt das Betriebssystem die Verbindung zurück und der Client sieht einen Abbruch statt der Antwort (`src/http/koerper.rs:1–5`; verworfen wird darum höchstens 16 MiB in 2 s). Laufzeit: `max_blocking_threads(64)`, Werkbank = Kerne (ADR-0039 Regeln 4, 5); Senden läuft über `spawn_blocking` (`src/http/senden.rs:89`).
- **Browser** ([ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md), dort geprobt): HTTP/1.1 hat je Host höchstens 6 Verbindungen je Browser; der Ereignisstrom belegt eine. `fetch` meldet keinen Fortschritt beim Hochladen, `XMLHttpRequest` (`upload.onprogress`) schon; Browser senden kein `Expect: 100-continue`.
- **Schreibkonto** ([ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md)): Kosten ⌈Bytes / 4.096⌉, Vorrat je Kennung 30, je Herkunft 120; eine Herkunft schreibt im schlimmsten Fall ≈ 66 KB/s Text (Ergänzung Karte #31).
- **Quote** ([ADR-0044](0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md)): `belegt` im Baustein Gespräche ist einziger Halter; Standard 4 GiB; Warnschwelle 90 %; Pre-Mortem 1: „die Quote ist voll und morgens kann niemand senden“; Pre-Mortem 2: jede Interaction, die Gesprächsbytes schreibt, nennt ihren Anteil am Zähler.
- **Große Antworten** ([ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md) Regel 7c): Dateien nie ganz in den Speicher, in Stücken mit Gegendruck.
- **Designsystem** (`Dokumentation/Wireframes/designsystem/components/Anhang/README.md`, `components/Eingabe/README.md`, `1-tastatur.md`): „Hochladen läuft: Balken mit Prozent und ‚Abbrechen‘“, „Fehlgeschlagen … ‚Erneut‘“, „Zu groß: sofort beim Auswählen geprüft“; `3-offene-fragen.md` Frage 9 („welche Grenze gilt?“) beantwortet B2.
- **Sitzung** ([ADR-0012](0012-identitaet-und-sitzung.md)): Nachweis nur über den Header `X-LocalchatA-Sitzung`, kein Cookie, kein Token in der URL.

Szenarien mit Maß (ergänzen [A00001](../Architektur/A00001-localchata.md) Q11, Q12):

- **QA1 Senden**: ein Mensch hängt drei Bildschirmfotos (je ≈ 300 KB) an und sendet → die Nachricht steht mit allen dreien in jeder offenen Sicht < 1 s nach dem Ende des letzten Hochladens (Q3 ab der Bindung); ein Agent sendet einen 5-MB-Bericht mit drei Aufrufen ohne Browser.
- **QA2 Nichts halb**: bricht das Hochladen ab, scheitert das Anhängen des Satzes oder stürzt die Instanz zwischen Datei und Satz ab → es gibt nie einen Satz, dessen Anhang fehlt; eine verwaiste Datei zählt an der Quote und entfällt mit ihrem Tag; nicht gebundene Hochladungen und Teildateien sind nach ihrer Frist bzw. nach einem Neustart weg.
- **QA3 Groß und nicht blockierend** (B2): zwei Teilnehmer laden gleichzeitig je 1 GiB hoch → währenddessen `GET /api/gesundheit` p90 ≤ 0,15 s, Senden einer Textnachricht (Antwort) p90 ≤ 0,25 s und Zustellung im Strom < 1 s (Q3) — auch aus dem Tab, der gerade hochlädt; ein dritter Teilnehmer lädt 1 MiB in der Zeit hoch, die er ohne die beiden bräuchte (± 50 %); der Speicher der Instanz wächst je laufender Hochladung höchstens um ein Stück (≤ 1 MiB) plus 50 MB gesamt.
- **QA4 Quote-Erschöpfung durch einen Teilnehmer**: eine Herkunft lädt hoch, bis nichts mehr geht → die Quote wird nie überschritten, Anhänge werden mit 507 abgewiesen, **Text geht weiter**: es bleiben 10 % der Quote (Standard 409,6 MiB) für Sätze — beim Dauerprofil (5 Nachrichten/s) Tage, bei einer zusätzlichen Textflut einer Herkunft (≈ 66 KB/s) ≈ 1,8 h; die Warnschwelle 90 % (ADR-0044 Regel 7) meldet genau den Augenblick, in dem der Platz für Anhänge voll ist.
- **QA5 Kosten**: Laden eines Ordners mit 1.000 Anhängen kostet ≤ 50 ms zusätzlich (ein `stat` je Datei, Metazeilen aus dem ohnehin gelesenen Rohtext) — **gerechnet, nicht gemessen**.

## Optionen

### Frage 1: Wo und wie liegt ein Anhang?

#### Option O1: Im Ordner des Gesprächs unter dem Tag der Nachricht, Name lesbar, Anhang-ID nur in der Metazeile (gewählt)
`channels/<name>/anhaenge/<JJJJ-MM-TT>/<hhmmssmmm>-<n>-<bereinigter-name>`; der Satz trägt je Datei `+ anhang: <id> <bytes> <art> <maße> <datei> <anzeigename>`.
- Vorteile: genau das Layout von ADR-0003 — der Tag entfällt mit einem Ordner, `ls` zeigt lesbare Namen in Sendefolge, jedes Gespräch bleibt für sich kopierbar und mit `less` lesbar (Q5); die Anhang-ID (128 Bit Zufall, [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md)) steht nur im Log, nicht im Dateinamen.
- Nachteile: zum Ausliefern braucht die Instanz eine Zuordnung ID → Datei im Speicher (beim Laden aus den Metazeilen gebaut); keine Dubletten-Erkennung.

#### Option O2: Zentraler Speicher `anhaenge/<id>` mit Verweiszählung
- Vorteile: kurze Pfade; gleiche Datei in zwei Nachrichten nur einmal (bei Inhalts-Hash) — bei großen Dateien (B2) spart das echten Platz.
- Nachteile: Entfallen braucht eine Verweiszählung über alle Gespräche und Tage — eine zweite Löschlogik neben ADR-0030; Archivieren (I0022) nimmt die Anhänge nicht mit; der Ordner eines Gesprächs ist nicht mehr vollständig.

#### Option O3: Inhalt im Log als Base64-Metazeile
- Vorteile: eine Datei je Tag, atomar mit dem Satz.
- Nachteile: das Log wird unlesbar (Q5), der Verlaufsindex hält den Rohtext im Speicher (ADR-0006) — mit B2 lägen Gigabytes im RAM.

#### Option O4: Anhang-ID als Dateiname (`anhaenge/<tag>/<id>`), Anzeigename nur im Satz
- Vorteile: Auslieferung ohne Zuordnung im Speicher.
- Nachteile: `ls` zeigt Zufallszeichen statt Namen (Q5 für Anhänge verloren); die URL müsste Gespräch und Tag tragen; die ID stünde in jedem Verzeichnislisting.

### Frage 2: Wie kommt die Datei zur Instanz? (nach B2 neu gefasst)

Mit „fast unbegrenzt“ ändert sich die Lage: eine Abweisung nach dem Kopf (Quote, Platz, Konto) kann der Server nicht mehr durch Verwerfen des Rests sauber beantworten — bei Gigabytes schließt er ungelesen, das Betriebssystem setzt zurück, und der Browser (ohne `Expect: 100-continue`) sieht „Netzwerkfehler“ mitten im Hochladen statt „kein Platz“.

#### Option U1: Ein Rohkörper je Datei `POST /api/anhaenge?name=…`, Grenzen über `Content-Length` vor dem Lesen (Entwurf)
- Vorteile: ein Aufruf je Datei; kein Parser; Grenzen greifen vor dem ersten geschriebenen Byte.
- Nachteile: jede Abweisung vor dem Lesen kommt im Browser als Abbruch an — „kein Platz“ ist für Menschen nicht unterscheidbar von einem Netzfehler, und der Browser hat bis dahin schon Megabytes gesendet; Abbrechen ist nur das Schließen der Verbindung.

#### Option U5: Anmelden, dann hochladen — `POST /api/anhaenge` mit `{name, bytes}` reserviert und antwortet als JSON, `PUT /api/anhaenge/{id}` streamt genau so viele Bytes, `DELETE` gibt frei (gewählt)
- Vorteile: **jede** Abweisung (Name, offene Hochladungen, Konto, Quote, Platte) kommt als kleine JSON-Antwort, **bevor** ein Byte der Datei unterwegs ist — der Browser zeigt „kein Platz“ statt „Fehlgeschlagen“; der `PUT` hat nur noch Abweisungen, die ein gutartiger Client nie auslöst (falsche Länge, zu langsam); „Abbrechen“ ist `DELETE` und gibt die Reservierung sofort frei; „Erneut“ ist ein neuer `PUT` auf dieselbe Reservierung; Fortschritt je Datei über `XMLHttpRequest`; Agenten brauchen drei `curl`-Aufrufe mit denselben Headern; die Senderoute bleibt JSON mit kleiner Grenze; `PUT` und `DELETE` sind keine „einfachen“ Methoden — ein fremder Ursprung käme nur über eine CORS-Vorabfrage, die die Instanz nie beantwortet (ADR-0014 Regel 4).
- Nachteile: ein Aufruf mehr je Datei; ein Zustand mehr (angemeldet, noch ohne Daten) mit eigener Frist.

#### Option U6: Ein Aufruf mit `Expect: 100-continue` — Server antwortet 507 vor dem `100 Continue`
- Vorteile: ein Aufruf; `curl` sendet den Kopf bei großen Körpern von selbst so; hyper schickt `100 Continue` erst, wenn der Handler den Körper liest.
- Nachteile: Browser senden `Expect` nie — der Fall, der zählt (Menschen mit großen Dateien), bliebe ungelöst.

#### Option U7: Wiederaufnehmbares Hochladen in Teilen (Offset je `PATCH`, Muster „tus“)
- Vorteile: ein Abriss nach 1,5 von 2 GiB kostet nur den Rest.
- Nachteile: Protokoll mit Offset-Zustand je Hochladung, Prüfung lückenloser Teile, mehr Fehlerwege; im LAN sind Abrisse selten. U5 lässt sich später um `Content-Range` im `PUT` erweitern, ohne die Anmeldung zu ändern — **Folgeoption** bei beobachteten Abbrüchen.

#### Verworfen aus dem Entwurf
- U2 `multipart/form-data` in der Senderoute (zweite Körperform, Parser-Abhängigkeit, kein Fortschritt je Datei, einfacher Cross-Origin-Request), U3 Base64 im JSON (bei B2 Gigabytes im RAM), U4 Nachricht zuerst, Dateien danach (Leser sähen Anhänge, die es nie gibt; widerspricht ADR-0003 und ADR-0011).

### Frage 3: Größengrenzen — entschieden vom Betreiber (B2)

| | je Datei | Stärke | Schwäche |
|---|---|---|---|
| G1 (Entwurf) | 25 MiB, 100 MiB je Nachricht | Quote-Druck klein | widerspricht B2 („fast unbegrenzt“) — verworfen |
| G4 | einstellbar | jeder Betrieb wählt | dritte Einstellung neben der Quote ohne Szenario — verworfen |
| **G5 (gewählt)** | **keine feste Grenze**: höchstens der freie **Platz für Anhänge** (Quote minus Textreserve minus belegt) und der freie Platz der Platte minus 1 GiB | B2 wörtlich; eine Zahl weniger; „zu groß“ heißt immer „passt gerade nicht“ und nennt, wie viel passt | „Zu groß“ lässt sich nicht beim Auswählen gegen eine feste Zahl prüfen — die Antwort gibt erst die Anmeldung (Frage 2) |

Fest unabhängig von der Größe: **höchstens 10 Dateien je Nachricht** (die Liste im JSON-Körper der Senderoute bleibt klein; kein Bezug zu B2).

### Frage 4: Wer bremst die Bytes? (nach B2 neu gefasst)

B2 verlangt zweierlei, das sich mit dem Entwurf (K1) nicht verträgt: eine 2-GiB-Datei muss gehen, und die Quote darf nicht verletzt werden. Der Schutz, den K1 bot — Stunden Vorwarnung, bevor eine flutende Herkunft die Quote füllt —, verträgt sich mit „fast unbegrenzt“ nicht: wer eine Datei von der Größe der halben Quote hochladen darf, füllt sie in Minuten. Was bleibt zu schützen, ist **der Chat selbst** (B2: „non-blocking“; ADR-0044 Pre-Mortem 1).

#### Option K1: Eigenes Eimerpaar in Bytes (Entwurf: Vorrat 100 MiB je Kennung, 200 MiB je Herkunft, Nachschub 32/64 KiB/s)
- Vorteile: Vorwarnzeit der Quote ≥ 5 h bei Flut.
- Nachteile: eine Datei über 100 MiB ginge **nie** — widerspricht B2. Verworfen.

#### Option K6: Byte-Eimer mit großem Vorrat (z. B. halbe Quote) und kleinem Nachschub
- Vorteile: eine große Datei geht; Dauerflut danach gebremst.
- Nachteile: der erste Schub füllt die halbe Quote ohne Bremse — die Vorwarnung, für die der Eimer da wäre, gibt es genau beim gefährlichen ersten Mal nicht; die zweite große Datei desselben Tages wartet Stunden („fast unbegrenzt — einmal am Tag“); zusätzlicher Zustand ohne Schutzgewinn. Verworfen.

#### Option K3: Keine Bremse — nur Quote, offene Hochladungen und Platte
- Vorteile: kein Code im Konto; Menschen warten nie.
- Nachteile: eine Herkunft füllt die Quote mit Anhängen in Minuten — danach ist die Instanz für **alle** stumm, auch für Text (ADR-0044 Pre-Mortem 1). Verworfen.

#### Option K5: Textreserve in der Quote, Anmeldung kostet am bestehenden Schreibkonto, Anhänge in Blockmaß gezählt (gewählt)
- **Textreserve**: Anhänge dürfen `belegt` höchstens bis `hoechstens − reserve` treiben, `reserve` = 10 % der Quote (Standard 409,6 MiB); Sätze dürfen bis zur vollen Quote. Ist der Platz für Anhänge voll, geht Text weiter (QA4).
- **Anmeldung kostet 1 Einheit** am Text-Schreibkonto (ADR-0036 Regel 3 „Arbeit vor dem Parse kostet“) — das bremst Fluten vieler Anmeldungen wie Fluten von Nachrichten, ohne große Dateien zu bremsen.
- **Blockmaß**: jeder Anhang zählt an der Quote mit ⌈Bytes / 4.096⌉ × 4.096 Bytes — eine Flut winziger Dateien erschöpft die Quote, bevor sie Inodes und Verzeichnisse der Platte erschöpft (4 GiB / 4 KiB ≈ 1 Mio Dateien höchstens).
- Vorteile: große Dateien gehen sofort, so groß der Platz ist (B2); kein neuer Eimer, kein neuer Zustand über Neustarts; der Chat bleibt bei voller Anhangquote bedienbar; das Monitoring sieht den Zustand an der vorhandenen 90-%-Schwelle.
- Nachteile: **keine Vorwarnzeit gegen Anhänge** — ein Teilnehmer kann den Platz für Anhänge in Minuten füllen, danach lädt bis zum Entfallen von Tagen niemand mehr etwas hoch (Text geht weiter); 10 % der Quote stehen Anhängen nie zur Verfügung.

#### Option K7: Anteil je Herkunft am Platz für Anhänge (z. B. höchstens die Hälfte)
- Vorteile: ein Teilnehmer allein kann den Anhangplatz nicht füllen.
- Nachteile: braucht die Herkunft je gebundenem Anhang über Neustarts (IP-Adresse in der Metazeile oder Verlust beim Start); 50 Agenten auf einem Rechner teilen sich eine Herkunft; ob ein Teilnehmer den ganzen Platz belegen darf, ist eine **fachliche** Frage (Fairness) — gesammelt als offene Frage F1 an den Betreiber, **Folgeoption** zu K5.

### Frage 5: Wie lange lebt ein nicht gebundener Anhang?

- **E1' (gewählt)**: Eingang `<ordner>/.eingang/`; eine **Anmeldung ohne laufendes Hochladen verfällt nach 60 s** (ab Anmeldung oder ab einem abgebrochenen `PUT`); ein **vollständig hochgeladener** Anhang gilt **15 min**; beim Start wird der Eingang geleert. Stärke: eine Reservierung ohne Daten bindet die Quote höchstens eine Minute; ein Abbruch des Tabs hinterlässt nichts Dauerhaftes; die Oberfläche sendet die Nachricht unmittelbar nach dem letzten Hochladen (Regel 13), 15 min decken Störungen und Agenten. Schwäche: wer nach einem Abbruch länger als 60 s wartet, meldet neu an (der Browser tut das selbst).
- E2: Ablauf 24 h (Entwürfe mit Anhängen überleben eine Pause) — Reservierungen von Gigabytes hielten die Quote einen Tag; der Entwurf (ADR-0038) kann ohnehin keine Dateien halten. Verworfen.

### Frage 6: Anteil an der Quote (ADR-0044 Pre-Mortem 2)

- **Q-A (gewählt)**: reservieren bei der Anmeldung, binden beim Senden, abziehen mit dem Tag; `belegt = gebunden + reserviert`; Anhänge im Blockmaß (Frage 4). Stärke: geprüft wird, **bevor** ein Byte auf der Platte liegt (B2: „Festplattenquota darf nicht verletzt werden“); was reserviert ist, kann keine zweite Hochladung bekommen. Schwäche: ein dritter Zustand neben „im Index“ und „entfallen“.
- Q-B: erst beim Senden prüfen — der Eingang füllte die Platte außerhalb der Quote, bei B2 um Gigabytes. Verworfen.
- Q-C: Eingang nicht zählen, eigene Grenze — zweite Zahl für denselben Zweck. Verworfen.

### Frage 7: Was heißt „non-blocking“ im Bau? (neu, B2)

Was ein großes Hochladen blockieren könnte, und die Antwort je Weg:

| Weg | Gefahr | Antwort (Regel) |
|---|---|---|
| Sperren | Gesprächs- oder Schreibsperre während des Hochladens | Hochladen hält keine Sperre; die Schreibsperre nur für O(1)-Schritte: Reservieren, Abschluss eintragen, Binden per `rename` (Regeln 1, 2, 6) |
| Threads | 64 Blocking-Threads; Senden braucht einen (`senden.rs:89`); 16 Hochladungen je Herkunft × mehrere Herkünfte | **Hochladebank**: eigene Semaphore mit 8 Erlaubnissen für das Schreiben der Stücke; höchstens 32 offene Hochladungen gesamt — Hochladungen belegen nie mehr als 8 Blocking-Threads und nie die Werkbank (Regel 2.3) |
| Speicher | Körper im RAM | gestreamt, je Hochladung ≤ 1 Stück (≤ 1 MiB bei Gegendruck) |
| Platte | Rückschreiben großer Mengen bremst das Anhängen der Sätze | geprobt (oben): p90 unverändert ≈ 0,1 ms; Messwert auf dem Zielrechner (QA3) |
| Browser | 6 Verbindungen je Host; Strom belegt 1 | höchstens **2 Hochladungen je Browser** gleichzeitig (Web Locks über alle Tabs), weitere warten — mindestens 3 Verbindungen bleiben für Senden und Verlauf (Regel 13) |
| Sendeschlange im Tab | ein Sendeauftrag zugleich je Tab (ADR-0037) — eine Nachricht mit 2-GiB-Anhang hielte jede folgende Textnachricht fest | die Nachricht mit Anhängen tritt erst **nach** dem letzten Hochladen in die Sendeschlange; Text, der in der Zwischenzeit gesendet wird, geht vor (N1, Regel 13) |
| Andere Hochladungen | eine große belegt alles | offene Hochladungen je Kennung 4, je Herkunft 16, gesamt 32; Platz reserviert je Hochladung |

Varianten zur Sendeschlange: **N1 (gewählt)** Nachricht mit Anhängen wartet außerhalb der Schlange, bis ihre Dateien oben sind — Stärke: Text fließt weiter (B2); Schwäche: die Nachricht erscheint nach Text, der nach ihr eingegeben wurde (ihre Zeit vergibt der Server beim Binden). N2 Schlange wartet auf das Hochladen (Entwurf) — blockiert den Chat des Tabs für die Dauer des Hochladens, widerspricht B2. N3 Nachricht sofort, Dateien nachreichen — U4, verworfen.

## Entscheidung

**Gewählt: O1, U5, G5, K5, E1', Q-A, N1.**

Es überwiegt, dass U5 jede Abweisung beantwortet, bevor eine große Datei unterwegs ist, und Abbrechen und Erneut ohne Sonderwege trägt; dass G5 die Antwort des Betreibers wörtlich umsetzt und die Quote trotzdem vor dem ersten Byte schützt; dass K5 nicht die Größe bremst, sondern den Chat schützt — eine Reserve für Text, Kosten je Anmeldung am vorhandenen Konto, Blockmaß gegen Kleinstdatei-Fluten —, und dass Hochladen weder eine Sperre noch die Werkbank noch mehr als 8 Blocking-Threads belegt. In Kauf genommen: ein Aufruf mehr je Datei; keine Vorwarnzeit, wenn ein Teilnehmer den Platz für Anhänge füllt (Text geht weiter; Fairness je Herkunft ist offene Frage F1); 10 % der Quote stehen Anhängen nie zur Verfügung; eine Nachricht mit Anhängen erscheint nach Text, der während ihres Hochladens gesendet wurde.

**Akzeptiert**, weil die Richtungsfragen beantwortet sind (B1, B2, B5) und die tragenden Annahmen geprobt oder aus Gebautem übertragen sind (Plattenprobe; Sperren, Konto und Quote aus ADR-0035, ADR-0036, ADR-0044). Die Aussage „blockiert nichts“ belegt für den Bau der Messwert QA3 im Entwicklertest; fällt er, ist das ein Befund an der Umsetzung, nicht an dieser Entscheidung — es sei denn, die Plattenseite fällt auf dem Zielrechner (dann neue Entscheidung, Pre-Mortem 4).

Regeln:

1. **Anmelden** `POST /api/anhaenge` unter `grenzschutz`, Extraktor `Teilnehmer` (Mensch und Agent gleich), Körper JSON `{name, bytes}`, **Körpergrenze 2.048 Bytes** (Herleitung: Name ≤ 255 Bytes, also ≤ 255 Zeichen, längste Schreibweise je Zeichen `\uXXXX` = 1.530, dazu `{"name":"","bytes":18446744073709551615}` = 40, plus 30 % ≈ 2.041 → 2.048; unter 16 KiB, also Parse auf dem Handler, ADR-0010 Ergänzung Regel 2). Prüfreihenfolge (abgewiesen, nie umgeformt):
   1. fremder Origin/Host 403 · ohne Sitzung 401 · Vorab-Buchung **1 Einheit** am Schreibkonto (ADR-0036 Regel 3) → 429 · Körperfehler 415/400/413/408 (eine Antwort, ADR-0010 Ergänzung Karte #31) · weiteres Feld → 422 `{regel: "feld"}`;
   2. `bytes` keine ganze Zahl in u64 → 422 `{regel: "bytes"}`; `bytes` = 0 → 422 `{regel: "leer"}`;
   3. `name` verletzt die Namensregel (Regel 4) → 422 `{fehler, regel: "name", grund}`;
   4. offene Hochladungen (angemeldet, laufend oder fertig, nicht gebunden) über **4 je Kennung, 16 je Herkunft, 32 gesamt** → 429 `{fehler, grenze: "offen_kennung" | "offen_herkunft" | "offen_gesamt", hoechstens}` mit `Retry-After: 5`;
   5. **Platte**: `bytes + reserviert + 1 GiB > frei` (`frei` = `f_bavail × f_frsize` aus `statvfs` des Ordners über `rustix`, Merkmal `fs` — keine neue Abhängigkeit) → **507** `{fehler, regel: "platte", frei}`; gilt auch bei `--plattenquote aus`;
   6. **Quote** (unter der Schreibsperre): `gebunden + reserviert + blockmass(bytes) > hoechstens − reserve` → **507** `{fehler, regel: "quote", belegt, hoechstens, frei_fuer_anhaenge}`, Text „Nicht hochgeladen: der Platz für Anhänge ist voll — noch <n> frei; Text kann weiter gesendet werden.“ (Zahlen wie ADR-0044 Regel 4); sonst `reserviert += blockmass(bytes)`;
   7. Erfolg → **201** `{anhang: {id, name, bytes, hochladen: "/api/anhaenge/<id>", anlauf_bis}}`; Zustand „angemeldet“, Anlauffrist 60 s.
2. **Hochladen** `PUT /api/anhaenge/{id}` unter `grenzschutz`, Extraktor `Teilnehmer`, Rohkörper. Prüfreihenfolge, alles **vor** dem ersten gelesenen Byte:
   1. 403 · 401 · `id` unbekannt, verfallen oder einer anderen Kennung → **404** `{fehler, grund: "unbekannt" | "abgelaufen"}` (fremd erscheint als `unbekannt`) · schon fertig oder ein `PUT` läuft → **409** `{regel: "laeuft" | "fertig"}`;
   2. `Content-Type` nicht `application/octet-stream` → 415 · ohne `Content-Length` (auch `chunked`) → 411 · `Content-Length` ≠ angemeldetes `bytes` → 422 `{regel: "laenge", erwartet}`; jede dieser Abweisungen mit `Connection: close` (ein gutartiger Client löst sie nie aus);
   3. **Lesen und Schreiben**: Stücke, wie hyper sie liefert, nach `.eingang/<id>.teil` (`O_CREAT|O_EXCL`); jedes Stück in `spawn_blocking` unter einer Erlaubnis der **Hochladebank** (eigene `tokio::sync::Semaphore`, **8** Erlaubnisse — nie die Werkbank, nie eine Gesprächs- oder Schreibsperre); dabei Art und Maße bestimmen ([A00018](../Architektur/A00018-anhangpruefung.md); Text-Prüfung über alle Stücke mit Übertrag an den Stückgrenzen);
   4. **Fristen**: **Stückfrist** — kommen 10 s lang keine weiteren 64 KiB an (bzw. der Rest, wenn weniger fehlt) → 408; **Gesamtfrist** = 60 s + `bytes` / 1 MiB/s (2 GiB → ≈ 35 min) → 408; Text in der Form von ADR-0010 Ergänzung Karte #26 („Der Körper kam nicht … vollständig an — Anfrage abgebrochen; über eine schnellere Verbindung erneut senden.“). Herleitung: Mindestrate 1 MiB/s ≈ 8 Mbit/s liegt weit unter jedem LAN (B2: „im LAN“) und begrenzt, wie lange ein Tröpfler eine Reservierung hält;
   5. **Abbruch** (Fristen, Lesefehler, Client weg, weniger Bytes als angekündigt, Ende der Instanz): Teildatei löschen, Zustand zurück auf „angemeldet“, Anlauffrist 60 s neu — „Erneut“ ist ein neuer `PUT` auf dieselbe ID; danach verfällt die Reservierung;
   6. Erfolg: kein `fsync` (wie bei den Sätzen); `rename` nach `.eingang/<id>` → **200** `{anhang: {id, name, bytes, art, breite, hoehe, gueltig_bis}}`; Zustand „fertig“, gültig 15 min.
3. **Abbrechen** `DELETE /api/anhaenge/{id}` (Sitzung, eigene ID) → **204**, auch wenn die ID schon weg ist (idempotent; fremde IDs bleiben unberührt); ein laufender `PUT` dieser ID endet als Abbruch (Regel 2.5), Teildatei und Reservierung sind sofort frei.
4. **Namensregel** (rein, [A00018 Anhangprüfung](../Architektur/A00018-anhangpruefung.md)): Anzeigename 1–255 Bytes UTF-8; **abgewiesen**: die verbotenen Zeichen aus ADR-0034 Regel 3.6 sowie Tab, Zeilenumbruch, `\r`; Richtungszeichen U+200E, U+200F, U+202A–U+202E, U+2066–U+2069; `/`, `\`, `"`; die Namen `.` und `..`. Gespeicherter Dateiname: `<hhmmssmmm>-<n>-<ascii>` mit `<ascii>` = Anzeigename, jedes Zeichen außerhalb `[A-Za-z0-9._-]` zu `_`, höchstens 64 Zeichen (Endung bleibt, wenn sie ≤ 16 Zeichen hat), nie leer (`datei`). Der Anzeigename erscheint nur in der Metazeile, in der API und in `Content-Disposition` (ADR-0047).
5. **Senden mit Anhängen** — [ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) wächst um ein Feld: Körper `{text, anhaenge?: [<id>, …]}`, 1–10 IDs, keine doppelt. **Text leer ist erlaubt, wenn mindestens ein Anhang dabei ist** (B5); ohne Text und ohne Anhang bleibt 422 `leer`. Neue Abweisungen nach Schritt 3.6 von ADR-0034: unbekannte, verfallene, fremde, doppelte oder noch nicht fertig hochgeladene ID → 422 `{fehler, regel: "anhang", id, grund: "unbekannt" | "abgelaufen" | "fremd" | "doppelt" | "unvollstaendig"}`; mehr als 10 → 422 `{regel: "anhaenge", hoechstens: 10}`. Kosten am Text-Konto bleiben ⌈Bytes des Texts / 4.096⌉, mindestens 1. Der Wiederholschlüssel prüft Text **und** Anhang-IDs (Prüfsumme über beide); eine Wiederholung mit bekanntem Schlüssel antwortet 200 ohne Bindung.
6. **Bindung** in `Gespraeche::senden` unter Gesprächs- und Schreibsperre ([ADR-0035](0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md)), nach der Prüfung des Wiederholschlüssels: (a) Zeit vergeben, Tag bestimmen; (b) Quote nur noch für den Satz: `gebunden + reserviert + Bytes(Satz) > hoechstens` → 507 (die Anhänge sind schon reserviert; der Satz darf in die Textreserve); (c) je Anhang `rename(.eingang/<id>, <gespräch>/anhaenge/<tag>/<datei>)` — O(1), unabhängig von der Größe; der Ordner des Tages wird bei Bedarf angelegt; (d) Satz mit den Metazeilen anhängen (ADR-0011); (e) erst danach `gebunden += Bytes(Satz) + Σ blockmass(Anhang)`, `reserviert −= Σ blockmass(Anhang)`, Einträge ins [Anhangverzeichnis](../Architektur/A00016-anhangverzeichnis.md), Eintrag aus dem Eingang, Ereignis. Scheitert (c) oder (d): bereits verschobene Dateien zurück nach `.eingang/` (`rename`), Satz zurückgebaut (ADR-0011), 500, nichts im Index, kein Ereignis, Reservierung bleibt (die Anhänge sind bis zum Ablauf weiter sendbar). Scheitert das Zurückverschieben, bleibt die Datei als Waise im Tag — sie zählt nach dem nächsten Start an der Quote und entfällt mit dem Tag (QA2); `ERROR senden` mit Datei und Grund.
7. **Metazeile** (Präzisierung von [ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md); eine je Anhang, in Sendefolge, nach `+ art:`):
   `+ anhang: <id> <bytes> <art> <breite>x<hoehe>|- <datei> <anzeigename>`
   — `<id>` 26 Zeichen `[a-z2-7]`, `<bytes>` dezimal (u64), `<art>` aus der Tabelle von ADR-0047 (`image/png`, …, `video/mp4`, `audio/ogg`, …, `application/octet-stream`), Maße nur für Bilder, `<datei>` der gespeicherte Name (ohne Leerzeichen, Regel 4), `<anzeigename>` der Rest der Zeile. Beispiel:
   `+ anhang: k3v7q2m5xpc4h6n6tewaz2b5yr 2457600 application/pdf - 140712345-1-Bericht_M_rz.pdf Bericht März.pdf`
   Eine Metazeile, die dieser Grammatik nicht folgt, ist kein Defekt des Satzes (ADR-0004), aber der Anhang erscheint nicht im Verzeichnis; `WARNING laden` mit Datei und Zeile.
8. **Quote** (ergänzt [ADR-0044](0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) Regeln 1, 2, 4, 9): `belegt = gebunden + reserviert`; `blockmass(n) = ⌈n / 4.096⌉ × 4.096`; `reserve = hoechstens / 10` (bei `aus`: keine Quote, keine Reserve, nur die Platte nach Regel 1.5). **Laden**: `gebunden` = Σ Rohtext der Tage im Index + Σ `blockmass(Größe)` der regulären Dateien in `anhaenge/<tag>/` dieser Tage (`symlink_metadata`; Symlinks und Unterordner zählen nicht) — gezählt wird, was auf der Platte liegt, auch Waisen. **Entfallen**: je Tag `gebunden −= Bytes(Tagesdatei) + Anhangbytes(Tag)` (im Blockmaß, das Anhangverzeichnis hält die Summe je Tag). **Invariante**: `gebunden = Σ Verlauf::bytes() + Σ Anhangverzeichnis::bytes_je_tag()` und `reserviert = Σ Eingang::blockmass()` zu jedem Zeitpunkt ohne laufende Operation. Anhänge prüfen gegen `hoechstens − reserve`, Sätze gegen `hoechstens`; die 90-%-Schwelle (ADR-0044 Regel 7) gilt für `belegt` und fällt mit Standardwerten auf den Augenblick „Platz für Anhänge voll“.
9. **Entfallen**: unverändert ADR-0030 Regel 8 und ADR-0032 Regel 7 — nach der Tagesdatei `anhaenge/<tag>/` mit `remove_dir_all` (folgt keinen Symlinks), dann Einträge des Tages aus dem Anhangverzeichnis, dann der Zähler. Scheitert das Löschen, bleibt der Tag mit seinen Anhängen im Verzeichnis und im Zähler (wie ADR-0030 Regel 4).
10. **Eingang** `<ordner>/.eingang/` ([A00017 Anhangeingang](../Architektur/A00017-anhangeingang.md)): je Eintrag Kennung, Herkunft, Name, Bytes, Zustand (`angemeldet` | `laeuft` | `fertig`), Art, Maße, Frist; nur im Speicher verzeichnet. Fristen im Sekundentakt der Pflege (ADR-0030): `angemeldet` 60 s, `fertig` 15 min → Datei löschen, `reserviert −= blockmass`. Beim Start wird `.eingang/` vor dem Laden geleert (nur reguläre Dateien; ein Unterordner oder Symlink ist ein `WARNING` und bleibt). `.eingang` ist kein gültiger Channelname (ADR-0013) und liegt neben `channels/`, also auf demselben Dateisystem — `rename` ist O(1).
11. **Archivieren** (I0022): Anhänge wandern mit dem Ordner des Channels; ihre Bytes bleiben im Zähler; die Auslieferung archivierter Anhänge entscheidet I0022.
12. **Direktgespräche** (I0012): dieselbe Ablage unter `direkt/<…>/anhaenge/<tag>/`, dieselbe Bindung; Sichtbarkeit ADR-0047 Regel 2 (B4).
13. **Oberfläche** ([A00005](../Architektur/A00005-weboberflaeche.md)): Hochladen beim Senden — zuerst alle Dateien der Nachricht anmelden (eine Abweisung erscheint an der Datei, bevor etwas hochlädt), dann je Datei `PUT` über `XMLHttpRequest` mit Fortschritt (`upload.onprogress`); höchstens 2 Hochladungen je Browser zugleich (Web Locks über alle Tabs); „Abbrechen“ = `xhr.abort()` + `DELETE`; „Erneut“ = neuer `PUT`, nach Ablauf neue Anmeldung; die Nachricht tritt nach dem letzten Hochladen in die Sendeschlange (N1); bis dahin steht sie im eigenen Verlauf als lokaler Eintrag „wird hochgeladen“ mit Balken und „Abbrechen“.

## Konsequenzen

**Positiv:**
- Dateien so groß, wie der Platz reicht (B2) — eine 2-GiB-Datei geht in einem Zug; jede Grenze (Quote, Platte, offene Hochladungen, Konto) greift vor dem ersten Byte und kommt als JSON an.
- Der Chat bleibt bedienbar, während hochgeladen wird und wenn der Platz für Anhänge voll ist.
- Kein Satz ohne Datei, keine Datei außerhalb der Quote; nicht gebundene Hochladungen räumen sich selbst weg.
- Die Senderoute bleibt klein und JSON; alle Zusagen von ADR-0034 gelten weiter, ein Feld kommt dazu.
- Agent und Mensch nutzen dieselben drei Routen (`curl -X POST …/api/anhaenge -d '{"name":"bericht.pdf","bytes":2457600}'`, `curl -X PUT --data-binary @bericht.pdf -H 'Content-Type: application/octet-stream' …/api/anhaenge/<id>`, Senden mit `anhaenge`).

**Negativ / Inkauf zu nehmen:**
- Drei Aufrufe für eine Nachricht mit einem Anhang; Zwischenzustände mit Fristen (60 s, 15 min).
- Keine Vorwarnzeit, wenn ein Teilnehmer den Platz für Anhänge füllt; bis Tage entfallen, lädt niemand mehr hoch (offene Frage F1).
- 10 % der Quote bleiben Anhängen verschlossen; Kleinstdateien zählen mit 4 KiB.
- „Zu groß“ zeigt die Oberfläche erst beim Senden (Anmeldung), nicht beim Auswählen — Abweichung vom Designsystem (`components/Anhang/README.md`), Hinweis an den `ux-reviewer`.
- Unter 1 MiB/s (langsames WLAN, VPN) scheitern große Dateien an der Gesamtfrist.
- Eine Nachricht mit Anhängen erscheint nach Text, der während ihres Hochladens gesendet wurde.
- Der Start stat-et jede Anhangdatei (QA5, gerechnet); Dateinamen auf der Platte verlieren Umlaute (`Bericht_M_rz.pdf`).

**Pre-Mortem (ein Monat nach dem Bau gescheitert, weil …):**
1. … ein Mitarbeiter morgens ein 3-GiB-Abbild teilt und danach niemand mehr Bildschirmfotos senden kann. Früh-Indikator: `WARNING quote … 90 %`, 507 `quote` an Anmeldungen. Gegenmaßnahme: Text geht weiter (K5); Frist des Channels verkürzen; Quote beim Neustart anheben; Fairness je Herkunft als Folgeoption K7 (F1).
2. … `gebunden` vom Bestand abdriftet, weil ein Weg (Rückbau, Waise, Löschfehler, Archiv, Blockmaß an einer Stelle vergessen) die Anhangbytes falsch zählt. Früh-Indikator: Unit-Invariante (Regel 8) nach jeder Operation; `belegt` springt nach einem Neustart. Gegenmaßnahme: erster Unit-Test unten; Review-Punkt `code-reviewer`: jede Stelle, die unter `anhaenge/` oder `.eingang/` schreibt, verschiebt oder löscht, ändert genau einen der beiden Zähler, immer über `blockmass`.
3. … `.eingang/` voll mit Teildateien liegt, weil ein Fehlerpfad sie nicht löscht und die Reservierung nie frei wird. Früh-Indikator: `reserviert` > 0 ohne offene Hochladungen im Health-Check; `du .eingang`. Gegenmaßnahme: Start leert den Eingang; Fristen im Sekundentakt; Mutationsprobe „Abbruch löscht Teildatei nicht“.
4. … ein großes Hochladen den Chat doch bremst — auf dem Zielrechner mit Festplatte stockt das Anhängen der Sätze, während Gigabytes zurückgeschrieben werden. Früh-Indikator: Messwert QA3; Senden p90 steigt während Hochladungen. Gegenmaßnahme: neue Entscheidung — Hochladebank auf 2 Erlaubnisse, Drosselung je Hochladung, oder `sync_file_range` je Stück, damit sich keine Gigabytes schmutziger Seiten stauen.
5. … Agenten die Fristen nicht kennen und „verlorene“ Anhänge melden. Früh-Indikator: 404 `abgelaufen`, 422 `anhang`/`abgelaufen`. Gegenmaßnahme: `anlauf_bis`, `gueltig_bis` in den Antworten; Anleitung I0019 „anmelden, sofort hochladen, sofort senden“.

**Folgeentscheidungen / Aufgaben:**
- [x] **Betreiber**: B1, B2, B5 beantwortet (Karte #9, 2026-10-29).
- [ ] **Offene Frage F1** an den Betreiber (Bericht Karte #9): darf ein Teilnehmer den ganzen Platz für Anhänge belegen, oder soll eine Herkunft höchstens einen Anteil halten (K7)?
- [ ] **Erster Unit-Test** (Gespräche mit Eingang, Speicherablage, gestellte Uhr; ersetzt: Logablage, Uhr, Zufall, Plattenauskunft): Quote 100.000 Bytes (Reserve 10.000), geladen ein Tag mit 6.000 Bytes Log und einem Anhang zu 100 Bytes → `gebunden = 6.000 + 4.096`, `reserviert = 0`. Anmelden 70.000 → ok, `reserviert = blockmass(70.000) = 73.728`; Anmelden 4.097 → `quote` (10.096 + 73.728 + 8.192 > 90.000), `reserviert` unverändert; Anmelden 4.096 → ok. Senden mit dem 70.000er-Anhang und einem Satz von 100 Bytes → `gebunden = 10.096 + 73.728 + 100`, `reserviert = 4.096`, Datei unter `anhaenge/<tag>/`, Verzeichnis kennt die ID. Ein Satz, der in die Reserve fällt, geht; ein Satz über `hoechstens` → `quote`. Anhängen scheitert (Attrappe) → Datei zurück im Eingang, Satz nicht im Index, `reserviert` unverändert. Anmeldung ohne `PUT`, Uhr + 60 s + 1 ms → Reservierung frei; fertige Datei, Uhr + 15 min + 1 s → Eingang leer. `pflegen` nimmt den Tag → `gebunden −= Log + Anhänge`. Plattenauskunft „frei = bytes + reserviert + 1 GiB − 1“ → `platte`. Nach jedem Schritt die Invariante aus Regel 8. Mutanten „Bindung zieht `reserviert` nicht ab“, „Entfallen vergisst Anhangbytes“, „Blockmaß fehlt beim Laden“, „Anhang prüft gegen `hoechstens` statt `hoechstens − reserve`“, „Prüfung `>=`“ werden rot.
- [ ] **Unit Namensregel** (rein): `Bericht März.pdf` → gespeichert `…-Bericht_M_rz.pdf`, Anzeige gleich; `../x`, `a/b`, `a\b`, `a"b`, `.`, `..`, leer, 256 Bytes, `rechnung‮fdp.exe`, `a\tb`, `a\nb`, `\u{1b}[31m` → 422 `name` mit `grund`; 255 Bytes → ok; Endung `.tar.gz` bleibt `.gz`, Endung länger als 16 Zeichen entfällt.
- [ ] **Unit Fristen** (rein, `tokio::time::pause`): Stückfrist — 64 KiB, dann 9,999 s Stille → läuft, 10,000 s → 408; 1 Byte je 9 s → 408 nach 10 s ohne 64 KiB; Gesamtfrist für `bytes` = 10 MiB → 70 s; 1 MiB/s gleichmäßig 10 MiB → gelesen; 0,9 MiB/s → 408 bei 70 s.
- [ ] **Erster Integrationstest** (echtes Programm, Wegwerf-Ordner, `--plattenquote 10MiB`): Anmelden 300 KiB → 201; `PUT` 300 KiB → 200 mit `art`, Datei unter `.eingang/`; Senden `{text:"", anhaenge:[id]}` → 201 (B5), `channels/allgemein/anhaenge/<heute>/…` hat 307.200 Bytes, Satz trägt die Metazeile, Ereignis trägt `anhaenge`; zweites Senden mit derselben ID → 422 `anhang`/`unbekannt`; `{text:""}` ohne Anhang → 422 `leer`; Anmelden 9 MiB → 507 `quote` als JSON (Reserve 1 MiB); `PUT` mit `Content-Length` ≠ angemeldet → 422 `laenge` ohne Lesen; ohne `Content-Length` → 411; `PUT` abbrechen nach 100 KiB → keine `.teil`-Datei, Health `reserviert` unverändert bis 60 s; `DELETE` → 204, `reserviert` sofort 0; fremde Kennung `PUT`/`DELETE` → 404/204 ohne Wirkung; Neustart → `.eingang/` leer, Health `belegt` = Log + `blockmass(307.200)`.
- [ ] **Messwert QA3** (Entwicklertest, Release-Build, Instanz auf dem Zielrechner, Clients im LAN): zwei `PUT` à 1 GiB gleichzeitig; währenddessen alle 100 ms `GET /api/gesundheit` → p90 ≤ 0,15 s; ein dritter Client sendet 5 Textnachrichten/s → Antwort p90 ≤ 0,25 s, Zustellung im Strom < 1 s; ein vierter lädt 1 MiB hoch; im Browser: während eines 1-GiB-Hochladens Text senden → erscheint < 1 s; RSS-Zuwachs ≤ 2 × 1 MiB + 50 MB. Vergleichslauf ohne Hochladungen; Rechner und Platte (SSD/HDD) im Bericht nennen.
- [x] **Ergänzungen bei Annahme** (architekt, Karte #9): ADR-0003, ADR-0004, ADR-0010, ADR-0034, ADR-0036, ADR-0042, ADR-0043, ADR-0044 — geschrieben.
- [ ] I0019 (Anleitung, `lc`): drei Schritte, `anlauf_bis`, `gueltig_bis`, 507 nicht wiederholen, 429 nach `Retry-After`, 404 `abgelaufen` → neu anmelden.

## Quellen / Bezuege

- [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00008 Logablage](../Architektur/A00008-logablage.md), [A00014 Schreibkonto](../Architektur/A00014-schreibkonto.md), [A00016 Anhangverzeichnis](../Architektur/A00016-anhangverzeichnis.md), [A00017 Anhangeingang](../Architektur/A00017-anhangeingang.md), [A00018 Anhangprüfung](../Architektur/A00018-anhangpruefung.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md)
- ADR-0003, ADR-0004, ADR-0010, ADR-0011, ADR-0013, ADR-0014, ADR-0015, ADR-0030, ADR-0032, ADR-0034, ADR-0035, ADR-0036, ADR-0037, ADR-0039, ADR-0042, ADR-0043, ADR-0044, ADR-0047
- Vision R00000 (Zielbild „Anhänge“, Leitplanken Plattenquote und Lastprofil); WBS I0010; Betreiber-Antworten B1, B2, B5 (Karte #9, Kommentar 2026-10-29 07:40); Designsystem `components/Anhang/README.md`, `components/Eingabe/README.md`, `1-tastatur.md`, `3-offene-fragen.md` Frage 9
- `src/http/koerper.rs:1–5` (Abbruch statt Antwort bei ungelesenem Körper), `:215` (`roh_lesen`); `src/http/senden.rs:89` (`spawn_blocking`); `src/http/werkbank.rs`; `src/gespraeche/logablage.rs:153–165` (Anhängen ohne `fsync`); `Cargo.toml` (`rustix` vorhanden)
- `411`, `413`, `507`: RFC 9110 bzw. RFC 4918 — aus dem Gedächtnis, nicht geöffnet; `statvfs`: POSIX — aus dem Gedächtnis; Web Locks API, `XMLHttpRequest.upload` — aus dem Gedächtnis, im Bau zu proben (E2E)

## Ergänzungen

- **2026-10-29 (Karte #9 Gate, Befund B1 des `security-reviewer`) — absolute Lebensdauer je Anmeldung, `offen_gesamt` zählt nur laufende Hochladungen, Wettläufe unter einer Sperre:** Entscheider: Claude (autonom, architekt, Auftrag des Directors). Status der Ergänzung: Akzeptiert (die Mechanik verschärft nur Fristen und Zählungen dieser ADR; die Lücken sind vom `security-reviewer` aus Regel 2.5 und 1.4 hergeleitet).

  **Anlass** (Gate-Bericht, Kommentar auf Karte #9, 2026-10-29 08:55): Grenzen des Eingangs sind ohne Kosten dauerhaft haltbar. (a) Jeder Abbruch setzt die Anlauffrist von 60 s neu (Regel 2.5, E1′) — wer alle 50 s einen `PUT` beginnt, 1 Byte sendet und schließt, hält seine Reservierung (bis zum ganzen Platz für Anhänge) unbegrenzt, und ein `PUT` kostet am Schreibkonto nichts; die Aussage „eine Reservierung ohne Daten bindet die Quote höchstens eine Minute“ (Frage 5) und ADR-0010 Ergänzung Karte #9 Punkt (3) sind damit falsch. (b) `offen_gesamt` = 32 zählt auch `angemeldet` und `fertig`: zwei Herkünfte × 4 Kennungen × 4 Anmeldungen à 1 Byte (2 × 16 = 32) sperren das Hochladen für alle.

  **Varianten zu (a)** — was begrenzt das Leben einer Reservierung?

  | | Regel | Stärke | Schwäche |
  |---|---|---|---|
  | L0 | wie bisher: Anlauffrist nach jedem Abbruch neu | „Erneut“ geht beliebig oft | Reservierung unbegrenzt haltbar (Befund) — verworfen |
  | **L1 (gewählt)** | **absolute Lebensdauer ab Anmeldung**, durch Abbrüche nie verlängert: `lebensende = angemeldet_um + 60 s + 2 × bytes / 1 MiB/s` | eine Frist, gerechnet bei der Anmeldung, im Sekundentakt ohnehin geprüft; Raum für die Anlauffrist und zwei Übertragungen mit der Mindestrate (Erstversuch und ein „Erneut“), im LAN für viele; die Frist wächst mit der Datei wie die Gesamtfrist (Regel 2.4) | wer nach mehreren Abbrüchen auf einer langsamen Leitung das Lebensende erreicht, meldet neu an (der Browser tut das schon, Regel 13); große Anmeldungen leben länger (2 GiB ≈ 69 min) — so lange, wie ein ehrlicher Client mit Mindestrate und einem Erneut bräuchte |
  | L2 | höchstens 3 × „Erneut“ (Zähler je Anmeldung) | unabhängig von der Größe; einfach zu erklären | die Dauer bleibt Summe aus vier Gesamtfristen und drei Anlauffristen (2 GiB ≈ 2,4 h) — größer als L1; ein flackerndes WLAN verbraucht die Versuche einer kleinen Datei in Sekunden — verworfen |
  | L3 | jeder Abbruch gibt die Reservierung frei, „Erneut“ meldet neu an | kein Zustand „zurück auf angemeldet“; jedes Erneut kostet 1 Einheit am Konto | „Erneut“ verliert seinen Platz an der Quote an jeden, der in der Lücke anmeldet (507 nach einem Netzfehler); bricht mit U5 („Erneut ist ein neuer `PUT` auf dieselbe Reservierung“) und der Oberfläche (Regel 13) — verworfen |

  **Varianten zu (b)** — was zählt `offen_gesamt`?

  | | Regel | Stärke | Schwäche |
  |---|---|---|---|
  | G0 | wie bisher: alle drei Zustände, 32 | Speicher je Eintrag begrenzt | zwei Herkünfte sperren alle (Befund) — verworfen |
  | **G1 (gewählt)** | **`offen_gesamt` zählt nur `laeuft`**, Grenze **64 = 4 × `offen_herkunft`**; `angemeldet` und `fertig` zählen nur je Kennung (4) und je Herkunft (16) | geschützt wird, was knapp ist: laufende Ströme (Verbindungen, je ein Stück Speicher, Warteplätze vor der Hochladebank); eine Sperre für alle braucht vier Herkünfte mit echten, laufenden Strömen, die die Stückfrist erfüllen und an der Gesamtfrist enden | bis zu 64 Stücke ≤ 1 MiB im Speicher statt 32 (QA3 gilt je Hochladung weiter) |
  | G2 | `offen_gesamt` entfällt, nur die Hochladebank (8) | eine Zahl weniger | laufende Ströme ohne Obergrenze — Speicher und Verbindungen nur durch die Zahl der Herkünfte begrenzt — verworfen |

  **Regeln** (ergänzen bzw. ersetzen, wo genannt):

  1. **Lebensende** (ersetzt in Regel 2.5 „Anlauffrist 60 s neu“ und in Regel 10 die Frist für `angemeldet`): bei der Anmeldung wird `lebensende = jetzt + 60 s + 2 × bytes / 1 MiB/s` festgelegt (ganzzahlig in Millisekunden, aufgerundet) und nie verändert. Anlauffrist der Anmeldung: `min(jetzt + 60 s, lebensende)`; nach einem Abbruch: `min(jetzt + 60 s, lebensende)`; die Gesamtfrist eines `PUT` ist `min(beginn + 60 s + bytes / 1 MiB/s, lebensende)`. Am Lebensende verfällt ein Eintrag in `angemeldet` oder `laeuft`: Reservierung sofort frei; ein laufender `PUT` endet als Abbruch und antwortet 408 (Text wie Regel 2.4); jeder spätere `PUT` → 404 `abgelaufen`, Senden → 422 `anhang`/`abgelaufen`. `fertig` ist davon frei: gültig 15 min ab Abschluss (eine Reservierung ohne gebundene Nachricht lebt also höchstens `lebensende + 15 min`, und dann liegt die Datei vollständig auf der Platte).
  2. **Antwort der Anmeldung** (Regel 1.7): 201 `{anhang: {id, name, bytes, hochladen, anlauf_bis, hochladen_bis}}` — `hochladen_bis` ist das Lebensende.
  3. **Offene Hochladungen** (ersetzt Regel 1.4 für `offen_gesamt`): je Kennung 4 und je Herkunft 16 zählen `angemeldet`, `laeuft`, `fertig` wie bisher; `offen_gesamt` zählt nur `laeuft`, **höchstens 64**. Geprüft an zwei Stellen: bei der Anmeldung (frühe JSON-Antwort 429 `{grenze: "offen_gesamt", hoechstens: 64}`, `Retry-After: 5`) und hart beim Beginn eines `PUT` (Regel 2.1, nach 409, vor 415): 429 wie oben mit `Connection: close`, der Eintrag bleibt `angemeldet`, seine Fristen laufen weiter.
  4. **Sichtbarkeit des Haltens** (Matrix M22): 507 `quote` nennt zusätzlich `reserviert`; ein Eintrag, der verfällt, ohne je `fertig` geworden zu sein, zählt in die Episode `WARNING anhang Reservierung verfallen ohne Hochladen` je Herkunft (erste Zeile mit Kennung und `bytes`, Episoden-Mechanik ADR-0042); Health `reserviert` wie bisher.
  5. **Eine Sperre für alle Übergänge** (Matrix M23): die Einträge des Eingangs liegen mit `reserviert` unter der **Schreibsperre** (ADR-0035) — `anmelden`, `beginnen`, `abschliessen`, Abbruch, `verwerfen`, `ablaufen`, `nehmen`, `zuruecklegen` sind je ein O(1)-Übergang unter ihr; keine zweite Sperre, also keine Reihenfolge zwischen zweien. Dateiarbeit (Stücke schreiben, `unlink`) geschieht außerhalb. **Wer den Übergang verliert, räumt seine Datei**: ein `PUT`, dessen Eintrag inzwischen verworfen, verfallen oder genommen ist, löscht seine `.teil` bzw. `.eingang/<id>` selbst und antwortet 404 bzw. 408; ein `DELETE` oder Ablauf nach `nehmen` findet nichts (204 ohne Wirkung); zwei `PUT` auf dieselbe ID → genau einer `laeuft`, der andere 409 `laeuft`; dieselbe ID in zwei gleichzeitigen Sendungen (zwei Channels) → genau eine bindet, die andere 422 `anhang`/`unbekannt`.
  6. **Nie überschreiben** (Matrix M39): jedes `rename` des Eingangs und der Bindung (`.teil` → `.eingang/<id>`, `.eingang/<id>` → `anhaenge/<tag>/<datei>`, Zurücklegen) mit `RENAME_NOREPLACE` (`renameat2` über `rustix`); kann das Dateisystem es nicht (`EINVAL`), `link` + `unlink`, das ebenfalls nie überschreibt. Ein vorhandenes Ziel ist ein Fehler des Schritts: beim Binden Rückbau nach Regel 6 (500, `ERROR senden`), das vorhandene Ziel bleibt unverändert. Teildateien mit `O_CREAT|O_EXCL|O_NOFOLLOW`.
  7. **`.eingang` ist ein echter Ordner** (Matrix M26; ergänzt Regel 10): beim Start, vor dem Leeren — fehlt er, wird er angelegt; ist `<ordner>/.eingang` ein Symlink oder kein Ordner (`symlink_metadata`), endet der Start mit **Exit 2** und einer Zeile auf stderr, die den Pfad nennt; gelöscht wird nichts.
  8. **Schreibfehler beim Stück** (Matrix M24): Fehler beim Schreiben (`ENOSPC`, `EIO`) ist ein Abbruch nach Regel 2.5 — Teildatei gelöscht, Zustand `angemeldet` bis zum Lebensende; Antwort 507 `{regel: "platte", frei}` bei `ENOSPC`, sonst 500; `ERROR anhang` mit Herkunft und Grund (Datei nur als ID-Präfix, ADR-0047 Ergänzung Karte #9 Gate Regel 3).

  **Offen, nicht entschieden — Frage F1 an den Betreiber** (Stelle: K7, Folgeentscheidungen oben): darf ein Teilnehmer den ganzen Platz für Anhänge belegen? Empfehlung des Directors (aus dem Gate): keine Grenze für gebundene Anhänge (eine zweite IP genügt zum Umgehen, und B2 verlangt Dateien bis zur Größe des Platzes), Reservierungen je Herkunft höchstens 50 % des Platzes für Anhänge, nur im Speicher. Bis zur Antwort gilt diese ADR ohne Anteil je Herkunft. L1 begrenzt die einzelne Reservierung; eine Herkunft, die im Takt neu anmeldet, kann den Platz weiter halten — jede Erneuerung kostet dann 1 Einheit am Schreibkonto und erscheint in der Episode aus Regel 4; das ist die in K5 und F1 benannte Schwäche, nicht der Befund B1.

  **Konsequenzen:** positiv — keine Reservierung lebt länger als ein ehrlicher Client mit Mindestrate und einem Erneut bräuchte; zwei oder drei Herkünfte sperren niemanden mehr; Wettläufe haben genau einen Gewinner; kein `rename` überschreibt eine Datei. Negativ — eine Frist mehr in der Antwort (`hochladen_bis`), ein 429 mehr am `PUT` (für gutartige Clients nur bei mehr als 64 gleichzeitig laufenden Hochladungen); Abbrüche auf schlechten Leitungen enden früher in einer Neu-Anmeldung; bis zu 64 MiB Stücke im Speicher bei Vollauslastung. Pre-Mortem: … ein Fehlerpfad setzt `lebensende` doch neu (z. B. beim Zurücklegen nach gescheitertem Binden) — Früh-Indikator: M20 rot; Gegenmaßnahme: `lebensende` ist ein Feld ohne Setter außer in `anmelden`, Mutante „Abbruch verlängert“.

  **Erste Tests:**
  - Unit (A00017, L1; Speicherablage, Testuhr, fester Zufall): Anmelden 9.437.184 Bytes bei t0 → `hochladen_bis` = t0 + 78 s, `anlauf_bis` = t0 + 60 s; bei t0 + 50 s `beginnen`, 1 Byte, Abbruch → `anlauf_bis` = t0 + 78 s (nicht t0 + 110 s); bei t0 + 77,999 s `beginnen` ok, Abbruch; t0 + 78,001 s `ablaufen` → `reserviert` 0, `beginnen` → `abgelaufen`. Ein laufender Platz bei t0 + 78 s → Abbruchsignal. Fertig bei t0 + 70 s → `gueltig_bis` = t0 + 70 s + 15 min, `ablaufen` bei t0 + 79 s lässt ihn stehen. 127.0.0.2 und 127.0.0.3 je 16 `angemeldet` → Anmeldung von 127.0.0.1 ok; 64 `laeuft` → 65. `beginnen` → `offen_gesamt`, Eintrag bleibt `angemeldet`; 17. Anmeldung von 127.0.0.2 → `offen_herkunft`. Mutanten „Abbruch setzt Lebensende neu“, „`offen_gesamt` zählt `angemeldet`“, „Anlauffrist nach Abbruch ohne `min`“ werden rot.
  - Probe M20 (wörtlich aus dem Gate, L1 mit Testuhr und L2): „Anmelden 9.437.184, 20 min alle 50 s PUT nach 1 Byte geschlossen → spätestens nach der absoluten Lebensdauer 404 `abgelaufen`, reserviert 0.“
  - Probe M21 (wörtlich aus dem Gate, L2): „127.0.0.2 und .3 je 4 Kennungen × 4 Anmeldungen à 1 Byte erneuert → beta@127.0.0.1 Anmelden 201, PUT 200; Kontrolle mit einer vollen Herkunft.“ (Kontrolle: die 17. Anmeldung von 127.0.0.2 → 429 `offen_herkunft`.)
  - Wettläufe (L1, Barriere in der Attrappe der Logablage): `DELETE` zwischen `nehmen` und `rename` → Bindung gelingt, `DELETE` 204 ohne Wirkung; `ablaufen` während `nehmen` wartet auf die Sperre; zwei `beginnen` → einmal `laeuft`; nach jedem Schritt die Invariante aus Regel 8.

  **Richtigstellung:** ADR-0010 Ergänzung Karte #9 Punkt (3) („ein Tröpfler hält Platz und Reservierung damit höchstens bis zur Gesamtfrist“) — richtiggestellt in ADR-0010, Ergänzung „Karte #9 Gate“. Matrixzeilen M20–M24, M26, M39 und M7 (präzisiert): [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md), Ergänzung „Karte #9 Gate“.
