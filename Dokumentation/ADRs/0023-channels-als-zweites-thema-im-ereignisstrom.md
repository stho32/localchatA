# ADR-0023: Channels als zweites Thema im vorhandenen Ereignisstrom — eigener `stand`, veröffentlicht unter der Sperre des Verzeichnisses

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #6 I0007)
**Kontext-Anforderung**: — (WBS I0007; prägt I0018 und alle späteren Ereignisarten)
**Erprobt durch**: — (überträgt die gebaute und in #4 nachgearbeitete Mechanik der Anwesenheit: `stand` unter der Sperre, Veröffentlichung unter der Sperre, `stand_mit`, Filter `zuletzt` im Strom — `src/anwesenheit/bestand.rs:219`, `:416`, `src/http/ereignisse.rs`)

## Kontext

- Fertig-Kriterium I0007: „neuer Channel erscheint in der Liste aller Teilnehmer“; Vision „Live überall“; `design-prompt.md` 4.3 „erscheint sofort bei allen Teilnehmern“.
- ADR-0015: genau **ein** Strom je Browser (`SharedWorker`), Token nur im Header; der Worker merkt sich den jüngsten Stand und gibt ihn jedem neuen Tab.
- ADR-0016 Regel 3/5 und Ergänzung aus #4 (Code-Review B1/B2): ein Wechsel trägt eine Nummer `stand`, die unter der Sperre des Bestands wächst; veröffentlicht wird **unter derselben Sperre**, damit die Reihenfolge auf dem Bus die von `stand` ist. Der Strom verwirft Wechsel mit `stand ≤ zuletzt`. Der Befund in #4 entstand genau dort, wo Nummer und Veröffentlichung auseinanderliefen.
- Bus (`src/live/bus.rs`): `Ereignis` hat heute nur `Anwesenheit(Wechsel)`; `Lagged` wird zu `NeuerStand`. A00011 plant schon die Ereignisart `Channel`.
- Oberfläche: `zusammenfuehren` (`web/teilnehmerliste.js`) übergeht unbekannte `art`; `standAus` im Worker (`web/verteiler.js:116`) kennt nur die Teilnehmer.

## Optionen

### Option A: Zweites Thema im selben Strom, eigener `stand` je Thema
- Strom beginnt mit `{"art":"teilnehmer",…}`, dann `{"art":"channels","stand","liste"}`; danach Wechsel beider Themen, je Thema gefiltert über ein eigenes `zuletzt`.
- Vorteile: ein Strom je Browser bleibt; jedes Thema hält seine Reihenfolge mit seiner eigenen Sperre — kein gemeinsamer Zähler, keine Sperrordnung zwischen Anwesenheit und Gespräche; dieselbe Zusammenführungsregel wie bei Teilnehmern, im Browser getrennt testbar.
- Nachteile: der Strom führt zwei Zähler; jede künftige Ereignisart muss sagen, zu welchem Thema sie gehört.

### Option B: Ein gemeinsamer `stand` über alle Themen
- Vorteile: ein Zähler, eine Vergleichsregel im Strom und im Browser.
- Nachteile: den Zähler müssten Anwesenheit und Channelverzeichnis gemeinsam halten — entweder eine gemeinsame Sperre (koppelt zwei Bausteine, die nichts miteinander zu tun haben, und verlängert die Sperre der Anwesenheit um Plattenarbeit) oder Vergabe ohne Sperre des Bestands, dann kann Nummer 6 vor Nummer 5 auf dem Bus liegen und der Filter `zuletzt` verwirft 5 — genau Befund B1 aus #4.

### Option C: Kein Live-Ereignis; Browser und Agenten fragen `GET /api/channels` ab (beim Fokus oder alle n Sekunden)
- Vorteile: kein Eingriff in Strom und Worker.
- Nachteile: „erscheint sofort bei allen“ hängt am Abfragetakt; zusätzliche Last je Tab; widerspricht „Live überall“.

### Option D: Eigener Strom `/api/channels/ereignisse`
- Vorteile: Themen sauber getrennt.
- Nachteile: zwei Verbindungen je Browser — bricht ADR-0015 (6-Verbindungen-Grenze im Rückfall, ein Halter je Sitzung).

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass die bewährte Mechanik unverändert je Thema gilt und kein Baustein die Sperre eines anderen braucht. In Kauf genommen: zwei Zähler im Strom und im Worker.

Regeln:

1. **Bestand und Nummer**: das [Channelverzeichnis](../Architektur/A00012-channelverzeichnis.md) führt `stand: u64`, unter seiner Sperre mit jedem Anlegen (ab I0018 auch jeder Änderung) hochgezählt. Neustart beginnt bei 0 — wie die Anwesenheit; der Browser ersetzt beim Stand die ganze Liste, auch mit kleinerer Nummer.
2. **Veröffentlichen unter der Sperre**: `anlegen` ruft `veroeffentlichen(ChannelWechsel)` noch unter der Sperre des Verzeichnisses (Rückruf wie `Anwesenheit::mit_veroeffentlichen`; Gespräche kennt den Bus nicht). Bus: `Ereignis::Channel(ChannelWechsel)` neben `Anwesenheit`.
3. **Formen an der Grenze**:
   - Stand: `{"art":"channels","stand":n,"liste":[{name, angelegt, angelegt_von, beschreibung}]}`, nach Name sortiert; unbekannte Werte `null`.
   - Wechsel: `{"art":"channel","stand":n,"channel":{…}}` — Einfügen oder Ersetzen nach `name` (I0007 kennt nur Einfügen).
   - `GET /api/channels` → `{stand, liste}` (ADR-0021 Regel 2).
4. **Strom öffnen** (`src/http/ereignisse.rs`): wie bisher unter der Sperre der Anwesenheit abonnieren und den Teilnehmer-Stand holen (`stand_mit`); **danach** den Channel-Stand unter der Sperre des Verzeichnisses holen. Erstes Ereignis `teilnehmer`, zweites `channels`. Korrekt, weil jeder Channel-Wechsel nach dem Abonnieren im Abonnement liegt und jeder vor dem Channel-Stand im Stand steckt; doppelte filtert Regel 5. Keine verschachtelten Sperren, also keine Sperrordnung.
5. **Filter je Thema**: der Strom führt `zuletzt_teilnehmer` und `zuletzt_channels`; ein Wechsel wird nur gesendet, wenn sein `stand` größer ist als der seines Themas. `NeuerStand` (`Lagged`) sendet **beide** Stände neu.
6. **Browser**: eine reine Funktion `channelsZusammenfuehren(liste, ereignis)` — Stand ersetzt, Wechsel mit `stand > liste.stand` fügt ein bzw. ersetzt, sonst unverändert; unbekannte `art` lässt die Liste stehen. Der Worker merkt sich beide Listen und gibt jedem neuen Tab beide Stände (Erweiterung von `standAus`, ADR-0015 Regel 2). Die Worker-URL trägt die Programmversion (ADR-0015 Regel 3) — ein alter Worker ohne Channel-Thema läuft nach dem Update nicht weiter.
7. **Agenten**: `curl -N` auf `/api/ereignisse` sieht `channels` und `channel` wie der Browser; wer nur abfragt, nimmt `GET /api/channels`. Für den späteren Warte-Aufruf (I0016) sind Channel-Wechsel kein Weckgrund.

## Konsequenzen

**Positiv:**
- Neue Themen (später Frist, Ungelesen-Zähler) folgen demselben Muster, ohne die Anwesenheit anzufassen.
- Die Korrektheit des Stroms hängt je Thema an genau einer Sperre — prüfbar mit denselben Testformen wie in #4.

**Negativ / Inkauf zu nehmen:**
- Strom, Worker und Testhilfen führen zwei Zähler. Konkret im Bestand: `tests/gemeinsam/strom.rs` `staende()` sammelt `stand` aus **allen** Ereignissen und `tests/ereignisse.rs:27` prüft sie als streng steigend — mit dem Channel-Stand (bei leerer Liste `stand` 0) an zweiter Stelle wird das rot, obwohl der Strom richtig ist. Die Testhilfe filtert künftig nach Thema (`art` `teilnehmer`/`anwesenheit` bzw. `channels`/`channel`); das ist Teil des ersten Slices, nicht Nacharbeit.
- Jede (Wieder-)Verbindung überträgt die ganze Channel-Liste (ADR-0021: höchstens etwa 256 KB).

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein neuer Tab keine Channels zeigt, bis jemand einen anlegt. Früh-Indikator: Seitenleiste leer im zweiten Tab. Gegenmaßnahme: Regel 6 (Worker gibt beide Stände), E2E „zweiter Tab sieht die Liste ohne neue Anlage“.
2. … ein Channel-Wechsel, der zwischen Abonnieren und Channel-Stand entsteht, doppelt oder gar nicht ankommt. Früh-Indikator: Integrationstest mit gleichzeitigem Anlegen und Strom-Öffnen. Gegenmaßnahme: Regel 4/5, Test zuerst (unten).
3. … `anlegen` die Sperre des Verzeichnisses für Plattenarbeit hält und Senden (I0009) darauf wartet. Früh-Indikator: Sendezeit steigt beim Anlegen. Gegenmaßnahme: Senden nimmt die Verzeichnissperre nur lesend und kurz (Verlauf nachschlagen), die Plattenarbeit des Anlegens ist ein `mkdir` und ein `rename` — hingenommen, gemessen mit I0009.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (Verzeichnis, Speicher-Ablage, aufzeichnender Rückruf): drei Anlagen → Rückruf erhält `stand` 1, 2, 3 in dieser Reihenfolge, jeweils noch unter der Sperre (Rückruf versucht `try_lock` auf das Verzeichnis → besetzt); `stand()` liefert danach 3 und drei Einträge sortiert.
- [ ] Erster Integrationstest (echtes Programm): A öffnet den Strom → Ereignis 1 `teilnehmer`, Ereignis 2 `channels` mit leerer Liste; B legt `allgemein` an → A erhält `channel` mit `stand` 1 in < 2 s ab der 201; 30 Anlagen parallel zu 10 Strom-Öffnungen → jeder Strom kennt am Ende genau 30 Channels, jede `stand`-Nummer je Strom höchstens einmal und streng steigend.
- [ ] Erster Node-Test (Oberfläche): Stand 5 ersetzt, Wechsel 4 verworfen, Wechsel 6 eingefügt in Namensreihenfolge, Stand 2 nach Neustart ersetzt wieder; unbekannte `art` → unverändert; Worker gibt neuem Tab beide Stände.

## Quellen / Bezuege

- [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00012 Channelverzeichnis](../Architektur/A00012-channelverzeichnis.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md)
- ADR-0007, ADR-0015, ADR-0016 (samt Ergänzung #4), ADR-0021
- Logbuch #4 (Code-Review B1 „stand nicht monoton“, B2 „Reihenfolge ohne Test“)
