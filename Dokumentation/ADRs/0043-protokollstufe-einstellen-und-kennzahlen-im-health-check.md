# ADR-0043: Protokollstufe über `--protokoll-stufe` und `LOCALCHATA_PROTOKOLL_STUFE`, nur beim Start; Kennzahlen des Protokolls im Health-Check, `status` bleibt Bereitschaft

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Vorlauf „Betriebsprotokoll“)
**Kontext-Anforderung**: — (neue Interaction „Betriebsprotokoll“)
**Erprobt durch**: — (überträgt gebaute Mechanik: Optionen in `Startauftrag::aus_argumenten` und `HILFE`, `src/start/aufruf.rs`; Umgebungswerte `LOCALCHATA_*` mit Exit 2 bei falschem Wert, `src/start/mod.rs:47–70`; Health-Check `src/http/gesundheit.rs`)

## Kontext

- Vorgabe: Standard Information, Debug zuschaltbar (`AI-Team/umgebung.md`). Ziel: Monitoring der Anwendung.
- Bestand: Einstellungen kommen als Option (`--port`, `--host`, `--kein-browser`, `--erreichbar-als`) oder als Umgebungswert für Tests (`LOCALCHATA_JETZT`, `LOCALCHATA_GNADENFRIST_MS`, `LOCALCHATA_HERZSCHLAG_MS`; falscher Wert → Exit 2). `--hilfe` listet alle Optionen.
- `GET /api/gesundheit` ([ADR-0009](0009-teststrategie-gate-und-health-check.md)) antwortet ohne Sitzung mit `status: "bereit"` und Kennzahlen ohne Namen; Tests und Director nutzen es als Bereitschaftsprobe (`AI-Team/umgebung.md`). Das Wort „Log“ meint dort die Tageslogs (`log_bytes`).
- Die Instanz steht ohne Anmeldung im LAN (`AI-Team/gelerntes.md` #1); jede neue Route ist Angriffsfläche.

## Optionen

### Einstellung

#### Option A: Option `--protokoll-stufe <stufe>` und Umgebungswert `LOCALCHATA_PROTOKOLL_STUFE`, gelesen beim Start; Option schlägt Umgebung
- Vorteile: wie alle übrigen Einstellungen; ein Dienst setzt die Umgebung, ein Mensch die Option; Tests setzen sie über den echten Weg (Lehre „Konfiguration über den echten Weg prüfen“).
- Nachteile: Umschalten heißt Neustart — Sitzungen gehen nach der Gnadenfrist verloren (ADR-0012).

#### Option B: Zusätzlich zur Laufzeit umschaltbar über eine Route (`PUT /api/protokoll/stufe`)
- Vorteile: Diagnose ohne Neustart.
- Nachteile: ohne Anmeldung kann jeder im LAN Debug zuschalten und so das Debug-Budget füllen; Vision schließt Rechte aus — es gäbe keinen, der es allein darf.

#### Option C: Umschalten über ein Signal (SIGUSR1 schaltet Debug um)
- Vorteile: nur, wer auf dem Rechner Prozessrechte hat; ohne Neustart.
- Nachteile: nicht unter Windows; Zustand unsichtbar ohne Health-Check; weiterer Signalpfad im Start.

Namen: der Auftrag schlug `--log-stufe`/`LOCALCHATA_LOG` vor. Gewählt ist `protokoll`, weil „Log“ im Projekt das Tageslog eines Gesprächs ist (ADR-0004, `log_bytes`) — `--log-stufe` läse sich als Einstellung der Gesprächslogs.

### Monitoring

#### Option D: Kennzahlen des Protokolls als zusätzliche Felder im Health-Check, `status` unverändert
- Vorteile: eine Abfrage für Bereitschaft und Zustand; Monitoring entscheidet selbst über Schwellen; bestehende Bereitschaftsproben bleiben gültig.
- Nachteile: wer nur `status` liest, sieht keinen Programmfehler.

#### Option E: `status` wird `"beeintraechtigt"`, sobald eine Exception oder ein Error auftrat
- Vorteile: auf einen Blick.
- Nachteile: Bereitschaft und Zustand vermischt — Tests und Director, die auf `"bereit"` warten, würden nach einem einzigen Plattenfehler die Instanz für tot halten; ein Zustand ohne Rückweg (wann wäre sie wieder „bereit“?).

#### Option F: Eigene Route `/api/protokoll` mit den letzten Einträgen
- Vorteile: Betreiber liest Fehler im Browser.
- Nachteile: Kennungen und Herkünfte aller Teilnehmer für jeden im LAN lesbar, ohne Anmeldung; Datei lesen kann der Betreiber ohnehin.

## Entscheidung

**Gewählt: A und D**

Es überwiegt, dass die Einstellung den bestehenden Wegen folgt und kein Fremder im LAN sie ändern kann, und dass der Health-Check Zahlen liefert, ohne seine Bedeutung „bereit“ zu verlieren. In Kauf genommen: Umschalten verlangt einen Neustart; wer nur `status` liest, sieht keine Fehler — Monitoring muss die Felder lesen.

Regeln:

1. **Werte**: `debug`, `information`, `warning`, `error` (klein; Großschreibung wird angenommen). `exception` ist nicht wählbar — Exceptions werden immer geschrieben. Standard `information`.
2. **Vorrang**: `--protokoll-stufe` vor `LOCALCHATA_PROTOKOLL_STUFE` vor Standard. Falscher Wert in Option oder Umgebung → Meldung auf stderr mit den erlaubten Werten, **Exit 2**, nichts geschrieben, kein `logs/` angelegt — wie die übrigen Aufruf- und Umgebungsfehler. Fehlt der Wert hinter der Option → `--protokoll-stufe braucht einen Wert — Aufruf siehe localchatA --hilfe`.
3. **`--hilfe`** nennt: `--protokoll-stufe <stufe>  Mindeststufe des Betriebsprotokolls unter <ordner>/logs/: debug, information (Standard), warning, error; auch über LOCALCHATA_PROTOKOLL_STUFE`.
4. **Start-Eintrag** nennt die wirksame Stufe (`stufe=…`) und woher sie kommt (`option=--protokoll-stufe|LOCALCHATA_PROTOKOLL_STUFE|standard`).
5. **Health-Check** — neue Felder, flach wie die bestehenden, alle ohne Namen, Kennungen oder Pfade außer dem schon genannten Ordner:

   | Feld | Bedeutung |
   |---|---|
   | `protokoll_stufe` | wirksame Stufe |
   | `protokoll_ok` | `true`, solange Einträge in die Datei gehen; `false` im Rückfall auf stderr (ADR-0040 Regel 8) |
   | `protokoll_bytes_heute` | Größe der heutigen Protokolldatei laut Modul |
   | `protokoll_verworfen` | über dem Tagesbudget verworfene Einträge seit dem Start (ADR-0042 Regel 6) |
   | `warnings`, `errors`, `exceptions` | Zahl der Einträge dieser Stufe seit dem Start — gezählt beim Entstehen, unabhängig von Budget und Rückfall; eine Episode zählt jedes Auftreten |
   | `letzter_error`, `letzte_exception` | Zeit (RFC 3339 wie ADR-0040) des jüngsten Eintrags dieser Stufe, sonst `null` |
   | `aufgaben_ausgefallen` | Liste der Namen beendeter Hintergrundaufgaben (`["takt"]`), sonst `[]` (ADR-0042 Regel 4) |

   `status` bleibt `"bereit"`, solange der Port antwortet. Monitoring-Empfehlung (für die Anleitung, kein Code): Alarm bei `exceptions` steigend oder `aufgaben_ausgefallen` nicht leer; Hinweis bei `errors` steigend, `protokoll_ok = false` oder `protokoll_verworfen > 0`.
6. **Keine Route** zum Lesen oder Umschalten des Protokolls.

## Konsequenzen

**Positiv:**
- Monitoring braucht nur `curl <adresse>api/gesundheit` und zwei Schwellen.
- Tests setzen die Stufe über den echten Weg (Umgebung oder Option), wie die übrigen Testknöpfe.

**Negativ / Inkauf zu nehmen:**
- Debug zuschalten heißt neu starten.
- Der Test `leerer_ordner_ist_bereit_mit_kennzahlen_null` (`src/http/gesundheit.rs`) vergleicht die ganze Antwort und wächst um die neuen Felder.

**Folgeentscheidungen / Aufgaben:**
- [ ] Unit (Aufruf, rein): `--protokoll-stufe debug` → Stufe Debug; `--protokoll-stufe Warning` → Warning; `--protokoll-stufe laut` → Aufruffehler mit „debug, information, warning, error“; `--protokoll-stufe` ohne Wert → `WertFehlt`; `--protokoll-stufe exception` → Aufruffehler.
- [ ] Integrationstest (echtes Programm, Wegwerf-Ordner): ohne Angabe → kein `DEBUG` in `logs/<heute>.log` nach einem Verlaufsabruf; `LOCALCHATA_PROTOKOLL_STUFE=debug` → `DEBUG verlauf`-Zeile vorhanden; `LOCALCHATA_PROTOKOLL_STUFE=debug` und `--protokoll-stufe warning` → keine `INFORMATION`-Zeile (auch die Startzeile fehlt dann — bewusst, die Stufe gilt für alles), `protokoll_stufe = "warning"`; `LOCALCHATA_PROTOKOLL_STUFE=laut` → Exit 2, kein Ordner `logs/`. `--hilfe` enthält `--protokoll-stufe`.
- [ ] Health-Check (Integration): frische Instanz → `warnings = 0`, `errors = 0`, `exceptions = 0`, `letzte_exception = null`, `protokoll_ok = true`, `aufgaben_ausgefallen = []`; nach `LOCALCHATA_TESTPANIK=verlauf` und einem Verlaufsabruf → `exceptions = 1`, `errors = 1`, `letzte_exception` = Zeit der EXCEPTION-Zeile im Protokoll.
- [ ] `AI-Team/umgebung.md` (Director): Testknöpfe `LOCALCHATA_PROTOKOLL_STUFE`, `LOCALCHATA_TESTPANIK` (nur Debug-Build) ergänzen, sobald gebaut.

## Quellen / Bezuege

- [A00015 Betriebsprotokoll](../Architektur/A00015-betriebsprotokoll.md), [A00003 Start](../Architektur/A00003-start.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md); Aspekte [X00001](../Architektur/Aspekte/X00001-logging-und-diagnose.md), [X00006 Konfiguration und Geheimnisse](../Architektur/Aspekte/X00006-konfiguration-und-geheimnisse.md)
- ADR-0009, ADR-0012, ADR-0014, ADR-0040, ADR-0041, ADR-0042; `gemeinsam/gelerntes.md` der Factory, „Konfiguration über den echten Weg prüfen“

## Ergänzungen

- **2026-10-28 (Proben des `observability-reviewer` an #21 Körpergrenze) — Zähler der Abweisungen:** Regel 5 wächst um das Feld `abgewiesen: {zu_gross, kein_json, kaputt, abgebrochen}`. Es zählt die Körperfehler aller JSON-Routen seit dem Start ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung Punkt 2). Gezählt wird beim Entstehen, unabhängig von Stufe, Episode und Budget. Ein Monitoring sieht Fluten daran, dass der Zähler steigt, ohne das Protokoll zu lesen. Das Feld ist ausnahmsweise verschachtelt, weil die vier Zahlen eine Einheit bilden. Monitoring-Empfehlung: Hinweis, wenn `abgewiesen.zu_gross + abgewiesen.kaputt` in 5 Minuten um mehr als 100 steigt.
- **2026-10-28 (R00015, F0056) — fünfte Art `zu_langsam` für die 408 der Lesefrist:** Mit der Lesefrist für den Körper ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), Ergänzung R00015) entsteht ein fünfter Körperfehler. Frage aus R00015: zählt eine 408 in `abgewiesen`, und wenn ja, wo?
  - Option A: **eigene Art `zu_langsam`** — `abgewiesen: {zu_gross, kein_json, kaputt, abgebrochen, zu_langsam}`. Vorteil: ein Monitoring trennt „Client schickt zu viel“ (Flut) von „Client hält Plätze mit schweigenden Körpern“ (Slowloris auf den Körper) — zwei verschiedene Angriffe mit verschiedener Gegenmaßnahme; jede Art bleibt genau eine Ursache. Schwäche: die Antwortform wächst um ein Feld; der Test, der die ganze Antwort vergleicht, wächst mit.
  - Option B: in `abgebrochen` mitzählen — kein neues Feld. Schwäche: `abgebrochen` heißt „Lesefehler im Strom, der Client ist weg“; eine 408 heißt „der Client ist da und schweigt“. Eine Zahl für zwei Ursachen macht das Signal unlesbar.
  - Option C: nicht zählen, nur DEBUG — Schwäche: genau der Angriff, den die Frist abwehrt, bliebe für ein Monitoring auf Information unsichtbar; widerspricht „jeder Fehlerpfad hat eine Spur“.
  - **Gewählt: A** (Empfehlung des planer). In Kauf genommen: ein weiteres Feld in der verschachtelten Einheit `abgewiesen`.
  - **Regeln**: (1) Gezählt wird beim Entstehen jeder 408 der Lesefrist, unabhängig von Stufe, Episode und Budget, wie die übrigen Arten. (2) Protokoll: Art `ZuLangsam` folgt `ZuGross` und `Kaputt` — eine `DEBUG`-Zeile je Fall (`route`, `herkunft`, `kennung` falls bekannt, `art=zu_langsam`, `bytes` = gelesen bis zum Ablauf, `status=408`) **und** die `WARNING`-Körper-Episode je Herkunft und Route ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung 1 und 3, Ergänzung R00015). Grund für Warning statt nur Debug (anders als `Abgebrochen`): ein schweigender Körper hält 10 s lang einen Platz — das ist eine Abweisung mit Betriebsbedeutung, und gehäuft ist es der Angriff, gegen den die Frist besteht; die Episode drosselt die Menge. Die Zusammenfassung nennt `zu_langsam <n>` neben den übrigen Arten; ein Verwerfen-Ergebnis gibt es für diese Art nicht. (3) Monitoring-Empfehlung erweitert: Hinweis, wenn `abgewiesen.zu_langsam` in 5 Minuten um mehr als 20 steigt (eine Herkunft mit 160 Plätzen schafft ≈ 16 je Sekunde; einzelne 408 bei WLAN-Aussetzern sind selten).
  - **Reihenfolge des Baus**: R00014 F0054 nennt die Antwort mit vier Arten. Wer zuerst gebaut wird, ist unerheblich — die zweite Karte bringt das Feld mit: kommt F0056 nach F0054, setzt F0056 `zu_langsam` dazu und stellt den Vergleichstest der Gesundheit um; kommt F0056 vorher, zählt F0056 nur intern, und F0054 legt das Feld mit fünf Arten an. Hinweis an den planer, kein Umbau.
  - Erster Test (Integration, echtes Programm, `LOCALCHATA_EPISODE_RUHE_MS=1000`): 3 Rohverbindungen von `127.0.0.1` mit `POST /api/beitreten`, `Content-Length: 4096`, 10 Bytes, Stille → je 408 nach ≈ 10 s; danach `abgewiesen.zu_langsam = 3`, die übrigen Arten 0; im Protokoll genau eine `WARNING körper … Episode beginnt` und eine `WARNING körper … Episode endet: 3 … (zu_langsam 3)` für `route="POST /api/beitreten" herkunft=127.0.0.1`. Mutanten: Zählung in `abgebrochen` → rot; Zeile je Fall auf Warning → rot.
