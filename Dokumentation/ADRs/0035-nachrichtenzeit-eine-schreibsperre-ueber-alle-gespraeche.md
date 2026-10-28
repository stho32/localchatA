# ADR-0035: Nachrichtenzeit und Ordnung — Zeit vergeben, anhängen, eintragen und veröffentlichen unter einer Schreibsperre über alle Gespräche; Vorlauf der Zeit sichtbar machen

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #8 I0009)
**Kontext-Anforderung**: — (WBS I0009; prägt I0012, I0015, I0016, I0019)
**Erprobt durch**: — (begründet am Code, nicht geprobt: `Gespraeche::zeit_vergeben` gibt die Sperre `letzte` frei, bevor `senden` anhängt und den Index ergänzt — `src/gespraeche/mod.rs`, `senden` und `zeit_vergeben`; die gebaute Regel `max(jetzt, letzte + 1 ms)` samt Test `zwei_nachrichten_in_derselben_millisekunde_folgen_mit_einer_millisekunde_abstand` und `die_tagesdatei_folgt_dem_datum_der_vergebenen_zeit`)

Ergänzt [ADR-0005](0005-programm-als-einziger-schreiber-und-reihenfolge.md) (Vorgeschlagen) und berichtigt eine Annahme in [ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md) Regel 8.

## Kontext

- [ADR-0005](0005-programm-als-einziger-schreiber-und-reihenfolge.md): die Nachrichtenzeit ist ID und Wartezeiger; `max(jetzt, letzte + 1 ms)` **global** über alle Gespräche, beim Start aus dem jüngsten Satz fortgesetzt; eine Sperre **je Gespräch**. Gebaut (I0002, R00002): gleiche Millisekunde → +1 ms; Uhr zurück → die Zeiten laufen vor, bis die Uhr aufholt; die Tagesdatei folgt dem lokalen Datum der **vergebenen** Zeit.
- [ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md): Veröffentlichen unter der Gesprächssperre; Ordnung je Channel über die Zeit; Regel 8: „`warten` (I0016) holt zuerst aus dem Index nach `seit` nach und wartet dann auf den Bus — die Busreihenfolge über Channels spielt dafür keine Rolle.“
- Vision: „Agenten können warten und reagieren“; WBS I0016: ein Agent wartet auf Neues **für sich** — Channel, Direktnachricht, Erwähnung, also **über Gespräche hinweg** mit **einem** Zeiger `seit`.
- Lehre #4 (Auftrag): Reihenfolge Abonnieren und Stand ohne Verlust oder Doppel; die Zeit als Ordnung — gleiche Millisekunde, Uhr springt zurück.
- Die Schreibkosten je Nachricht: ein `write_all` an eine offene Datei im Anhängemodus ohne `fsync` (A00008) und ein `push_str` in den Index (ADR-0006) — Mikrosekunden; die erwartete Last ist ein LAN-Chat mit einigen Menschen und Agenten, begrenzt durch das Schreibkonto ([ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md)).

**Die Lücke in ADR-0028 Regel 8.** Mit einer Sperre je Gespräch vergibt `senden` in Channel A die Zeit `t1`, gibt die Zeitsperre frei und schreibt dann; `senden` in Channel B vergibt `t2 > t1` und kann **vor** A anhängen, eintragen und veröffentlichen. Ein Agent, der in diesem Moment `warten(seit = t0)` ruft, erhält `t2`, setzt `seit = t2` und wartet erneut — der Index enthält `t1` nun zwar, aber `t1 < seit`: die Nachricht aus A ist für ihn **verloren**. Für den Browser (Vergleich je Channel) gilt das nicht, für jeden Zeiger über mehrere Gespräche schon.

## Optionen

### Option A: Wie gebaut — globale Zeit, Sperre je Gespräch; `warten` bekommt je Gespräch einen eigenen Zeiger
- Vorteile: Schreiben in verschiedene Channels läuft parallel; nichts am Senden ändert sich.
- Nachteile: der Wartezeiger eines Agenten wird eine Tabelle `{gespraech → zeit}` statt einer Zeit — jede Erwähnung in einem neuen Channel braucht einen neuen Eintrag; die schöne Eigenschaft „eine Zeit sagt, was ich gesehen habe“ (ADR-0005, ADR-0025 Konsequenzen) gilt nur noch je Channel; jeder Leser über Gespräche hinweg (Suche I0011, Erwähnungen I0013) muss die Regel kennen.

