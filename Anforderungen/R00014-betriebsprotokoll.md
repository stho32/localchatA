---
id: R00014
status: Neu
datum: 2026-10-28
---

# R00014: Betriebsprotokoll

## Beschreibung

Die Instanz schreibt ihr Betriebsprotokoll als Textdatei je lokalem Tag unter `<ordner>/logs/JJJJ-MM-TT.log`. Jeder Eintrag ist genau eine Zeile mit Zeit samt Versatz, Stufe, Bereich, deutschem Text und Kontext (`schlüssel=wert`). Programmfehler stehen als `EXCEPTION`, Betriebsfehler als `ERROR`, Abweisungen mit Betriebsbedeutung als gedrosselte Episode. Einträge ab `WARNING` erscheinen zusätzlich auf stderr, in derselben Zeilenform. Dateien, die älter als 14 Kalendertage sind, entfallen beim Start und beim Tageswechsel. Die Mindeststufe ist beim Start einstellbar (Standard `information`, `debug` zuschaltbar). `/api/gesundheit` zählt Warnings, Errors, Exceptions und abgewiesene Körper, damit ein Monitoring ohne das Protokoll zu lesen merkt, dass etwas schiefgeht. Kein Token und kein Nachrichtentext stehen je im Protokoll, und kein Client kann es über ein festes Tagesbudget treiben.

Zahlt ein auf: [Vision](R00000-vision.md) — „Ein Chat, der ein Ordner ist“ (auch das Protokoll ist eine lesbare Datei im Ordner, ohne Werkzeug mit `less` und `grep` lesbar) und „eine gemeinsame LAN-Instanz“ (der Betreiber erfährt, was im Betrieb schiefging). Die Vision nennt das Betriebsprotokoll nicht. Es ist eine Vorgabe des Betreibers (`AI-Team/umgebung.md`, „Logging-Konzept“, 2026-10-28) und widerspricht der Vision nicht.

Entschieden vom architekt: [ADR-0040](../Dokumentation/ADRs/0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) (Ort, Zeile, Escape, Rückfall), [ADR-0041](../Dokumentation/ADRs/0041-betriebsprotokoll-als-eigenes-modul-rotation-beim-schreiben-bereinigung-14-tage.md) (eigenes Modul, Rotation, Bereinigung, Tagestakt), [ADR-0042](../Dokumentation/ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) (Stufen, Ereigniskatalog, Episoden, Tagesbudget) und [ADR-0043](../Dokumentation/ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md) (Stufe, Kennzahlen). Hinzu kommen die Ergänzungen dieser ADRs aus den Proben des `observability-reviewer` an #21: Körperfehler zentral eingetragen, Art `Abgebrochen`, Episode je Herkunft und Route, `JoinError` ohne Panik, Zähler `abgewiesen`, Testknopf `LOCALCHATA_EPISODE_RUHE_MS`. Den Tagestakt regelt ADR-0041, Ergänzung „Vertrag des Tagestakts“ (mit ADR-0042, Ergänzung „gescheiterte Tages- und Sekundenaufgaben“). Baustein ist [A00015](../Dokumentation/Architektur/A00015-betriebsprotokoll.md), das Qualitätsziel Q9 Nachvollziehbarkeit (A00001).

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0021 Betriebsprotokoll): F0051 Protokoll schreibt, F0052 Bereinigung nach 14 Tagen, F0053 Programmfehler, F0054 Stufe und Kennzahlen, F0055 Grenzen, Episoden und Tagesbudget.

## Geschäftlicher Nutzen

Heute gehen Fehler auf stderr. Dort sind sie verloren, sobald niemand das Terminal liest, und manche Pfade sind ganz still: eine abgewiesene Verbindung, eine Panik in einem Hintergrund-Task, ein abgebrochener Blocking-Task. Gerade an diesen Stellen stoßen 50 Agenten auf einem Rechner zuerst an. Mit R00014 findet die Betreiberin am Morgen mit `grep ' ERROR \| EXCEPTION ' logs/*.log`, was wann bei wem warum scheiterte, ohne Code zu lesen. Ein Monitoring braucht nur `curl …/api/gesundheit` und zwei Schwellen: `exceptions` steigt, `aufgaben_ausgefallen` ist nicht leer. Die Flut-Grenzen aus R00010, R00011 und R00013 werden damit erst beobachtbar. Die Auslegung des Schreibkontos hängt ausdrücklich an dieser Messung im Betrieb (Entscheidung Director 2026-10-28; Anbindung F0057, R00016).

## Funktionale Anforderungen

### Protokoll schreibt (F0051; ADR-0040 Regeln 1–8 mit Ergänzung, ADR-0041 Regeln 1–3, ADR-0042 Regeln 2–3 und Ergänzung 7)

- **Ort**: `<ordner>/logs/` wird nach der Sperre und vor dem Laden angelegt. Vor der Sperre (Aufruf, Ordner, Sperre, Umgebungswerte) bleiben stderr und die Exit-Codes wie heute.
- **Datei** `logs/JJJJ-MM-TT.log` trägt das lokale Datum der Eintragszeit aus dem Uhr-Provider. Ein Eintrag um 23:59:59.999 steht in der Datei des Tages, einer um 00:00:00.000 in der des Folgetags. Liegt das Datum zurück (Uhr zurückgestellt), wird an die ältere Datei angehängt. Jeder Eintrag öffnet seine Datei mit append und create (fehlt `logs/`, wird es angelegt), schreibt die ganze Zeile mit einem `write_all` und schließt die Datei wieder; keine Datei bleibt über einen Eintrag hinaus offen. Löscht, verschiebt oder kürzt die Betreiberin im laufenden Tag `heute.log` oder ganz `logs/`, steht schon der nächste Eintrag in einer neuen `heute.log` (ADR-0040, Ergänzung „gelöschte Tagesdatei“; ADR-0041, Ergänzung M1). `protokoll_bytes_heute` ist die Größe der Datei nach dem Schreiben.
- **Zeile**: `<zeit> <STUFE> <bereich> <text>[ | <schlüssel>=<wert> …]` und `\n`.
  - `<zeit>` ist RFC 3339 mit Millisekunden und Versatz.
  - `<STUFE>` ist eine von `DEBUG`, `INFORMATION`, `WARNING`, `ERROR`, `EXCEPTION`.
  - `<bereich>` stammt aus der festen Liste in ADR-0040 Regel 3.
  - Die Schlüssel stammen aus der Erlaubnisliste in ADR-0040 Regel 3 samt Ergänzung (`typ`, `name`, `bytes`, `art`, `verworfen`, `operation`). Werte mit Leerzeichen, `=`, `|` oder `"` stehen in `"…"` mit `\"`.
