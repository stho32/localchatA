# ADR-0042: Stufen und Ereigniskatalog des Betriebsprotokolls — Error heißt „die Anfrage scheitert erwartbar“, Exception heißt „Programmfehler“; 4xx nur als Debug oder als Episode; Tagesbudget je Stufenklasse, ausgelegt auf das Lastprofil „Team-LAN“

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Vorlauf „Betriebsprotokoll“)
**Kontext-Anforderung**: — (neue Interaction „Betriebsprotokoll“)
**Erprobt durch**: — (Mengen gerechnet, nicht gemessen; Messung als Folgeaufgabe)

## Kontext

- Vorgabe des Betreibers: Stufen Debug · Information · Warning · Error (Fachlogik) · Exception (Programmfehler); Standard Information; Fehler müssen irgendwo mitgeschrieben werden (`AI-Team/umgebung.md`). Prüfraster des `observability-reviewer`: jeder Fehlerpfad hat eine Spur, richtige Stufe, Kontext ohne Secrets, Fluten als Episode.
- **Lastprofil „Team-LAN“** (Betreiber 2026-10-28, `AI-Team/umgebung.md`): bis 20 Menschen und 50 Agenten gleichzeitig; Spitze 30 Nachrichten/s gesamt, dauerhaft 5/s; bis 100.000 Nachrichten je Channel, 50 Channels aktiv; normaler Rechner im LAN.
- Lehre CWE-770 aus den Befundkarten #5/#6 (`ADR-0010` Ergänzung, `ADR-0036`, R00010): alles, was ein Client auslösen kann, braucht eine Grenze — auch das, was er ins Protokoll schreiben lässt.
- Bestand: entschiedene stderr-Zeilen in ADR-0004, ADR-0011, ADR-0017 Regel 4, ADR-0021, ADR-0030 Regel 6, ADR-0031 Regel 5, ADR-0032 Regeln 6 und 8, ADR-0035 Regel 4, ADR-0036 Regel 7; Verbindungsabweisung in der Annahmeschleife heute **ohne** Spur (`src/start/ausliefern.rs:44–51`); `JoinError` an der Grenze wird zu 500 **ohne** Spur (`src/http/nachrichten.rs:40`, `src/http/channels.rs:52, 60`); Panik in einem Hintergrund-Task (`src/start/mod.rs:114`, Takt) beendet ihn **still**.
- [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) (Format), [ADR-0041](0041-betriebsprotokoll-als-eigenes-modul-rotation-beim-schreiben-bereinigung-14-tage.md) (Modul, Tagestakt).

## Optionen

### Abgrenzung Error gegen Exception

#### Option A: Exception nur für Programmfehler (Panik, verletzte Annahme), genau eine je Fehler; die Folge an der Grenze ist Error
- Vorteile: die Zahl der Exceptions ist die Zahl der Programmfehler — ein Monitoring-Signal ohne Rauschen; die Grenze ergänzt Route und Kennung, die der Panik-Haken nicht kennt.
- Nachteile: eine Panik in einem Handler ergibt zwei Zeilen (Exception + Error), die man über die Zeit zusammenliest.

#### Option B: Jede 5xx-Antwort ist Exception
- Vorteile: eine Regel an einer Stelle (Statuscode).
- Nachteile: volle Platte ist kein Programmfehler; das Signal „Exception > 0“ hieße dann auch „Platte voll“ — der Betreiber kann Bug und Betrieb nicht trennen.

#### Option C: Panik-Haken schreibt nichts, nur die Grenze schreibt Exception aus dem `JoinError`
- Vorteile: eine Zeile je Fehler, mit Route.
- Nachteile: Paniken außerhalb von `spawn_blocking` an der Grenze (Hintergrund-Tasks, Verbindungs-Tasks, Start) blieben ohne Spur — genau der heutige stille Pfad.

### 4xx

#### Option D: 4xx nie protokollieren
- Vorteile: kein Rauschen, keine Flut über das Protokoll.
- Nachteile: Angriffe (fremder Host, fremder Origin) und Fluten (429) bleiben unsichtbar; ADR-0036 Regel 7 verlangt eine Spur.

