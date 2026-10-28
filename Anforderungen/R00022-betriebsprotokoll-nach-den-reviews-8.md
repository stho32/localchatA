---
id: R00022
status: Neu
datum: 2026-10-28
---

# R00022: Betriebsprotokoll nach den Reviews #8 nachgezogen

## Beschreibung

Das Betriebsprotokoll aus R00014 und seine Anbindung des Sendens aus R00016 schließen vier Lücken, die der Observability-Review zu #8 gefunden hat (O1–O4). Danach gilt:

- **Gelungenes Senden ist auf Debug sichtbar**: wer die Stufe Debug zuschaltet, um einen Einzelfall nachzustellen, sieht je gelungenem Senden eine Zeile „Nachricht gesendet“ mit Channel, Kennung, Länge und Dauer — nie den Text.
- **Ein Vorlauf der Nachrichtenzeit fällt schon beim Start auf**: startet die Instanz mit einer zurückgestellten Uhr, steht die `WARNING zeit` im Protokoll, bevor die erste Nachricht kommt.
- **Ein überholter Ereignisstrom ist eine Warnung**: fällt der Strom eines Teilnehmers so weit zurück, dass er einen neuen Stand bekommt, steht das als `WARNING live`-Episode je Kennung mit der Zahl verpasster Ereignisse im Protokoll.
- **Die Ende-Zeile einer Episode nennt nur, was für die ganze Episode gilt**: den Kontext ihrer Quelle, die Zählungen und zusammengefasste Werte. Werte eines einzelnen Auftretens — etwa die Kennung, die zuerst an einer vollen Platte scheiterte — stehen nur in der Beginn-Zeile.

`/api/gesundheit` bekommt kein neues Feld: Aufteilungen zur Diagnose stehen im Protokoll, der Health-Check nennt nur, ob und wie oft etwas geschieht.

Zahlt ein auf: [Vision](R00000-vision.md) — „eine gemeinsame LAN-Instanz“ (die Betreiberin erkennt, wenn ein Rechner nicht mitkommt oder die Uhr zurückgestellt ist, bevor Nutzer es melden) und die Leitplanke „Team-LAN“ (Zustellung live < 1 s; ein überholter Strom ist genau der Fall, in dem ein Tab eine Nachricht erst nachholen muss). Kein Widerspruch zur Vision.

Herkunft: Karte #31 — Observability-Review #8, Optional O1–O4; entschieden vom architekt in ADR-0042, Ergänzung Karte #31, Punkte 3–6, und ADR-0043, Ergänzung Karte #31.

Slice (WBS `Dokumentation/Planung/localchata.md`, unter I0021 Betriebsprotokoll): F0073 Betriebsprotokoll nach den Reviews #8 nachgezogen. Zugeordnet zu I0021, nicht zu I0009: drei der vier Punkte sind kein Senden (Regel der Episoden-Mechanik für alle Bereiche, Ereignisstrom, Start), und das Fertig-Kriterium von I0021 trägt sie.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00014 und R00016: Betriebsprotokoll, Stufe, Bereich, Kontext, Episode (Beginn-Zeile, Zwischenzeile, Ende-Zeile), Quelle, Auftreten, Ruhe, Vorlauf, Ereignisstrom, neuer Stand.

## Geschäftlicher Nutzen

Heute stolpert die Betreiberin an vier Stellen. Stellt sie auf Debug einen Einzelfall nach, sieht sie nur die gescheiterten Sendungen; für die gelungenen muss sie das Tageslog lesen, das die Dauer nicht kennt. Startet die Instanz nach einem Neustart mit zurückgestellter Uhr, erfährt sie es erst mit der nächsten Nachricht. Kommt ein Rechner oder eine Leitung nicht mit und ein Tab bekommt einen neuen Stand, steht nirgends etwas — gerade auf diesem Pfad fehlt im Tab eine Nachricht, bis nachgeholt ist. Und die Ende-Zeile einer Schreibfehler-Episode nennt die Kennung, die zuerst scheiterte, als wäre sie die einzige — bei einer vollen Platte ist aber die Datei die Ursache, nicht der Absender.

Mit R00022 liest die Betreiberin aus dem Protokoll, was im Betrieb geschieht, ohne falsche Spur.

## Funktionale Anforderungen

### Betriebsprotokoll nach den Reviews #8 nachgezogen (F0073; ADR-0042 Ergänzung Karte #31 Punkte 3–6)