- **Escape** an genau einer Stelle, für Text und jeden Wert:
  - `\` wird zu `\\`.
  - U+0000–U+001F, U+007F–U+009F, U+061C, U+2028, U+2029, U+200E, U+200F, U+202A–U+202E und U+2066–U+2069 werden zu `\u{…}` in Kleinbuchstaben-Hex.
  - Im Text wird jedes `|` zu `\u{7c}`; so ist das erste „ | “ einer Zeile immer der Trenner zum Kontext, und ein fremder Text (etwa ein Panik-Text) kann keinen Kontext vortäuschen (ADR-0040, Ergänzung „`|` im Text wird escaped“).
  - Ein Wert wird auf höchstens 256 Zeichen gekürzt, der Text auf höchstens 1.024, jeweils mit `…` am Ende.
  - Gekürzt wird **an der Quelle**: ein Wert schon beim Eintritt in den Kontext, ein Text beim Eintritt in den Eintrag, roh auf ein Zeichen mehr als die Grenze (257 bzw. 1.025), damit der Formatierer das `…` setzen kann. Eine offene Episode hält so je Wert höchstens ≈ 1 KiB, 1.024 Episoden höchstens wenige MiB (CWE-770; ADR-0040, Ergänzung „an der Quelle kürzen; U+061C“).
- **Nie im Protokoll**: Sitzungs-Token, der Kopf `X-LocalchatA-Sitzung` (auch nicht gekürzt oder gehasht), `Authorization`, `Cookie`, Nachrichtentexte, Channel-Beschreibungen, Anfragekörper, ganze Kopfzeilen. Routen erscheinen als Muster (`GET /api/channels/{name}/nachrichten`).
- **stderr**: jeder Eintrag ab `WARNING` steht zusätzlich auf stderr, in derselben Zeilenform. `DEBUG` und `INFORMATION` gehen nie auf stderr (Ausnahme: der Rückfall). stdout bleibt allein der Startzeile.
- **Start und Ende**: `INFORMATION start` steht **vor** der Startzeile (ADR-0041, Ergänzung „`INFORMATION start` steht vor der Startzeile“): wer die Startzeile gelesen hat, findet die Zeile schon in der Datei. Sie trägt `version`, `adresse`, `ordner`, `stufe`, `anzahl` (Nachrichten) und `dauer_ms` (Laden). `INFORMATION ende` mit `grund` (`Strg+C` oder `SIGTERM`) und `dauer_ms` (Laufzeit) wird geschrieben, sobald das Ende-Signal eintrifft, und noch einmal als „beendet“ nach dem Ablauf der Ende-Frist.
- **Bisherige Meldungen**: jede stderr-Meldung nach der Sperre wird ein Eintrag mit der Stufe aus dem Katalog in ADR-0042 Regel 2:
  - Ladedefekt WARNING, je Datei höchstens 10 einzeln, dann eine Zeile „und 1 weiterer Defekt in dieser Datei“ bzw. ab zwei „und n weitere Defekte in dieser Datei“;
  - Einstellungsdatei defekt und Ordner mit ungültigem Channelnamen WARNING;
  - Abrisspflege nicht setzbar WARNING;
  - Rückbau gescheitert ERROR;
  - Channel nicht angelegt ERROR;
  - Signal nicht abfangbar und Browser nicht zu öffnen WARNING;
  - Startfehler nach der Sperre ERROR.
- **Betriebsereignisse** als `INFORMATION`: `sitzung` beigetreten, verlassen und weg (Name frei), jeweils mit `kennung`; `channels` angelegt mit `channel` und `kennung`.
- **Rückfall**: lässt sich `logs/` nicht anlegen oder eine Zeile nicht schreiben, arbeitet die Instanz weiter.
  - Die Einträge gehen dann auf stderr, alle ab der Mindeststufe.
  - Je Episode erscheint eine `WARNING protokoll Betriebsprotokoll nicht schreibbar — Einträge gehen bis auf Weiteres nur auf stderr | datei=… grund=…`.
  - Die nächste gelungene Zeile beendet die Episode mit einer `INFORMATION`. Der nächste Eintrag versucht dabei `logs/` neu anzulegen.

### Bereinigung nach 14 Tagen (F0052; ADR-0041 Regeln 4–7)

- Eine Datei vom Datum D entfällt genau dann, wenn D < heute − 13 ist (heute zählt mit). Am 2026-10-28 bleiben also `2026-10-15.log` bis `2026-10-28.log`.
- Gelöscht werden nur reguläre Dateien direkt in `logs/`, deren Name genau `JJJJ-MM-TT.log` mit gültigem Datum ist. Die Prüfung läuft mit `symlink_metadata`, der Pfad entsteht aus dem Datum. Unterordner, Symlinks und fremde Dateien bleiben.
- Anlässe sind der Start (synchron direkt nach dem Öffnen des Protokolls) und jeder Wechsel des lokalen Datums. Den Wechsel erkennt ein Tagestakt, der jede Sekunde nachsieht. I0018 meldet daran die Fristpflege als zweite Tagesaufgabe an, nach der Bereinigung.
- **Vertrag des Tagestakts** (ADR-0041, Ergänzung):
  - Er startet **vor** der Startzeile, nach allen Start-Läufen, und endet mit dem Ende-Signal.
  - Je Tagesaufgabe führt er ein eigenes `zuletzt`. Vorbelegt ist es mit dem Datum, das ihr Start-Lauf bekam (Bereinigung: das Datum der Bereinigung beim Start).
  - Fällig ist eine Tagesaufgabe, sobald das lokale Datum von ihrem `zuletzt` **abweicht**, also auch, wenn die Uhr zurückgestellt wird. Verglichen wird das Datum, nicht die Uhrzeit.
  - Je Takt laufen zuerst die Sekundenaufgaben, dann die fälligen Tagesaufgaben in Anmeldereihenfolge, jede für sich über `spawn_blocking` und abgewartet. `zuletzt` wird **vor** dem Aufruf gesetzt.
  - Gerät eine Tagesaufgabe in Panik, läuft der Takt weiter, und die übrigen Aufgaben dieses Takts laufen auch. Der Takt schreibt `ERROR programmfehler Tagesaufgabe <name> gescheitert — <folge>; erneuter Versuch beim nächsten Tageswechsel | operation=<name> grund=panik`; bei einem Abbruch ohne Panik steht dieselbe Zeile als `EXCEPTION` mit `grund=abgebrochen`. Erneut versucht wird erst beim nächsten Datumswechsel. Folge-Text der Bereinigung: „alte Protokolldateien bleiben bis dahin liegen“.
  - Gerät eine Sekundenaufgabe in Panik, wird nur sie abgemeldet: `ERROR programmfehler Hintergrundaufgabe <name> beendet — <folge>`, danach wird sie nie wieder aufgerufen. Die Tagesaufgaben laufen weiter.
- Die Bereinigung schreibt `INFORMATION protokoll 1 Protokolldatei entfällt (<datum> bis <datum>; Aufbewahrung 14 Tage; Start|Tageswechsel)` bzw. ab zwei Dateien `INFORMATION protokoll <n> Protokolldateien entfallen (<älteste> bis <jüngste>; Aufbewahrung 14 Tage; Start|Tageswechsel)`, aber nur, wenn etwas entfiel. Je gescheiterter Datei kommt `ERROR protokoll Protokolldatei nicht entfallen | datei=… grund=…`. Der nächste Versuch folgt erst beim nächsten Anlass.

### Programmfehler (F0053; ADR-0042 Regel 4 und Ergänzung 6)

- **Panik-Haken**: er ersetzt den Standard-Haken und schreibt je Panik in jedem Thread genau einen `EXCEPTION programmfehler`-Eintrag mit `ort` (Datei:Zeile) und `thread`; der Panik-Text ist der Text des Eintrags. Die Protokollsperre nimmt er nur mit `try_lock`; ist sie belegt oder vergiftet, schreibt er auf stderr. Zähler und Zeit der letzten Exception setzt er immer.
- **An der Grenze**: `channel_anlegen` (`src/http/channels.rs:52`), `channels` (`:60`) und `verlauf` (`src/http/nachrichten.rs:40–42`) teilen sich eine Hilfsfunktion.
  - Ein `JoinError` aus einer Panik ergibt 500 und `ERROR programmfehler` mit `route`, `operation`, `status=500` und `grund=panik`, dazu `kennung`, falls bekannt.
  - Ein `JoinError` ohne Panik (abgebrochen) ergibt 500 und `EXCEPTION programmfehler` mit `grund=abgebrochen`.
  - Die Antwort ist ein deutscher Satz ohne Fremdtext, beim Verlauf „Interner Fehler — Verlauf nicht gelesen; bitte erneut versuchen“. Der englische Text des `JoinError` steht nur im Protokoll (`grund`).
- **Hintergrundaufgaben** (Anwesenheits-Takt, Tagestakt, Pflege) laufen bewacht. Endet eine vor dem Ende-Signal, kommt `ERROR programmfehler Hintergrundaufgabe <name> beendet — <folge>`, und ihr Name steht in `aufgaben_ausgefallen`.
- **Tagesaufgaben sind keine beendeten Hintergrundaufgaben** (ADR-0042, Ergänzung „gescheiterte Tages- und Sekundenaufgaben“): eine gescheiterte Tagesaufgabe beendet den Tagestakt nicht und steht nie in `aufgaben_ausgefallen`; sie zählt in `errors` bzw. `exceptions`. `aufgaben_ausgefallen` kann `takt` (Anwesenheit), `tagestakt` (der Takt selbst) und `episoden` (die abgemeldete Sekundenaufgabe aus F0055) enthalten.
- **Verletzte Annahme ohne Panik**: `exception(bereich, text, kontext)` nimmt denselben Weg.
- **Testknopf** `LOCALCHATA_TESTPANIK=verlauf|takt` wirkt nur im Debug-Build. Bei `verlauf` gerät der nächste Verlaufsblock einmal in Panik, bei `takt` der Anwesenheits-Takt. Ein anderer Wert ergibt Exit 2. Im Release-Build ist der Knopf wirkungslos.

### Stufe und Kennzahlen (F0054; ADR-0043 Regeln 1–6 und Ergänzung, ADR-0042 Regel 2 und Ergänzungen 2–4)

- **Stufe**: `--protokoll-stufe <stufe>` geht vor `LOCALCHATA_PROTOKOLL_STUFE`, beide gehen vor dem Standard `information`.
  - Werte: `debug`, `information`, `warning`, `error`; Großschreibung wird angenommen. `exception` ist nicht wählbar.
  - Ein falscher Wert oder ein fehlender Wert hinter der Option ergibt eine Meldung mit den erlaubten Werten und Exit 2. Dabei wird kein `logs/` angelegt.
  - `--hilfe` nennt die Option. Der Start-Eintrag nennt `stufe=` und `option=` (`--protokoll-stufe`, `LOCALCHATA_PROTOKOLL_STUFE` oder `standard`).
- **Debug** zeigt die einzelnen Abläufe:
  - Verbindung angenommen und beendet;
  - online/offline-Wechsel;
  - Verlaufsblock gelesen (`channel`, `anzahl`, `dauer_ms`);
  - jede einzelne 400, 401, 404, 405, 409, 413, 415 und 422 mit `route` (Muster; nur bei 404 auf unbekannte Pfade der Pfad selbst, escaped und gekürzt), `status`, `kennung` oder `herkunft` und `grund`.
  - Bei Körperfehlern stehen zusätzlich `art`, `bytes`, `verworfen` (`vollständig`, `frist` oder `obergrenze`) und bei `KeinJson` der angekündigte `typ`.
- **Körperfehler**: `Abgebrochen` (Lesefehler im Strom) ist von `Kaputt` (gelesen, kein gültiges JSON) getrennt.
- **`/api/gesundheit`** bekommt neue Felder; `status` bleibt `"bereit"`, solange der Port antwortet:
  - `protokoll_stufe`, `protokoll_ok`, `protokoll_bytes_heute`, `protokoll_verworfen`;
  - `warnings`, `errors`, `exceptions` — jeder Eintrag zählt beim Entstehen, unabhängig von Mindeststufe, Budget und Rückfall, und eine Episode zählt jedes Auftreten;
  - `letzter_error`, `letzte_exception` (RFC 3339 oder `null`);
  - `aufgaben_ausgefallen` (Liste);
  - `abgewiesen: {zu_gross, kein_json, kaputt, abgebrochen}` über alle JSON-Routen.

### Grenzen, Episoden und Tagesbudget (F0055; ADR-0042 Regeln 5–6 und Ergänzungen 1, 3, 5, 8)

- **Episode** je Schlüssel aus Bereich, Ereignis und Quelle; Quelle ist Herkunft, Kennung oder Datei+Grund.
  - Das erste Auftreten schreibt „… — Episode beginnt“ in der Stufe der Episode. Weitere Auftreten werden nur gezählt.
  - Läuft eine Episode länger, kommt alle 10 Minuten eine Zwischenzeile „läuft weiter — n seit Beginn“.
  - Nach der Ruhe (Standard 60 s, `LOCALCHATA_EPISODE_RUHE_MS`; 0 oder kein Wert ergibt Exit 2) kommt **eine** Zusammenfassung in der **Stufe der Episode**: „… — Episode endet: <n> in <d> s“. `d` ist die Zeit vom ersten bis zum letzten Auftreten, auf ganze Sekunden aufgerundet. Bei Körperfehlern nennt sie die Zahl je Art und je Verwerfen-Ergebnis sowie die größte gelesene Bytezahl.
  - Höchstens 1.024 Episoden sind offen; jede weitere Quelle zählt in eine Sammelepisode „weitere Quellen“ derselben Art.
  - Eine zurückspringende Uhr beendet keine Episode vorzeitig.
  - Geschlossen werden Episoden im Sekundentakt des Tagestakts.
- **Episoden-Quellen** (WARNING je Herkunft):
  - Verbindung abgewiesen an der Platzgrenze (`verbindung`, `herkunft`, `grenze`);
  - Grenzschutz 403 für fremden Host oder Origin (`grenzschutz`, `herkunft`, `grund`);
  - Sitzungsgrenze 429 (`sitzung`) und Channel-Grenze 429 (`channels`), je mit `herkunft` und `grenze`;
  - Körper 413 und 400 (`körper`, je Herkunft **und** Route). Diese Episode trägt `json_lesen` zentral mit dem Kontext {route, herkunft, kennung?} ein. `Abgebrochen` zählt zusätzlich in eine offene Körper-Episode derselben Quelle.
- **Tagesbudget** je Protokolldatei und Stufenklasse: `DEBUG` 64 MiB, `INFORMATION`+`WARNING` 32 MiB, `ERROR`+`EXCEPTION` 32 MiB.
  - Erreicht eine Klasse ihr Budget, kommt eine `WARNING protokoll Tagesbudget <Klasse> erreicht — weitere Einträge bis Mitternacht nur gezählt`; sie zählt nicht ins Budget.
  - Jeder weitere Eintrag dieser Klasse zählt in `protokoll_verworfen`.
  - Ab dem nächsten lokalen Tag gilt ein neues Budget.

## Nicht-funktionale Anforderungen

- **Sicherheit** (CWE-117, CWE-770, CWE-532): eine Zeile je Eintrag, was immer ein Client als Name, Host oder Pfad schickt; kein Token, kein Körper, kein Nachrichtentext im Protokoll; Clients bestimmen die Protokollmenge nicht (Einzel-4xx nur auf Debug, Abweisungen als Episode, Tagesbudget). Höchstens 128 MiB je Tag, über 14 Tage höchstens 1,75 GiB (Rechnung ADR-0042 Regel 6).
- **Leistung**: vier Systemaufrufe je Eintrag (öffnen, schreiben, Größe, schließen); im Lastprofil auf Information ≈ 0,1 Einträge/s, also ≈ 0,4 Aufrufe/s, auf Debug unter Volllast ≈ 1.000 Aufrufe/s (gerechnet, ADR-0041 Regel 3 mit Ergänzung M1). Die Sperre des Protokolls wird nie über ein `.await` gehalten.
- **Betrieb**: das Protokoll verhindert nie den Start oder die Arbeit (Rückfall).
- **Zeitkriterien** nach `AI-Team/planer.md`: jede Zeitgrenze nennt Zweck, Startpunkt der Uhr und erwarteten Wert und liegt mindestens 30 % über dem erwarteten Wert. Regeln prüft der Unit-Test mit gestellter Uhr exakt, das echte Programm nur die obere Grenze.

## Akzeptanzkriterien

Gemeinsam für alle Integrationskriterien: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem frischen Wegwerf-Ordner, `TZ=Europe/Berlin`. `<p>` ist der Port aus der Startzeile, `heute.log` die Datei `logs/<lokales Datum>.log`. „Protokollzeile“ heißt eine Zeile, die auf den Ausdruck `^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}[+-]\d\d:\d\d (DEBUG|INFORMATION|WARNING|ERROR|EXCEPTION) [a-zäöü]+ .+$` passt. Tests, die `chmod` brauchen, laufen nicht als root.

### F0051 Protokoll schreibt

- [ ] **Formatierer** (Unit, rein; erster Test aus ADR-0040):
  - Eingabe: Eintrag `Warning`, Bereich `grenzschutz`, Text „Anfrage mit fremdem Host abgewiesen“, Kontext `herkunft=10.0.0.9` und `grund="Host böse\nZEILE\u{1b}[31m\u{2028}x"`, Zeit `2026-10-28T14:03:07.123+01:00`.
  - Erwartet: genau `2026-10-28T14:03:07.123+01:00 WARNING grenzschutz Anfrage mit fremdem Host abgewiesen | herkunft=10.0.0.9 grund="Host böse\u{a}ZEILE\u{1b}[31m\u{2028}x"` und ein `\n`.
  - Weitere Fälle: `\r`, U+202E, U+2066 und `\` werden je escaped; ein Wert aus 300 `a` wird zu 256 Zeichen, nämlich 255 `a` und `…`; ein Text aus 1.100 Zeichen wird zu 1.024; ein Wert `a=b|c "d"` wird zu `"a=b|c \"d\""`.
  - Mutanten, die rot werden müssen: Escape von U+2028 entfernt; Kürzung bei 257.
- [ ] **Tagesdatei** (Unit, gestellte Uhr, Wegwerf-Ordner):
  - Einträge um `2026-10-28T23:59:59.999+01:00` und `2026-10-29T00:00:00.000+01:00` landen je in `logs/2026-10-28.log` bzw. `logs/2026-10-29.log`.
  - Rückstellung: `2026-10-25T02:30:00.000+02:00`, dann `2026-10-25T02:10:00.000+01:00` → beide in `logs/2026-10-25.log`, in Schreibreihenfolge.
  - Vorstellung: `2026-03-29T01:59:59.999+01:00` und `2026-03-29T03:00:00.000+02:00` → beide in `logs/2026-03-29.log`.
  - Rückwärts über Mitternacht: erst `2026-10-29T00:00:01.000+01:00`, dann `2026-10-28T23:59:59.000+01:00` → der zweite Eintrag hängt an `logs/2026-10-28.log` an.
  - Mutante „Datum in UTC“ wird rot: der Eintrag um 00:30+01:00 landete am Vortag.
- [ ] **Gelöschte Tagesdatei am selben Tag** (Unit, Wegwerf-Ordner, gleicher Tag 2026-10-28; Abnahme-Probe M1 aus dem Observability-Review #24 und ADR-0040, Ergänzung „gelöschte Tagesdatei“, wörtlich übernommen):
  - `anhaengen(tag, "a\n")`; `remove_file(logs/2026-10-28.log)`; `anhaengen(tag, "b\n")` → die Datei existiert und enthält genau `b\n`, Rückgabe 2.
  - Varianten im selben Test: `remove_dir_all(logs)` → `c\n` steht in einer neuen Datei; `rename` nach `logs/alt.log` → `d\n` steht in einer neuen `2026-10-28.log`, `alt.log` bleibt unverändert; Datei auf 0 Bytes abgeschnitten → Rückgabe = Länge der neuen Zeile.
  - Mutante „Datei bleibt offen“ (Handle bei gleichem Tag wiederverwenden) wird in allen vier Varianten rot.
- [ ] **Gelöschtes Protokoll im laufenden Betrieb** (Integration): `alpha` tritt bei; danach `rm logs/<heute>.log` (Lauf 1) bzw. `rm -rf logs` (Lauf 2); dann tritt `beta` bei.
  - `heute.log` existiert und enthält die Zeile `INFORMATION sitzung Sitzung beigetreten | kennung=beta@127.0.0.1`.
  - `/proc/<pid>/fd` zeigt keine Datei mit `(deleted)`; `/api/gesundheit` nennt `protokoll_ok: true`.
  - Gegenprobe: ist `logs` danach eine Datei, greift der Rückfall wie im Kriterium „Rückfall“.
- [ ] **Kürzen an der Quelle** (Unit und Integration; Abnahme-Probe aus dem Security-Review #24, O1, und ADR-0040, Ergänzung, wörtlich übernommen):
  - Unit: `Kontext::mit(Grund, "x".repeat(400_000))` hält höchstens 257 Zeichen; die Zeile endet den Wert auf `…` (255 `x` und `…`, wie im Formatierer-Kriterium).
  - Integration (Release-Build): 300 Herkünfte (`127.0.0.2` bis `127.0.0.255` und weitere aus `127.0.1.0/24`) schicken je `GET /` mit einem `Host:`-Kopf von 400 KB → je 403; der RSS-Zuwachs der Instanz ist höchstens 50 MB (gemessen vor dem Fix: 309 MB).
  - Mutante „Kürzung erst im Formatierer“ wird in der Speicherprobe rot.
- [ ] **Strich und U+061C** (Unit, Formatierer):
  - Text `x | kennung=admin` ohne Kontext → die Zeile enthält `x \u{7c} kennung=admin` und kein „ | “; mit Kontext `grund=panik` → genau ein „ | “, das vor `grund=`.
  - Wert mit U+061C → `\u{61c}` in der Zeile.
  - Mutanten „Escape von `|` aus“ und „U+061C fehlt in der Liste“ werden je rot.
- [ ] **Start** (Integration, echtes Programm, `LOCALCHATA_JETZT=2026-10-28T12:00:00+01:00`):
  - Sobald die Startzeile gelesen ist — ohne Warten, die Zeile steht vor der Startzeile —, enthält `logs/2026-10-28.log` genau eine `INFORMATION start`-Zeile mit `version=`, `adresse=`, `ordner=`, `stufe=information`, `option=standard`, `anzahl=0` und `dauer_ms=`.
  - Ein Channel `logs` lässt sich anlegen und liegt unter `channels/logs/`; `logs/` enthält danach nur `2026-10-28.log`.
- [ ] **Ende** (Integration): nach SIGTERM endet das Programm mit Exit 0, und `heute.log` enthält in dieser Reihenfolge eine `INFORMATION ende`-Zeile mit `grund=SIGTERM` und `dauer_ms=` sowie eine zweite `INFORMATION ende`-Zeile „beendet“.
- [ ] **Ladedefekte** (Integration): vor dem Start liegt `channels/allgemein/2026-10-28.log` mit 13 defekten Kopfzeilen. Erwartet:
  - `heute.log` enthält genau 10 `WARNING laden`-Zeilen mit `datei=channels/allgemein/2026-10-28.log` und `zeile=` sowie eine Zeile „und 3 weitere Defekte in dieser Datei“;
  - `/api/gesundheit` meldet weiter `defekte_zeilen: 13`;
  - stderr enthält dieselben 11 Zeilen.
  - Unit: mit 11 Defekten heißt die Restzeile „und 1 weiterer Defekt in dieser Datei“; Mutante „immer Mehrzahl“ wird rot.
- [ ] **stderr nur im Protokollformat ab Warning** (Integration, Lauf aus dem Ladedefekt-Kriterium): jede stderr-Zeile nach der Startzeile ist eine Protokollzeile mit `WARNING`, `ERROR` oder `EXCEPTION` und steht wortgleich in `heute.log`. `INFORMATION`- und `DEBUG`-Zeilen stehen nie auf stderr, auch nicht mit `--protokoll-stufe debug`. stdout enthält genau die Startzeile.
- [ ] **Betriebsereignisse und keine Tokens** (Integration, mit `LOCALCHATA_PROTOKOLL_STUFE=debug`):
  - Ablauf: `beta` tritt bei; `beta` legt `# planung` mit Beschreibung `GEHEIM-BESCHREIBUNG-41` an; `GET /api/ich` mit dem Kopf `X-LocalchatA-Sitzung: 0123456789abcdef0123456789abcdef` (unbekannt) ergibt 401; `beta` verlässt.
  - Danach enthält `heute.log` `INFORMATION sitzung`-Zeilen mit `kennung=beta@127.0.0.1` für Beitritt und Verlassen und eine `INFORMATION channels`-Zeile mit `channel=planung`.
  - `grep -c <token von beta> logs/*.log` ergibt 0, ebenso `grep -c 0123456789abcdef0123456789abcdef logs/*.log`, `grep -c GEHEIM-BESCHREIBUNG-41 logs/*.log` und `grep -ci 'x-localchata-sitzung' logs/*.log`.