#### Option E: Jede 4xx-Antwort als Information
- Vorteile: lückenlos.
- Nachteile: Clients bestimmen die Menge; eine Flut von 429 (ADR-0036 S3) schriebe Zeilen im Takt der Anfragen — CWE-770 über das Protokoll.

#### Option F: Einzelne 4xx als Debug; Abweisungen mit Betriebsbedeutung (Grenzschutz, Grenzen, Bremse) als Warning-**Episode** je Quelle
- Vorteile: im Standard nur, was der Betreiber handeln muss, gedrosselt; zum Nachstellen eines Einzelfalls Debug zuschalten.
- Nachteile: zwei Mechanismen (Debug je Anfrage, Episode je Quelle); eine Episode verdeckt Einzelheiten (welche Route genau).

### Grenze gegen Flut

#### Option G: Tagesbudget je Stufenklasse (Debug · Information+Warning · Error+Exception) plus Episoden
- Vorteile: zugeschaltetes Debug kann Fehler nicht verdrängen; die Platte ist auch im schlimmsten Fall begrenzt (Budget × 14 Tage).
- Nachteile: drei Zähler; nach Erreichen sind Einträge des Tages verloren (nur gezählt).

#### Option H: Ein Gesamtbudget je Tag
- Vorteile: eine Zahl.
- Nachteile: Debug oder eine Warning-Flut verdrängen am selben Tag die Errors — das Gegenteil des Ziels.

#### Option I: Nur Episoden, kein Budget
- Vorteile: kein Datenverlust.
- Nachteile: Debug ist nicht episodisch; vergessenes Debug im Lastprofil schreibt ≈ 4 GB am Tag (Rechnung unten).

## Entscheidung

**Gewählt: A, F, G**

Es überwiegt, dass „Exception > 0“ eindeutig „Programmfehler“ heißt, Clients die Protokollmenge nicht bestimmen können und Fehler nie von Debug verdrängt werden. In Kauf genommen: eine Handler-Panik ergibt zwei Zeilen; Einzelheiten einer Episode sieht nur, wer Debug zuschaltet; über dem Budget gehen Einträge des Tages verloren — sie werden gezählt und im Health-Check gezeigt.

Regeln:

1. **Stufen** (Standard Information, ADR-0043):

   | Stufe | Wofür | Monitoring-Bedeutung |
   |---|---|---|
   | `DEBUG` | Ablauf je Anfrage und Takt, alle einzelnen 4xx | nur zur Diagnose zugeschaltet |
   | `INFORMATION` | Ereignisse des Betriebs, die gelingen: Start, Ende, Beitreten, Verlassen, weg, Channel angelegt, Frist gesetzt, Entfallen, Bereinigung, Episode endet | Verlauf des Tages |
   | `WARNING` | ungewöhnlich, abgefangen, ohne Datenverlust: Ladedefekte, Grenzen und Bremsen (Episode), Grenzschutz (Episode), Uhr läuft rückwärts, Pflege nicht setzbar, Protokoll-Rückfall | hinsehen, wenn es sich häuft |
   | `ERROR` | eine Operation scheitert erwartbar an der Umgebung, der Nutzer merkt es (500, nicht gesendet, nicht gelöscht, nicht angelegt) | handeln (Platte, Rechte) |
   | `EXCEPTION` | Programmfehler: Panik, verletzte Annahme, die der Code ausdrücklich erkennt | Fehler melden, Version nennen |

