# ADR-0011: Anhängen mit Rückbau bei Fehler und Zeilenende-Wächter

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #2 I0002)
**Kontext-Anforderung**: — (WBS I0002; prägt I0009, I0010, I0015)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (Python, `RLIMIT_FSIZE` erzwingt ein Teilschreiben): ein Satz bricht nach 20 von 79 Bytes ab; ohne Rückbau verschmilzt der halbe Kopf mit dem nächsten Satz zu `@ 2026-10-27T14:00:0@ 2026-10-27T14:00:00.002+01:00 c@127.0.0.1` — zwei Nachrichten verloren statt einer; mit `ftruncate` auf die alte Länge bleibt der nächste Satz heil. Probe danach gelöscht

## Kontext

I0002: jede Nachricht steht mit Absender und Zeit im Log ihres Channels. ADR-0004 macht das Format zeilensicher, solange jeder Satz vollständig ist, und nimmt nur den Absturz mitten im `write_all` in Kauf. ADR-0005 legt die Reihenfolge Log → Index → Live fest, Aspekt Fehlerbehandlung (X00004) sagt: scheitert das Anhängen, gilt die Nachricht als nicht gesendet, Index und Live bleiben unberührt. Offen ist, was dann **in der Datei** steht. `write_all` kann nach einem Teil der Bytes scheitern (Platte voll, Kontingent, Dateigrößengrenze, E/A-Fehler); die Datei endet dann mitten in einer Zeile, und der nächste Satz beginnt nicht in Spalte 1. Weil das Programm einziger Schreiber ist (ADR-0005) und die Datei im Anhängemodus offen hält (A00008), ist nur es selbst für diesen Zustand zuständig.

## Optionen

### Option A: Länge vor dem Schreiben merken, bei Fehler auf sie zurückschneiden (`set_len`); dazu ein Zeilenende-Wächter
- Vorteile: Datei, Index und Live bleiben gleich (der gescheiterte Satz existiert nirgends); ein Systemaufruf im seltenen Fehlerfall, nichts im Normalfall; der Wächter (letztes Byte ist `\n`, sonst zuerst `\n` schreiben) fängt auch den Absturz-Fall beim nächsten Öffnen.
- Nachteile: scheitert auch der Rückbau, bleibt ein Rest; zwei Pfade (Rückbau, Wächter), die eigene Tests brauchen.

### Option B: Je Nachricht die Tagesdatei kopieren, Satz anhängen, per `rename` austauschen
- Vorteile: atomar — die Datei ist immer entweder alt oder neu, auch bei Absturz.
- Nachteile: Kosten wachsen mit der Dateigröße (ein voller Tag wird je Nachricht neu geschrieben); verdoppelt kurzzeitig den Platzbedarf genau dann, wenn die Platte voll ist.

### Option C: Nichts zurückbauen; beim Laden meldet der Parser den Defekt (Stand ADR-0004)
- Vorteile: kein zusätzlicher Code beim Schreiben.
- Nachteile: der Rest des gescheiterten Satzes zerstört den **nächsten** gelungenen Satz (Probe) — eine Nachricht, die der Absender als gesendet bestätigt bekam, fehlt nach dem Neustart; bis zum Neustart weichen Datei und Index voneinander ab.

### Option D: Nach einem Schreibfehler das Gespräch bis zum Neustart schreibgesperrt halten
- Vorteile: kein weiterer Satz kann einen Rest treffen; einfach zu begründen.
- Nachteile: ein vorübergehender Fehler (kurz volle Platte) legt den Channel dauerhaft still; der Rest in der Datei bleibt trotzdem.

## Entscheidung

**Gewählt: Option A**

Sie hält die Invariante „Log, Index und Live zeigen dieselben Nachrichten“ auch im Fehlerfall und kostet im Normalfall nichts. Regeln für die Logablage ([A00008](../Architektur/A00008-logablage.md)):

- `anhaengen` merkt die Dateilänge vor dem Schreiben; scheitert `write_all`, schneidet es auf diese Länge zurück und meldet den Fehler. Gespräche ([A00006](../Architektur/A00006-gespraeche.md)) ergänzt Index und Live nur nach Erfolg; die vergebene Zeit verfällt (Zeiten sind streng steigend, nicht lückenlos).
- **Zeilenende-Wächter**: beim Öffnen einer Tagesdatei zum Anhängen — und nach einem gescheiterten Rückbau — prüft die Logablage das letzte Byte; ist es nicht `\n`, beginnt der nächste Schreibvorgang mit `\n`. Damit beginnt jeder neue Satz in Spalte 1, auch nach einem Absturz.
- Scheitert der Rückbau selbst, wird das auf stderr gemeldet (Datei, Länge); der Rest bleibt als defekte bzw. verkürzte Zeile stehen und erscheint beim nächsten Laden im Health-Check (`defekte_zeilen`).

In Kauf genommen: ein Rest nach doppeltem Fehler (Schreiben **und** Rückbau scheitern) und der verkürzte letzte Satz nach Stromausfall bleiben möglich — beide sind sichtbar, keiner zerstört einen Nachbarn.

## Konsequenzen

**Positiv:**
- Eine Nachricht, deren Senden mit Erfolg beantwortet wurde, ist nach einem Neustart genau so da; eine mit Fehler beantwortete ist nirgends.
- Der Zeilenende-Wächter macht auch von Hand gekürzte Dateien wieder beschreibbar.

**Negativ / Inkauf zu nehmen:**
- Die Logablage kennt ein Stück Format (Satzbeginn in Spalte 1, `\n` als Zeilenende), obwohl sie Sätze sonst als fertigen Text behandelt.
- Kein `fsync` (A00008): bei Stromausfall gilt weiter, dass die letzten Sekunden fehlen können.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erste Tests in I0002 (Integration gegen `tempfile` im Modul): (1) Datei endet ohne `\n` → nächster Satz beginnt in neuer Zeile, beide Sätze lesbar; (2) erzwungenes Teilschreiben (Speicher-Attrappe der Logablage liefert Fehler nach n Bytes, bzw. unter Unix `RLIMIT_FSIZE` im Kindprozess) → Datei hat danach die alte Länge, `senden` liefert Fehler, `block` enthält die Nachricht nicht, der nächste Satz ist heil.

## Quellen / Bezuege

- [A00008 Logablage](../Architektur/A00008-logablage.md) (primär), [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00007 Logformat](../Architektur/A00007-logformat.md); Aspekt [X00004 Fehlerbehandlung](../Architektur/Aspekte/X00004-fehlerbehandlung.md)
- ADR-0004 (Absturz-Fall, Defekt-Regel), ADR-0005 (Reihenfolge Log → Index → Live), ADR-0006 (Index als `push_str` auf den Rohtext)
- Vision, Zielbild „Das Log ist das Gedächtnis“; WBS I0002

## Ergänzungen

- **2026-10-28 (Vorlauf „Betriebsprotokoll“) — stderr wird Betriebsprotokoll:** „Scheitert der Rückbau … auf stderr gemeldet“: `ERROR senden` mit `datei`, `grund` und Länge ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regeln 2 und 3; Format [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md), stderr spiegelt ab Warning). Die übrigen Regeln dieser ADR gelten unverändert.
