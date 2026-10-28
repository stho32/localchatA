# ADR-0030: Entfallen beim Start, beim Tageswechsel im Sekundentakt und beim Verkürzen — Löschen der Tagesdatei unter der Gesprächssperre, dann Index

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #12 I0018)
**Kontext-Anforderung**: — (WBS I0018)
**Erprobt durch**: — (überträgt gebaute Mechanik: Takt-Task, der nur das Wann bestimmt, Regel über `pflegen(jetzt)` mit gestellter Uhr — `src/live/takt.rs`, ADR-0016; Startuhr `LOCALCHATA_JETZT` läuft ab dem Start weiter — `src/uhr.rs`, Test `startzeit_aus_der_umgebung_gilt_als_jetzt_und_laeuft_weiter`)

## Kontext

- Vision: Tage, die älter sind als die Frist, **entfallen**. [ADR-0003](0003-ablage-layout-im-ordner.md): „geprüft beim Start, beim Tageswechsel und beim Ändern der Frist“; Frist-Pflege ist Löschen ganzer Dateien. Grenze: [ADR-0029](0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md).
- Heute lädt der Start nur Tage in der Frist; ältere bleiben unangetastet liegen ([A00006](../Architektur/A00006-gespraeche.md), Ergänzung I0002). Eine Instanz läuft im Betrieb wochenlang.
- [ADR-0005](0005-programm-als-einziger-schreiber-und-reihenfolge.md) / [ADR-0011](0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md): das Programm ist einziger Schreiber; Log, Index und Live zeigen dieselben Nachrichten, auch im Fehlerfall. Sperrordnung: Verzeichnis vor Gespräch ([A00012](../Architektur/A00012-channelverzeichnis.md)).
- [ADR-0006](0006-verlaufsindex-im-speicher.md): „Entfallen: Eintrag fallen lassen, Dateien löschen — kein Umbau des Index.“
- [ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md) Regel 2: ein Cursor muss zu keiner Nachricht gehören; `aelter`/`neuer` ergeben sich aus der Lage im Index. [ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md): ein Tab hält bis zu 500 Nachrichten im Fenster und lädt über `vor`/`nach` nach.
- Projektregel (AI-Team/planer.md, entwickler.md, Gelerntes #4): Regeln exakt über die gestellte Uhr, Zustellung nur als obere Grenze.

## Optionen

### Option A: Pflegen beim Start (vor der Startzeile), im Sekundentakt bei geändertem Datum, und sofort beim Verkürzen
- Ein Task fragt jede Sekunde `uhr.jetzt()`; nur wenn sich das lokale Datum seit der letzten Pflege geändert hat, läuft `pflegen(heute)` über alle Channels. Der Start pflegt einmal synchron nach `laden`, das Setzen der Frist sofort für seinen Channel.
- Vorteile: Wann (Takt) und Was (`pflegen(heute)`) getrennt wie bei der Anwesenheit — die Regel ist mit gestellter Uhr deterministisch, die Zustellung höchstens eine Sekunde nach Mitternacht; billig (ein Datumsvergleich je Sekunde); deckt Zeitsprünge und Ruhezustand des Rechners ab.
- Nachteile: ein Task mehr; „Tageswechsel“ wird bis zu 1 s spät bemerkt.

### Option B: Nur beim Start und beim Ändern der Frist
- Vorteile: kein Task, kein Takt; am einfachsten.
- Nachteile: eine Instanz, die drei Wochen läuft, hält drei Wochen Verlauf — das Fertig-Kriterium „Logtage älter als die Frist entfallen“ gilt nur zufällig; der Speicher wächst ohne Grenze.

### Option C: Beim ersten Satz eines neuen Tages (Rotation in `senden`)
- Vorteile: kein Takt; Entfallen geschieht dort, wo ohnehin die Datei wechselt.
- Nachteile: ein stiller Channel rotiert nie — seine alten Tage bleiben; `senden` bekommt Löscharbeit unter der Gesprächssperre, die die Sendezeit verlängert.

### Option D: Ein Timer schläft bis zur nächsten lokalen Mitternacht
- Vorteile: genau ein Aufwachen am Tag.
- Nachteile: die Dauer bis Mitternacht hängt an Zeitumstellung und Uhrsprüngen; `tokio::time` folgt der Monotonuhr, nicht dem Uhr-Provider — mit `LOCALCHATA_JETZT` stimmt der Zeitpunkt nicht, Tests müssten echte Mitternächte abwarten.

### Löschen oder Verschieben
- **Löschen** (`remove_file`): entspricht „entfallen“ und ADR-0003; Platz wird frei. Schwäche: nicht rückgängig.
- **Verschieben** in einen Papierkorb (`<ordner>/entfallen/…`): rückholbar. Schwäche: der Platz wird nie frei, solange niemand den Papierkorb leert — es braucht eine zweite Frist und eine zweite Pflege; `ls` und `grep -r` im Ordner finden „entfallene“ Nachrichten weiter.
- **Verschieben mit Nachfrist** (Papierkorb, der nach n Tagen geleert wird): rückholbar und begrenzt. Schwäche: doppelte Mechanik für den einen Fall „versehentlich verkürzt“, den die Bestätigung beim Verkürzen ([ADR-0031](0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md)) schon abfängt.

## Entscheidung

**Gewählt: Option A, Löschen**

Es überwiegt, dass die gebaute Takt-Form (Takt fragt, `pflegen(heute)` entscheidet) die Regel deterministisch testbar hält und auch stille Channels und lange Laufzeiten abdeckt; beim Löschen, dass „entfallen“ wirklich Platz freigibt und der Ordner zeigt, was gilt. In Kauf genommen: ein Task mehr, bis zu 1 s Verzug nach Mitternacht, und ein versehentliches Verkürzen ist nach der Bestätigung nicht rückholbar.

Regeln:

1. **Operation** `Gespraeche::pflegen(heute: NaiveDate, anlass: Anlass) -> Vec<Pflegebericht>` mit `Anlass = Start | Tageswechsel | FristGesetzt { von: Kennung }`, und `frist_setzen(…)` ruft sie für seinen Channel ([ADR-0031](0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md)). `heute` kommt als Argument — die Operation fragt keine Uhr.
2. **Anlässe**:
   - **Start**: einmal synchron nach `laden` und **vor** der Startzeile (A00003). Wer die Startzeile liest, findet die alten Tage schon nicht mehr.
   - **Tageswechsel**: Task `fristpflege::takten(gespraeche, uhr, ende, PERIODE = 1 s)`, gestartet vor der Startzeile, beendet mit dem Ende-Signal. Je Takt `heute = uhr.jetzt().date_naive()`; nur wenn `heute ≠ zuletzt_gepflegt`, läuft `pflegen` über `spawn_blocking`. Ein Datum rückwärts (zurückgestellte Uhr) pflegt ebenfalls — es löscht höchstens weniger, nie mehr.
   - **Verkürzen**: sofort beim Setzen der Frist, nur für diesen Channel.
3. **Welche Tage**: die Tage auf der **Platte** (`Logablage::tage`, also nur reguläre Dateien `JJJJ-MM-TT.log`, [ADR-0032](0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md)), nicht nur die im Index — beim Start liegen die alten Tage nur auf der Platte. Auswahl über `frist::teilen` (ADR-0029 Regel 3). Nur Channels mit lesbaren Einstellungen (ADR-0032).
4. **Reihenfolge je Tag, unter der Gesprächssperre**: Verzeichnis lesend nachschlagen → Sperre des Gesprächs → **Datei löschen** (`tag_entfernen`) → nur bei Erfolg den Tag aus dem Index nehmen → Sperre frei. Scheitert das Löschen, bleibt der Tag in Datei **und** Index (Log und Index bleiben gleich, ADR-0011), eine Zeile auf stderr nennt Datei und Grund; erneut versucht wird beim nächsten Anlass (nächster Tageswechsel, Neustart, Setzen der Frist) — nicht in jedem Takt, damit stderr nicht jede Sekunde dieselbe Zeile bekommt.
5. **Kein Ereignis für Entfallen**: Entfallen ändert keinen `stand` und veröffentlicht nichts. Ein offener Tab behält, was er zeigt; sein nächster Abruf `vor=<älteste gezeigte Zeit>` liefert `nachrichten: []`, `aelter: false` — der Cursor gilt weiter, weil er zu keiner Nachricht gehören muss (ADR-0025 Regel 2). `nach=<zeit vor der Frist>` liefert die älteste noch vorhandene Nachricht; der Abschnitt dazwischen ist entfallen, ohne dass die Antwort es sagt (Grenze siehe Konsequenzen). Neu geöffnet zeigt der Channel nur noch, was in der Frist liegt. Der Tab, der selbst verkürzt hat, öffnet den Channel neu ([ADR-0033](0033-frist-in-der-kopfzeile-dialog-mit-bestaetigung-beim-verkuerzen.md)).
6. **Protokoll auf stderr**, eine Zeile je Channel und Pflege, nur wenn etwas entfallen ist oder scheiterte: `channels/allgemein: 3 Tage entfallen (2026-10-19 bis 2026-10-21; Frist 7 Tage; Tageswechsel)` bzw. `…; Frist gesetzt von shoff@127.0.0.1)`; je gescheitertem Tag eine Zeile `channels/allgemein/2026-10-21.log: nicht entfallen (<Grund>)`.
7. **Kennzahlen**: `nachrichten` und `log_bytes` im Health-Check sinken mit dem Index; nichts Neues im Health-Check.
8. **Anhänge** (ab I0010): `tag_entfernen` löscht danach `anhaenge/<tag>/` (ADR-0003; Regeln in ADR-0032).

## Konsequenzen

**Positiv:**
- Speicher und Platte bleiben bei jeder Laufzeit durch die Frist begrenzt (ADR-0006).
- Testbar ohne echte Mitternacht: Unit mit `pflegen(heute)` über die Speicherablage; Integration mit `LOCALCHATA_JETZT=…T23:59:58` — die Startuhr läuft weiter, der Tageswechsel kommt nach 2 s, bemerkt nach höchstens einem Takt mehr.
- Entfallen hält das Senden nur für das Löschen einer Datei auf (Gesprächssperre, ein `unlink`).

**Negativ / Inkauf zu nehmen:**
- Agenten, die mit `nach=t` fortsetzen und länger als die Frist weg waren, sehen nicht, dass dazwischen etwas entfallen ist. Ein späteres Feld `beginn` (ältester Tag in der Frist) in der Verlauf-Antwort wäre eine Ergänzung nach ADR-0025 Regel 3, kein Umbau — nicht in I0018.
- Ein Tab zeigt entfallene Nachrichten weiter, bis er den Channel neu öffnet.
- Löschen ist endgültig.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … der Rechner nach dem Ruhezustand mit einer Uhr startet, die Jahre vorgeht (leere RTC-Batterie, falsches NTP), und die Pflege alles bis auf heute löscht. Früh-Indikator: stderr-Zeile „n Tage entfallen“ mit ungewöhnlich großem `n`; Zeiten neuer Nachrichten in der Zukunft. Gegenmaßnahme: hingenommen — die Instanz vertraut der Systemuhr wie jede andere Anwendung (Aspekt Zeit); die stderr-Zeile nennt Anzahl und Spanne, damit der Betreiber es sieht.
2. … der Takt die Pflege jede Sekunde wiederholt, weil ein Löschen scheitert, und stderr flutet. Früh-Indikator: Integrationstest mit nicht löschbarer Datei zählt Zeilen. Gegenmaßnahme: Regel 2 (nur bei geändertem Datum) und Regel 4 (Wiederholung beim nächsten Anlass).
3. … ein Tab nach dem Entfallen Nachrichten doppelt oder lückenhaft zeigt, weil `vor` auf einen entfallenen Tag zeigt. Früh-Indikator: Unit-Test des Index „Cursor in einem entfallenen Tag“. Gegenmaßnahme: Regel 5 — exklusive Grenzen über die `BTreeMap`, ein Cursor ohne Tag läuft zum nächsten vorhandenen.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (Gespräche, Speicherablage, ohne Uhr): Channel `allgemein` mit Frist 7 und Tagen 2026-10-20, -21, -22, -28 auf der Ablage, nur -22 und -28 im Index → `pflegen(2026-10-28, Start)` entfernt -20 und -21 aus der Ablage, Index unverändert; `pflegen(2026-10-29, Tageswechsel)` entfernt -22 aus Ablage **und** Index, `nachrichten(Vor(erste Zeit des 28.))` → leer, `aelter: false`. Ersetzt: Logablage (Speicher).
- [ ] Fehlerpfad (Unit): Speicherablage lässt `tag_entfernen` scheitern → Tag bleibt in Ablage und Index, Bericht nennt ihn; die nächste `pflegen` mit demselben `heute` über den Takt löst keinen zweiten Versuch aus, `pflegen` mit dem nächsten Tag schon.
- [ ] Takt (Unit, `tokio::time::pause`, gestellte Uhr): Uhr auf 23:59:59.500 → kein Aufruf; Uhr auf 00:00:00.000 des Folgetags, ein Takt → genau ein Aufruf mit dem neuen Datum; weitere Takte am selben Tag → keiner.
- [ ] Erster Integrationstest (echtes Programm): Wegwerf-Ordner `channels/allgemein/` mit `einstellungen.toml` (`frist_tage = 7`) und Tagen 2026-10-20, -21, -22, -28; Start mit `LOCALCHATA_JETZT=2026-10-28T23:59:58+01:00`, `TZ=Europe/Berlin` → direkt nach der Startzeile fehlen -20 und -21, -22 und -28 liegen da; danach fehlt -22 spätestens 4 s nach der Startzeile (erwartet ≈ 2–3 s: 2 s bis Mitternacht plus höchstens ein Takt; Grenze 30 % darüber), -28 liegt weiter da; stderr nennt beide Pflegen. Die Regel selbst (23:59:59.999 noch da, 00:00:00.000 weg) prüft der Takt-Unit-Test mit gestellter Uhr, nicht dieser Test.

## Quellen / Bezuege

- [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00008 Logablage](../Architektur/A00008-logablage.md), [A00009 Verlaufsindex](../Architektur/A00009-verlaufsindex.md), [A00003 Start](../Architektur/A00003-start.md); Aspekte [X00007 Zeit](../Architektur/Aspekte/X00007-zeit.md), [X00001 Logging und Diagnose](../Architektur/Aspekte/X00001-logging-und-diagnose.md)
- ADR-0003, ADR-0005, ADR-0006, ADR-0011, ADR-0016 (Takt-Form), ADR-0025, ADR-0026, ADR-0029; `src/live/takt.rs`, `src/uhr.rs`
- Vision R00000, Zielbild „Das Log ist das Gedächtnis“; WBS I0018

## Ergänzungen

- **2026-10-28 (Vorlauf „Betriebsprotokoll“) — stderr wird Betriebsprotokoll:** Regel 6 „Protokoll auf stderr“: `INFORMATION frist` je Channel und Pflege, `ERROR frist` je nicht entfallenem Tag; Regel 2 „Tageswechsel“: der Takt ist der gemeinsame Tagestakt aus ADR-0041 Regel 6 (Fristpflege meldet sich dort als Tagesaufgabe an), die Regel selbst bleibt ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regeln 2 und 3; Format [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md), stderr spiegelt ab Warning). Die übrigen Regeln dieser ADR gelten unverändert.
- **2026-10-28 (Detailplanung R00014; Konflikt R00008 ↔ ADR-0040 Regel 6) — wie I0018 an das Betriebsprotokoll anbindet:** R00008 (I0018) sagt Zeilen auf **stderr** zu, auch die gelingenden: „Frist 7 → 30 Tage, gesetzt von …“ und „n Tage entfallen (…)“. Nach der Ergänzung oben und nach ADR-0042 Regel 2 sind das `INFORMATION frist`-Einträge, und nach [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) Regel 6 gehen `INFORMATION`-Einträge **nie** auf stderr. R00008 bleibt als Anforderung unverändert; also braucht der Bau eine Stelle, die die Zusage ändert.
  - Option A: **Anbindungs-Feature in I0018** wie F0057 für I0009 (R00016): eine eigene Anforderung ändert die stderr-Zusagen von R00008 in Protokoll-Einträge; Regel 6 von ADR-0040 bleibt ausnahmslos. Schwäche: ein weiteres Feature und eine Anforderung vor dem Bau von I0018; der Mensch am Terminal sieht ein gelingendes Entfallen nicht mehr live, sondern in `logs/<tag>.log`.
  - Option B: Ausnahme in ADR-0040 Regel 6 — `frist`-Einträge auf Information werden zusätzlich auf stderr gespiegelt. Schwäche: zwei Regeln für stderr; jede ältere stderr-Zusage könnte dieselbe Ausnahme verlangen (R00011 wurde über F0057 gerade **ohne** Ausnahme angebunden); im Dienstbetrieb stünde Routine im Journal.
  - Option C: Frist-Ereignisse auf `WARNING` heben, weil Löschen endgültig ist. Schwäche: Entfallen ist Routine (je Channel einmal am Tag); `warnings` stiege täglich, und das Signal „hinsehen, wenn es sich häuft“ verwässert — Stufe nach Wunschkanal statt nach Bedeutung.
  - **Gewählt: A** (Empfehlung des planer). In Kauf genommen: gelingendes Setzen und Entfallen steht nur in der Datei; mit `--protokoll-stufe warning` oder höher fehlt diese Spur ganz — wer die Rechenschaft über Löschungen braucht (X00002), lässt die Stufe auf `information` (Standard).
  - **Abbildung der Zusagen aus R00008** (Soll des Anbindungs-Features; Text ohne Kennungen, Kennungen und Namen im Kontext nach ADR-0040 Regel 3):

    | R00008 heute (stderr) | Betriebsprotokoll | stderr |
    |---|---|---|
    | `channels/allgemein: Frist 7 → 30 Tage, gesetzt von beta@127.0.0.1` | `INFORMATION frist Frist 7 → 30 Tage gesetzt \| channel=allgemein kennung=beta@127.0.0.1` | nein |
    | `channels/allgemein: 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage; Start)` bzw. `…; Frist gesetzt von <kennung>)` | `INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) \| channel=allgemein anzahl=2 grund=start` — `grund` ist der Anlass `start`, `tageswechsel` oder `frist_gesetzt`, bei `frist_gesetzt` zusätzlich `kennung=` | nein |
    | `channels/allgemein/2026-10-21.log: nicht entfallen (<Grund>)` | `ERROR frist Tag nicht entfallen — erneuter Versuch beim nächsten Anlass \| channel=allgemein datei=channels/allgemein/2026-10-21.log grund="…"` | ja |
    | `channels/kaputt: nichts entfällt — Einstellungen defekt (channels/kaputt/einstellungen.toml:1)` | `WARNING frist Nichts entfällt — Einstellungen defekt \| channel=kaputt datei=channels/kaputt/einstellungen.toml zeile=1` | ja |
    | PUT: Einstellungen nicht schreibbar (500), „eine Zeile auf stderr“ | `ERROR frist Frist nicht gesetzt — Einstellungen ließen sich nicht schreiben \| channel=… kennung=… datei=… grund="…"` | ja |
    | Laden: Einstellungsdatei defekt (R00008 „Laden“) | `WARNING laden` nach ADR-0042 Regel 2 (schon im Katalog) | ja |

  - **Unverändert**: Häufigkeit („eine Zeile je Channel und Pflege, nur wenn etwas entfallen ist oder scheiterte“, Regel 6), Wiederholung erst beim nächsten Anlass (Regel 4) und die Ruhe Z4 aus R00008 („keine weitere Zeile“) — geprüft jetzt an Datei **und** stderr. Die Pflege beim Start schreibt vor der Startzeile, also steht ihre Zeile in `logs/<heute>.log`, sobald die Startzeile gelesen ist (die Zusage „steht schon“ bleibt wahr, nur der Ort ändert sich).
  - **Für den planer**: das Feature braucht F0051 (Protokoll schreibt) und F0052 (Tagestakt, an dem sich die Fristpflege anmeldet, ADR-0041 Regel 6), gehört unter I0018 und wird mit F0033/F0034 im selben Lauf gebaut — vor dem ersten Bau von I0018, damit nie stderr-Zeilen entstehen, die gleich wieder umgestellt werden. Die Tests aus R00008, die stderr lesen (Setzen, Bestätigt verkürzen, Start, Mitternacht), lesen für die beiden INFORMATION-Zeilen die Protokolldatei; die Tests für „nicht entfallen“, „Einstellungen defekt“ und den Schreibfehler dürfen stderr weiter lesen.
  - Die beiden neuen Katalogzeilen (`WARNING frist` defekt, `ERROR frist` nicht gesetzt) stehen in [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung „I0018“.
- **2026-10-28 (Abgleich I0018 ↔ Tagestakt, vor Karte #24) — Fristpflege als Tagesaufgabe:** Präzisiert Regel 2 „Tageswechsel“; die Regel selbst (Takt 1 s, nur bei geändertem Datum, auch rückwärts, Ende mit dem Ende-Signal, Start vor der Startzeile) bleibt. Entschieden in [ADR-0041](0041-betriebsprotokoll-als-eigenes-modul-rotation-beim-schreiben-bereinigung-14-tage.md), Ergänzung „Vertrag des Tagestakts“ (a)–(e):
  1. Es gibt **keinen** Task `fristpflege::takten`. Die Fristpflege ist die zweite Tagesaufgabe des Tagestakts (nach der Bereinigung des Betriebsprotokolls); sie ruft `pflegen(heute, Tageswechsel)`.
  2. `zuletzt_gepflegt` ist das `zuletzt` dieser Tagesaufgabe, vorbelegt mit dem `heute` der Pflege beim Start (nach `laden`). Liegt Mitternacht zwischen Start-Bereinigung und Start-Pflege, pflegt der Takt den neuen Tag nicht ein zweites Mal; liegt sie zwischen Start-Pflege und Takt, holt der erste Takt den Tag nach.
  3. Fällig ⇔ `heute ≠ zuletzt_gepflegt` (auch rückwärts, wie bisher); `zuletzt_gepflegt` wird **vor** dem Aufruf gesetzt.
  4. Gerät `pflegen` in Panik, läuft der Takt weiter: `EXCEPTION programmfehler` (Haken) und `ERROR programmfehler Tagesaufgabe fristpflege gescheitert — Tage jenseits der Frist bleiben bis dahin liegen; erneuter Versuch beim nächsten Tageswechsel | operation=fristpflege grund=panik`; nächster Versuch beim nächsten Anlass (Regel 4 sinngemäß).
  5. Folgeaufgabe „Takt (Unit …)“ oben geht im Tagestakt-Test auf (ADR-0041, Ergänzung, „Erste Tests“: Grenze, Rückwärts, Startwert, Panik). Für I0018 bleibt die Integration über Mitternacht (oben, Z3 aus R00008) — sie belegt die Anmeldung der Fristpflege am Takt.
