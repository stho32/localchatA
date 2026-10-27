---
application: localchatA
system: localchatA
vision: Anforderungen/R00000-vision.md
wireframes: Dokumentation/Wireframes/
kanban: {folder: /home/shoff/Projekte/localchatA-kanbanD, board: entwicklung, application: lc, url: http://127.0.0.1:7421/}
sprache: de
zuletzt: 2026-10-27
---

# WBS — localchatA

## Knoten

| ID | Ebene | Eltern | Name | Status | Fertig-Kriterium | Eingabe → Ausgabe | Aufwand | Ausbaustufe | Braucht | Requirement | Notiz |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A0001 | Application | — | localchatA | gelb | alle Dialogs gruen | | | | | | Leitplanken aus der Vision: Rust, ein Programm für Weboberfläche und WebAPI, täglich rotierende Logdateien statt Git oder Datenbank, eine gemeinsame LAN-Instanz, Bedienkonzept wie kanbanD, Oberfläche deutsch, MIT. Leistungsziel 100.000 Nachrichten gilt für jede Interaction mit Verlauf |
| D0001 | Dialog | A0001 | Start und Ablage | gelb | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0001 | Interaction | D0001 | Instanz starten | gruen | `localchatA <ordner>` startet Weboberfläche und API, im LAN erreichbar | | | | | | aus Vision; Walking Skeleton (Gerüst nach ADR-0001/0002/0009); drei Slices F0001–F0003 mit Requirement R00001; Logablage, Index, Live-Kanal gehören nicht hierher |
| F0001 | Feature | I0001 | Starten mit Adresse | gruen | `localchatA <ordner>` lauscht (Standard 0.0.0.0:7430, belegt → nächster freier, `--port 0` wählt das System), schreibt die Startzeile mit der tatsächlichen Adresse, `GET /api/gesundheit` → 200 `bereit`; `--hilfe` Exit 0, falscher Aufruf Exit 2, SIGTERM/Strg+C beendet geordnet mit Exit 0 |  |  |  |  | R00001 | A00002, A00003; ADR-0001, ADR-0009 |
| B0001 | Bubble | F0001 | Crate-Gerüst | gruen |  | Crate-Liste ADR-0001 → baubare Crate `localchata` (lib + bin `localchatA`, Edition 2024), fmt und clippy grün | 2 |  |  |  | Integration; Module nach ADR-0001 zunächst leer |
| B0002 | Bubble | F0001 | Aufruf lesen | gruen |  | Argumente → `Startauftrag {ordner, port, host, browser_oeffnen}` \| `Aufruffehler` \| Hilfewunsch | 2 |  |  |  | Operation, rein; erster Unit-Test laut A00003 |
| B0003 | Bubble | F0001 | Ordner vorbereiten | gruen |  | `Startauftrag.ordner` → angelegter Ordner als absoluter Pfad \| `Startfehler` | 2 |  |  |  | Operation |
| B0004 | Bubble | F0001 | Listener binden | gruen |  | Host, Wunschport (Standard 7430 \| n \| 0) → gebundener Listener + tatsächliche Adresse \| `Startfehler` | 2 |  |  |  | Operation; Ausweichen höchstens 50 Versuche, nur beim Standard-Port |
| B0005 | Bubble | F0001 | Gesundheit beantworten | gruen |  | `GET /api/gesundheit` → 200 `{"status":"bereit","ordner",…}` | 2 |  |  |  | Integration an der HTTP-Grenze; Kennzahlen für leeren Ordner 0 |
| B0006 | Bubble | F0001 | Start verdrahten | gruen |  | `Startauftrag` → laufender Server + Startzeile auf stdout; SIGTERM/Strg+C → geordnetes Ende, Exit-Code | 2-4 |  |  |  | Integration (`main`); Signale und geordnetes Beenden unklar |
| B0007 | Bubble | F0001 | Testhilfe Programm starten | gruen |  | Wegwerf-Ordner → laufende Instanz mit Adresse aus der Startzeile, beendet beim Drop | 2 |  |  |  | `tests/gemeinsam/mod.rs` nach ADR-0009 |
| B0014 | Bubble | F0001 | Signale vor der Startzeile abfangen | gruen |  | Startauftrag → SIGTERM/SIGINT-Empfang eingerichtet, bevor die Startzeile erscheint; Signal ab der Startzeile → geordnetes Ende, Exit 0, Port und Sperre frei | 2 |  |  |  | Nacharbeit Code-Review B1 (Muss); Integration über das echte Programm, Signal per Systemaufruf |
| B0015 | Bubble | F0001 | Ausweichregel ohne festen Port prüfen | gruen |  | Portwunsch → Kandidaten und Ausweichen (rein); kein Test bindet 7430–7479 | 2 |  |  |  | Nacharbeit Code-Review B2 (Muss); Unit statt Integration auf festen Ports (R00001, ADR-0009) |
| B0016 | Bubble | F0001 | Startzeile mit Zeitgrenze abwarten | gruen |  | gestartetes Programm → Startzeile \| nach Zeitgrenze roter Test und beendeter Prozess statt Hängen | 2 |  |  |  | Code-Review A2 (optional); Testhilfe `tests/gemeinsam/mod.rs` |
| B0017 | Bubble | F0001 | Verbindungen begrenzen | gruen |  | angenommene Verbindung, Gegenstelle → bedient mit Kopf-Zeitgrenze (gilt auch als Leerlaufgrenze zwischen Requests) \| abgewiesen über der Grenze je Gegenstelle oder gesamt; Signal → offene Verbindungen nach Frist abgebrochen, Exit 0 | 2 |  |  |  | Nacharbeit Security-Review (Wichtig, CWE-400 Slowloris); Integration über das echte Programm, Angriffsmatrix Zeilen 1–6 |
| F0002 | Feature | I0001 | Ordner gegen zweite Instanz sperren | gruen | ein zweiter Start auf demselben Ordner endet mit Exit 1 und einer Meldung, die die Sperre nennt; die erste Instanz läuft weiter; nach ihrem Ende startet eine neue |  |  |  | F0001 | R00001 | ADR-0005 (Ordner-Sperre); A00003 |
| B0008 | Bubble | F0002 | Ordner sperren | gruen |  | angelegter Ordner → gehaltene Sperre `.localchatA.sperre` (`File::try_lock`) \| `Sperrfehler` | 2 |  |  |  | Operation |
| B0009 | Bubble | F0002 | Startfehler melden | gruen |  | `Startfehler` (Ordner, Port, Sperre) → eine Zeile auf stderr + Exit 1; `Aufruffehler` → Exit 2 | 2 |  |  |  | Integration; Aspekt X00004, X00001 |
| F0003 | Feature | I0001 | Oberfläche ausliefern | gruen | `GET /` liefert die eingebettete deutsche Seite mit `<title>localchatA</title>`, unbekannte `/web/…` → 404; ohne `--kein-browser` öffnet sich der Browser mit der Seite |  |  |  | F0001 | R00001 | ADR-0002; A00004, A00005 |
| B0010 | Bubble | F0003 | Eingebettete Datei nachschlagen | gruen |  | Pfad unter `/web/` → eingebettete Datei mit Content-Type \| keine | 2 |  |  |  | Operation, rein (`include_bytes!`) |
| B0011 | Bubble | F0003 | Oberfläche ausliefern | gruen |  | `GET /`, `GET /web/<datei>` → 200 mit Inhalt und `Cache-Control: no-cache` \| 404 | 2 |  |  |  | Integration an der HTTP-Grenze |
| B0012 | Bubble | F0003 | Browser öffnen | gruen |  | Adresse, `browser_oeffnen` → Aufruf des Standardbrowsers \| Hinweis auf stderr, Start läuft weiter | 2-4 |  |  |  | Operation; Mechanismus je Plattform unklar |
| B0013 | Bubble | F0003 | E2E-Gerüst | gruen |  | `e2e/` (Konfiguration, Bauen, Hilfen) → Playwright startet das Programm auf Port 0 im Wegwerf-Ordner und sieht die Seite | 2-4 |  |  |  | nach ADR-0009; erste Einrichtung Playwright unklar |
| I0002 | Interaction | D0001 | Nachricht landet im Log | gelb | jede Nachricht steht mit Absender und Zeit im Log ihres Channels; Logs rotieren täglich | | | | I0001 | | aus Vision; zwei Slices F0004 (Laden, sichtbar an der Grenze) und F0005 (Anhängen, Prüfung im Modul und über den Rundweg); keine Schreibroute (Grenzregel A00004); Entfallen alter Tage ist I0018 |
| F0004 | Feature | I0002 | Verlauf beim Start laden | gelb | ein Start auf einem Ordner mit Tagesdateien lädt je Channel die Tage in der Frist (heute nach lokalem Datum und die 6 Vortage, Standard 7); `/api/gesundheit` meldet `nachrichten`, `log_bytes` (Summe der geladenen Dateien) und `defekte_zeilen`; jede defekte Zeile steht auf stderr als `<datei>:<zeile>`; ältere Tage bleiben unangetastet liegen |  |  |  |  | R00002 | ADR-0003, ADR-0004, ADR-0006; A00003, A00006–A00009; X00004, X00007. Sichtbar an der Grenze ohne Schreibroute (Grenzregel A00004); Testuhr `LOCALCHATA_JETZT` |
| B0018 | Bubble | F0004 | Uhr mit Startzeit aus der Umgebung | gruen |  | `LOCALCHATA_JETZT` (RFC 3339) \| nicht gesetzt → `Uhr` (jetzt als Zeitpunkt mit lokalem Versatz, lokales Datum) \| `Aufruffehler` (Exit 2) | 0,4 |  |  |  | Operation; `src/uhr.rs` einziger Ort für die Systemuhr (X00007); belegt (Ist-Zeiten R00001, Typ Standard, bestätigt ≤ 0,05h → Untergrenze) |
| B0019 | Bubble | F0004 | Kennung bilden | gruen |  | Text → `Kennung` \| ungültig (leer, Leer- oder Steuerzeichen, nicht genau ein `@` zwischen Name und Herkunft; `:` einer IPv6-Herkunft erlaubt) | 0,4 |  |  |  | Operation, rein (A00007 Ergänzung); die Namensregel selbst ist I0004; belegt |
| B0020 | Bubble | F0004 | Sätze lesen | rot |  | Rohtext einer Tagesdatei → Sätze (Zeit, Kennung, Bytespanne) und `Defekt {zeile}`; Kopf mit unlesbarer Zeit oder Kennung = ein Defekt | 0,4 |  |  |  | Operation, rein (A00007 `saetze`, `text`; ADR-0004); belegt |
| B0021 | Bubble | F0004 | Ablage durchsuchen | rot |  | Ordner → Gespräche (`channels/<slug>`, `direkt/<name>`) mit ihren Tagen `JJJJ-MM-TT.log`; `lesen(gespraech, tag)` → Rohtext | 0,4 |  |  |  | Datei-Logablage (A00008), Integration gegen `tempfile`; andere Dateien ignoriert, `einstellungen.toml` wird nicht gelesen (I0007/I0018); belegt |
| B0022 | Bubble | F0004 | Tage in der Frist wählen | rot |  | Tage eines Gesprächs, heute, Frist 7 → Tage ab heute − 6 (spätere eingeschlossen) | 0,4 |  |  |  | Operation, rein; heute zählt mit (Entscheidung Karte #2); belegt |
| B0023 | Bubble | F0004 | Verlauf aufnehmen | rot |  | Gespräch, Tag, Rohtext, Sätze → Index (Rohtext, Satzanfänge); Kennzahlen Nachrichten und Bytes | 0,4 |  |  |  | Operation (A00009 `tag_aufnehmen`, `anhaengen`, Zählung); `block` und `suchen` gehören zu I0008/I0011; belegt |
| B0024 | Bubble | F0004 | Ordner laden | rot |  | Logablage, Uhr → `Gespraeche` mit Index, Defekten (Datei, Zeile), jüngster Nachrichtenzeit (als Zeitpunkt) und Ladezeit | 0,4 |  |  |  | Integration (A00006 `laden`); belegt |
| B0025 | Bubble | F0004 | Laden in den Start einhängen | rot |  | Startauftrag nach der Sperre → geladene Gespräche vor dem Binden; stderr je Defekt `<datei>:<zeile>`; `/api/gesundheit` mit `nachrichten`, `log_bytes`, `ladezeit_ms`, `defekte_zeilen` aus dem Index | 0,4 |  |  |  | Integration (A00003, A00004); Testhilfe `tests/gemeinsam/mod.rs` reicht `LOCALCHATA_JETZT` und `TZ` durch; belegt |
| F0005 | Feature | I0002 | Nachricht an das Tageslog anhängen | rot | `senden` in einen bekannten Channel hängt den Satz `@ <zeit> <kennung>` mit eingerücktem Text an die Tagesdatei des lokalen Datums der Nachrichtenzeit; Zeiten sind global streng steigend (gleiche Millisekunde → +1 ms, verglichen als Zeitpunkt); ein Schreibfehler lässt Datei und Index unverändert; ein Neustart lädt jede gesendete Nachricht wieder |  |  |  | F0004 | R00002 | ADR-0004, ADR-0005, ADR-0011; A00006–A00008; X00004, X00007. Ohne Schreibroute: geprüft in Modul- und Integrationstests, sichtbar an der Grenze über den Rundweg (senden → Neustart → `/api/gesundheit`); Sendeweg an der Grenze ab I0009 |
| B0026 | Bubble | F0005 | Satz schreiben | rot |  | `Nachricht {zeit, von: Kennung, text}` → Satztext (Kopf, Textzeilen mit zwei Leerzeichen, `\r\n` → `\n`, abschließende Leerzeilen entfallen, Leerzeile als Abschluss) | 0,4 |  |  |  | Operation, rein (A00007 `satz`); Rundweg mit B0020; belegt |
| B0027 | Bubble | F0005 | Zeit vergeben | rot |  | jetzt, letzte Zeit → `max(jetzt, letzte + 1 ms)` als Zeitpunkt | 0,4 |  |  |  | Operation, rein (ADR-0005, X00007 Ergänzung); belegt |
| B0028 | Bubble | F0005 | Satz an die Tagesdatei anhängen | rot |  | Gespräch, Tag, Satztext → in einem `write_all` angehängt; neuer Tag öffnet neue Datei; endet die Datei nicht auf `\n`, zuerst `\n` | 0,4 |  |  |  | Datei-Logablage (A00008; ADR-0011 Zeilenende-Wächter), Integration gegen `tempfile`; belegt |
| B0029 | Bubble | F0005 | Teilschreiben zurückbauen | rot |  | Schreibfehler nach n Bytes → Datei auf der Länge vor dem Schreiben, Fehler gemeldet \| Rückbau scheitert → Zeile auf stderr (Datei, Länge), nächster Satz nach `\n` | 2-4 |  |  |  | Operation an der Datei (ADR-0011); unklar: Fehler an einer echten Datei erzwingen (`RLIMIT_FSIZE` im Kindprozess) — Richtwert, ungemessen |
| B0030 | Bubble | F0005 | Speicher-Logablage | rot |  | Port-Aufrufe → Inhalt im Speicher; einstellbar „Fehler nach n Bytes“ | 0,4 |  |  |  | zweite Implementierung des Ports (A00008), kein Mock-Framework; belegt |
| B0031 | Bubble | F0005 | Nachricht senden | rot |  | Gespräch, Kennung, Text → `Nachricht {zeit}` \| unbekannt \| Schreibfehler; unter der Sperre des Gesprächs: Zeit → Satz → Tagesdatei nach lokalem Datum der Zeit → anhängen → erst dann Index | 0,4 |  |  |  | Integration (A00006 `senden`, ADR-0005, ADR-0011); belegt |
| I0003 | Interaction | D0001 | Fremde Änderung erscheint live | verworfen | Änderung per Editor oder `git pull` erscheint ohne Reload in allen Sichten | | | | I0002 | | verworfen 2026-10-27: Datenhaltung ohne Git (Vision) |
| D0002 | Dialog | A0001 | Teilnehmen | rot | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0004 | Interaction | D0002 | Namen wählen und beitreten | rot | Name plus Herkunft ergibt `name@herkunft`; ein vergebener Name vor dem `@` wird abgewiesen, egal von wo | | | | I0001 | | aus Vision, kein Requirement |
| I0005 | Interaction | D0002 | Teilnehmer mit Online-Status sehen | rot | Liste zeigt alle aktiven Teilnehmer mit online/offline | | | | I0004 | | aus Vision, kein Requirement |
| I0006 | Interaction | D0002 | Verbindung bricht ab | rot | sofort offline; nach etwa einer Minute ohne Rückkehr aus der Liste entfernt, Name wieder frei | | | | I0005 | | aus Vision, kein Requirement |
| D0003 | Dialog | A0001 | Channels | rot | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0007 | Interaction | D0003 | Channel anlegen | rot | neuer Channel erscheint in der Liste aller Teilnehmer | | | | I0004 | | aus Vision, kein Requirement |
| I0008 | Interaction | D0003 | Channel wechseln und Verlauf lesen | rot | Auswahl zeigt den Verlauf mit Absender und Zeit; bei 100.000 Nachrichten öffnet der Channel sofort und scrollt flüssig | | | | I0007 | | aus Vision, kein Requirement |
| I0009 | Interaction | D0003 | Nachricht senden | rot | Nachricht erscheint im Channel bei allen offenen Sichten sofort | | | | I0008, I0002 | | aus Vision, kein Requirement |
| I0010 | Interaction | D0003 | Anhänge senden und öffnen | rot | eine oder mehrere Dateien hängen an einer Nachricht und lassen sich öffnen | | | | I0009 | | aus Vision, kein Requirement |
| I0011 | Interaction | D0003 | Nachrichten durchsuchen | rot | Suche über alle sichtbaren Nachrichten liefert bei 100.000 Nachrichten Treffer ohne spürbare Wartezeit | | | | I0009 | | aus Vision, kein Requirement |
| I0017 | Interaction | D0003 | Nachricht mit Markdown formatieren | rot | Markdown in einer Nachricht erscheint formatiert | | | | I0009 | | aus Vision, kein Requirement; Designsystem-Baustein Markdown |
| I0018 | Interaction | D0003 | Aufbewahrungsfrist je Channel festlegen | rot | Frist je Channel wählbar, Standard 7 Tage; Logtage älter als die Frist entfallen | | | | I0007, I0002 | | aus Vision, kein Requirement |
| D0004 | Dialog | A0001 | Direktnachrichten und Erwähnungen | rot | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0012 | Interaction | D0004 | Direktnachricht senden | rot | nur Absender und Empfänger sehen das Gespräch | | | | I0009 | | aus Vision, kein Requirement |
| I0013 | Interaction | D0004 | Erwähnt werden | rot | `@name` hebt die Nachricht beim Erwähnten hervor | | | | I0009 | | aus Vision, kein Requirement |
| D0005 | Dialog | A0001 | Agent an der API | rot | alle Interactions gruen | | | | | | aus Vision, kein Requirement |
| I0014 | Interaction | D0005 | Als Agent beitreten | rot | Agent wird über die API mit `name@herkunft` Teilnehmer, gleiche Eindeutigkeit wie im Browser | | | | I0004 | | aus Vision, kein Requirement |
| I0015 | Interaction | D0005 | Nachricht über die API senden | rot | in Channel oder direkt; erscheint live in der Oberfläche | | | | I0014, I0009 | | aus Vision, kein Requirement |
| I0016 | Interaction | D0005 | Auf Nachrichten warten | rot | Agent wartet blockierend auf Neues für sich (Channel, Direktnachricht, Erwähnung) und erhält es ohne Polling-Schleife | | | | I0015 | | aus Vision, kein Requirement |

## Offene Fragen

- I0002 (verfeinern --still, 2026-10-27): Aufwand der Standard-Bubbles aus den Ist-Zeiten von R00001 (Typ Standard, `bestaetigt` 0,01–0,05h, autonom) auf die Untergrenze 0,4h gehoben; die Messwerte wirken wie ein Artefakt des Commit-Fensters — prüfen, ob sie als Beleg taugen. B0029 bleibt Richtwert 2-4 (Bandbreite statt Untergrenze, damit die Unsicherheit sichtbar bleibt).
- I0002: F0005 ist bis I0009 nur über Modul- und Integrationstests und den Rundweg prüfbar (keine Schreibroute, Grenzregel A00004) — von außen sichtbar wird das Senden erst mit I0009/I0015.
- I0002: ADR-0003 „Tag D entfällt, sobald D älter ist als frist_tage Kalendertage vor heute“ lässt offen, ob heute mitzählt; entschieden für R00002: heute zählt mit (Frist 7 = heute und 6 Vortage). Den Wortlaut der ADR gleicht der architekt an; das Entfallen selbst baut I0018.
- B0006 Start verdrahten, B0012 Browser öffnen, B0013 E2E-Gerüst als unklar (2-4) geschätzt — Signale, Plattform-Aufruf des Browsers und erste Playwright-Einrichtung sind im Projekt noch unerprobt (verfeinern --still, 2026-10-27).

## Notizen / Quellen

- Alle Knoten aus `Anforderungen/R00000-vision.md` (Stand 2026-10-27); noch keine Anforderungen.
- Designsystem unter `Dokumentation/Wireframes/designsystem/` (Export vom 2026-10-27) mit eigenen offenen Fragen in `3-offene-fragen.md`; Briefing `design-prompt.md`.