- **Ende-Zeile mit dem Kontext der Quelle** (O3; Punkt 4): die Ende-Zeile einer Episode trägt
  1. die Schlüssel der **Quelle**, aus denen der Episodenschlüssel gebildet ist, und was aus ihr folgt (`channel` aus `datei`);
  2. die Zählungen (`n`, je Art, je Verwerfen);
  3. zusammengefasste Werte: `bytes` = der größte, `dauer_ms` = der letzte.

  Werte eines **einzelnen** Auftretens, die über die Episode wechseln können, stehen nur in der Beginn-Zeile. Betroffen heute:

  | Episode | Quelle | Ende-Zeile künftig | Beginn-Zeile |
  |---|---|---|---|
  | `ERROR senden` „Nachricht nicht gespeichert“ (Schreibfehler) | Datei und Grund | `datei=`, `channel=`, Grund; **ohne** `kennung=` | wie bisher, mit `kennung=` des ersten Auftretens |
  | `WARNING körper` an Routen mit Sitzung | Herkunft und Route | `herkunft=`, `route=`, Zählungen, `bytes=` (größter); **ohne** `kennung=` | wie bisher |
  | `WARNING schreibkonto` (Senden und ab R00021 Frist) | Kennung | `kennung=`, Zählung je Grenze; **ohne** `grenze=` (kommt mit R00020 F0070) | wie bisher, mit `grenze=` |
  | `WARNING zeit` (Vorlauf) | Instanz | `dauer_ms=` des **letzten** Auftretens | `dauer_ms=` des ersten |

  Die übrigen Episoden (Grenzschutz, Verbindung an der Platzgrenze) tragen schon nur den Kontext ihrer Quelle; ihre Zeilen ändern sich nicht.
