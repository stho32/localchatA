# ADR-0028: Nachrichten live als drittes Thema im Ereignisstrom — Ordnung über die Nachrichtenzeit je Channel, kein Stand im Strom, Lücken über die Verlauf-API

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #7 I0008 — Vorbereitung, gebaut wird mit I0009)
**Kontext-Anforderung**: — (WBS I0009; vorbereitet in I0008; prägt I0012, I0013, I0016)
**Erprobt durch**: — (überträgt die gebaute Mechanik: Veröffentlichung unter der Sperre, die die Ordnungszahl vergibt — `src/anwesenheit/bestand.rs`, `src/gespraeche/mod.rs` `channel_anlegen`; streng steigende Nachrichtenzeit unter einer Sperre — `Gespraeche::zeit_vergeben`; ADR-0016, ADR-0023)

## Kontext

- Vision „Live überall“: eine neue Nachricht erscheint sofort in allen offenen Sichten; Q3 in [A00001](../Architektur/A00001-localchata.md): < 1 s im LAN. Fertig-Kriterium I0009: „Nachricht erscheint im Channel bei allen offenen Sichten sofort“.
- [ADR-0005](0005-programm-als-einziger-schreiber-und-reihenfolge.md): je Nachricht unter der Sperre des Gesprächs Zeit vergeben → anhängen → Index → **Ereignis veröffentlichen** → Sperre frei. Die Zeit ist global streng steigend und überlebt Neustarts.
- [ADR-0016](0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md) (Ergänzungen aus #4) und [ADR-0023](0023-channels-als-zweites-thema-im-ereignisstrom.md): je Thema eine Ordnungszahl, vergeben und veröffentlicht unter **derselben** Sperre; der Strom beginnt mit dem Stand jedes Themas; kein gemeinsamer Zähler über Themen (Option B in ADR-0023 verworfen, Befund B1 aus #4); „jede künftige Ereignisart nennt ihr Thema und dessen Nummer“ ([A00011](../Architektur/A00011-live-kanal.md), Ergänzung #6).
- [ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md): ein Strom je Browser; der Worker reicht Ereignisse an alle Tabs; mehrere Tabs zeigen verschiedene Channels ([ADR-0027](0027-channel-waehlen-adresse-als-hash-zuletzt-gewaehlt-je-tab.md)).
- [ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md): der Verlauf hat keinen Stand, sondern Blöcke mit der Zeit als Cursor (`nach=<zeit>`); [ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md): der Tab hält ein Fenster aus höchstens fünf Blöcken.
- Designsystem `VerlaufsMarken`: nicht am Ende → nie scrollen, Pille „n neue Nachrichten“; am Ende → der Verlauf folgt. Verbindung wieder da → „Wieder verbunden · n nachgeladen“.

## Optionen

### Option A: Thema `nachricht`, Ordnung über die Nachrichtenzeit je Channel; kein Stand im Strom; Lücken holt der Empfänger über `nach=<zeit>`
- Vorteile: kein neuer Zähler — die Zeit ist schon Ordnung, ID und Cursor; sie überlebt Neustarts (anders als `stand`, der bei 0 beginnt); veröffentlicht unter der Gesprächssperre ist die Reihenfolge je Channel auf dem Bus die der Zeit; der Stand eines Channels ist sein Verlauf, und den holt der Tab ohnehin über die API; derselbe Cursor dient dem Nachholen nach einem Abriss und dem Warten der Agenten (ADR-0007 `seit`).
- Nachteile: der Empfänger muss Block und Ereignisse an der Zeit zusammenführen (eine reine Funktion); über Channels hinweg kann die Busreihenfolge von der Zeit abweichen (je Channel eine Sperre) — ohne Folgen, weil nur je Channel verglichen wird.

### Option B: Eigener `stand` je Channel (Zähler unter der Gesprächssperre), Stand-plus-Wechsel wie bei Channels
- Vorteile: dieselbe Form wie Teilnehmer und Channels.
- Nachteile: zweites Ordnungsmerkmal neben der Zeit; beginnt nach einem Neustart bei 0 (der Tab muss den Rücksprung erkennen); ein „Stand“ eines Channels wäre sein ganzer Verlauf — beim Öffnen des Stroms nicht sendbar, also doch über die API.

### Option C: Ein gemeinsamer `stand` über alle Channels
- Vorteile: ein Vergleich je Ereignis.
- Nachteile: braucht eine Sperre über alle Gespräche — hebt die Sperre je Gespräch (ADR-0005) auf oder lässt Nummern ungeordnet auf den Bus (Befund B1 aus #4); in ADR-0023 schon verworfen.

### Option D: Nur ein Hinweis `{art:"neu", channel, zeit}`; der Tab holt den Inhalt per API
- Vorteile: kleiner Strom; der Inhalt kommt immer aus einer Quelle.
- Nachteile: je Nachricht und offenem Tab ein zusätzlicher Abruf — bei drei schreibenden Agenten und zehn Tabs dreißig Abrufe je Takt; zusätzliche Latenz gegen Q3.

### Option E: Eigener Strom je geöffnetem Channel
- Vorteile: der Tab bekommt nur, was er zeigt.
- Nachteile: bricht ADR-0015 (ein Strom je Browser, 6-Verbindungen-Grenze).

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass die Nachrichtenzeit die Rolle der Ordnungszahl schon trägt — unter der Sperre vergeben und unter derselben Sperre veröffentlicht, wie es die Regel aus ADR-0016/0023 verlangt — und Verlauf, Live und Nachholen eine einzige Vergleichsregel teilen. In Kauf genommen: das Thema weicht in der Form von Teilnehmern und Channels ab (kein Stand beim Öffnen), und der Tab führt Block und Ereignisse selbst zusammen.

Regeln (gebaut mit I0009):

1. **Thema und Nummer**: Thema `nachricht`, Ordnungsmerkmal die Nachrichtenzeit, verglichen **je Channel**. Bus: `Ereignis::Nachricht { channel, nachricht }` neben `Anwesenheit` und `Channel`.
2. **Veröffentlichen unter der Gesprächssperre**: `senden` ruft den Rückruf nach Log und Index, noch unter der Sperre des Gesprächs (ADR-0005; Rückruf wie `Gespraeche::mit_veroeffentlichen`, Gespräche kennt den Bus nicht). Scheitert das Anhängen, gibt es kein Ereignis (ADR-0011).
3. **Form**: `{"art":"nachricht","channel":"<name>","nachricht":{zeit, von, text}}` — die Nachricht in genau der Form der Verlauf-API (ADR-0025 Regel 3), mit denselben späteren Zusatzfeldern.
4. **Kein Stand, kein Filter im Strom**: beim Öffnen des Stroms kommt für dieses Thema nichts; der Strom filtert Nachrichten nicht nach Zeit. Jeder Browser erhält die Nachrichten **aller** Channels (alle sehen alle; spätere Zähler für Ungelesen brauchen sie). Direktgespräche (I0012) gehen nur an die Beteiligten (Filter „sichtbar für“, A00011). `NeuerStand` (`Lagged`) sendet weiterhin die Stände von Teilnehmern und Channels; für Nachrichten ist er das Signal „Lücke möglich“.
5. **Zusammenführen im Tab** (reine Funktion): der Tab hört die Ereignisse schon, bevor er einen Channel öffnet, und puffert die Nachrichten dieses Channels; nach dem Block übernimmt er nur Nachrichten mit `zeit` **größer** als die jüngste bekannte Zeit des Channels; eine Nachricht mit kleiner oder gleicher Zeit ist schon da und wird verworfen.
6. **Lücken**: nach einem Wiederverbinden des Stroms und nach `NeuerStand` holt der Tab für den offenen Channel `nach=<jüngste bekannte Zeit>`; meldet die Antwort `neuer: true`, öffnet er den Channel neu wie „Zum Neuesten“ (ADR-0026 Regel 6) und zeigt die Pille. Der Worker merkt sich keine Nachrichten — ein neuer Tab öffnet über die API.
7. **Fenster und Blick** (ADR-0026, Designsystem): steht der neueste Block im Fenster, wird angefügt; steht der Blick am Ende, folgt er, sonst zählt die Pille „n neue Nachrichten“ und nichts scrollt. Ist der neueste Block abgeworfen (`neuer`), wird nur gezählt. Der jüngste Block wächst bis 100, dann beginnt ein neuer; über fünf Blöcke fällt der älteste mit Anker-Korrektur (ADR-0026 Regel 3).
8. **Agenten**: wer `curl -N` auf `/api/ereignisse` liest, sieht `nachricht` wie der Browser; `warten` (I0016) holt zuerst aus dem Index nach `seit` nach und wartet dann auf den Bus (A00011) — die Busreihenfolge über Channels spielt dafür keine Rolle.

## Konsequenzen

**Positiv:**
- Kein neuer Zähler, kein neuer Stand, kein Sperrpaar; die Korrektheit hängt je Channel an der einen Sperre, die ohnehin die Zeit vergibt.
- Ein Agent, der eine Nachricht mit Zeit `t` gelesen hat, setzt nach jedem Abriss und Neustart mit `nach=t` fort — dieselbe Regel wie der Browser.

**Negativ / Inkauf zu nehmen:**
- Jeder Browser bekommt alle Nachrichten aller Channels über den Strom, auch die, die er gerade nicht zeigt (Größenordnung: ein Satz ≈ 0,5 KB; bei 10 Nachrichten/s ≈ 5 KB/s je Browser).
- Das Thema hat eine andere Form als Teilnehmer und Channels; wer den Strom liest, muss das wissen (Anleitung I0019).

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … eine Nachricht, die zwischen Block und erstem Ereignis gesendet wurde, fehlt oder doppelt steht. Früh-Indikator: E2E „senden, während der Channel öffnet“ mit verzögerter Verlauf-Route. Gegenmaßnahme: Regel 5 (erst hören, dann Block, Zeitvergleich), Node-Test zuerst.
2. … ein Tab nach dem Ruhezustand des Rechners stundenlang veraltet ist. Früh-Indikator: Verbindung wieder da, aber keine nachgeladenen Nachrichten. Gegenmaßnahme: Regel 6 (nach jedem Wiederverbinden `nach=`), Anzeige „Wieder verbunden · n nachgeladen“.
3. … ein Agent im Takt schreibt und alle Tabs mit Zeichnen beschäftigt. Früh-Indikator: M1 aus ADR-0026 unter Last (Messung mit I0009). Gegenmaßnahme: Fenster-Grenze und Zählen statt Zeichnen, wenn der neueste Block nicht im Fenster steht (Regel 7); Bündeln der Zeichenarbeit je Frame ist ein Umbau im Tab, keiner der Architektur.

**Folgeentscheidungen / Aufgaben:**
- [ ] I0008 baut davon nichts; es reicht, dass der Tab je offenem Channel die jüngste bekannte Zeit führt (Fenster, ADR-0026).
- [ ] Erster Unit-Test in I0009 (Gespräche, Speicherablage, aufzeichnender Rückruf): drei Nachrichten in denselben Channel → Rückruf erhält sie in Zeitfolge, jeweils noch unter der Sperre des Gesprächs (Rückruf versucht `try_lock` → besetzt); Anhängen scheitert → kein Rückruf.
- [ ] Erster Node-Test in I0009: Block mit Zeiten 1–100, gepufferte Ereignisse 99, 101, 100, 102 → Ergebnis 1–102 ohne Doppel.
- [ ] Erster Integrationstest in I0009: 20 Tasks senden je 50 Nachrichten in denselben Channel, ein Strom liest mit → Zeiten im Strom streng steigend, gleich der Folge im Log (Folgeaufgabe aus ADR-0005).

## Quellen / Bezuege

- [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md)
- ADR-0005, ADR-0007, ADR-0011, ADR-0015, ADR-0016 (samt Ergänzungen #4), ADR-0023, ADR-0025, ADR-0026, ADR-0027
- Vision R00000 „Live überall“, „Agenten können warten und reagieren“; WBS I0009, I0016; Designsystem `components/VerlaufsMarken/README.md`

## Ergänzungen

- **2026-10-28 (Karte #8 I0009) — Berichtigung zu Regel 8:** Die Annahme „die Busreihenfolge über Channels spielt für `warten` keine Rolle“ trägt nicht: mit einer Sperre je Gespräch kann Channel B eine jüngere Zeit veröffentlichen, bevor Channel A seine ältere angehängt hat; ein Agent, der danach mit `seit` = Zeit von B wartet, verliert die Nachricht aus A. Abhilfe: Veröffentlichen unter einer Schreibsperre über alle Gespräche ([ADR-0035](0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md)) — Regel 2 gilt damit verschärft; die Regeln 1, 3–7 bleiben. Die Gestalt im Tab (vorläufig, Bündeln je Frame, Pille, Lücken vor gepufferten eigenen) steht in [ADR-0037](0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md); die Nachricht trägt zusätzlich `art` (ADR-0034).
- **2026-10-29 (Karte #9 I0010, Anhänge; [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) akzeptiert) — Feld `anhaenge` im Thema `nachricht`:** Das Ereignis trägt dieselbe Liste `anhaenge` wie der Verlauf ([ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md), Ergänzung Karte #9), immer eine Liste; `text` darf leer sein, wenn Anhänge dabei sind (B5). Veröffentlicht wird nach dem Binden der Dateien und dem Anhängen des Satzes, unter derselben Schreibsperre wie bisher ([ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 6) — ein Leser bekommt nie eine Anhang-ID, deren Datei noch im Eingang liegt. Regeln 1–8 gelten unverändert.
- **2026-10-29 (Karte #48, I0032/I0033; [ADR-0048](0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) akzeptiert):** Neues Thema `aenderung` mit der ganzen Nachricht in neuer Fassung; Ausnahme zu Regel 5 nur für dieses Thema: übernommen wird über `geaendert`, nicht über `zeit`; Nachholen nach Regel 6 lädt die Blöcke des Fensters neu und übernimmt neuere Fassungen ([ADR-0048](0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) Regel 11). Regeln 1–8 gelten sonst unverändert.
- **2026-10-30 (Karte #77):** Ein Ende der Aktivität, das eine Nachricht auslöst, kommt auf dem Signalbus **nach** dem Ereignis `nachricht` auf dem Ereignisbus; über zwei Busse ist die Reihenfolge nicht garantiert, der Strom wählt den Ereignisbus zuerst (`biased`), und der Tab blendet die Anzeige des Absenders bei Ankunft seiner `nachricht` selbst aus. Die Regeln 1–8 bleiben. Quelle: [ADR-0051](0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) Regeln 6, 7 b, c.
