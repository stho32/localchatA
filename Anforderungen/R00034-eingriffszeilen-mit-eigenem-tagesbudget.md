---
id: R00034
status: Neu
datum: 2026-10-29
---

# R00034: Eingriffszeilen mit eigenem Tagesbudget

## Beschreibung

Wer gemeinsame oder fremde Daten ändert oder löscht, steht mit seiner Kennung im Betriebsprotokoll — auch nachdem eine Flut harmloser Zeilen das Tagesbudget der Stufen `INFORMATION` und `WARNING` erschöpft hat. Heute teilen sich `INFORMATION sitzung` (Beitreten, Verlassen, weg) und `INFORMATION frist` das Budget von 32 MiB; eine Beitrittsflut leert es in ≈ 3,6 h (viele Herkünfte) bzw. ≈ 14,5 h (eine Herkunft), danach werden auch die Fristzeilen nur noch gezählt. Danach gilt:

- **Eigene Klasse `EINGRIFF`** mit eigenem Tagesbudget von 16 MiB für die Zeilen, die Eingriffe belegen: `INFORMATION frist` („Frist <alt> → <neu> gesetzt“, „<n> Tage entfallen (…)“) und `INFORMATION anhang` mit dem Text „Anhänge gelöscht“. Die Zeilen behalten Stufe, Format und Bereich; nur das Budget, in das sie zählen, ist ein anderes.
- **Eine Zuordnung an einem Ort**: eine reine Funktion `ist_eingriff(stufe, bereich, text)` in `src/protokoll/eingriffe.rs`, mit der Konstante `ANHAENGE_GELOESCHT = "Anhänge gelöscht"`. Beim Schreiben und beim Wiederlesen der Tagesdatei nach einem Neustart gilt dieselbe Zuordnung.
- **Erschöpft die Klasse** (nur durch Eingriffe), steht einmal je Tag `WARNING protokoll Tagesbudget EINGRIFF erreicht — weitere Einträge bis Mitternacht nur gezählt`; weitere Eingriffszeilen zählen in `protokoll_verworfen`. Keine neue Health-Zahl.
- **Beitreten und Verlassen bleiben** je Ereignis eine `INFORMATION sitzung`-Zeile ohne Buchung am Schreibkonto; der Normaltag („wer kam, wer ging“) liest sich wie bisher. Das Informations-Budget bleibt durch eine Beitrittsflut erschöpfbar (gezählt, im Health-Check sichtbar); das nimmt die Entscheidung in Kauf.
- **Grenzen**: höchstens 64 + 32 + 32 + 16 = 144 MiB je Tag (bisher 128 MiB), über 14 Tage ≈ 1,97 GiB (bisher ≈ 1,75 GiB).

Zahlt ein auf: [Vision](R00000-vision.md) — „eine gemeinsame LAN-Instanz“ und „Mensch und KI sind gleichberechtigt“: wer in einer gemeinsamen Instanz Fristen setzt oder Anhänge löscht, bleibt für die Betreiberin nachvollziehbar, egal wie viele Beitritte vorher das Protokoll gefüllt haben. Kein Widerspruch zur Vision; der architekt hat keine Richtungsfrage gesehen (ADR-0049).

Herkunft: Karte #46 — Security-Gate #9 (security-reviewer und architekt, 2026-10-29), Gegenmittel K-b der Löschzeile aus [R00029](R00029-anhaenge-ueber-die-api-ablegen-senden-und-ausliefern.md) („Betrieb“). Entschieden in [ADR-0049](../Dokumentation/ADRs/0049-eingriffszeilen-mit-eigenem-tagesbudget-beitreten-und-verlassen-bleiben-einzelzeilen.md) (Option B; Episode je Herkunft, Buchung am Konto und die Kombination verworfen bzw. als Folgekarte nach Messung).

Weiche (`rollen/planer/befunde-einplanen.md`): **reifer** — R00014/R00018 halten ihre Zusagen (Tagesbudget je Stufenklasse, über Neustarts); kein Kriterium verlangt, dass eine Eingriffszeile nach einer Flut anderer Zeilen noch steht. Deshalb ein Feature unter I0021, kein `aus-bug`.