2. **Ereigniskatalog** — heute gebaut (✓) bzw. entschieden (→ Karte, die es anbindet). Kontext immer nach ADR-0040 Regeln 3–5.

   | Bereich | Ereignis | Stufe | Kontext | Quelle |
   |---|---|---|---|---|
   | `start` | Instanz bereit (nach der Startzeile) | INFORMATION | `version`, `adresse`, `ordner`, `stufe`, `anzahl` (Nachrichten), `dauer_ms` (Laden) | ✓ A00003 |
   | `start` | Port, Laden oder Bereitstellen scheitert nach der Sperre | ERROR | `grund` | ✓ `src/start/mod.rs:73–81` |
   | `start` | Signal nicht abfangbar; Browser nicht zu öffnen | WARNING | `grund` | ✓ `src/start/mod.rs:203, 229`, `browser.rs:32` |
   | `ende` | Ende-Signal, Instanz beendet | INFORMATION | `grund` (Strg+C/SIGTERM), `dauer_ms` (Laufzeit) | neu |
   | `laden` | defekte Zeile | WARNING | `datei`, `zeile`; je Datei höchstens 10 Zeilen einzeln, dann eine Zeile „und n weitere“ | ✓ ADR-0004 |
   | `laden` | Einstellungsdatei defekt; Ordner mit ungültigem Channelnamen | WARNING | `datei`, `grund` | ✓ ADR-0022 Regel 3, ADR-0020 Regel 6 |
   | `verbindung` | Verbindung abgewiesen (Platzgrenze je Herkunft oder gesamt) | WARNING, Episode je Herkunft | `herkunft`, `grenze` | neu (heute still) |
   | `verbindung` | Abrisspflege nicht setzbar (erster Fehler je Lauf) | WARNING | `option`, `grund` | ✓ ADR-0017 Regel 4 |
   | `verbindung` | angenommen, beendet | DEBUG | `herkunft`, `dauer_ms` | neu |
   | `grenzschutz` | fremder Host oder Origin (403) | WARNING, Episode je Herkunft | `herkunft`, `grund` (Host/Origin, escaped, gekürzt) | neu, ADR-0014 |
   | `sitzung` | beigetreten, verlassen, weg (Name frei) | INFORMATION | `kennung` | neu, ADR-0012, ADR-0016, ADR-0018 |
   | `sitzung` | online/offline-Wechsel | DEBUG | `kennung` | neu |
   | `sitzung` | Sitzungsgrenze erreicht (429) | WARNING, Episode je Herkunft | `herkunft`, `grenze` | neu |
   | `channels` | angelegt | INFORMATION | `channel`, `kennung` | neu, ADR-0021 |
   | `channels` | Channel-Grenze erreicht (429) | WARNING, Episode je Herkunft | `herkunft`, `grenze` | neu, ADR-0021 |
   | `channels` | Ordner/Einstellungen nicht anlegbar (500), Rückbau | ERROR | `channel`, `datei`, `grund` | ✓ ADR-0021, `channels.rs:135` |
   | `verlauf` | Block gelesen | DEBUG | `channel`, `anzahl`, `dauer_ms` | neu |
   | `senden` | Nachricht nicht gesendet (Schreibfehler, 500) | ERROR, Episode je `datei`+`grund` | `channel`, `kennung`, `datei`, `grund` | → I0009, ADR-0011 |
   | `senden` | Rückbau gescheitert | ERROR | `datei`, `grund`, `anzahl` (Länge) | ✓ ADR-0011, `logablage.rs:153` |
   | `schreibkonto` | gebremst (429) | WARNING, Episode je Kennung | `kennung`, `grenze` | → I0009, ADR-0036 Regel 7 |
   | `zeit` | Nachrichtenzeit läuft der Uhr voraus (> 60 s) | WARNING, Episode | `dauer_ms` | → I0009, ADR-0035 Regel 4 |
   | `frist` | Frist gesetzt; Tage entfallen | INFORMATION | `channel`, `kennung`, `anzahl`, `grund` (Anlass) | → I0018, ADR-0030 Regel 6, ADR-0031 Regel 5 |
   | `frist` | Tag nicht entfallen | ERROR | `datei`, `grund` | → I0018, ADR-0032 Regel 6 |
   | `protokoll` | Bereinigung; Rückfall auf stderr; Budget erreicht | INFORMATION / WARNING / WARNING | ADR-0041 Regel 7, ADR-0040 Regel 8, Regel 6 hier | neu |
   | `programmfehler` | Panik (Haken) | EXCEPTION | `ort` (Datei:Zeile), `thread`, Text der Panik als `<text>` | neu |
   | `programmfehler` | Anfrage scheiterte an einer Panik oder einem Abbruch (`JoinError`) | ERROR | `route`, `status=500`, `kennung` falls bekannt | neu |
   | `programmfehler` | Hintergrundaufgabe beendet (Takt, Tagestakt, Pflege) | ERROR | `grund` (Aufgabe und Folge: „Anwesenheit wird nicht mehr gepflegt“) | neu |
   | jede Route | einzelne 400, 401, 404, 405, 409, 413, 415, 422 | DEBUG | `route`, `status`, `kennung` oder `herkunft`, `grund` (Regel) | neu |

   Neue Fehlerpfade späterer Karten tragen ihre Zeile in diese Tabelle ein (Ergänzung dieser ADR), bevor gebaut wird.

