# ADR-0005: Das Programm ist einziger Schreiber; Reihenfolge über Sperre je Gespräch und monotone Zeit

**Status**: Vorgeschlagen
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #1 I0001) — wartet auf den Betreiber (Richtungsfrage, siehe unten)
**Kontext-Anforderung**: — (WBS I0002, I0009, I0015, I0016)
**Erprobt durch**: —

## Kontext

Mehrere Browser-Tabs und Agenten senden gleichzeitig in denselben Channel. Die Reihenfolge im Log muss eindeutig sein und mit der im Index, im Live-Kanal und in der Anzeige übereinstimmen; ein wartender Agent braucht einen Zeiger „alles nach X“, der auch über einen Neustart trägt. Die Vision sagt einerseits „Der Agent [schreibt] über die API oder an den Dateien“, andererseits ist I0003 „Fremde Änderung erscheint live“ verworfen (2026-10-27, mit dem Wegfall von Git). Ob Dritte an den Dateien schreiben dürfen, ist damit eine offene Richtungsfrage der Vision.

## Optionen

### Option A: Nur das Programm schreibt; eine Sperre je Gespräch; global streng steigende Nachrichtenzeit als ID
- Ablauf je Nachricht unter der Sperre des Gesprächs: Zeit vergeben (`max(jetzt, letzte + 1 ms)`, global über alle Gespräche, beim Start aus dem jüngsten Satz im Ordner fortgesetzt) → Satz in **einem** `write_all` anhängen → Index ergänzen → Ereignis veröffentlichen → Sperre frei.
- Ordner-Sperre `.localchatA.sperre` (`File::try_lock`): eine zweite Instanz auf demselben Ordner endet mit Fehler.
- Vorteile: Log-, Index- und Live-Reihenfolge sind per Konstruktion gleich; die Zeit ist ID und Wartezeiger zugleich; kein Dateibeobachter.
- Nachteile: wer von außen an eine Logdatei anhängt, wird erst nach einem Neustart gesehen, und seine Zeit kann die Monotonie verletzen.

### Option B: Programm und Dritte schreiben; Dateibeobachter (`notify`) liest fremde Anhänge nach
- Vorteile: erfüllt den Wortlaut „an den Dateien“; Agenten können ohne HTTP schreiben.
- Nachteile: zwei Schreiber je Datei — verschränkte Sätze bei gleichzeitigem Anhängen, Zeitstempel von außen ungeordnet, Index muss Dateiendstände nachverfolgen; der Baustein Gespräche verliert die Datenhoheit (Schnittfrage „wer besitzt die Daten“ scheitert).

### Option C: Ein Schreiber-Task je Gespräch mit Warteschlange (Aktor) statt Sperre
- Vorteile: keine Sperre im Code; Gegendruck über die Länge der Warteschlange.
- Nachteile: mehr Mechanik (Kanäle, Antworten per Rückkanal) für dieselbe Garantie; Fehler beim Schreiben müssen über den Rückkanal zurück.

## Entscheidung

**Gewählt (vorgeschlagen): Option A**

Ein Schreiber hält die Invariante „gleiche Reihenfolge überall“ ohne Abstimmung, und die Zeit als ID gibt Agenten einen verlustfreien Wartezeiger. Option C bleibt ein Umbau innerhalb von [A00006](../Architektur/A00006-gespraeche.md), falls die Sperre zum Engpass wird. In Kauf genommen: die Dateien sind für Dritte nur lesbar. **Vorgeschlagen**, weil das dem Satz „oder an den Dateien“ der Vision widerspricht — der Betreiber entscheidet: (1) nur lesbar (Empfehlung; Vision ergänzen über `/vision fortschreiben`), oder (2) Schreiben an den Dateien unterstützen (dann Option B als eigene Interaction mit eigener Abnahme, Datenhoheit neu schneiden).

## Konsequenzen

**Positiv:**
- Zwei gleichzeitige `POST` in denselben Channel landen in der Reihenfolge der Sperrvergabe, mit unterschiedlichen Zeiten, in Log, Index und Live gleich.
- Zwei Programme auf einem Ordner sind ausgeschlossen (Exit 1 mit Meldung).

**Negativ / Inkauf zu nehmen:**
- Bei sehr hoher Last (> 1000 Nachrichten/s) läuft die Nachrichtenzeit der Uhr voraus — für einen LAN-Chat unerheblich.
- Stellt jemand die Systemuhr zurück, bleiben die Zeiten monoton, aber sie zeigen bis zum Aufholen die alte Zeit.

**Folgeentscheidungen / Aufgaben:**
- [ ] Betreiber: Richtungsfrage „an den Dateien“ beantworten.
- [ ] Test der Reihenfolge (Integration): 20 Tasks senden je 50 Nachrichten in denselben Channel; Log, `block` und SSE-Mitschnitt liefern dieselbe Folge, alle Zeiten streng steigend.

## Quellen / Bezuege

- [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00003 Start](../Architektur/A00003-start.md)
- Vision, Zielbild „Mensch und KI sind gleichberechtigt“; WBS I0003 (verworfen); ADR-0003, ADR-0004

## Ergänzungen

- **2026-10-28 (Karte #8 I0009):** Die Sperre je Gespräch genügt für die Ordnung **je Channel**, nicht für einen Wartezeiger über Gespräche. Ab I0009 laufen Zeitvergabe, Anhängen, Index und Veröffentlichen unter einer zusätzlichen Schreibsperre über alle Gespräche ([ADR-0035](0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md)); Option C (Aktor je Gespräch) entfällt damit als Ausweg. Die Richtungsfrage „an den Dateien“ bleibt offen und unverändert.