### Option B: Eine Schreibsperre über alle Gespräche: Zeit vergeben → anhängen → Index → veröffentlichen, dann frei
- Vorteile: die Reihenfolge auf dem Bus, im Index aller Gespräche zusammen und die Zeitfolge sind **eine** Folge — ein einziger Zeiger `seit` ist verlustfrei, auch über Gespräche; die Gesprächssperre bleibt für Leser (Block kopieren) unverändert; einfachste Regel für alle späteren Leser (I0011, I0013, I0016).
- Nachteile: Schreiben ist über alle Channels seriell; hängt ein Plattenschreiben (volle Platte, Netzlaufwerk), warten alle Sender, nicht nur die eines Channels.

### Option C: Wasserstand — Sperre je Gespräch bleibt; Gespräche führt die Menge der vergebenen, noch nicht veröffentlichten Zeiten; Leser über Gespräche liefern nur bis „kleinste offene Zeit − 1 ms“
- Vorteile: parallel **und** verlustfrei.
- Nachteile: zweiter Zustand (offene Zeiten) mit eigenen Fehlerpfaden (Schreibfehler muss die Zeit austragen, sonst steht der Wasserstand); Ereignisse oberhalb des Wasserstands müssen zurückgehalten werden — Mechanik für einen Gewinn (paralleles Schreiben), den die Last nicht braucht.

### Option D: Hybride logische Uhr / Zähler je Gespräch statt Wanduhrzeit
- Vorteile: unabhängig von Sprüngen der Systemuhr.
- Nachteile: die Zeit ist im Log, in der API und für Menschen auch Uhrzeit — ein zweites Ordnungsmerkmal bricht ADR-0025 und ADR-0028 (Option B dort schon verworfen).

## Entscheidung

**Gewählt: Option B**

Es überwiegt, dass ein Agent mit **einer** Zeit verlustfrei über alle Gespräche warten kann und jede künftige Leseart (Warten, Erwähnungen, Suche) dieselbe Regel hat wie Verlauf und Live; das Schreiben, das dafür seriell wird, kostet je Nachricht Mikrosekunden. In Kauf genommen: ein hängendes Plattenschreiben hält alle Channels an, nicht nur einen — das Programm ist dann ohnehin in einem Fehlerzustand, den ADR-0011 meldet.

Regeln (gebaut mit I0009):

1. **Schreibsperre**: `Gespraeche` hält eine Sperre `schreiben` über alle Gespräche (sie ersetzt die heutige Sperre `letzte` und hält die zuletzt vergebene Zeit). `senden` nimmt zuerst die Gesprächssperre, dann `schreiben`, und hält beide über: Wiederholschlüssel prüfen ([ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) Regel 6) → Zeit vergeben → Satz anhängen → Index → Rückruf `veroeffentlichen` → beide frei. **Sperrordnung**: Verzeichnis (lesend, nur zum Nachschlagen, vorher wieder frei) → Gespräch → `schreiben`; nichts nimmt `schreiben` und danach eine Gesprächssperre. Direktgespräche (I0012) nutzen dieselbe Sperre.
2. **Zeit**: `max(jetzt auf ms, letzte + 1 ms)` wie gebaut, verglichen als Zeitpunkt (X00007); der Versatz ist der von `jetzt`. Damit ist jede Zeit im Ordner eindeutig — auch über Channels — und streng steigend in der Folge, in der Log, Index und Bus sie sehen.
3. **Gleiche Millisekunde**: die zweite Nachricht erhält +1 ms (gebaut). **Uhr zurück**: die Zeiten laufen vor, bis die Uhr aufholt; nichts wird umgeordnet (gebaut, ADR-0005 Konsequenzen).
4. **Vorlauf sichtbar**: liegt die vergebene Zeit mehr als **60 s** vor `jetzt`, schreibt `senden` **einmal je Vorlauf-Episode** eine Zeile auf stderr — `Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt.` —, und `/api/gesundheit` nennt `zeit_vorlauf_ms` (0, wenn kein Vorlauf). Eine Episode endet, sobald `jetzt` die letzte Zeit wieder einholt. Die letzte Zeit liest der Health-Check aus einem eigenen Atomwert (Millisekunden), den `senden` unter `schreiben` setzt — nie über die Schreibsperre, damit ein hängendes Plattenschreiben den Health-Check nicht mit anhält.
5. **Tageswechsel**: die Tagesdatei ist das lokale Datum der vergebenen Zeit in ihrem Versatz (gebaut). Läuft die Zeit vor, landet eine Nachricht kurz vor Mitternacht der Uhr schon in der Datei des nächsten Tages — Datei und Zeit stimmen immer überein. Die Frist rechnet `heute` aus der Uhr ([ADR-0029](0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md)); eine Datei „von morgen“ entfällt nie zu früh. Beim Wechsel der Sommerzeit wechselt der Versatz innerhalb eines Tages; geordnet wird nach Zeitpunkt, die Anzeige `HH:MM` kann dabei einmal rückwärts springen (hingenommen, der Tooltip trägt Datum und Versatz).
6. **Kein Zeitstempel vom Client**: weder Körper noch Kopf können eine Zeit setzen; die Zeit entsteht nur hier.

