# ADR-0049: Eingriffszeilen mit eigenem Tagesbudget — `frist` und „Anhänge gelöscht“ zählen in die Klasse `EINGRIFF` (16 MiB), Beitreten und Verlassen bleiben Einzelzeilen ohne Buchung

**Status**: Akzeptiert (nichts gebaut; die Rechnung steht unten, die Probe ist das Abnahmekriterium der Karte)
**Datum**: 2026-10-29
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #46)
**Kontext-Anforderung**: — (Karte #46 „Härtung Protokoll: Beitreten und Verlassen erschöpfen das INFORMATION-Budget“; Herkunft Gate #9 des `security-reviewer`, „Außerhalb“)
**Erprobt durch**: — (gerechnet aus [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regel 6, [ADR-0018](0018-verlassen-sofort-weg-nie-beim-schliessen.md) Regel 2a und `src/anwesenheit/bestand.rs:23–24`; nicht gemessen)

## Kontext

- **Befund**: `POST /api/beitreten` und `POST /api/verlassen` schreiben je eine `INFORMATION sitzung`-Zeile (`src/http/beitreten.rs:51–56`, `src/http/verlassen.rs:26–31`), der Takt eine dritte für „weg“ (`src/live/takt.rs:31–37`) — ohne Buchung am Schreibkonto ([ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md), Folgeentscheidung „Keine Buchung … für `beitreten`/`verlassen`“). Sie teilen sich das Tagesbudget `INFORMATION` + `WARNING` (32 MiB, ADR-0042 Regel 6) mit den Zeilen, die Eingriffe in gemeinsame oder fremde Daten belegen: `INFORMATION frist … gesetzt`, `INFORMATION frist … entfallen` (`src/gespraeche/fristeintrag.rs:28, 98`) und `INFORMATION anhang Anhänge gelöscht` (ADR-0042, Ergänzungen „Karte #9 Betreiber F1′“ und „Karte #9 Security-Gate“, P-a; in Bau auf Karte #9).
- **Rechnung der Flut**: die Gnadenfrist deckelt (ADR-0018 Regel 2a): höchstens 128 Beitritte je Herkunft und 512 gesamt je Gnadenfrist (Standard 60 s). Je Beitritt zwei Zeilen (beigetreten + verlassen bzw. weg) à ≈ 150 B.
  - eine Herkunft: 256 Zeilen / 60 s ≈ 4,3/s ≈ 640 B/s ≈ 55 MB/Tag → 32 MiB in ≈ 14,5 h;
  - vier oder mehr Herkünfte (Gesamtgrenze 512): ≈ 17 Zeilen/s ≈ 2,6 KB/s → 32 MiB in ≈ 3,6 h.
  Danach wird jede `INFORMATION` des Tages nur gezählt — auch die Eingriffszeilen. Damit fällt **K-b** (Löschen mit Kennung im Protokoll, [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „F1′“ Frage K), obwohl P-a die Löschzeile selbst nicht mehr flutbar macht.
- **Qualitätsziel** (Szenario mit Maß): *Ein Teilnehmer löscht Anhänge oder setzt eine Frist, nachdem ein anderer das Informations-Budget des Tages erschöpft hat → die Eingriffszeile mit Kennung steht im Protokoll; die Platte bleibt je Tag gedeckelt.* Nebenziel: der normale Tag des Betreibers (wer kam, wer ging) liest sich wie bisher.
- **Muster im Bestand**: ADR-0042 Regel 6 trennt schon „Debug kann Fehler nicht verdrängen“ durch Klassen. P-b (Löschzeilen vom Budget ausnehmen) ist verworfen, weil die Platte dann ungedeckelt ist (ADR-0042, Ergänzung „Security-Gate“).

## Optionen

| | Regel | Stärke | Schwäche |
|---|---|---|---|
| **A** | Beitreten/Verlassen/Weg als **INFORMATION-Episode je Herkunft** | eine Flut aus einer Herkunft ergibt zwei Zeilen; Mechanik vorhanden (`src/protokoll/episoden.rs`) | verändert den Normaltag: jeder einzelne Beitritt wird Beginn + Ende (doppelt so viele Zeilen), die Kennung steht nur in der Beginn-Zeile — „wer kam“ sieht nur Debug; viele Herkünfte (IPv6) umgehen die Episode je Herkunft; schützt nur gegen **diese** Quelle — die nächste flutbare `INFORMATION`/`WARNING` (z. B. 1.024 Sammel- und Grenzepisoden, ADR-0042 Regel 6 „passt knapp“) nimmt K-b wieder |
| **B (gewählt)** | **eigene Budgetklasse `EINGRIFF`** (16 MiB je Tag) für die Zeilen, die Eingriffe in gemeinsame oder fremde Daten belegen; Beitreten/Verlassen unverändert | K-b hält gegen **jede** Flut anderer Zeilen, ohne die Quelle zu kennen; Platte bleibt gedeckelt (+16 MiB/Tag); kein Handler ändert sich; Normaltag unverändert; Muster schon da (Klassen der Regel 6) | eine vierte Klasse; die Zuordnung hängt an Bereich und Text — eine neue Eingriffszeile, die niemand einträgt, landet still in `INFORMATION`+`WARNING`; das Informations-Budget bleibt erschöpfbar (übrige Warnungen des Tages fehlen dann, gezählt) |
| **C** | **Buchung am Schreibkonto** für Beitreten/Verlassen | ein Mechanismus, gebaut und getestet | Beitreten hat noch keine Kennung (`abbuchen(kennung, herkunft, …)`); bremst erst ab Kosten ≥ 4 stärker als die Gnadenfrist (8 Einheiten/s je Herkunft gegen ≈ 2,1 Beitritte/s heute); koppelt Beitreten an Senden — ein Schwarm, der den Eimer der Herkunft leert, sperrt auch den Beitritt des Menschen; viele Herkünfte umgehen es wie A |
| **D** | Kombination A + B | K-b hält, und eine Flut aus einer Herkunft verbraucht kaum Informations-Budget | zwei Mechanismen, > 5 Bausteine; der Gewinn über B (Warnungen des Tages bleiben bei **einer** flutenden Herkunft) ist ungemessen — als Folgekarte nach Messung, nicht jetzt |

## Entscheidung

**B.** Es überwiegt, dass K-b eine Zusage über die Eingriffszeilen ist, nicht über Beitreten: wer die Zeilen schützt, die belegen müssen, ist unabhängig davon, welche Quelle das allgemeine Budget leert — auch von Quellen, die erst noch gebaut werden. In Kauf genommen: das Informations-Budget bleibt durch Beitreten/Verlassen in ≈ 3,6 h erschöpfbar (wie ADR-0042 Regel 6 es für jede Flut hinnimmt: gezählt, im Health-Check sichtbar); eine Klasse mehr; die Zuordnung braucht Pflege bei neuen Eingriffszeilen.

### Regeln

1. **Klasse `EINGRIFF`** in Regel 6 von ADR-0042, eigenes Tagesbudget **16 MiB**. Die Zeile behält ihre Stufe `INFORMATION`, ihr Format und ihren Bereich — nur das Budget, in das sie zählt, ist ein anderes. Text der Budgetzeile wie Regel 6: `WARNING protokoll Tagesbudget EINGRIFF erreicht — weitere Einträge bis Mitternacht nur gezählt` (zählt nicht ins Budget, einmal je Tag und Prozess); weitere zählen in `protokoll_verworfen` (keine neue Health-Zahl).
2. **Was eine Eingriffszeile ist** (abschließend, eine reine Funktion an einem Ort, `src/protokoll/eingriffe.rs`): Stufe `INFORMATION` **und**
   - Bereich `frist` (heute „Frist <alt> → <neu> gesetzt“ und „<n> Tage entfallen (…)“ — beide ändern bzw. löschen gemeinsame Daten), oder
   - Bereich `anhang` und Text beginnt mit der Konstante `ANHAENGE_GELOESCHT = "Anhänge gelöscht"` (Zeile der Ergänzung „F1′“ mit Bedingung P-a; [ADR-0048](0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) nutzt dieselbe Zeile mit `grund=nachricht`).
   `WARNING`, `ERROR`, `EXCEPTION` und `DEBUG` derselben Bereiche bleiben in ihrer Klasse. Der Code, der „Anhänge gelöscht“ schreibt, nimmt den Text aus dieser Konstante (Karte #9).
3. **Dieselbe Zuordnung beim Schreiben und beim Wiederlesen**: `Protokoll::eintragen` bestimmt die Klasse aus Stufe, Bereich und Text (statt nur aus der Stufe, `src/protokoll/mod.rs:424`); `verbrauch_je_klasse` (ADR-0042 Ergänzung #24 Punkt 9) liest aus jeder Zeile Stufe (2. Wort), Bereich (3. Wort) und den Text danach und ruft dieselbe Funktion — eine fremde Zeile zählt weiter zu `DEBUG`. Ergebnis `[u64; 4]` in der Reihenfolge `DEBUG, INFORMATION+WARNING, ERROR+EXCEPTION, EINGRIFF`.
4. **Grenzen, berichtigt**: höchstens 64 + 32 + 32 + 16 = **144 MiB je Tag**, über 14 Tage ≈ **1,97 GiB** (bisher 128 MiB / 1,75 GiB). Begründung der 16 MiB: eine Eingriffszeile ≈ 170–250 B → ≥ 67.000 Eingriffe je Tag; normaler Tag: einige Dutzend. Die teuerste Eingriffsflut ist die Frist (10 Einheiten am Konto, ADR-0036 Ergänzung Karte #31 (b): ≤ 0,8/s je Herkunft ≈ 11,7 MB/Tag) — **eine** Herkunft erschöpft `EINGRIFF` nicht an einem Tag; wer es mit mehreren versucht, steht mit jeder Zeile und seiner Kennung im Protokoll.
5. **Beitreten, Verlassen, Weg bleiben** `INFORMATION sitzung` je Ereignis in der Klasse `INFORMATION`+`WARNING`, ohne Buchung am Konto (ADR-0036 Folgeentscheidung gilt weiter); die Gnadenfrist bleibt ihre einzige Bremse.
6. **Pflegeregel für den Katalog**: wer eine neue `INFORMATION`-Zeile in den Katalog (ADR-0042 Regel 2) aufnimmt, die gemeinsame oder fremde Daten ändert oder löscht, trägt sie in `eingriffe.rs` ein (mit Test auf den wörtlichen Katalogtext). Review-Punkt für `observability-reviewer` und `security-reviewer`.

### Abnahme-Probe (Kriterium für die Karte, fertig formuliert)

> **Eingriffszeile nach einer Beitrittsflut.** Gegeben das Programm mit Speicher-Senke und den Tagesbudgets `INFORMATION+WARNING` = 2 KiB und `EINGRIFF` = 2 KiB (übrige wie Standard; Aufbau wie `src/http/fristspur.rs`: `im_speicher(…).mit_budgets(…)`, Router mit Testserver) und ein vorhandener Channel mit Frist 7 Tage. Wenn von `127.0.0.1` 20-mal hintereinander `POST /api/beitreten` mit jeweils neuem Namen und danach `POST /api/verlassen` mit dem erhaltenen Token laufen und anschließend eine Sitzung von `127.0.0.1` `PUT /api/channels/<name>/frist` mit 30 Tagen setzt (200), dann
> - steht genau eine Zeile `WARNING protokoll Tagesbudget INFORMATION+WARNING erreicht — weitere Einträge bis Mitternacht nur gezählt`,
> - ist `protokoll_verworfen` ≥ 1 und mindestens eine der 40 Sitzungszeilen fehlt,
> - steht **danach** die Zeile `INFORMATION frist Frist 7 → 30 Tage gesetzt | channel=<name> … kennung=<name>@127.0.0.1`,
> - steht keine Zeile `Tagesbudget EINGRIFF erreicht`.
>
> Zweiter Fall, sobald „Anhänge löschen“ (Karte #9) auf `main` ist: dieselbe Flut, danach löscht die Sitzung einen eigenen gesendeten Anhang mit Bestätigung → die Zeile `INFORMATION anhang Anhänge gelöscht | … anhaenge=1 …` steht im Protokoll.
>
> Mutante „Klasse nur aus der Stufe“ (heutiges `Klasse::von(stufe)`) → die Frist-Zeile fehlt, rot.

Dazu je Regel ein Unit-Test (rein bzw. Wegwerf-Ordner):
- `ist_eingriff`: `(Information, frist, "Frist 7 → 30 Tage gesetzt")` ja; `(Information, frist, "2 Tage entfallen (…)")` ja; `(Information, anhang, "Anhänge gelöscht")` ja; `(Information, anhang, "Großes Hochladen abgeschlossen")` nein; `(Error, frist, "Tag nicht entfallen")` nein; `(Information, sitzung, "Sitzung beigetreten")` nein; `ANHAENGE_GELOESCHT == "Anhänge gelöscht"` wörtlich.
- Budget `EINGRIFF` = 1 KiB: Eingriffszeilen über 1 KiB → genau eine „Tagesbudget EINGRIFF erreicht“-Zeile, weitere verworfen; eine `INFORMATION sitzung`-Zeile danach steht in der Datei (Klassen getrennt in beide Richtungen).
- Wiederlesen: `logs/<heute>.log` mit 2.100 B `INFORMATION sitzung`-Zeilen und 100 B `INFORMATION frist`-Zeile; `verbrauch_je_klasse` → `[0, 2100, 0, 100]`; neues Protokoll am selben Tag (Budgets 2 KiB) → eine Sitzungszeile verworfen, eine Frist-Zeile geschrieben. Mutante „Wiederlesen nur nach Stufe“ → `[0, 2200, 0, 0]`, rot.
- `BUDGETS.eingriff == 16_777_216`.

## Konsequenzen

### Positiv
- K-b hält gegen jede Flut anderer Zeilen, auch gegen künftige Quellen.
- Kein Handler ändert sich; der Normaltag liest sich wie bisher.
- Die Platte bleibt gedeckelt.

### Negativ
- Eine vierte Klasse; 16 MiB mehr je Tag.
- Das Informations-Budget bleibt durch Beitreten/Verlassen in ≈ 3,6 h (viele Herkünfte) bzw. ≈ 14,5 h (eine) erschöpfbar; danach fehlen übrige `INFORMATION`/`WARNING` des Tages (gezählt).
- Die Zuordnung hängt an einem Text-Präfix für `anhang` — eine Umformulierung ohne Konstante fiele still zurück.

### Pre-Mortem
1. … eine neue Eingriffszeile kommt hinzu (z. B. Channel löschen) und niemand trägt sie ein. Früh-Indikator: Katalogzeile mit „gelöscht“/„gesetzt“ ohne Eintrag in `eingriffe.rs`. Gegenmaßnahme: Regel 6 (Review-Punkt), Test auf wörtliche Katalogtexte.
2. … „Anhänge gelöscht“ wird umformuliert. Früh-Indikator: Unit-Test auf die Konstante rot. Gegenmaßnahme: Regel 2 (Text aus der Konstante).
3. … Betreiber vermissen nach einer Beitrittsflut die Warnungen des Tages. Früh-Indikator: `protokoll_verworfen` > 0 mit „Tagesbudget INFORMATION+WARNING erreicht“ und vielen `sitzung`-Zeilen. Gegenmaßnahme: Option A als Folgekarte nach Messung (Messwert-Aufgabe aus ADR-0042 „Folgeentscheidungen“).

## Folgeentscheidungen
- [ ] Bau nach den Regeln 1–3 und der Abnahme-Probe (Karte #46).
- [ ] Karte #9: Text „Anhänge gelöscht“ aus `protokoll::eingriffe::ANHAENGE_GELOESCHT`; zweiter Fall der Probe nach dem Merge.
- [ ] Option A erst, wenn der Messwert aus ADR-0042 zeigt, dass Beitrittsfluten im Betrieb vorkommen.

## Ergänzungen
