---
id: R00018
status: Neu
datum: 2026-10-28
---

# R00018: Betriebsprotokoll gehärtet

## Beschreibung

Das Betriebsprotokoll aus R00014 schließt die Lücken, die die Auto-Reviews zu #23 und #24 gefunden haben. Danach gilt:

- **Kein Fehlerpfad bleibt still**: ein nicht geholter Stand der Channels im Ereignisstrom, eine gescheiterte Annahme einer Verbindung, ein Verlaufsblock, der sich nicht serialisieren lässt, und eine geschlossene Werkbank stehen je mit Route bzw. Quelle und festem `grund` im Protokoll. Fremdtext (englisch, aus Bibliotheken) steht nur im neuen Schlüssel `ursache`.
- **Debug-Zeilen nennen die Regel**: jede einzelne 4xx trägt einen Regelschlüssel wie `grund=name_ungueltig` statt eines festen Satzes je Status; jede Körperabweisung ergibt genau eine Debug-Zeile; ein gescheiterter Rückbau steht immer im Protokoll.
- **Abriss und Werkbank sind sichtbar**: eine Verbindung, die der Kernel an der Abrissgrenze schließt, ist eine `WARNING verbindung`-Episode je Herkunft und zählt in `abgerissen`; `/api/gesundheit` nennt dazu `werkbank_plaetze` und `werkbank_belegt`.
- **Das Tagesbudget gilt über Neustarts**: ein Neustart am selben Tag rechnet die vorhandene Tagesdatei je Stufenklasse an.
- **Meldungen ohne Rust-Schreibweise**: `--hilfe` zeigt `--protokoll-stufe` auf eigener Zeile, Exit-2-Meldungen nennen den Wert in „…“, die Körper-Episode schreibt „zu groß“.

Zahlt ein auf: [Vision](R00000-vision.md) — „eine gemeinsame LAN-Instanz“: die Betreiberin erfährt auch an den Rändern (Abriss, volle Deskriptoren, Programmfehler im Strom), was schiefging, und ein Monitoring merkt es an einer Abfrage. Die Vision nennt das Betriebsprotokoll nicht; es ist eine Vorgabe des Betreibers (`AI-Team/umgebung.md`, „Logging-Konzept“) und widerspricht ihr nicht.

Entschieden vom architekt am 2026-10-28 (Commit 2c7c80e): [ADR-0040](../Dokumentation/ADRs/0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md), Ergänzung „eigener Schlüssel `ursache`“; [ADR-0042](../Dokumentation/ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung #24 Punkte 1–9; [ADR-0043](../Dokumentation/ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md), Ergänzung #24; [ADR-0039](../Dokumentation/ADRs/0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md), Ergänzung „die Sendefrist hat eine Spur“; Baustein [A00015](../Dokumentation/Architektur/A00015-betriebsprotokoll.md).

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0021 Betriebsprotokoll): F0059 Jeder Fehlerpfad mit Spur und festem Grund, F0060 Abriss und Werkbank im Health-Check, F0061 Tagesbudget über Neustarts, F0062 Meldungen ohne Rust-Schreibweise. Sie brauchen die Nacharbeit #24 an R00014 (B0292–B0299) und sind danach untereinander parallel.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00014: Betriebsprotokoll, Eintrag, Stufe, Bereich, Kontext, Episode, Tagesbudget, Stufenklasse; neu: **Regelschlüssel** (festes ASCII-Wort in `snake_case`, das sagt, welche Regel eine Anfrage abwies), **Abrissgrenze** (= Sendefrist, 2 × Herzschlag, ADR-0039 Regel 6), **Werkbank** (die begrenzten Plätze für Verlaufsblöcke, ADR-0039).

## Geschäftlicher Nutzen

Die Reviews zu #24 haben gezeigt, wo das Protokoll heute schweigt oder zweideutig ist. Ein Ereignisstrom, der am Stand der Channels scheitert, endet still; eine Instanz ohne freie Deskriptoren nimmt keine Verbindung mehr an und schreibt nichts; ein Angriff „Langsame Leser“ ist nur am Speicher zu sehen. Debug-Zeilen sagen „ungültig“, aber nicht, welche Regel griff, und der englische `JoinError`-Text steht im Schlüssel `art`, nach dem das Monitoring die Körperfehler greppt. Ein Neustart zum Zuschalten von Debug gibt das volle Tagesbudget neu, und eine Absturzschleife unter `Restart=always` hebt die Plattengrenze aus R00014 auf.