## Konsequenzen

**Positiv:**
- `warten(seit)`, Erwähnungen und Direktnachrichten brauchen später keinen Zeiger je Gespräch; ein Agent, der die Zeit `t` gesehen hat, hat **alles** bis `t` gesehen — Gespräch egal, Neustart egal.
- Der Integrationstest „20 Tasks × 50 Nachrichten“ (ADR-0005, ADR-0028) gilt jetzt auch über zwei Channels gemischt.

**Negativ / Inkauf zu nehmen:**
- Kein paralleles Schreiben; Obergrenze ist der Durchsatz eines Kerns für `write_all` + Index + `broadcast::send` (Größenordnung zehntausende Nachrichten je Sekunde, gerechnet aus ADR-0006 — nicht gemessen), weit über dem Schreibkonto.
- Eine einmal weit vorgestellte Uhr (etwa ein Jahr in der Zukunft) vergiftet den Ordner: auch nach Korrektur und Neustart setzen die Zeiten am jüngsten Satz fort. Regel 4 macht das sichtbar; reparieren lässt es sich nur von Hand (Programm stoppen, Sätze aus der Zukunft entfernen).

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein Agent nach `warten` Erwähnungen aus einem zweiten Channel verpasst. Früh-Indikator: Unit-Test mit einer Speicherablage, die für Channel A beim Anhängen wartet. Gegenmaßnahme: Regel 1; der Test unten muss vor dem Umbau rot sein.
2. … ein Deadlock zwischen Pflege (ADR-0030, Gesprächssperre) und Senden entsteht. Früh-Indikator: hängender Test mit Pflege und Senden gleichzeitig. Gegenmaßnahme: Sperrordnung Regel 1 — Pflege nimmt `schreiben` nie.
3. … nach einer falsch gestellten Uhr alle Zeiten in der Zukunft stehen und niemand es merkt. Früh-Indikator: `zeit_vorlauf_ms` > 0 im Health-Check. Gegenmaßnahme: Regel 4.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (Gespräche, Speicherablage mit Schranke, aufzeichnender Rückruf): Thread 1 sendet in `a`, die Ablage hält dessen Anhängen an einer Schranke fest; Thread 2 sendet in `b`; Schranke frei → der Rückruf sieht `a` vor `b`, und `zeit(a) < zeit(b)`. (Mit Sperre je Gespräch kommt `b` zuerst — der Test ist vor dem Umbau rot.)
- [ ] Erster Integrationstest (echtes Programm): 20 Tasks senden je 50 Nachrichten abwechselnd in zwei Channels; ein Strom liest mit → alle 1.000 Zeiten im Strom streng steigend, je Channel gleich der Folge im Log (ADR-0005, ADR-0028).
- [ ] Unit-Test Vorlauf (gestellte Uhr): letzte Zeit 2026-10-28T12:00:00, Uhr 11:58:59 → vergebene Zeit 12:00:00.001, eine stderr-Zeile, `zeit_vorlauf_ms` = 61.001; zweite Nachricht → keine zweite Zeile; Uhr 12:00:01 → Vorlauf 0, Episode zu.
- [ ] I0016: `warten` nimmt **einen** Zeiger `seit` (A00011); die Ergänzung zu ADR-0028 hält fest, dass das erst mit dieser ADR verlustfrei ist.

## Quellen / Bezuege

- [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00008 Logablage](../Architektur/A00008-logablage.md); Aspekte [X00007 Zeit](../Architektur/Aspekte/X00007-zeit.md), [X00008 Transaktionen und Nebenläufigkeit](../Architektur/Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)
- ADR-0005, ADR-0006, ADR-0011, ADR-0025, ADR-0028, ADR-0029, ADR-0030, ADR-0034, ADR-0036; `src/gespraeche/mod.rs`, `src/uhr.rs` (`naechste_zeit`)
- Vision R00000 „Agenten können warten und reagieren“; WBS I0009, I0016