- [ ] **Rückfall** (Integration): vor dem Start ist `logs` eine **Datei**, kein Ordner.
  - Die Instanz startet: die Startzeile erscheint, und `/api/gesundheit` ergibt 200 mit `protokoll_ok: false`.
  - stderr enthält genau eine `WARNING protokoll … nicht schreibbar`-Zeile mit `datei=` und `grund=` und die `INFORMATION start`-Zeile (im Rückfall gehen alle Stufen ab der Mindeststufe auf stderr).
  - Wird danach die Datei `logs` entfernt und tritt `beta` bei, dann existiert `logs/<heute>.log` mit einer `INFORMATION`-Zeile, die die Episode beendet, und der Beitrittszeile; `protokoll_ok` ist `true`.
- [ ] **Umgestellte Meldungen mit Test** (Unit, Speicher-Senke; Nacharbeit aus dem Code-Review #24, B1): je Meldung ein Test, der mit der Mutante „Eintrag entfernt“ rot wird:
  - die erste gescheiterte Abrisspflege → genau eine `WARNING verbindung Abrisspflege nicht setzbar — …` mit `option=` und `grund=`, eine zweite gescheiterte keine weitere;
  - ein gescheiterter Rückbau → genau eine `ERROR senden Rückbau gescheitert — …` mit `datei=`, `grund=` und `anzahl=`;
  - „Verlauf nicht gelesen“ → genau eine `ERROR verlauf Verlauf nicht gelesen`-Zeile mit `grund=` (R00018 macht daraus eine `EXCEPTION`, derselbe Test zieht mit).
- [ ] **Durchsetzung** (Review-Punkt): `eprintln!` und `println!` kommen nur in `src/protokoll/`, in der Startzeile und auf den Pfaden vor der Sperre vor (A00015, „Durchsetzung“).

### F0052 Bereinigung nach 14 Tagen

- [ ] **Bereinigung** (Unit, Wegwerf-Ordner, ohne Uhr).
  - Vorher liegen in `logs/`: `2026-10-13.log`, `2026-10-14.log`, `2026-10-15.log`, `2026-10-28.log`, `notizen.txt`, `2026-10-14.log.gz`, `2026-10-01.log` als Symlink auf eine Datei außerhalb von `logs/` und ein Ordner `2026-10-02.log/`.
  - `bereinigen(2026-10-28)` entfernt genau `-13` und `-14`. `-15` (Grenze: 15 ≥ 28 − 13), `-28`, `notizen.txt`, `.gz`, der Symlink, sein Ziel und der Ordner bleiben.
  - Mutanten, die rot werden müssen: Aufbewahrung 13 (`-15` fehlt), Aufbewahrung 15 (`-14` bleibt), `metadata` statt `symlink_metadata` (Symlink oder Ziel fehlt), Pfad aus dem gelesenen Namen statt aus dem Datum.
- [ ] **Bericht** (Unit): die Bereinigung oben schreibt genau eine Zeile `INFORMATION protokoll 2 Protokolldateien entfallen (2026-10-13 bis 2026-10-14; Aufbewahrung 14 Tage; Start)`. Entfällt nur `2026-10-14.log`, heißt sie `INFORMATION protokoll 1 Protokolldatei entfällt (2026-10-14 bis 2026-10-14; Aufbewahrung 14 Tage; Start)`; Mutante „immer Mehrzahl“ wird rot, und `grep -r '1 Protokolldateien' src tests Anforderungen` findet nichts. Ohne Entfallendes kommt keine Zeile. Ist eine Datei nicht löschbar (`logs/` ohne Schreibrecht), kommt je Datei eine `ERROR protokoll Protokolldatei nicht entfallen`-Zeile mit `datei=` und `grund=`.
- [ ] **Tagestakt** (Unit, `tokio::time::pause`, gestellte Uhr, aufzeichnende Aufgaben A und B, Protokoll in eine Speicher-Senke; ADR-0041, Ergänzung, „Erste Tests“). Fünf Teile:
  - **Grenze**: `zuletzt` beider Aufgaben 2026-10-28. Uhr 2026-10-28T23:59:59.999 und ein Takt → kein Aufruf. Uhr 2026-10-29T00:00:00.000 und ein Takt → je Aufgabe genau ein Aufruf mit 2026-10-29, erst A, dann B. Zehn weitere Takte am 29. → kein Aufruf. Mutanten, die rot werden müssen: „läuft in jedem Takt“, „vergleicht die Uhrzeit statt des Datums“.
  - **Rückwärts**: danach Uhr 2026-10-28T23:00 und ein Takt → je Aufgabe genau ein Aufruf mit 2026-10-28; weitere Takte → keiner; wieder 2026-10-29T00:00:00.000 → je Aufgabe einer mit 2026-10-29. Mutante „`>` statt `≠`“ → rot.
  - **Startwert**: A mit `zuletzt` 2026-10-28, B mit `zuletzt` 2026-10-29 (Mitternacht fiel zwischen die Start-Läufe). Uhr 2026-10-29T00:00:00.500 und ein Takt → nur A läuft, mit 2026-10-29. Mutanten „ein gemeinsames `zuletzt` aus der ersten Aufgabe“ (B läuft) und „`zuletzt` = Datum beim Start des Takts“ (A läuft nicht) → je rot.
  - **Panik**: A gerät bei jedem Aufruf in Panik, B zeichnet auf. Wechsel auf den 29. → B läuft mit 2026-10-29; die Senke enthält genau eine `ERROR programmfehler Tagesaufgabe A gescheitert …`-Zeile mit `operation=A grund=panik`. Zehn weitere Takte am 29. → kein neuer Aufruf von A, keine weitere Zeile. Wechsel auf den 30. → A und B laufen wieder. Mutanten „Takt endet bei Panik“ (der 30. fehlt), „`zuletzt` erst nach Erfolg“ (A je Takt, eine Zeile je Sekunde) und „alle Aufgaben in einem `spawn_blocking`“ (B fehlt am 29.) → je rot.
  - **Sekundenaufgabe**: eine aufzeichnende Sekundenaufgabe läuft je Takt einmal; sie gerät beim dritten Takt in Panik → genau eine `ERROR programmfehler Hintergrundaufgabe <name> beendet`-Zeile, danach kein Aufruf mehr; ein folgender Tageswechsel ruft A und B weiter. Mutante „Sekundenaufgabe bleibt angemeldet“ (eine Zeile je Sekunde) → rot.
- [ ] **Tageswechsel im echten Programm** (Integration).
  - **Zweck**: der Tagestakt ist im laufenden Programm verdrahtet, Bereinigung und Rotation greifen ohne Neustart.
  - **Gegeben**: vor dem Start liegen `logs/2026-10-14.log` und `logs/2026-10-15.log`; Start mit `LOCALCHATA_JETZT=2026-10-28T23:59:58+01:00`.
  - **Sobald die Startzeile gelesen ist** (Startpunkt der Uhr): `-14` fehlt (Start-Bereinigung), `-15` liegt noch da.
  - **Danach** fehlt `-15` spätestens **4,0 s** nach der Startzeile.
    - Erwartet sind ≈ 2–3 s: höchstens 2 s bis Mitternacht (Laden und Binden laufen schon vorher), höchstens 1 s bis zum nächsten Takt, dazu wenige Millisekunden Arbeit.
    - Die Grenze liegt 33 % über 3 s.
    - Geprüft wird durch Abfrage alle 100 ms, nicht mit festem `sleep`.
  - `logs/2026-10-29.log` enthält eine `INFORMATION protokoll 1 Protokolldatei entfällt (2026-10-15 bis 2026-10-15; Aufbewahrung 14 Tage; Tageswechsel)`-Zeile.
  - `logs/2026-10-28.log` enthält die Start-Bereinigung von `-14`.
  - Die Mitternachtsregel selbst prüft der Takt-Unit-Test, nicht dieser Test.

### F0053 Programmfehler

- [ ] **Exception über `LOCALCHATA_TESTPANIK`** (Integration, echtes Programm, Debug-Build). Vor dem Start liegt `channels/allgemein/`, gestartet wird mit `LOCALCHATA_TESTPANIK=verlauf`. `beta` tritt bei, dann `GET /api/channels/allgemein/nachrichten`. Erwartet:
  - Die Antwort ist 500 `{"fehler":"Interner Fehler — Verlauf nicht gelesen; bitte erneut versuchen"}` und enthält kein `panicked`, kein `task` und kein `JoinError`.
  - In `heute.log` steht genau eine `EXCEPTION programmfehler`-Zeile mit `ort=src/` und `thread=`, danach genau eine `ERROR programmfehler`-Zeile mit `route="GET /api/channels/{name}/nachrichten"`, `operation=verlauf`, `status=500`, `grund=panik` und `kennung=beta@127.0.0.1`.
  - stderr enthält beide Zeilen und **keine** Zeile `thread '…' panicked at`; der Standard-Haken ist ersetzt.
  - Ein zweiter Abruf ergibt 200, und `/api/gesundheit` ergibt 200.
  - `grep -c <token von beta> logs/*.log` ergibt 0.
- [ ] **Hintergrundaufgabe** (Integration, Debug-Build, `LOCALCHATA_TESTPANIK=takt`, `LOCALCHATA_HERZSCHLAG_MS=1000`). Erwartet nach dem ersten Takt:
  - genau eine `EXCEPTION programmfehler`-Zeile und genau eine `ERROR programmfehler Hintergrundaufgabe takt beendet — …`-Zeile;
  - `/api/gesundheit` ergibt 200 mit `aufgaben_ausgefallen: ["takt"]`.
  - Geprüft wird durch Abfrage alle 100 ms. Erwartet sind ≤ 1 s nach der Startzeile (ein Takt), die Grenze ist 2 s.
- [ ] **Ausgefallene Aufgaben** (Unit am Tagestakt aus F0052, Speicher-Senke wie dort; gebaut mit dem späteren der beiden Slices F0052 und F0053): nach dem Teil „Panik“ steht A **nicht** in der Liste der ausgefallenen Aufgaben; nach dem Teil „Sekundenaufgabe“ steht deren Name genau einmal darin. Mutante „Tagesaufgabe als ausgefallen melden“ → rot. (Ab F0055 heißt die Sekundenaufgabe `episoden`.)
- [ ] **Abgebrochen ohne Panik** (Unit der Hilfsfunktion): ein abgebrochener `JoinHandle` (Task mit `abort()`), Route `GET /api/channels` und Operation `channels` ergeben:
  - 500 mit einem deutschen `fehler` ohne Fremdtext;
  - genau eine `EXCEPTION programmfehler`-Zeile mit `grund=abgebrochen`, `route=`, `operation=channels`; der englische Text steht nur in einem weiteren Wert, nie in der Antwort;
  - Mutante „abgebrochen als ERROR“ wird rot.
- [ ] **Panik bei gehaltener Sperre** (Unit): während ein Test die Protokollsperre hält, gerät ein anderer Thread in Panik → der Haken kehrt ohne Verklemmung zurück (Test endet in < 1 s), die Zeile steht auf stderr, `exceptions` ist um 1 gestiegen.
- [ ] **Testknopf** (Integration): `LOCALCHATA_TESTPANIK=laut` ergibt Exit 2 und eine stderr-Zeile, die `LOCALCHATA_TESTPANIK` nennt. Im Release-Build wirkt `LOCALCHATA_TESTPANIK=verlauf` nicht; das ist ein Review-Punkt `cfg(debug_assertions)`, den der Entwicklertest einmal mit `cargo build --release` probt.

### F0054 Stufe und Kennzahlen

- [ ] **Aufruf** (Unit, rein):
  - `--protokoll-stufe debug` → Debug; `--protokoll-stufe Warning` → Warning.
  - `--protokoll-stufe laut` → Aufruffehler mit „debug, information, warning, error“.
  - `--protokoll-stufe` ohne Wert → `--protokoll-stufe braucht einen Wert — Aufruf siehe localchatA --hilfe`.
  - `--protokoll-stufe exception` → Aufruffehler.
- [ ] **Stufe im echten Programm** (Integration):
  - ohne Angabe: nach einem Verlaufsabruf steht keine `DEBUG`-Zeile in `heute.log`;
  - `LOCALCHATA_PROTOKOLL_STUFE=debug`: eine `DEBUG verlauf`-Zeile mit `channel=`, `anzahl=`, `dauer_ms=` ist vorhanden;
  - `LOCALCHATA_PROTOKOLL_STUFE=debug` zusammen mit `--protokoll-stufe warning`: keine `INFORMATION`-Zeile, auch die Startzeile nicht, und `protokoll_stufe: "warning"`;
  - `LOCALCHATA_PROTOKOLL_STUFE=laut`: Exit 2, kein Ordner `logs/`;
  - `--hilfe` enthält `--protokoll-stufe` und `LOCALCHATA_PROTOKOLL_STUFE`.
- [ ] **Health-Check frisch** (Integration):
  - Werte: `warnings: 0`, `errors: 0`, `exceptions: 0`, `letzter_error: null`, `letzte_exception: null`, `protokoll_ok: true`, `protokoll_stufe: "information"`, `protokoll_verworfen: 0`, `aufgaben_ausgefallen: []`, `abgewiesen: {"zu_gross":0,"kein_json":0,"kaputt":0,"abgebrochen":0}`, `status: "bereit"`.
  - `protokoll_bytes_heute` ist gleich der Größe von `heute.log` (Startzeile und Start-Eintrag), gelesen nach dem Abruf; Toleranz ± die Länge einer Zeile, falls dazwischen ein Eintrag entsteht.
- [ ] **Health-Check nach Exception** (Integration, Debug-Build, wie im Exception-Kriterium): `exceptions: 1` und `errors: 1`; `letzte_exception` ist die Zeit der `EXCEPTION`-Zeile im Protokoll, `letzter_error` die Zeit der `ERROR`-Zeile.
- [ ] **Zähler unabhängig von der Stufe** (Integration): mit `--protokoll-stufe error` und einer Ladedefekt-Datei mit 2 Defekten → `warnings: 2`, aber keine `WARNING`-Zeile in `heute.log` und keine auf stderr.
- [ ] **Abgewiesene Körper** (Integration):
  - `POST /api/beitreten` mit 5 000 Bytes JSON → 413;
  - mit `Content-Type: text/plain` → 415;
  - mit `{` → 400;
  - ein Körper mit `Content-Length: 1000`, von dem nur 10 Bytes gesendet werden, bevor der Client die Verbindung schließt → keine Antwort, gezählt.
  - Danach: `abgewiesen: {"zu_gross":1,"kein_json":1,"kaputt":1,"abgebrochen":1}`.
  - Mutante „Lesefehler als `Kaputt`“ wird rot (`kaputt: 2`, `abgebrochen: 0`).
- [ ] **Debug je Anfrage** (Integration, `LOCALCHATA_PROTOKOLL_STUFE=debug`, Lauf aus „Abgewiesene Körper“):
  - je Abweisung genau eine `DEBUG`-Zeile mit `route="POST /api/beitreten"`, `status=` und `herkunft=127.0.0.1`;
  - die 415-Zeile trägt `typ=text/plain`, die 413-Zeile `art=zu_gross`, `bytes=` und `verworfen=vollständig`;
  - `GET /api/gibtsnicht%0aX` ergibt eine `DEBUG`-Zeile mit dem Pfad escaped (`\u{a}`) in genau einer Zeile.
- [ ] **Debug-Abläufe mit Test** (Unit, Speicher-Senke, Stufe debug; Nacharbeit aus dem Code-Review #24, B1): je Eintrag ein Test, der mit der Mutante „Eintrag entfernt“ rot wird:
  - eine angenommene Verbindung endet → genau eine `DEBUG verbindung Verbindung beendet` mit `herkunft=` und `dauer_ms=`;
  - ein Wechsel nach online bzw. offline → je genau eine `DEBUG sitzung Sitzung online` bzw. `Sitzung offline` mit `kennung=`; ein Wechsel nach `weg` schreibt hier keine Zeile.

### F0055 Grenzen, Episoden und Tagesbudget

- [ ] **Episode** (Unit, gestellte Uhr, Speicher-Senke): 100 × `grenzschutz` von `10.0.0.9` zu den Zeiten t = 0,0; 0,1; … 9,9 s.
  - Es entsteht genau eine Zeile `WARNING grenzschutz … — Episode beginnt | herkunft=10.0.0.9 …`.
  - Bei t = 9,9 + 59,999 s entsteht keine weitere Zeile.
  - Bei t = 9,9 + 60,000 s entsteht genau eine Zeile `WARNING grenzschutz … — Episode endet: 100 in 10 s` (Dauer 9,9 s, aufgerundet).
  - Mutanten, die rot werden müssen: Ende nach 59 s; eine Zeile je Auftreten; Ende als `INFORMATION` (die Episoden-Gegenprobe bei Stufe warning findet sie dann nicht).
- [ ] **Lange Episode** (Unit): ein Auftreten je Sekunde von t = 0 bis t = 1.499 s ergibt genau zwei Zwischenzeilen „läuft weiter — n seit Beginn“, nach 600 s und nach 1.200 s; `n` steigt. Nach 60 s Ruhe folgt „Episode endet: 1500 in 1499 s“.
- [ ] **Höchstens 1.024 offene Episoden** (Unit): je ein Auftreten von 1.030 verschiedenen Herkünften ergibt 1.024 Beginn-Zeilen und genau eine Zeile „weitere Quellen — Episode beginnt“. Deren Ende nennt 6.
- [ ] **Uhr rückwärts** (Unit): nach einem Auftreten bei t = 10 s springt die Uhr auf t = −3.600 s und läuft 120 s weiter → kein Ende. Bei t = 70 s endet die Episode.
- [ ] **Gegenprobe des `observability-reviewer`** (Integration, Abnahme-Probe aus ADR-0042, Ergänzung 8, wörtlich übernommen):
  - Gegeben ist `LOCALCHATA_EPISODE_RUHE_MS=1000`. Vorher tritt eine echte Sitzung bei, ihr Token wird gemerkt. Dann kommen von `127.0.0.1` 200 × `POST /api/beitreten` mit 5 KB JSON, im Körper der Marker `KOERPER-MARKER-7f3a`.
  - Erwartet nach 2 s Ruhe: für `route="POST /api/beitreten" herkunft=127.0.0.1` genau eine `WARNING … Episode beginnt` und genau eine `WARNING … Episode endet: 200 …` mit `zu_gross 200` und `verworfen vollständig 200`, dazu keine weitere WARNING-Zeile dieser Quelle;
  - `/api/gesundheit` zeigt `abgewiesen.zu_gross = 200`;
  - `grep -c KOERPER-MARKER-7f3a logs/*.log` ergibt 0, `grep -c <token> logs/*.log` ergibt 0.
  - Dieselbe Probe mit `--protokoll-stufe warning` enthält dieselben zwei Zeilen.
  - Mutanten, die rot werden müssen: eine Zeile je Abweisung (200 statt 2), keine Ende-Zeile, Ende-Zeile als `INFORMATION` (fehlt bei Stufe warning), Körper im Kontext (Marker gefunden).
- [ ] **Verbindungsgrenze** (Integration): `G` ist die Verbindungsgrenze je Gegenstelle; der Test liest sie aus der Konstante, heute 64, nach R00015 160.
  - `G + 1` offene Verbindungen von `127.0.0.1` ergeben genau eine `WARNING verbindung … Episode beginnt | herkunft=127.0.0.1 grenze=<G>`.
  - 30 weitere abgewiesene ergeben keine zusätzliche Zeile.
  - Mit `LOCALCHATA_EPISODE_RUHE_MS=1000` steht 2 s nach dem Schließen genau eine Ende-Zeile mit `31` da.
- [ ] **Grenzschutz** (Integration): 50 × `GET /` mit `Host: boese.example` ergeben 50 × 403 und genau eine `WARNING grenzschutz … Episode beginnt` mit `grund=` (escaped, höchstens 256 Zeichen).
- [ ] **Grenz-Episoden mit Test** (Nacharbeit aus dem Code-Review #24, B1): je Quelle ein Test, der mit der Mutante „Eintrag entfernt“ rot wird; die Grenzen liest der Test aus den Konstanten (nach R00015 je Herkunft 128):
  - ein Beitritt über der Sitzungsgrenze (429) → genau eine `WARNING sitzung Beitritt an der Sitzungsgrenze abgewiesen — Episode beginnt` mit `herkunft=` und `grenze=`;
  - ein Anlegen über der Channel-Grenze (429) → genau eine `WARNING channels Anlegen an der Channel-Grenze abgewiesen — Episode beginnt` mit `herkunft=` und `grenze=`;
  - ein `ZuGross` und danach ein `Abgebrochen` derselben Quelle (Speicher-Senke, Ruhe abgelaufen) → die Zusammenfassung der Körper-Episode nennt 2, je Art 1; Mutante „`episode_falls_offen` entfernt“ → 1, rot.
- [ ] **Ruhe einstellen** (Integration): `LOCALCHATA_EPISODE_RUHE_MS=0` ergibt Exit 2, die stderr-Zeile nennt die Variable.
- [ ] **Tagesbudget** (Unit, Budget im Konstruktor 1 KiB je Klasse, gestellte Uhr):
  - Debug-Einträge zu je 100 Bytes bis über das Budget ergeben 10 geschriebene und genau eine `WARNING protokoll Tagesbudget DEBUG erreicht …`-Zeile.
  - Jeder weitere Debug-Eintrag ist verworfen, `protokoll_verworfen` steigt um genau 1 je Eintrag.
  - Ein danach geschriebener `ERROR` steht in der Datei.
  - Am nächsten lokalen Tag werden Debug-Einträge wieder geschrieben.
  - Die Konstanten sind genau 64, 32 und 32 MiB (eigener Test).
  - Mutante „ein Gesamtbudget“ wird rot, weil der Error fehlt.

## Betroffene Verzeichnisstruktur

- Neu `src/protokoll/`: Formatierer, Datei je Tag, Rückfall, Bereinigung, Episoden, Budget, Kennzahlen (A00015); Tests im Modul (Unit) und unter `tests/` (Integration über das echte Programm, Muster `tests/gemeinsam/mod.rs`).
- `src/start/` (Öffnen nach der Sperre, Start- und Ende-Einträge, Tagestakt, Wache, Testknöpfe, `aufruf.rs` für `--protokoll-stufe`), `src/main.rs` unverändert bis auf den Haken.
- `src/http/` (`koerper.rs` Art `Abgebrochen` und zentraler Eintrag, `gesundheit.rs` Felder, Hilfsfunktion für `JoinError`, `grenzschutz.rs` Episode, `channels.rs`, `nachrichten.rs`, `beitreten.rs`).
- `src/start/ausliefern.rs` (Episode der Platzgrenze), `src/anwesenheit/` und `src/gespraeche/` (Einträge über den Provider statt `eprintln!`).

## Technische Überlegungen

### Ablauf

1. **Start**: Aufruf lesen (inkl. Stufe) → Ordner → Sperre → `Protokoll::oeffnen` (Rückfall möglich) → Panik-Haken setzen → `bereinigen(heute, Start)` (Datum b) → Laden (Defekte als Einträge) → (I0018: Fristpflege beim Start, Datum p) → Binden → Ende-Signal → Anwesenheits-Takt und Tagestakt (beide bewacht; Tagesaufgaben `bereinigen` mit `zuletzt = b`, ab I0018 `fristpflege` mit `zuletzt = p`; Sekundenaufgabe `episoden_schliessen`) → `INFORMATION start` → Startzeile. Der Tagestakt startet vor der Startzeile (ADR-0041, Ergänzung (b)); fällt ein Tageswechsel in die Millisekunden davor, stehen seine Einträge vor `INFORMATION start`.
2. **Eintrag**: Stufe < Mindeststufe → nur zählen; sonst Budget prüfen → Zeile bilden (Escape) → Datei des lokalen Datums öffnen → `write_all` → Größe → schließen → ab Warning stderr; Fehler → Rückfall.
3. **Abweisung**: Grenze trifft → `episode(art, quelle, …)` → beginnt/zählt; Debug-Zeile je Einzelfall.
4. **Ende**: Signal → `INFORMATION ende` → Ende-Frist → `INFORMATION ende` „beendet“.

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche Bubbles es am Ende werden, entscheidet der Entwickler.

- F0051: B0259 Wert neutralisieren · B0260 Zeile bilden · B0261 In die Tagesdatei eintragen · B0262 Rückfall auf stderr · B0263 Protokoll im Start · B0264 Meldungen umstellen · B0265 Betriebsereignisse eintragen
- F0052: B0266 Entfällt-Regel · B0267 Bereinigen · B0268 Tagestakt · B0269 Bereinigung verdrahten
- F0053: B0270 Panik-Haken · B0271 Abbruch an der Grenze · B0272 Hintergrundaufgaben bewachen · B0273 Testknopf Panik
- F0054: B0274 Stufe lesen · B0275 Stufe wirksam · B0276 Kennzahlen im Health-Check · B0277 Körperfehler unterscheiden und zählen · B0278 Debug je Anfrage
- F0055: B0279 Episode führen · B0280 Episoden im Sekundentakt schließen · B0281 Abweisungen als Episode · B0282 Tagesbudget
- Nacharbeit #24 (Auto-Reviews: Observability M1, Code-Review B1 und H1, Rechtschreib-Review Muss, Security-Review O1 und O2): F0051 B0292 Tagesdatei je Eintrag öffnen · B0293 Umgestellte Meldungen belegt · B0294 Rest der Ladedefekte in Einzahl und Mehrzahl · B0295 An der Quelle kürzen · B0296 Strich und U+061C escapen; F0052 B0297 Bereinigungszeile in Einzahl und Mehrzahl; F0054 B0298 Debug-Abläufe belegt; F0055 B0299 Grenz-Episoden belegt

Elemente nach A00015 (Signaturen dort): `Protokoll` (Integration über Datei und Uhr; `oeffnen`, `eintragen`, `episode`, `exception`, `bereinigen`, `episoden_schliessen`, `kennzahlen`), `Eintrag` (immutable: Stufe, Bereich, Text, Kontext), `Kontext` (benannte Sammlung typisierter Schlüssel), `Stufe`, `Bereich`, `Schluessel` (Aufzählungen), `zeile(&Eintrag, Zeit) -> String` und `entfaellt(datei_tag, heute) -> bool` (rein), `Episoden` (Zustand mit Uhr), `Tagesbudget`, `Protokollkennzahlen` (DTO), `tagestakt::takten`, `bewacht(name, folge, future)`.

### Änderungen an bestehenden Klassen

- `Koerperfehler` + `Abgebrochen`; `json_lesen` bekommt einen Kontext {route, herkunft, kennung?} und trägt selbst ein.
- `Startauftrag` + Stufe und Herkunft der Stufe; `HILFE` + Zeile.
- Gesundheitsantwort + Felder (Test `leerer_ordner_ist_bereit_mit_kennzahlen_null` wächst).
- `Gespraeche`, `Logablage`, `Abrisspflege`, Annahmeschleife, Grenzschutz, Anwesenheit: Protokoll als Provider statt `eprintln!`.

## Tests

Nach Skill `test-pyramide`.

- **Unit (rein)**: Formatierer und Escape, Tagesdatei mit gestellter Uhr, Entfällt-Regel, Bereinigung im Wegwerf-Ordner, Tagestakt mit `tokio::time::pause` (Grenze, Rückwärts, Startwert, Panik, Sekundenaufgabe, ausgefallene Aufgaben), Episoden, Budget, Aufruf der Stufe, Hilfsfunktion `JoinError`, Panik bei gehaltener Sperre.
- **Integration (echtes Programm)**: Start, Ende, Ladedefekte, stderr-Form, keine Tokens, Rückfall, Tageswechsel, Exception und Wache über `LOCALCHATA_TESTPANIK`, Stufe über Option und Umgebung, Health-Check, abgewiesene Körper, Gegenprobe 200 × 5 KB, Verbindungsgrenze, Grenzschutz.
- **E2E**: keine (keine Oberfläche). Die bestehenden Playwright-Tests bleiben grün; sie dürfen keine neue Konsolenzeile zeigen.
- Mutationsproben je Kriterium oben; das Ergebnis steht im Bericht des Entwicklers (Lehre „Mutationsprobe für Wächter“).

## Abhängigkeiten

- Abhängig von: I0001 Instanz starten (gruen, R00001); die Grenzen, deren Abweisungen eingetragen werden: F0008 Sitzungsgrenze (R00003), F0022 Channel-Grenze (R00006), F0040 Körpergrenze (R00010) — alle gruen.
- Blockiert: I0009 Nachricht senden (R00011) und I0018 Aufbewahrungsfrist (R00008) über `Braucht`; F0057 Senden im Betriebsprotokoll (R00016) braucht F0055.
- Nicht blockiert: F0050 Große Verlaufsblöcke (R00013; Entscheidung Director 2026-10-28).

## Offene Fragen

- Keine für den Bau. Die folgenden Punkte sind für R00014 entschieden (Wortlaut gleicht der architekt an):
  - (1) `warnings`, `errors` und `exceptions` zählen auch Einträge unter der Mindeststufe (ADR-0043 nennt die Stufe nicht).
  - (2) Ein unbekannter Wert in `LOCALCHATA_TESTPANIK` ergibt Exit 2.
  - (3) Die Dauer einer Episode reicht vom ersten bis zum letzten Auftreten, auf ganze Sekunden aufgerundet.
  - (4) Im Rückfall legt der nächste Eintrag `logs/` neu an.
- An den Director, nicht blockierend: R00008 (I0018) sagt INFORMATION-Zeilen auf stderr zu, etwa „Frist 7 → 30 Tage, gesetzt von …“. Das widerspricht ADR-0040 Regel 6; Empfehlung ist ein Anbindungs-Feature in I0018 wie F0057, vor dem Bau von I0018.
- Geklärt 2026-10-28 (ADR-0041, Ergänzung „Vertrag des Tagestakts“): der Tagestakt startet vor der Startzeile, führt `zuletzt` je Tagesaufgabe, vergleicht auf Ungleichheit und läuft nach der Panik einer Tagesaufgabe weiter; die Fristpflege aus I0018 ist seine zweite Tagesaufgabe, kein eigener Takt (WBS, „Offene Fragen“, Punkt geschlossen).
- Nachgezogen 2026-10-28 (`/anforderung review` nach den Auto-Reviews #24 und den ADR-Ergänzungen vom selben Tag): Senke öffnet je Eintrag, `INFORMATION start` vor der Startzeile, Pluralformen, `|` im Text, U+061C, Kürzen an der Quelle, Tests für acht Einträge ohne Test. Die übrigen Ergänzungen (Ereignisstrom, Annahmefehler, Regelschlüssel, Serialisierung als Exception, `ursache`, `werkbank_geschlossen`, Abriss-Episode, Werkbank-Felder, Budget über Neustarts, Meldungstexte) baut R00018 (F0059–F0062).
- Nicht blockierend: die Budgets (64/32/32 MiB) sind gerechnet, nicht gemessen. Der Messwert unter Last nach Profil (ADR-0042, Aufgaben) ist kein Teil dieser Anforderung.

## Manuelle Vorbereitungstätigkeiten

- Keine.

## Manuelle Nachbereitungstätigkeiten

- `AI-Team/umgebung.md` (Director): Zeile „Betriebsprotokoll `<ordner>/logs/JJJJ-MM-TT.log`“ und die Testknöpfe `LOCALCHATA_PROTOKOLL_STUFE`, `LOCALCHATA_TESTPANIK` (nur Debug-Build) und `LOCALCHATA_EPISODE_RUHE_MS` ergänzen.

## Umfang

Gezählt aus der WBS (Bubbles). 0,4 h ist die Untergrenze je Bubble nach dem Skill `work-breakdown-structure`; die Ist-Zeiten des Projekts liegen darunter, also ist das ein Richtwert, ungemessen für diese Bubble-Typen.

- F0051 Protokoll schreibt: 7 Bubbles · 5 klar (2,0 h) · 2 unklar (4–8 h)
- F0052 Bereinigung nach 14 Tagen: 4 Bubbles · 4 klar (1,6 h)
- F0053 Programmfehler: 4 Bubbles · 3 klar (1,2 h) · 1 unklar (2–4 h)
- F0054 Stufe und Kennzahlen: 5 Bubbles · 4 klar (1,6 h) · 1 unklar (2–4 h)
- F0055 Grenzen, Episoden und Tagesbudget: 4 Bubbles · 2 klar (0,8 h) · 2 unklar (4–8 h)
- Zusammen: 24 Bubbles · 7,2 h klar + 12–24 h unklar; Wellen F0051 → F0052 ‖ F0053 → F0054 → F0055
- Nacharbeit #24 (gezählt aus der WBS, 2026-10-28): 8 Bubbles · 6 klar (2,4 h, Richtwert ungemessen) · 2 unklar (B0293, B0299; 4–8 h); untereinander parallel

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist die Vorgabe des Betreibers: Fehler sollen irgendwo mitgeschrieben werden, und die Instanz soll überwachbar sein. Heute schreiben Meldungen auf stderr, das im Dienstbetrieb niemand liest, und drei Pfade sind ganz still (Verbindungsabweisung, `JoinError`, Panik im Hintergrund).

Die Wirkungskette:

- Jede Meldung wird ein Eintrag in eine Datei je lokalem Tag im Ordner. Deshalb findet die Betreiberin am Morgen mit `grep` was, wann, bei wem und warum.
- Programmfehler sind von Betriebsfehlern getrennt, und der Health-Check zählt beide. Deshalb reicht einem Monitoring eine Abfrage.
- Episoden und Tagesbudget begrenzen die Menge. Deshalb kann gerade die Flut, die man sehen will, das Protokoll und die Platte nicht füllen.

Der Hebel liegt in einem eigenen Modul mit einer einzigen Stelle für Format und Escape. Eine Bibliothek würde in Datum, Uhr und Aufbewahrung widersprechen (Probe in ADR-0041), und Disziplin je Aufrufer ließe die stillen Pfade still.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht). Begriffe wie Betriebsprotokoll, Episode, Stufe, Tagesbudget, Tagestakt sind aus ADR-0040 bis ADR-0043 übernommen.
- CWE-117 und die Liste der Richtungszeichen stammen aus dem Gedächtnis des architekt (ADR-0040, Quellen).

## Notizen

### Verworfene Alternativen

In ADR-0040 bis ADR-0043 entschieden, hier nur genannt:

- Datei nach UTC-Datum (ADR-0040 C): verworfen, weil die Einträge sonst am Vortag lägen.
- JSON Lines (F2): verworfen, weil sie ohne Werkzeug schwer lesbar sind.
- `tracing-appender` (ADR-0041 B): verworfen; die Probe zeigt UTC, eine nicht ersetzbare Uhr und eine Aufbewahrung, die Dateien statt Tage zählt.
- 5xx als Exception (ADR-0042 B): verworfen, weil volle Platte kein Programmfehler ist.
- Jede 4xx als Information (E): verworfen wegen der Flut.
- Ein Gesamtbudget (H): verworfen, weil Debug die Errors verdrängen würde.
- Umschalten zur Laufzeit über eine Route (ADR-0043 B): verworfen, weil das ohne Anmeldung im LAN ginge.
- `status: beeintraechtigt` (E): verworfen, weil es Bereitschaft und Zustand vermischt.

### Bewusst außerhalb

- Einträge der Fristpflege (I0018) — mit I0018.
- Einträge des Sendens und des Schreibkontos — F0057 (R00016).
- Abbruch langsamer Leser als Warning-Episode — Nachzug an F0050 nach I0021.
- Messwert der Budgets unter Last.
- `clippy::disallowed_macros` als Gate (A00015, „später“).