3. **Wo „stderr“ in einer älteren ADR steht**, gilt ab dieser ADR: ein Eintrag im Betriebsprotokoll mit der Stufe aus Regel 2; stderr spiegelt ihn ab Warning (ADR-0040 Regel 6). Texte und Episodenregeln der älteren ADRs bleiben gültig.
4. **Programmfehler**:
   - **Panik-Haken** (`std::panic::set_hook`, gesetzt direkt nach dem Öffnen des Protokolls, ersetzt den Standard-Haken): schreibt genau einen `EXCEPTION`-Eintrag je Panik, in jedem Thread (tokio-Worker, `spawn_blocking`, Start). `try_lock` auf das Protokoll (ADR-0041 Regel 3); Zähler und Zeit der letzten Exception für den Health-Check setzt er immer, auch wenn nicht geschrieben werden kann.
   - **An der Grenze**: jedes `.await` auf `spawn_blocking` bzw. einen `JoinHandle` bildet `Err(JoinError)` auf 500 **und** einen `ERROR programmfehler`-Eintrag mit Route ab — eine gemeinsame Hilfsfunktion im HTTP-Baustein, nicht je Handler von Hand.
   - **Hintergrundaufgaben** werden über `bewacht(name, folge, future)` gestartet: endet die Aufgabe vor dem Ende-Signal (Panik oder Rückkehr), schreibt die Wache `ERROR programmfehler Hintergrundaufgabe <name> beendet — <folge>`; der Health-Check nennt sie (ADR-0043).
   - **Verletzte Annahme ohne Panik**: wo der Code einen unmöglichen Zustand erkennt und weiterarbeitet, ruft er `protokoll::exception(bereich, text, kontext)` statt still zu ersetzen (X00004).
   - **Testknopf** `LOCALCHATA_TESTPANIK=<ort>` (nur mit `debug_assertions`, also nicht im Release-Build): `verlauf` lässt den nächsten Verlaufsblock im `spawn_blocking` paniken, `takt` den Anwesenheits-Takt einmal. Damit prüft der `observability-reviewer` Exception, Error an der Grenze und Wache im echten Programm (`cargo run`); im Release-Build ist die Variable wirkungslos.
5. **Episode** (eine gemeinsame Mechanik im Modul, Schlüssel = Bereich + Ereignis + Quelle, Quelle = Kennung, Herkunft oder Datei+Grund):
   - erstes Auftreten → ein `WARNING`- bzw. `ERROR`-Eintrag „… — Episode beginnt“;
   - weitere Auftreten → nur gezählt; läuft die Episode länger, **eine** Zwischenzeile je 10 Minuten „läuft weiter — n seit Beginn“;
   - 60 s ohne Auftreten → `INFORMATION` „… — Episode endet: n in d s“; geprüft im Sekundentakt des Tagestakts (ADR-0041 Regel 6 — der Takt ruft je Sekunde `episoden_schliessen(jetzt)`, je Tageswechsel die Tagesaufgaben);
   - höchstens **1.024** offene Episoden; jede weitere Quelle zählt in eine Sammelepisode „weitere Quellen“ derselben Art.
   - Zeiten über den Uhr-Provider; eine Uhr, die zurückspringt, beendet keine Episode vorzeitig (`jetzt − zuletzt` nie negativ).
