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