- **Vorlauf beim Start** (O2; Punkt 3): nach dem Laden der Gespräche prüft der Start einmal den Vorlauf der zuletzt vergebenen Nachrichtenzeit wie nach jedem Senden. Liegt sie mehr als 60 s vor der Uhr, beginnt die Episode `WARNING zeit` mit dem Text aus R00020 („Nachrichtenzeit läuft der Systemuhr <h> h <mm> min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt“). Die Zahl der Auftreten bleibt „Sendungen im Vorlauf“ plus das eine beim Start.
- **Gelungenes Senden auf Debug** (O1; Punkt 5): je 201 auf `POST /api/channels/{name}/nachrichten` eine Zeile `DEBUG senden Nachricht gesendet | channel=<name> kennung=<kennung> anzahl=<Bytes des Texts> dauer_ms=<Eintritt in den Handler bis geschrieben>`. Sobald R00012 F0046 die Antwort 200 bei Wiederholung liefert, schreibt auch sie die Zeile, mit `grund=wiederholung`. Nie der Text. Auf Stufe Information und höher keine Zeile.
- **Ereignisstrom überholt** (O4; Punkt 6): fällt ein Abonnent um mehr als die Kapazität des Busses (1.024 Ereignisse) zurück, trägt der neue Stand die Zahl verpasster Ereignisse, und der Strom schreibt eine Episode `WARNING live` „Ereignisstrom überholt, neuer Stand gesendet“ je Kennung: Beginn mit `kennung=` und `anzahl=` (verpasste Ereignisse dieses Auftretens), Ende mit `kennung=` und `n` = Zahl der Überholungen. Die Beginn-Zeile zählt in `warnings`. Der Strom schickt danach den neuen Stand wie bisher.
- **Kein neues Feld im Health-Check** (ADR-0043, Ergänzung Karte #31): `/api/gesundheit` behält seine Felder; Überholungen zählen nur in `warnings`.

## Nicht-funktionale Anforderungen

- **Betrieb**: Menge auf Debug in der Spitze des Lastprofils 30 Nachrichten/s × rund 150 Byte ≈ 4,5 KB/s (gerechnet in ADR-0042, Ergänzung Karte #31 Punkt 5); das Tagesbudget aus R00014 gilt. Die `WARNING live`-Episode hält die Menge bei einer Flut über alle Ströme mit der Grenze von 1.024 offenen Episoden und der Sammelepisode (R00014).
- **Sicherheit**: nie Nachrichtentext, Token oder Körper im Protokoll (R00014).
- **Zeitkriterien** nach `AI-Team/planer.md`: die einzige Zeitgrenze ist „Vorlauf beim Start binnen 1 s nach der Startzeile“ — Zweck: der Vorlauf steht im Protokoll, bevor jemand sendet; erwartet: wenige Millisekunden (eine Rechnung nach dem Laden); die Uhr startet mit der Startzeile auf stdout. Die Schwelle 60 s ist die aus R00011.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, `S(<name>)`, `POST` wie in R00011 und R00014; `heute.log` ist `logs/<heute>.log`; „Speicher-Senke“ ist die Senke im Speicher aus R00014. „Die Zeile `X`“ meint die Zeile ab der Stufe, ohne Zeitstempel.

### F0073 Betriebsprotokoll nach den Reviews #8 nachgezogen

- [ ] **Ende-Zeile des Schreibfehlers** (Unit, Speicher-Senke; erster Test aus ADR-0042 Ergänzung Karte #31 Punkt 7, wörtlich): „Schreibfehler-Episode mit zwei Kennungen → Beginn mit `kennung=` der ersten, Ende ohne `kennung=`, mit `datei=` und `channel=`. Mutante „Kontext des ersten Auftretens“ → rot.“
- [ ] **Ende-Zeile der Körper-Episode** (Unit, Speicher-Senke): eine Körper-Episode an `POST /api/channels/{name}/nachrichten` mit Auftreten von `beta@127.0.0.1` (zu groß, 131.073 Byte) und `gamma@127.0.0.1` (zu groß, 200.000 Byte gelesen) → Beginn mit `kennung=beta@127.0.0.1`; Ende `… Episode endet: 2 in <d> s (…)` mit der Zählung je Art 2 (Schreibweise der Art nach R00018), `herkunft=127.0.0.1`, `route=…`, `bytes=200000`, ohne `kennung=`. Die Mutante „Kontext des ersten Auftretens“ wird rot.
- [ ] **Ende-Zeile des Vorlaufs** (Unit, Speicher-Senke; erster Test aus ADR-0042 Ergänzung Karte #31 Punkt 7, wörtlich): „`zeit` mit 61.001 ms, dann 7.410.002 ms → Ende mit `dauer_ms=7410002`“. Die Beginn-Zeile trägt `dauer_ms=61001`. Die Mutante „Ende mit dem ersten `dauer_ms`“ wird rot.
- [ ] **Übrige Episoden unverändert** (Unit und Integration): die Kriterien zu Grenzschutz, Platzgrenze und Körperfehlern aus R00014, R00015 und R00018 laufen unverändert grün.
- [ ] **Vorlauf beim Start** (Integration, echtes Programm; erster Test aus ADR-0042 Ergänzung Karte #31 Punkt 7, wörtlich): „Ordner mit einer Nachricht von 12:00, Start mit Uhr 10:00 → binnen 1 s nach der Startzeile `WARNING zeit Nachrichtenzeit läuft der Systemuhr 2 h 00 min voraus … — Episode beginnt | dauer_ms=…`, ohne dass gesendet wurde. Mutante „nur beim Senden“ → rot.“ Ausgeschrieben: `channels/allgemein/2026-10-28.log` enthält einen Satz mit der Zeit `2026-10-28T12:00:00.000+01:00`; Start mit `LOCALCHATA_JETZT=2026-10-28T10:00:00.000+01:00` und `TZ=Europe/Berlin`; `dauer_ms` ≥ 7.200.000. Gegenprobe: Satz mit `2026-10-28T10:00:30.000+01:00` (30 s voraus) → keine `zeit`-Zeile nach 1 s.
- [ ] **Gelungenes Senden auf Debug** (Integration, Stufe Debug; erster Test aus ADR-0042 Ergänzung Karte #31 Punkt 7, wörtlich): „ein gelungenes `POST …/nachrichten` → genau eine `DEBUG senden Nachricht gesendet | channel=allgemein kennung=beta@127.0.0.1 anzahl=5 dauer_ms=…`; der Text „Hallo“ steht nicht im Protokoll.“ Gegenprobe auf Stufe Information: keine Zeile „Nachricht gesendet“. Die Mutante „keine Zeile“ wird rot.
- [ ] **Überholung mit Zahl** (Unit; erster Test aus ADR-0042 Ergänzung Karte #31 Punkt 7, wörtlich): „Bus mit Kapazität 2: 5 Ereignisse veröffentlicht → `naechstes` liefert `NeuerStand { verpasst: 3 }`“.
- [ ] **Überholter Strom als Episode** (Unit, Strom mit Speicher-Senke; erster Test aus ADR-0042 Ergänzung Karte #31 Punkt 7, wörtlich): „zwei Überholungen derselben Kennung → genau eine `WARNING live … Episode beginnt | kennung=… anzahl=3`. Mutante „keine Zeile“ → rot.“ Nach der Ruhe genau eine Ende-Zeile `… Episode endet: 2 in <d> s | kennung=…`; `warnings` steigt um 1; der Strom liefert nach jeder Überholung den neuen Stand.
- [ ] **Health-Check unverändert** (Integration): `/api/gesundheit` nennt dieselben Felder wie vor R00022 (Vergleich der Schlüssel mit dem Test `leerer_ordner_ist_bereit_mit_kennzahlen_null`).

## Betroffene Verzeichnisstruktur

- `src/protokoll/episoden.rs` (Kontext der Quelle gegen Werte des einzelnen Auftretens, Ende-Zeile), Aufrufer von `.episode(` in `src/http/senden.rs`, `src/http/koerper.rs` bzw. dort, wo Körper-Episoden entstehen, `src/http/grenzschutz.rs`, `src/start/ausliefern.rs` (nur prüfen, nicht ändern), `src/start/` (Vorlauf nach dem Laden), `src/live/bus.rs` (`NeuerStand` mit Zahl), `src/http/ereignisse.rs` (Episode `live`); Tests `tests/protokoll_*.rs`, `tests/senden_protokoll.rs`, Unit-Tests in den genannten Modulen.

## Technische Überlegungen

### Ablauf

1. **Auftreten** → Episode: der Schlüssel aus Bereich, Text und Quelle; die Beginn-Zeile mit dem ganzen Kontext des ersten Auftretens; jedes Auftreten zählt (je Art, je Verwerfen), führt `bytes` (größter) und `dauer_ms` (letzter) nach.
2. **Ende** nach der Ruhe → Kontext der Quelle + Zählungen + zusammengefasste Werte.
3. **Start** → Laden → Vorlauf prüfen → Auftreten `zeit` (falls > 60 s) → Startzeile.
4. **Senden** → geschrieben → `DEBUG senden Nachricht gesendet`.
5. **Strom** → `Lagged(n)` → `NeuerStand { verpasst: n }` → Auftreten `live` → neuer Stand an den Tab.

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0073: B0345 Ende-Zeile mit dem Kontext der Quelle (unklar) · B0346 Vorlauf beim Start prüfen · B0347 Gelungenes Senden auf Debug · B0348 Überholung mit Zahl · B0349 Überholten Ereignisstrom melden

Elemente: im `Auftreten` getrennt, was zur Quelle gehört und was nur zum einzelnen Auftreten (Form entscheidet der Entwickler); `Empfang::NeuerStand { verpasst: u64 }`; `dauer_ms` als zusammengefasster Wert wie `bytes`.

Wireframe: keine Oberfläche.

### Änderungen an bestehenden Klassen

- `protokoll::episoden::Episode::ende`: Kontext der Quelle statt des Kontexts des ersten Auftretens; `dauer_ms` letzter.
- `live::bus`: `NeuerStand` mit der Zahl aus `Lagged(n)`.
- `http::ereignisse`: Auftreten `live` beim neuen Stand.
- `http::senden`: Debug-Zeile nach dem Schreiben.
- Start: Vorlauf-Prüfung nach dem Laden.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Unit (Speicher-Senke)**: Ende-Zeilen von Schreibfehler, Körper und Vorlauf; Bus mit Kapazität 2; Strom mit zwei Überholungen.
- **Integration (echtes Programm)**: Vorlauf beim Start mit `LOCALCHATA_JETZT`; Debug-Zeile beim Senden; Health-Check unverändert; die Episoden-Kriterien aus R00014, R00015, R00016 und R00018 bleiben grün.

## Abhängigkeiten

- Abhängig von: F0057 Senden im Betriebsprotokoll (gruen), F0055 Grenzen, Episoden und Tagesbudget (gruen), **F0070 Senden im Betriebsprotokoll je Grenze** (rot, R00020, Karte #31) — dieselbe Mechanik in `src/protokoll/episoden.rs` und dieselbe Ende-Zeile des Schreibkontos: erst die Zählung je Grenze, dann die allgemeine Regel.
- Berührt: R00012 F0046 (Karte #22) für `grund=wiederholung` — ist F0046 noch nicht gebaut, entfällt dieser Teil bis dahin.
- Blockiert: —

## Offene Fragen

- **An den architekt (nicht blockierend)**: die Zwischenzeile „läuft weiter — n seit Beginn“ trägt heute denselben Kontext wie die Beginn-Zeile (das erste Auftreten). ADR-0042 Ergänzung Karte #31 Punkt 4 regelt nur die Ende-Zeile. Vorschlag: die Zwischenzeile wie die Ende-Zeile (Kontext der Quelle). Bis zur Entscheidung bleibt sie, wie sie ist, und kein Kriterium hier prüft sie.

Entschieden vom planer (Rückfragen mit Empfehlung beantwortet):

- **Ein Slice statt vier**: O1–O4 haben dieselbe Rolle (Betreiberin), denselben Prüfweg (Speicher-Senke und echtes Programm) und je ein bis zwei Bubbles; vier Slices wären kleiner als eine Story.
- **Unter I0021, nicht unter I0009**: siehe Beschreibung; Vorbild F0059–F0062 (Härtung des Protokolls aus Reviews).
- **Überholter Strom nur als Unit**: im echten Programm müsste ein Strom 1.024 Ereignisse zurückfallen; das Schreibkonto erlaubt einer Herkunft Ausbrüche bis 120 — ein verlässlicher Integrationstest bräuchte viele Herkünfte. Der Unit-Test mit Kapazität 2 prüft dieselbe Abbildung.

## Manuelle Vorbereitungstätigkeiten

- Keine.

## Manuelle Nachbereitungstätigkeiten

- Keine (die Katalogzeilen stehen seit Karte #31 in ADR-0042).

## Umfang

Gezählt aus der WBS (Bubbles). 0,4 h ist die Untergrenze je Bubble nach dem Skill `work-breakdown-structure`; Richtwert, ungemessen für diese Bubble-Typen.

- F0073 Betriebsprotokoll nach den Reviews #8 nachgezogen: 5 Bubbles · 4 klar (1,6 h) · 1 unklar (B0345, Bandbreite 2–4 h: wie die Aufrufer von `.episode(` Quelle und Einzelwerte trennen)

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser sind die optionalen Befunde O1–O4 des Observability-Reviews zu #8 und die Entscheidungen des architekt dazu (ADR-0042 und ADR-0043, je Ergänzung Karte #31).

Die Wirkungskette:

- Die Ende-Zeile nennt nur den Kontext der Quelle. Deshalb liest die Betreiberin bei einer vollen Platte die Datei als Ursache und nicht eine zufällige erste Kennung — sie räumt die Platte auf, statt einen Agenten zu suchen.
- Der Start prüft den Vorlauf. Deshalb steht eine zurückgestellte Uhr im Protokoll, bevor die erste Nachricht mit einer Zeit in der Zukunft gespeichert wird.
- Ein überholter Strom wird eine Warnung je Kennung. Deshalb sieht die Betreiberin, welcher Rechner nicht mitkommt — genau der Fall, in dem ein Tab Nachrichten nachholen muss.
- Gelungenes Senden steht auf Debug. Deshalb lässt sich ein Einzelfall mit Dauer nachstellen, ohne das Tageslog zu lesen.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus R00014 und R00016.

## Notizen

- **Zusagen aus R00014 und R00016 bleiben**: sie sagen den Kontext der Ende-Zeile nicht zu. Tests, die heute `kennung=` in der Ende-Zeile einer Schreibfehler- oder Körper-Episode erwarten, ziehen mit; R00016 „Vorlauf der Nachrichtenzeit“ liest sich mit einem Auftreten mehr beim Start.
- Verworfen (Optionen des architekt in ADR-0042, Ergänzung Karte #31): Vorlauf im Sekundentakt prüfen (Zählung würde Sekunden statt Sendungen, R00016 änderte sich); Zählung je Kennung in der Ende-Zeile (bis 512 Kennungen, die Zeile wäre unbegrenzt lang); Überholung auf Debug (die Betreiberin erführe es nie); eine Episode der Instanz für Überholungen (verdeckte den einzelnen langsamen Leser); Information je gesendeter Nachricht (Nachrichten stehen im Tageslog); eigene Felder im Health-Check (Aufteilungen gehören ins Protokoll).

### Bewusst außerhalb

- Die Zählung je Grenze in der Ende-Zeile des Schreibkontos: R00020 F0070.
- Die Episode „Frist gebremst“: R00021 F0072 (sie folgt hier derselben Ende-Regel).