6. **Tagesbudget** je Protokolldatei (lokaler Tag), getrennt nach Stufenklasse:

   | Klasse | Budget je Tag | Grund (Lastprofil „Team-LAN“) |
   |---|---|---|
   | `DEBUG` | 64 MiB | Debug je Anfrage: 50 Agenten mit wartendem Long-Poll wecken bei 5 Nachrichten/s ≈ 250 Anfragen/s, ≈ 200 B je Zeile → ≈ 50 KB/s, ≈ 4 GB/Tag; 64 MiB tragen ≈ 20 Minuten Diagnose in voller Last bzw. Stunden in normaler Last — Debug ist für den Einzelfall |
   | `INFORMATION` + `WARNING` | 32 MiB | normaler Tag gerechnet ≈ 1.500 Einträge ≈ 0,3 MB (600 Sitzungen × Beitreten/Weg, Start, Pflege) — Faktor ≈ 100; Episoden-Höchstfall 1.025 Quellen × 146 Zeilen ≈ 30 MB passt knapp |
   | `ERROR` + `EXCEPTION` | 32 MiB | ≈ 160.000 Einträge; Fehler gleicher Ursache sind Episoden, also wird das nur bei sehr vielen verschiedenen Ursachen erreicht |

   Höchstens 128 MiB je Tag, über 14 Tage höchstens 1,75 GiB — zum Vergleich: das Lastprofil schreibt dauerhaft 5 Nachrichten/s × ≈ 530 B ≈ 230 MB Gesprächslog je Tag (ADR-0006, Rechnung je Nachricht). Erreicht eine Klasse ihr Budget, schreibt das Modul **eine** `WARNING protokoll`-Zeile „Tagesbudget <Klasse> erreicht — weitere Einträge bis Mitternacht nur gezählt“ (zählt nicht ins Budget) und zählt jeden weiteren als `protokoll_verworfen`; ab dem nächsten lokalen Tag gilt ein neues Budget. Die Zähler im Health-Check (Error, Exception) zählen **unabhängig** vom Budget weiter.
7. **Was nie ins Protokoll kommt**: ADR-0040 Regel 5. Zusätzlich: keine Zeile je Nachricht auf Information (Nachrichten stehen im Tageslog); Debug nennt bei Nachrichten nur `channel`, `kennung`, `anzahl` (Bytes) und `dauer_ms`.

## Konsequenzen

**Positiv:**
- Monitoring kann an zwei Zahlen hängen: Exceptions (Programmfehler) und Errors (Betrieb) — siehe ADR-0043.
- Kein Client kann das Protokoll fluten: Einzel-4xx sind Debug, Abweisungen mit Bedeutung sind Episoden, alles hat ein Tagesbudget.
- Die heute stillen Pfade (Verbindungsabweisung, `JoinError`, Hintergrund-Panik) bekommen eine Spur — genau die Pfade, an denen 50 Agenten auf einem Rechner zuerst anstoßen (A00001, Ergänzung Lastprofil).

**Negativ / Inkauf zu nehmen:**
- Eine Handler-Panik ergibt zwei Zeilen (EXCEPTION + ERROR) mit gleicher Zeit.
- Einzelheiten einer Episode (welche Route, welcher Körper) sieht nur Debug.
- Budget erreicht ⇒ Einträge des Tages fehlen; der Health-Check zeigt, wie viele.
- Der Testknopf ist Code im Programm, der im Release nichts tut.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein Agent nach jedem Neustart erst 401 bekommt und das Protokoll auf Information voll mit „nicht angemeldet“ steht. Früh-Indikator: Anteil `status=401` unter Debug. Gegenmaßnahme: Regel 2 — 401 ist Debug.
2. … ein Rechner mit 50 Agenten an der Verbindungsgrenze hängt und niemand es merkt, weil die Abweisung still ist. Früh-Indikator: Warning-Episode `verbindung` im Protokoll, `warnings` im Health-Check. Gegenmaßnahme: Regel 2 (neu: Episode je Herkunft) — und die Grenze selbst (A00001, Ergänzung Lastprofil, Vorschlag an den Director).
3. … die Episoden-Tabelle bei einem Angreifer mit vielen IPv6-Adressen wächst. Früh-Indikator: Sammelepisode „weitere Quellen“. Gegenmaßnahme: Regel 5, höchstens 1.024 offene Episoden.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test Episode (gestellte Uhr, Protokoll in eine Speicher-Senke): 100 × `grenzschutz` von `10.0.0.9` in 10 s → genau ein `WARNING … Episode beginnt | herkunft=10.0.0.9`; Uhr +59,999 s nach dem letzten → keine weitere Zeile; +60,000 s → genau ein `INFORMATION … Episode endet: 100 in 10 s`. Mutanten: Ende bei 59 s → rot; Zeile je Auftreten → rot. Ersetzt: Uhr, Senke.
- [ ] Unit Budget (Budget im Konstruktor klein, z. B. 1 KiB): Debug bis über das Budget, dann ein Error → der Error steht in der Datei, genau eine „Tagesbudget DEBUG erreicht“-Zeile, `protokoll_verworfen` > 0; Uhr auf den nächsten Tag → Debug wird wieder geschrieben.
- [ ] Integrationstest Exception (echtes Programm, Debug-Build, `LOCALCHATA_TESTPANIK=verlauf`): Beitreten, `GET /api/channels/allgemein/nachrichten` → 500; im Protokoll genau eine `EXCEPTION programmfehler`-Zeile mit `ort=src/` und danach eine `ERROR programmfehler`-Zeile mit `route="GET /api/channels/{name}/nachrichten" status=500`; das Programm antwortet weiter auf `/api/gesundheit`; `grep <token> logs/*.log` = 0.
- [ ] Integrationstest Verbindungsgrenze (echtes Programm): 65 offene Verbindungen von 127.0.0.1 → genau eine `WARNING verbindung … Episode beginnt | herkunft=127.0.0.1`; 30 weitere abgewiesene → keine zusätzliche Zeile.
- [ ] Messwert (kein Gate): Zeilen und Bytes je Tag unter einer Last nach Profil (5 Nachrichten/s, 50 wartende Agenten) auf Information und auf Debug — bestätigt oder korrigiert die Budgets.