Mit R00018 findet die Betreiberin jeden dieser Fälle mit `grep ' ERROR \| EXCEPTION \| WARNING verbindung ' logs/*.log`, ein Monitoring sieht Abrisse und eine dauerhaft volle Werkbank an `/api/gesundheit`, und die Plattengrenze hält auch über Neustarts.

## Funktionale Anforderungen

### Jeder Fehlerpfad mit Spur und festem Grund (F0059; ADR-0042 Ergänzung #24 Punkte 1–6, ADR-0040 Ergänzung „`ursache`“)

- **Abbruch an der Grenze**: jedes `.await` auf `spawn_blocking` bzw. einen `JoinHandle` im HTTP-Baustein nimmt die gemeinsame Hilfe in `src/http/abbruch.rs`, auch in Strömen. Sie ist geteilt in `abbruch_eintragen` (nur der Eintrag) und `abbruch_beantworten` (Eintrag und 500).
  - Panik → `ERROR programmfehler … grund=panik` (die `EXCEPTION` mit `ort` schreibt weiter der Haken);
  - abgebrochen ohne Panik → `EXCEPTION programmfehler … grund=abgebrochen`;
  - Werkbank geschlossen → `EXCEPTION programmfehler … grund=werkbank_geschlossen` (heute `grund=abgebrochen`).
  - Der englische Text des `JoinError` steht in `ursache=`, nie mehr in `art=`; `art` bleibt der Fehlerart der Körpergrenze vorbehalten (`zu_gross`, `kaputt`, …).
- **Ereignisstrom** (`GET /api/ereignisse`):
  - scheitert der Stand der Channels **vor** dem Strom → 500 `{"fehler":"Interner Fehler — Stand der Channels nicht geholt; bitte erneut versuchen"}` und ein Eintrag mit `route="GET /api/ereignisse"`, `operation=channels_stand`, `kennung`;
  - scheitert er **im** Strom → ein Eintrag „Ereignisstrom beendet — Stand der Channels nicht geholt; der Client verbindet neu“ mit denselben Werten, danach endet der Strom wie bisher. Das reguläre Stromende bleibt ohne eigene Zeile.
- **Annahme gescheitert** (neue Katalogzeile): scheitert `accept`, steht `ERROR verbindung Verbindung nicht angenommen` als Episode je Fehlernummer mit `grund` (Text des `io::Error` mit Fehlernummer, z. B. `grund="Too many open files (os error 24)"`) im Protokoll. `ECONNABORTED`, `EPROTO` und `EINTR` sind `DEBUG` ohne Episode. Die Pause von 50 ms nach einem Fehler bleibt.
- **Regelschlüssel**: ein abweisender Handler hängt an seine Antwort die Erweiterung `Abweisung { regel, kennung }`; die Debug-Zeile der 4xx trägt `grund=<regel>` und `kennung=`, sonst `herkunft=`. Regelschlüssel sind feste Wörter aus dem Code, nie Eingaben — z. B. `sitzung_fehlt`, `sitzung_unbekannt`, `name_ungueltig`, `name_vergeben`, `text_leer`, `text_zu_lang`, `abfrage_ungueltig`, `channel_unbekannt`, `channel_existiert`. Ohne Erweiterung (Router-404, -405) gilt ein Schlüssel je Status: `ungueltig`, `nicht_angemeldet`, `unbekannt`, `methode`, `konflikt`, `zu_gross`, `kein_json`, `regel`.
- **Verlauf nicht gelesen**: ein Fehler beim Serialisieren eines Verlaufsblocks ist eine verletzte Annahme: `EXCEPTION verlauf Verlauf nicht gelesen — Block ließ sich nicht serialisieren` mit `route="GET /api/channels/{name}/nachrichten"`, `operation=verlauf`, `status=500`, `channel`, `kennung`, `grund=serialisierung`, `ursache=<Text von serde_json>`. Die Antwort bleibt „Interner Fehler — Verlauf nicht gelesen; bitte erneut versuchen“.
- **Körperabweisung genau einmal**: die Marke, die der Debug-Schicht sagt „schon eingetragen“, entsteht an einer Stelle aus dem Körperfehler, nicht je Handler.
- **Rückbau immer mit Spur**: eine Dateiablage gibt es nur mit Protokoll; ein gescheiterter Rückbau schreibt immer `ERROR senden Rückbau gescheitert — …`.