Slice (WBS `Dokumentation/Planung/localchata.md`, unter I0021 Betriebsprotokoll): F0125 Eingriffszeilen mit eigenem Tagesbudget.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00014, R00018 und R00029: Betriebsprotokoll, Stufe, Bereich, Tagesbudget, Klasse, Episode, Gnadenfrist, Schreibkonto, Kennung, Herkunft.

## Geschäftlicher Nutzen

Ein Teilnehmer meldet sich 20-mal an und ab (die Gnadenfrist deckelt auf 128 Beitritte je Herkunft und 512 gesamt je 60 s), bis das Informations-Budget des Tages voll ist. Danach löscht er die Anhänge eines anderen oder setzt eine Frist von 7 auf 1 Tag — und die Betreiberin findet im Protokoll nichts, weil die Zeile nur gezählt wurde. Mit R00034 steht die Zeile mit `kennung=<name>@<herkunft>` da, und die Platte bleibt gedeckelt.

## Funktionale Anforderungen

### Eingriffszeilen mit eigenem Tagesbudget (F0125; ADR-0049)

- **Klasse `EINGRIFF`**: vierte Klasse neben `DEBUG`, `INFORMATION+WARNING`, `ERROR+EXCEPTION`; Budget 16 MiB (`BUDGETS.eingriff == 16_777_216`). `WARNING`, `ERROR`, `EXCEPTION` und `DEBUG` derselben Bereiche bleiben in ihrer Klasse.
- **Was zählt** (abschließend, `ist_eingriff`): Stufe `INFORMATION` und Bereich `frist`, oder Bereich `anhang` und Text beginnt mit `ANHAENGE_GELOESCHT`. Der Code, der „Anhänge gelöscht“ schreibt (Karte #9), nimmt den Text aus dieser Konstante.
- **Beim Schreiben** bestimmt `Protokoll::eintragen` die Klasse aus Stufe, Bereich und Text (heute nur aus der Stufe).
- **Beim Wiederlesen** (Neustart am selben Tag) liest `verbrauch_je_klasse` aus jeder Zeile Stufe, Bereich und Text und ruft dieselbe Funktion; Ergebnis `[u64; 4]` in der Reihenfolge `DEBUG, INFORMATION+WARNING, ERROR+EXCEPTION, EINGRIFF`; eine fremde Zeile zählt weiter zu `DEBUG`.
- **Beitreten, Verlassen, Weg** unverändert (ADR-0049 Regel 5).
- **Pflegeregel**: wer eine neue `INFORMATION`-Zeile aufnimmt, die gemeinsame oder fremde Daten ändert oder löscht, trägt sie in `eingriffe.rs` ein, mit Test auf den wörtlichen Katalogtext (Review-Punkt, keine Laufzeitanforderung).

## Nicht-funktionale Anforderungen

- **Platte**: höchstens 144 MiB je Tag; eine Herkunft erschöpft `EINGRIFF` nicht an einem Tag (Frist: 10 Einheiten am Konto, ≤ 0,8/s je Herkunft ≈ 11,7 MB/Tag — Rechnung ADR-0049 Regel 4).
- Keine Änderung an Routen, Antworten oder Health-Check; kein Handler ändert sich.

## Akzeptanzkriterien

### F0125 Eingriffszeilen mit eigenem Tagesbudget

- [ ] **Abnahme-Probe „Eingriffszeile nach einer Beitrittsflut“** (Integration mit Speicher-Senke, Test im Modul `src/http`; wörtlich aus ADR-0049): Gegeben das Programm mit Speicher-Senke und den Tagesbudgets `INFORMATION+WARNING` = 2 KiB und `EINGRIFF` = 2 KiB (übrige wie Standard; Aufbau wie `src/http/fristspur.rs`: `im_speicher(…).mit_budgets(…)`, Router mit Testserver) und ein vorhandener Channel mit Frist 7 Tage. Wenn von `127.0.0.1` 20-mal hintereinander `POST /api/beitreten` mit jeweils neuem Namen und danach `POST /api/verlassen` mit dem erhaltenen Token laufen und anschließend eine Sitzung von `127.0.0.1` `PUT /api/channels/<name>/frist` mit 30 Tagen setzt (200), dann
  - steht genau eine Zeile `WARNING protokoll Tagesbudget INFORMATION+WARNING erreicht — weitere Einträge bis Mitternacht nur gezählt`,
  - ist `protokoll_verworfen` ≥ 1 und mindestens eine der 40 Sitzungszeilen fehlt,
  - steht **danach** die Zeile `INFORMATION frist Frist 7 → 30 Tage gesetzt | channel=<name> … kennung=<name>@127.0.0.1`,
  - steht keine Zeile `Tagesbudget EINGRIFF erreicht`.
  Die Mutante „Klasse nur aus der Stufe“ (heutiges `Klasse::von(stufe)`) lässt die Frist-Zeile fehlen — rot. Heute rot.
- [ ] **Zweiter Fall „Anhänge gelöscht“** (gilt, sobald F0116 Anhänge löschen über die API, Karte #9, auf `main` ist; wörtlich aus ADR-0049): dieselbe Flut, danach löscht die Sitzung einen eigenen gesendeten Anhang mit Bestätigung → die Zeile `INFORMATION anhang Anhänge gelöscht | … anhaenge=1 …` steht im Protokoll. Bis dahin ist das Kriterium offen und blockiert F0125 nicht.
- [ ] **`ist_eingriff`** (Unit, rein; wörtlich aus ADR-0049): `(Information, frist, "Frist 7 → 30 Tage gesetzt")` ja; `(Information, frist, "2 Tage entfallen (…)")` ja; `(Information, anhang, "Anhänge gelöscht")` ja; `(Information, anhang, "Großes Hochladen abgeschlossen")` nein; `(Error, frist, "Tag nicht entfallen")` nein; `(Information, sitzung, "Sitzung beigetreten")` nein; `ANHAENGE_GELOESCHT == "Anhänge gelöscht"` wörtlich.
- [ ] **Klassen getrennt** (Unit, Wegwerf-Ordner; wörtlich): Budget `EINGRIFF` = 1 KiB: Eingriffszeilen über 1 KiB → genau eine `Tagesbudget EINGRIFF erreicht`-Zeile, weitere verworfen; eine `INFORMATION sitzung`-Zeile danach steht in der Datei — und umgekehrt verdrängt eine erschöpfte `INFORMATION`-Klasse keine Eingriffszeile.
- [ ] **Wiederlesen** (Unit, Wegwerf-Ordner; wörtlich): `logs/<heute>.log` mit 2.100 B `INFORMATION sitzung`-Zeilen und einer `INFORMATION frist`-Zeile von 100 B; `verbrauch_je_klasse` → `[0, 2100, 0, 100]`; ein neues Protokoll am selben Tag (Budgets 2 KiB) verwirft die nächste Sitzungszeile und schreibt die nächste Frist-Zeile. Die Mutante „Wiederlesen nur nach Stufe“ → `[0, 2200, 0, 0]`, rot.
- [ ] **Budget** (Unit): `BUDGETS.eingriff == 16_777_216`; Summe der Tagesgrenzen 144 MiB.
- [ ] **Normaltag unverändert**: die Protokolltests zu R00014, R00018, R00022 und R00029 laufen grün (Teständerung nur dort, wo `verbrauch_je_klasse` `[u64; 3]` erwartet).

## Betroffene Verzeichnisstruktur

- Server: neu `src/protokoll/eingriffe.rs`; geändert `src/protokoll/budget.rs` (Klasse, `Budgets.eingriff`, `verbrauch_je_klasse`), `src/protokoll/mod.rs` (`eintragen`, `pub mod eingriffe`), `src/protokoll/senke.rs` (Wiederlesen).
- Tests: Units in den genannten Dateien; Probe neu unter `src/http/` (Datei neben `fristspur.rs`, eingebunden in `src/http/mod.rs` als Testmodul).
- Keine Oberfläche.

## Technische Überlegungen

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0125: B0555 Eingriffszeile erkennen · B0556 Klasse EINGRIFF mit 16 MiB · B0557 Eintragen ordnet nach Stufe, Bereich und Text · B0558 Wiederlesen mit derselben Zuordnung · B0559 Beitrittsflut lässt die Fristzeile stehen

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Unit (rein bzw. Wegwerf-Ordner)**: `ist_eingriff`, Klassen getrennt, Wiederlesen, Budget.
- **Integration (Speicher-Senke, Router mit Testserver)**: Abnahme-Probe nach der Beitrittsflut.
- **E2E**: keine (keine Oberfläche).

## Abhängigkeiten

- Abhängig von: F0055 Grenzen, Episoden und Tagesbudget, F0061 Tagesbudget über Neustarts (beide gruen), F0116 nur für den zweiten Fall der Probe.
- Konflikte: Karte #9 (I0010) ändert `src/http/*`; berührt wird hier nur `src/http/mod.rs` (eine `mod`-Zeile im Testmodul) und eine neue Datei. Die Konstante `ANHAENGE_GELOESCHT` liefert F0125; #9 nimmt den Text daraus (Hinweis architekt #46).
- Blockiert: —

## Offene Fragen

Keine an den Betreiber. Entschieden vom planer (Rückfragen mit Empfehlung beantwortet):

- **Zweiter Fall der Probe als Kriterium, nicht als Blocker**: F0116 ist noch nicht auf `main`; das Kriterium gilt, sobald es dort ist, F0125 wird davor abgenommen. Verworfen: F0125 hinter F0116 stellen — verzögert die Härtung ohne Not, der Hauptfall (Frist) belegt dieselbe Zuordnung.
- **User Stories mit Hinweis statt Handbeobachtung**: eine Beitrittsflut, die das Budget von 32 MiB leert, dauert Stunden; die Budgets verkleinern lässt sich nur im Test. Der Entwicklertest prüft US-1 deshalb über die Abnahme-Probe, US-2 von Hand.

## Manuelle Vorbereitungstätigkeiten

Keine.

## Manuelle Nachbereitungstätigkeiten

Keine.

## Umfang

Gezählt aus der WBS (Bubbles); 0,4 h je klare Bubble ist die Untergrenze nach dem Skill `work-breakdown-structure` (Hypothese, ungemessen für diesen Typ).

- F0125: 5 Bubbles · 5 klar (2,0 h) · 0 unklar
- Zusammen: 5 Bubbles · 2,0 h klar

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: die Probe des security-reviewer im Gate #9 — `beitreten`/`verlassen` schreiben `INFORMATION` ohne Buchung; eine Flut erschöpft das gemeinsame Budget, danach fehlt die Löschzeile.

- Eingriffszeilen zählen in ein eigenes Budget. Deshalb kann keine Flut anderer `INFORMATION`- oder `WARNING`-Zeilen sie verdrängen — auch keine, die erst noch gebaut wird.
- Die Zuordnung steht an einem Ort und gilt beim Schreiben und Wiederlesen. Deshalb hält die Zusage auch über einen Neustart am selben Tag.
- Das Budget ist gedeckelt (16 MiB). Deshalb bleibt die Platte je Tag begrenzt, anders als bei „Eingriffszeilen vom Budget ausnehmen“ (verworfen in ADR-0042).
- Beitreten und Verlassen bleiben Einzelzeilen. Deshalb liest sich der Normaltag wie bisher.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).

## Notizen

- Verworfen (ADR-0049): A Episode je Herkunft (verdoppelt den Normaltag, schützt nur gegen diese Quelle), C Buchung am Schreibkonto (Beitreten hat keine Kennung, koppelt Beitreten an Senden), D A + B (Gewinn ungemessen; Folgekarte erst, wenn der Messwert aus ADR-0042 Beitrittsfluten im Betrieb zeigt).
- Nicht hier: die Gnadenfrist und das Informations-Budget selbst; sie bleiben erschöpfbar und gezählt.