## Quellen / Bezuege

- [A00015 Betriebsprotokoll](../Architektur/A00015-betriebsprotokoll.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00003 Start](../Architektur/A00003-start.md), [A00001 localchatA](../Architektur/A00001-localchata.md) (Lastprofil); Aspekte [X00001](../Architektur/Aspekte/X00001-logging-und-diagnose.md), [X00004 Fehlerbehandlung](../Architektur/Aspekte/X00004-fehlerbehandlung.md)
- ADR-0004, ADR-0006, ADR-0010, ADR-0011, ADR-0012, ADR-0014, ADR-0016, ADR-0017, ADR-0018, ADR-0020, ADR-0021, ADR-0022, ADR-0030, ADR-0031, ADR-0032, ADR-0035, ADR-0036, ADR-0040, ADR-0041, ADR-0043
- `AI-Team/umgebung.md` „Logging-Konzept“ und „Lastprofil Team-LAN“; Factory-Rolle `observability-reviewer` (Standardstufen, Einstufung); CWE-770 — aus dem Gedächtnis

## Ergänzungen

- **2026-10-28 (Proben des `observability-reviewer` an #21 Körpergrenze) — Körperfehler, JoinError, Episodenende:** Sie präzisiert die Regeln 2, 4 und 5; ersetzt wird keine.
  1. **Ein zentraler Eintrag für Körperfehler**: `json_lesen` (`src/http/koerper.rs`) bekommt einen kleinen Kontext `{route, herkunft, kennung?}` vom Handler und trägt **selbst** ein, nicht jeder `kein_json` in `beitreten.rs` und `channels.rs` für sich. Kontext: `route` (Muster), `herkunft`, bei Routen mit Sitzung `kennung`, `art`, `grenze`, `bytes` (gelesen), `verworfen`, bei `KeinJson` `typ` (escaped, gekürzt). Nie Körperinhalt, nie `X-LocalchatA-Sitzung`.
  2. **Arten**: `Koerperfehler` bekommt die Art `Abgebrochen` (Lesefehler im Strom: der Client ist weg). Sie wird von `Kaputt` (Körper gelesen, kein gültiges JSON) getrennt. Heute bildet `begrenzt_lesen` beides auf `Kaputt` ab (`koerper.rs:45`). Abgebrochen bekommt keine sinnvolle Antwort mehr, wird aber gezählt.
  3. **Stufen der Körperfehler**: `ZuGross` (413) und `Kaputt` (400) laufen als **WARNING-Episode je Herkunft und Route** (Schlüssel `körper` + Herkunft + Route). Die erste Abweisung schreibt sofort eine Zeile, dann wird nur gezählt, und zum Schluss steht **eine** Zusammenfassung. `KeinJson` (415), `Abgebrochen` und alle übrigen einzelnen 400 und 422 (Regeln zu Feld, Name, Text) stehen auf DEBUG. `Abgebrochen` zählt zusätzlich in eine offene Körper-Episode derselben Quelle.
  4. **Verwerfen**: das Ergebnis von `rest_verwerfen` — `vollständig`, `frist` (2 s abgelaufen) oder `obergrenze` (16 MiB erreicht) — steht als `verworfen=` in der DEBUG-Zeile. Die Zusammenfassung der Episode zählt es je Ergebnis.
  5. **Zusammenfassung der Episode** (ändert Regel 5): Die Ende-Zeile hat die **Stufe der Episode** statt INFORMATION, damit Beginn und Zusammenfassung auch bei `--protokoll-stufe warning` zusammenbleiben. Sie nennt die Zahl je Art und je Verwerfen-Ergebnis, die Dauer und die größte gelesene Bytezahl. Beispiel: `WARNING körper Körper abgewiesen — Episode endet: 200 in 4 s (zu_gross 200; verworfen vollständig 200) | herkunft=10.0.0.9 route="POST /api/beitreten" bytes=5120`. Die Ruhe bis zum Ende bleibt 60 s, die Zwischenzeile bleibt alle 10 Minuten. Die vorgeschlagenen 60 s als Takt der Zusammenfassung hätten bei Dauerflut 1.440 Zeilen je Quelle und Tag ergeben statt 144, was das Budget aus Regel 6 sprengt. Für Integrationstests wird die Ruhe einstellbar über `LOCALCHATA_EPISODE_RUHE_MS` (Standard 60.000; 0 ist Fehler, Exit 2), wie die übrigen Testknöpfe.
  6. **JoinError ohne Panik**: Regel 4 bleibt — die Panik selbst schreibt der Haken als EXCEPTION mit `ort`, an der Grenze steht dann ERROR mit `route`, `operation` und `grund=panik`. **Neu**: Ist der `JoinError` **keine** Panik (`is_cancelled`), gibt es keine Haken-Zeile. Dann schreibt die Grenze selbst **EXCEPTION** mit `route`, `operation` und `grund=abgebrochen`, denn ein abgebrochener Blocking-Task außerhalb des Endes ist eine verletzte Annahme. Betroffen sind heute `channels.rs:52` (`channel_anlegen`), `channels.rs:60` (`channels`) und `nachrichten.rs:40` (`verlauf`). Der englische Text des `JoinError`, den `nachrichten.rs:42` heute in die Antwort schreibt, gehört nur ins Protokoll (`grund`). Die Antwort sagt deutsch „Interner Fehler — Verlauf nicht gelesen; bitte erneut versuchen“, ohne Fremdtext.
  7. **Start und Ende** (Regel 2, bestätigt): INFORMATION `start` nach der Startzeile. INFORMATION `ende` mit `grund=Strg+C|SIGTERM` und der Laufzeit, geschrieben, sobald das Ende-Signal eintrifft, und ein zweites Mal nach dem Ablauf der Ende-Frist („beendet“), damit ein Abbruch mitten im Herunterfahren sichtbar bleibt.
  8. **Gegenprobe für die Karte** (Abnahme-Probe des `observability-reviewer`): `LOCALCHATA_EPISODE_RUHE_MS=1000`, von `127.0.0.1` 200 × `POST /api/beitreten` mit 5 KB JSON, im Körper der Marker `KOERPER-MARKER-7f3a`; vorher eine echte Sitzung beitreten und ihr Token merken. Erwartet nach 2 s Ruhe:
     - für `route="POST /api/beitreten" herkunft=127.0.0.1` genau eine `WARNING … Episode beginnt` und genau eine `WARNING … Episode endet: 200 …`, keine weitere WARNING-Zeile dieser Quelle;
     - `/api/gesundheit` zeigt `abgewiesen.zu_gross = 200`;
     - `grep -c KOERPER-MARKER-7f3a logs/*.log` = 0 und `grep -c <token> logs/*.log` = 0.

     Mutanten, die rot werden müssen: eine Zeile je Abweisung (200 statt 2), keine Ende-Zeile, Ende-Zeile als INFORMATION bei `--protokoll-stufe warning` (fehlt dann), Körper im Kontext (Marker gefunden).