### Abriss und Werkbank im Health-Check (F0060; ADR-0042 Ergänzung #24 Punkt 7, ADR-0043 Ergänzung #24, ADR-0039 Ergänzung)

- **Abriss**: endet eine Verbindung mit einem Fehler, in dessen Quellenkette ein `io::Error` der Art `TimedOut` steht, ist das ein Abriss an der Abrissgrenze (Sendefrist oder stiller Abriss; der Kernel unterscheidet beides nicht). Er ergibt eine `WARNING verbindung Verbindung an der Abrissgrenze geschlossen`-Episode je Herkunft mit `herkunft` und `grenze` (Abrissgrenze in ms) und zählt in `abgerissen`.
- **`/api/gesundheit`** bekommt drei Felder:
  - `abgerissen`: Zahl der Abrisse seit dem Start, beim Entstehen gezählt, unabhängig von Stufe, Episode und Budget;
  - `werkbank_plaetze`: Plätze der Werkbank, fest seit dem Start;
  - `werkbank_belegt`: Plätze minus freie Plätze, Momentwert.
- **Zählerregel**: `warnings`, `errors` und `exceptions` zählen jedes Auftreten **mit der Stufe seiner Episode**. Ein `Abgebrochen` (DEBUG), das nur in eine offene Körper-Episode (WARNING) eingeht, zählt nicht in `warnings`, wohl aber in `abgewiesen.abgebrochen` (ADR-0043 Ergänzung #24 Punkt 1; so gebaut, jetzt mit Test).

### Tagesbudget über Neustarts (F0061; ADR-0042 Ergänzung #24 Punkt 9)

- Beim ersten Eintrag eines Tages liest der Prozess die vorhandene `logs/<tag>.log` einmal und rechnet je Zeile ihre Bytes der Stufenklasse an, die das zweite Wort der Zeile nennt; eine Zeile ohne erkennbare Stufe zählt zu `DEBUG`.
- Ist eine Klasse danach schon über ihrem Budget, gilt sie als erreicht: ihr nächster Eintrag wird verworfen, `protokoll_verworfen` zählt, und die Zeile `WARNING protokoll Tagesbudget <Klasse> erreicht — …` steht einmal je Prozess neu in der Datei (sie zählt nicht ins Budget). Die anderen Klassen schreiben weiter.
- Löscht die Betreiberin die Tagesdatei am selben Tag, bleibt der angerechnete Verbrauch (konservativ).

### Meldungen ohne Rust-Schreibweise (F0062; Rechtschreib-Review #24, Sollte)

- `--hilfe` zeigt `--protokoll-stufe <stufe>` auf einer eigenen Zeile und die Erklärung darunter eingerückt, wie bei `--erreichbar-als`.
- Die Exit-2-Meldungen zu `LOCALCHATA_PROTOKOLL_STUFE`, `LOCALCHATA_EPISODE_RUHE_MS` und `LOCALCHATA_TESTPANIK` nennen den falschen Wert in „…“ statt in Rust-Anführung (`"…"` aus `{:?}`). Steuer- und Richtungszeichen im Wert bleiben escaped wie im Protokoll.
- Die Zusammenfassung einer Körper-Episode nennt die Art `zu_gross` als „zu groß“ (`Episode endet: 200 in 1 s (zu groß 200; verworfen vollständig 200)`). Im Schlüssel `art=` der Debug-Zeilen und in `abgewiesen` bleibt `zu_gross`.

## Nicht-funktionale Anforderungen

- **Sicherheit** (CWE-117, CWE-770): Regelschlüssel und `grund`-Werte der neuen Zeilen sind feste Wörter aus dem Code; Fremdtext steht nur in `ursache` und wird escaped und gekürzt wie jeder Wert (R00014). Abriss- und Annahme-Fehler sind Episoden; ein Client kann damit keine Zeile je Ereignis erzwingen.
- **Leistung**: das Anrechnen der Tagesdatei liest einmal je Prozess und Tag höchstens 128 MiB (gerechnet ≈ 0,1–0,5 s), beim Start vor dem Laden bzw. beim ersten Eintrag nach Mitternacht (dann ist die Datei leer). `werkbank_belegt` ist ein Lesen ohne Sperre.
- **Zeitkriterien** nach `AI-Team/planer.md`: neue Zeitgrenzen gibt es nicht; das Gate K5 behält seine Grenzen aus R00013.

## Akzeptanzkriterien

Gemeinsam für alle Integrationskriterien: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem frischen Wegwerf-Ordner, `TZ=Europe/Berlin`; `heute.log` ist `logs/<lokales Datum>.log`; „Speicher-Senke“ wie in R00014. `beta` tritt von `127.0.0.1` bei.

### F0059 Jeder Fehlerpfad mit Spur und festem Grund

- [ ] **Abbruch-Hilfe** (Unit, Speicher-Senke): je ein Abbruch der Art Panik, abgebrochen (`abort()`) und Werkbank geschlossen mit `route="GET /api/channels"`, `operation=channels` ergibt genau eine Zeile:
  - Panik → `ERROR programmfehler … grund=panik`;
  - abgebrochen → `EXCEPTION programmfehler … grund=abgebrochen` mit `ursache=`;
  - geschlossen → `EXCEPTION programmfehler … grund=werkbank_geschlossen`.
  - Keine der drei Zeilen enthält `art=`. `abbruch_beantworten` antwortet 500 mit einem deutschen `fehler`; `abbruch_eintragen` gibt keine Antwort.
  - Mutanten, die rot werden müssen: „geschlossen als abgebrochen“; „JoinError-Text in `art`“.
- [ ] **Ereignisstrom** (Unit am HTTP-Baustein, Speicher-Senke; erster Test aus ADR-0042 Ergänzung #24 Punkt 1): ein Gesprächsbestand, dessen Stand der Channels panikt (oder ein abgebrochener `JoinHandle`):
  - vor dem Strom → 500 `{"fehler":"Interner Fehler — Stand der Channels nicht geholt; bitte erneut versuchen"}` und genau eine Zeile mit `route="GET /api/ereignisse"`, `operation=channels_stand`, `kennung=beta@127.0.0.1`;
  - im Strom (nach dem ersten `NeuerStand`) → der Strom endet, und es steht genau eine Zeile „Ereignisstrom beendet — Stand der Channels nicht geholt; der Client verbindet neu“ mit denselben Werten im Protokoll.
  - Mutante „`.ok()` wie heute“ → keine Zeile, rot.
- [ ] **Annahmefehler** (Unit, rein): `annahmefehler_einordnen` ordnet `EMFILE` (24), `ENFILE`, `ENOMEM` und `ENOBUFS` als `ERROR`-Episode ein, Quelle die Fehlernummer (`24`); `ECONNABORTED`, `EPROTO` und `EINTR` als `DEBUG` ohne Episode. 20 × `EMFILE` hintereinander in eine Speicher-Senke → genau eine Zeile `ERROR verbindung Verbindung nicht angenommen — Episode beginnt` mit `grund=` (enthält `os error 24`). Mutante „alles als DEBUG“ wird rot. Eine Integration über `ulimit -n` ist nicht verlangt.
- [ ] **Regelschlüssel** (Integration, `LOCALCHATA_PROTOKOLL_STUFE=debug`):
  - `POST /api/beitreten {"name":"a b"}` → 422 und genau eine `DEBUG`-Zeile mit `status=422`, `grund=name_ungueltig`, `herkunft=127.0.0.1`;
  - `GET /api/gibtsnicht` → 404 und eine `DEBUG`-Zeile mit `grund=unbekannt`;
  - jede `grund=`-Angabe einer `DEBUG`-Zeile mit `status=4…` in diesem Lauf passt auf `^[a-z_]+$`.
  - Mutante „Erweiterung übergangen“ → `grund=regel` statt `name_ungueltig`, rot.
- [ ] **Verlauf nicht gelesen** (Unit, Speicher-Senke; der Test aus R00014 „Umgestellte Meldungen mit Test“ zieht mit): ein Serialisierungsfehler → 500 „Interner Fehler — Verlauf nicht gelesen; bitte erneut versuchen“ und genau eine Zeile `EXCEPTION verlauf Verlauf nicht gelesen — Block ließ sich nicht serialisieren` mit `route="GET /api/channels/{name}/nachrichten"`, `operation=verlauf`, `status=500`, `channel=`, `kennung=`, `grund=serialisierung` und `ursache=`; `exceptions` steigt um 1, `errors` nicht. Mutante „als ERROR“ wird rot.
- [ ] **Körperabweisung genau einmal** (Integration, Stufe debug): `POST /api/beitreten` und `POST /api/channels` (mit `S(beta)`) mit `Content-Type: text/plain` → je 415 und je **genau eine** `DEBUG`-Zeile mit `status=415` und `typ=text/plain`. Mutante „Marke in einem Handler vergessen“ → zwei Zeilen, rot.
- [ ] **Rückbau immer mit Spur** (Review-Punkt und Unit): `Dateiablage` hat keinen Konstruktor ohne Protokoll; der Rückbau-Test aus R00014 bleibt grün.

### F0060 Abriss und Werkbank im Health-Check

- [ ] **Abriss erkennen** (Unit, rein): `ist_abriss` ist wahr für einen Fehler mit `io::ErrorKind::TimedOut` in der Quellenkette (auch eingepackt), falsch für `ConnectionReset`, `BrokenPipe` und `UnexpectedEof`.
- [ ] **Gate K5 mit Spur** (Gate, Integration, echtes Programm, Linux; Erweiterung von R00013 K5, Erwartung aus ADR-0042 Ergänzung #24 Punkt 7): Start mit `LOCALCHATA_HERZSCHLAG_MS=2000`; 4 Rohverbindungen holen je `?n=200` aus `riesig` und lesen nicht. Nachdem alle vier server-seitig zu sind (Beobachtung wie in K5):
  - `heute.log` enthält genau eine Zeile `WARNING verbindung Verbindung an der Abrissgrenze geschlossen — Episode beginnt | herkunft=<h> grenze=4000`, `<h>` die Herkunft der vier Verbindungen (im Gate heute `127.0.0.2`; ADR-0042 nennt `127.0.0.1`);
  - `/api/gesundheit` nennt `abgerissen: 4`.
  - Mutante „Abriss nicht erkannt“ → keine Zeile und `abgerissen: 0`, rot.
  - Zeigt die Probe, dass hyper eine andere Fehlerart durchreicht als `TimedOut`, folgt die Einordnung in `ist_abriss` der Probe (Vermutung des architekt, nicht geprobt).
- [ ] **Health-Check frisch** (Integration und Unit): eine frische Instanz nennt `abgerissen: 0`, `werkbank_belegt: 0` und `werkbank_plaetze` ≥ 2 (Kerne, mindestens 2; ADR-0039). Der Unit-Test `leerer_ordner_ist_bereit_mit_kennzahlen_null` mit `Werkbank::mit(2)` erwartet zusätzlich `"abgerissen": 0`, `"werkbank_plaetze": 2`, `"werkbank_belegt": 0`; hält der Test einen Platz, ist `werkbank_belegt: 1`.
- [ ] **Zählerregel Abgebrochen** (Unit, Speicher-Senke): ein `ZuGross` und danach ein `Abgebrochen` derselben Quelle, Ruhe abgelaufen → genau eine Zusammenfassung „Episode endet: 2 in … s“, die je Art 1 nennt; `warnings: 1`; `abgewiesen.abgebrochen: 1`. Mutante „Abgebrochen zählt in `warnings`“ → 2, rot.

### F0061 Tagesbudget über Neustarts

- [ ] **Verbrauch je Klasse** (Unit, rein): `verbrauch_je_klasse` über die Zeilen `… DEBUG …` (100 Bytes), `… WARNING …` (50 Bytes), `… EXCEPTION …` (30 Bytes) und `kaputte zeile` (13 Bytes) ergibt `[113, 50, 30]` — die Zeile ohne Stufe zählt zu DEBUG.
- [ ] **Budget aus der Tagesdatei** (Unit, Wegwerf-Ordner, Budget im Konstruktor 1 KiB je Klasse, gestellte Uhr 2026-10-28; erster Test aus ADR-0042 Ergänzung #24 Punkt 9): vorhanden `logs/2026-10-28.log` mit 1.100 Bytes `DEBUG`-Zeilen und 100 Bytes `ERROR`-Zeilen; ein neues Protokoll am 2026-10-28:
  - ein `DEBUG`-Eintrag wird verworfen, `protokoll_verworfen` ist 1, und genau eine Zeile „Tagesbudget DEBUG erreicht“ ist neu in der Datei;
  - ein danach geschriebener `ERROR`-Eintrag steht in der Datei.
  - Mutante „Budget beginnt bei 0“ (heute) → der Debug-Eintrag steht in der Datei, rot.
- [ ] **Neustart im echten Programm** (Integration, `LOCALCHATA_JETZT=2026-10-28T12:00:00+01:00`): vor dem Start liegt `logs/2026-10-28.log` mit 33 MiB `INFORMATION`-Zeilen (Budget der Klasse 32 MiB = 33.554.432 Bytes; 33 MiB = 34.603.008 Bytes liegen darüber). Nach dem Start tritt `beta` bei. Erwartet:
  - die Datei enthält keine neue `INFORMATION`-Zeile (weder `start` noch den Beitritt) und genau eine neue Zeile `WARNING protokoll Tagesbudget INFORMATION+WARNING erreicht — …` (Klassenname wie in R00014);
  - `/api/gesundheit` nennt `protokoll_verworfen` ≥ 2 (Start und Beitritt) und `status: "bereit"`.

### F0062 Meldungen ohne Rust-Schreibweise

- [ ] **Hilfe** (Integration): `localchatA --hilfe` → Exit 0; eine Zeile endet auf `--protokoll-stufe <stufe>` (nach führenden Leerzeichen), die Erklärung steht auf der nächsten Zeile, eingerückt wie bei `--erreichbar-als`; keine Zeile ist länger als die längste übrige Zeile der Hilfe.
- [ ] **Exit-2-Meldungen** (Integration): `LOCALCHATA_PROTOKOLL_STUFE=laut`, `LOCALCHATA_EPISODE_RUHE_MS=abc` und `LOCALCHATA_TESTPANIK=laut` (Debug-Build) ergeben je Exit 2 und eine stderr-Zeile, die die Variable nennt und `„laut“` bzw. `„abc“` enthält, aber kein `"laut"` bzw. `"abc"`. Unit: ein Wert mit `\u{1b}` erscheint escaped (`\u{1b}`), nicht roh.
- [ ] **Episodentext** (Unit am Episoden-Baustein und die Gegenprobe aus R00014 F0055): die Zusammenfassung einer Körper-Episode mit 200 × `zu_gross` lautet `… Episode endet: 200 in <d> s (zu groß 200; verworfen vollständig 200)`; die Debug-Zeilen derselben Abweisungen tragen weiter `art=zu_gross`, `abgewiesen.zu_gross` bleibt der Schlüssel.
- [ ] **Testname** (Review-Punkt): `src/protokoll/tagestakt.rs` enthält keinen Testnamen mit `nachster`.

## Betroffene Verzeichnisstruktur

- `src/http/`: `abbruch.rs` (Teilung, `ursache`, `werkbank_geschlossen`), `ereignisse.rs` (Strom), `anfrageprotokoll.rs` und die abweisenden Handler (Erweiterung `Abweisung`), `nachrichten.rs` (Serialisierung), `koerper.rs`, `beitreten.rs`, `channels.rs` (Marke an einer Stelle), `gesundheit.rs` (Felder), `werkbank.rs` (Belegung lesen).
- `src/start/`: `ausliefern.rs` (Annahmefehler, Abriss), `aufruf.rs` (Hilfe), `protokollstufe.rs`.
- `src/protokoll/`: Schlüssel `ursache`, `budget.rs` (Anrechnen), `episoden.rs` (Text der Art), `episodenruhe.rs`, `testpanik.rs`, `tagestakt.rs` (Testname).
- `src/gespraeche/logablage.rs` (Konstruktor mit Protokoll).
- Tests im Modul (Unit) und unter `tests/` (Integration; `tests/sendefrist.rs` für K5).

## Technische Überlegungen

### Ablauf

1. **Abbruch**: `.await` auf `JoinHandle` → `Ok` weiter \| `Err` → Art (Panik \| abgebrochen \| geschlossen) → `abbruch_eintragen(protokoll, abbruch, ort, was)` → mit Antwort möglich: 500 (`abbruch_beantworten`), im Strom: Strom endet.
2. **Annahme**: `accept` → `Err(e)` → `annahmefehler_einordnen(&e)` → Episode je Fehlernummer (ERROR) \| DEBUG → Pause 50 ms.
3. **Verbindungsende**: `serve_connection` → `Err(e)` → `ist_abriss(&e)` → Episode je Herkunft (WARNING) und `abgerissen` + 1.
4. **4xx**: Handler → Antwort mit `Abweisung { regel, kennung }` → Debug-Schicht → `grund=<regel>`.
5. **Tagesbudget**: erster Eintrag eines Tages → Datei lesen → `verbrauch_je_klasse` → Budget vorbelegt → wie bisher.

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0059: B0300 Abbruch-Hilfe teilen · B0301 Ereignisstrom mit Spur · B0302 Annahmefehler einordnen · B0303 Regelschlüssel als Grund · B0304 Verlauf-Serialisierung als Exception · B0305 Körperabweisung genau einmal eingetragen · B0306 Rückbau immer mit Spur
- F0060: B0307 Abriss erkennen · B0308 Abriss als Episode und Zähler · B0309 Kennzahlen erweitert · B0310 Gate K5 mit Abriss-Spur · B0311 Zählerregel Abgebrochen belegt
- F0061: B0312 Verbrauch je Klasse · B0313 Budget aus der Tagesdatei anrechnen
- F0062: B0314 Meldungstexte

Elemente (Signaturen nach A00015 und den ADR-Ergänzungen): `abbruch_eintragen(protokoll, abbruch, ort, was)`, `abbruch_beantworten(…) -> Response`, `annahmefehler_einordnen(&io::Error) -> Annahmefehler` (rein), `ist_abriss(&(dyn Error + 'static)) -> bool` (rein), `Abweisung { regel: &'static str, kennung: Option<String> }` (Antwort-Erweiterung), `verbrauch_je_klasse(zeilen) -> [u64; 3]` (rein), `Schluessel::Ursache`.

### Änderungen an bestehenden Klassen

- `Abbruchart::Geschlossen` bekommt eine eigene Grundangabe; `Schluessel` + `Ursache`.
- `Protokollkennzahlen` + `abgerissen`, `werkbank_plaetze`, `werkbank_belegt`; `Werkbank` gibt Plätze und freie Plätze heraus.
- `Tagesbudget` wird mit dem Verbrauch der vorhandenen Datei vorbelegt.
- `Dateiablage::neu` nimmt das Protokoll; `mit_protokoll` entfällt.

## Tests

Nach Skill `test-pyramide`.

- **Unit (rein)**: Abbruch-Hilfe (drei Arten), Annahmefehler, `ist_abriss`, Verbrauch je Klasse, Episodentext, Exit-2-Meldungen mit Steuerzeichen.
- **Unit mit Speicher-Senke bzw. Wegwerf-Ordner**: Ereignisstrom, Serialisierung, Zählerregel, Budget aus der Tagesdatei, Gesundheit mit `Werkbank::mit(2)`.
- **Integration (echtes Programm)**: Regelschlüssel, Körperabweisung einmal, Gate K5 mit Spur, Health-Check frisch, Neustart mit voller Klasse, Hilfe, Exit-2-Meldungen.
- **E2E**: keine (keine Oberfläche).
- Mutationsproben je Kriterium oben; das Ergebnis steht im Bericht des Entwicklers (Lehre „Mutationsprobe für Wächter“).

## Abhängigkeiten

- Abhängig von: F0051 Protokoll schreibt, F0054 Stufe und Kennzahlen, F0055 Grenzen, Episoden und Tagesbudget (R00014; gelb — Nacharbeit #24 B0292–B0299 offen), F0053 Programmfehler (R00014, gruen), F0050 Große Verlaufsblöcke begrenzt (R00013, gruen; Gate K5).
- Blockiert: —

## Offene Fragen

- Keine für den Bau.
- Nicht blockierend: dass hyper bei der Abrissgrenze `TimedOut` durchreicht, ist eine Vermutung des architekt (ADR-0042 Ergänzung #24 Punkt 7); das Gate K5 belegt es. Zeigt es eine andere Art, folgt `ist_abriss` der Probe.
- Nicht blockierend, an den Betreiber über den Director: die Monitoring-Schwellen aus ADR-0043 Ergänzung #24 (`abgerissen` steigt in 5 min um mehr als 20; `werkbank_belegt = werkbank_plaetze` in drei Abfragen im Abstand von 10 s) sind Empfehlungen für sein Monitoring, keine Kriterien.

## Manuelle Vorbereitungstätigkeiten

- Keine.

## Manuelle Nachbereitungstätigkeiten

- Keine.

## Umfang

Gezählt aus der WBS (Bubbles). 0,4 h ist die Untergrenze je Bubble nach dem Skill `work-breakdown-structure`; Richtwert, ungemessen für diese Bubble-Typen.

- F0059 Jeder Fehlerpfad mit Spur und festem Grund: 7 Bubbles · 5 klar (2,0 h) · 2 unklar (4–8 h)
- F0060 Abriss und Werkbank im Health-Check: 5 Bubbles · 4 klar (1,6 h) · 1 unklar (2–4 h)
- F0061 Tagesbudget über Neustarts: 2 Bubbles · 2 klar (0,8 h)
- F0062 Meldungen ohne Rust-Schreibweise: 1 Bubble · 1 klar (0,4 h)
- Zusammen: 15 Bubbles · 4,8 h klar + 6–12 h unklar; alle vier Slices brauchen die Nacharbeit #24 und sind danach parallel (eine Welle)

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser sind die Befunde der Auto-Reviews zu #24 (Observability S1–S4, O1; Code-Review A2, A3, A6, A7; Rechtschreib-Review Sollte) und die Katalog-Lücken aus dem Observability-Review zu #23, die der architekt am 2026-10-28 als ADR-Ergänzungen entschieden hat.

Die Wirkungskette:

- Jeder Fehlerpfad schreibt eine Zeile mit festem `grund` und Fremdtext nur in `ursache`. Deshalb findet ein `grep` nach `grund=` oder `art=` genau das, was er sucht, und ein still endender Strom hat eine Spur.
- Abrisse und die Werkbank-Belegung stehen im Health-Check. Deshalb sieht ein Monitoring einen Angriff „Langsame Leser“ und eine hängende Platte, ohne das Protokoll zu lesen.
- Das Tagesbudget rechnet die vorhandene Datei an. Deshalb gilt die Plattengrenze aus R00014 (128 MiB je Tag) auch, wenn die Instanz neu startet oder in einer Absturzschleife hängt.

Der Hebel liegt an den wenigen gemeinsamen Stellen (Abbruch-Hilfe, Debug-Schicht, Annahmeschleife, Budget), nicht an jedem Aufrufer: so bleiben neue Routen ohne eigene Disziplin richtig.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus ADR-0039 bis ADR-0043.
- Die Abnahme-Proben des `code-reviewer` zu A2, A3, A6, A7 stehen nicht wörtlich auf der Karte #24; die Kriterien oben folgen den ersten Tests der ADR-Ergänzungen.

## Notizen

- **Änderung von Zusagen aus R00014** (R00014 bleibt an diesen Stellen unverändert, R00018 nennt die Änderung):
  - „Verlauf nicht gelesen“ ist `EXCEPTION verlauf` statt `ERROR verlauf` (R00014 F0051, Kriterium „Umgestellte Meldungen mit Test“; R00014 F0053 „An der Grenze“: der englische Text steht in `ursache`, nicht in `grund`).
  - Die Gegenprobe aus R00014 F0055 erwartet in der Ende-Zeile `zu groß 200` statt `zu_gross 200`.
  - 4xx-Debug-Zeilen tragen einen Regelschlüssel als `grund` (R00014 F0054 „Debug“ nannte `grund` ohne Form).
- Herkunft: Karte #24 (Auto-Reviews 2026-10-28), Schnitt des Directors: Nacharbeit #24 = Muss und Kürzen an der Quelle (an R00014), der Rest als eigene Karte mit dieser Anforderung.
- Code-Review A5 (Lesbarkeit, verirrter Doc-Kommentar `src/protokoll/zeile.rs:4` u. a.) gehört in die Konsolidierung nach dem letzten grünen Bubble (Skill `lernen-zuerst`), nicht in ein Kriterium.

### Bewusst außerhalb

- Unterscheidung von Sendefrist und stillem Abriss über `TCP_INFO` (ADR-0042 Ergänzung #24 Punkt 7, Option C verworfen).
- Wartende vor der Werkbank zählen (ADR-0043 Ergänzung #24 Punkt 3, verworfen).
- Optionale Punkte des Observability-Reviews #24, die der architekt nicht entschieden hat: O3 `operation=` an „Hintergrundaufgabe beendet“, O4 `grenze=` ohne je Herkunft/gesamt, O5 404 im Bereich `verbindung`, O6 Rückfall-Ende ab Mindeststufe warning; Rechtschreib-Optional „Keine gültige Protokollstufe“ und der englische `ExitStatus`-Text in `browser.rs:25`.
- `cargo audit` im Gate (Lücke aus dem Security-Review #24) — Sache der Umgebung, nicht dieser Anforderung.