- **2026-10-28 (Detailplanung R00014, Festlegung (1) des planer) — Zähler unabhängig von der Mindeststufe:** Regel 5 sagt „gezählt beim Entstehen, unabhängig von Budget und Rückfall“, nennt die Stufe aber nicht. Präzisiert: `warnings`, `errors`, `exceptions` sowie `letzter_error` und `letzte_exception` zählen **auch Einträge unter der Mindeststufe** — die Mindeststufe steuert, was geschrieben wird, nie, was gezählt wird. Grund: wer mit `--protokoll-stufe error` startet, soll im Health-Check trotzdem sehen, dass Warnings steigen; sonst hinge das Monitoring-Signal an einer Diagnose-Einstellung. Eine Episode zählt jedes Auftreten, auch wenn ihre Zeilen unter der Mindeststufe liegen. Für `DEBUG` und `INFORMATION` gibt es keinen Zähler. Geprüft gegen ADR-0042 Regel 4 (der Panik-Haken setzt Zähler immer) und Ergänzung oben (`abgewiesen` unabhängig von der Stufe): stimmig, übernommen wie in R00014 F0054. Erster Test (Integration): `--protokoll-stufe error`, eine Grenzschutz-Abweisung (403) → keine `WARNING`-Zeile in `logs/<heute>.log`, `warnings = 1`.
- **2026-10-28 (Code-Review #24, Befund A7; Lücken aus dem Observability-Review #23) — was die Stufen-Zähler zählen; neue Felder `abgerissen`, `werkbank_plaetze`, `werkbank_belegt`:**
  1. **„Eine Episode zählt jedes Auftreten“** (Regel 5) heißt: jedes Auftreten **mit der Stufe der Episode** zählt in den Zähler dieser Stufe. Ein Auftreten, das selbst eine andere Stufe hat und nur in die Zusammenfassung einer offenen Episode eingeht — heute `Abgebrochen` (DEBUG, ADR-0042 Ergänzung 3) in der Körper-Episode (WARNING) —, zählt **nicht** in `warnings`; es zählt in `abgewiesen.abgebrochen`. Beleg: `src/http/koerper.rs:128` (`episode_falls_offen`); die Episode meldet 2, `warnings` steigt um 1 — so richtig. Option „mitzählen“ verworfen: dasselbe Ereignis zählte dann je nach Zeitpunkt (Episode offen oder nicht) als Warning oder nicht — ein Monitoring-Signal, das vom Zufall abhängt. In Kauf genommen: `warnings` und die Zahl in der Zusammenfassung einer Körper-Episode weichen um die abgebrochenen Fälle ab; die Zusammenfassung nennt sie getrennt je Art. Kein Code; Test (Unit, Speicher-Senke): eine `ZuGross`, danach ein `Abgebrochen` derselben Quelle → Zusammenfassung „2 … (zu_gross 1; abgebrochen 1)“, `warnings = 1`, `abgewiesen.abgebrochen = 1`.
  2. **`abgerissen`**: Zahl der Verbindungen seit dem Start, die der Kernel an der Abrissgrenze (= Sendefrist, ADR-0039 Regel 6) schloss ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung #24 Punkt 7) — flach, gezählt beim Entstehen, unabhängig von Stufe, Episode und Budget. Monitoring-Empfehlung: Hinweis, wenn `abgerissen` in 5 Minuten um mehr als 20 steigt (zugeklappte Laptops sind wenige je Stunde; ein Angriff „Langsame Leser“ belegt bis 160 Plätze je Herkunft).
  3. **Werkbank-Belegung** (optional laut #23, aufgenommen): `werkbank_plaetze` (Erlaubnisse, fest seit dem Start) und `werkbank_belegt` (Momentwert: Plätze − freie Erlaubnisse). Option „nicht aufnehmen“ verworfen: eine hängende Platte zeigt sich zuerst als dauerhaft volle Werkbank, und die Zahl ist ein Lesen ohne Kosten; Option „Wartende zählen“ verworfen: das Semaphor von tokio nennt keine Wartenden, ein eigener Zähler um `erwerben` wäre Code für eine Zahl, die `belegt = plaetze` schon andeutet. Monitoring-Empfehlung: Hinweis, wenn `werkbank_belegt = werkbank_plaetze` in drei Abfragen im Abstand von 10 s. Keine Namen, keine Pfade — die Regel aus Regel 5 hält.
  4. Der Test `leerer_ordner_ist_bereit_mit_kennzahlen_null` (`src/http/gesundheit.rs`) wächst um `"abgerissen": 0`, `"werkbank_plaetze": <n>`, `"werkbank_belegt": 0` (die Plätze hängen an der Maschine — der Test setzt die Werkbank mit `Werkbank::mit(2)`).
- **2026-10-28 (Karte #31; Observability-Review #8 S1 optional und O4) — keine Aufteilungen im Health-Check:** Regel für neue Felder: der Health-Check nennt Signale für das Monitoring — **ob** etwas geschieht und wie oft (`gebremst`, `warnings`, `errors`, `exceptions`, `abgerissen`); **Aufteilungen zur Diagnose** (je Grenze, je Kennung, je Strom) stehen im Betriebsprotokoll. Deshalb kein `gebremst_herkunft` (die Ende-Zeile der Schreibkonto-Episode zählt je Grenze und je Kennung, [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung Karte #31, Punkt 2) und kein eigenes Feld für den überholten Ereignisstrom (Episode `WARNING live`, zählt in `warnings`; ebenda Punkt 6). Option „Feld je Aufteilung“ verworfen: die Summe über alle Kennungen beantwortet die Auslegungsfrage nicht (welcher Agent scheitert an der Adresse), und jedes Feld ist eine Zusage der Form des Health-Checks (R00011, R00016). Ein Feld kommt dazu, wenn das Monitoring ohne Protokoll etwas **entscheiden** muss — dann mit eigener Ergänzung.
- **2026-10-29 (Karte #34) — Plattenquote im Health-Check und als Einstellung:** Zwei neue flache Felder nach der Regel der Ergänzung Karte #31 — das Monitoring muss ohne Protokoll entscheiden, ob die Ablage bald voll ist, und braucht dafür die Grenze neben `log_bytes`: `plattenquote` (wirksame Quote in Bytes, `null` bei `aus`) und `quote_abgewiesen` (Zahl der 507 wegen der Quote seit dem Start, gezählt beim Entstehen, unabhängig von Stufe, Episode und Budget). `log_bytes` behält seine Bedeutung (Σ Tageslogs im Index) und liest künftig den Zähler der Quote statt die Summe zu bilden. Keine Aufteilung je Channel oder Herkunft. Die Einstellung `--plattenquote` / `LOCALCHATA_PLATTENQUOTE` folgt Regel 2 dieser ADR (Vorrang, Exit 2, nichts geschrieben) und Regel 4 (Start-Eintrag nennt Wert und Herkunft). Monitoring-Empfehlung: Hinweis bei `log_bytes ≥ 0,9 × plattenquote`, Alarm bei `quote_abgewiesen` steigend. Der Test `leerer_ordner_ist_bereit_mit_kennzahlen_null` wächst um `"plattenquote": 4294967296`, `"quote_abgewiesen": 0`. Entschieden in [ADR-0044](0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) Regeln 3 und 9.
