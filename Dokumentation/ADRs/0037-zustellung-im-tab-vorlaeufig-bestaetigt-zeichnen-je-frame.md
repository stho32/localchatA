# ADR-0037: Zustellung im Tab — eigene Nachricht erst vorläufig, dann bestätigt über die Zeit; ein Sendeauftrag zugleich je Tab; fremde Nachrichten je Frame gebündelt angefügt oder gezählt

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #8 I0009)
**Kontext-Anforderung**: — (WBS I0009; prägt I0010, I0012, I0013, I0017)
**Erprobt durch**: — (überträgt Gebautes und Entschiedenes: Thema `nachricht` und Zusammenführen über die Zeit — [ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md); Fenster, Anker, „Zum Neuesten“ — [ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md); Wiederverbinden des Stroms im `SharedWorker` — [ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md), `web/strom.js`, `web/verbindung.js`; einzelne Nachricht live anhängen: 33 ms Median bis nach zwei Frames in jeder Größe bis 64 KiB, Probe zu [ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md))

## Kontext

- Fertig-Kriterium I0009: „Nachricht erscheint im Channel bei allen offenen Sichten sofort“; Q3 in [A00001](../Architektur/A00001-localchata.md): < 1 s im LAN.
- [ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md) legt Thema, Form, Zusammenführen (Regel 5), Lücken (Regel 6) und Fenster (Regel 7) fest. Offen blieb, wie die **eigene** Nachricht zwischen Absenden und Bestätigung erscheint, wie die Antwort des `POST` und das eigene Ereignis zusammenfinden, und wie viel Zeichenarbeit ein Ausbruch fremder Nachrichten auslöst (ADR-0028 Pre-Mortem 3: „Bündeln je Frame ist ein Umbau im Tab“).
- Designsystem `components/Nachricht`: „wird gesendet …“ (Text in `kd-ink-muted`, Zeile darunter ◌), gesendet ohne Marke, fehlgeschlagen ⚠ mit Grund in `kd-danger`, „Erneut senden“, „Verwerfen“, bleibt an ihrer Stelle. `VerlaufsMarken`: nicht am Ende → nie scrollen, Pille „n neue Nachrichten“, Klick oder `Ende` springt zur ersten neuen; am Ende folgt der Verlauf; Verbindung verloren → Gesendetes gepuffert und „wird gesendet“; wieder da → „✓ Wieder verbunden · n nachgeladen“, 4 s.
- [ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md): Antwort 201/200 mit `{zeit, von, art, text}`; Wiederholschlüssel; [ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md): 429 mit `Retry-After`.
- Das Schreibkonto erlaubt je Herkunft Ausbrüche bis 120 Nachrichten; mehrere Herkünfte zusammen mehr.
- Gelerntes (Factory, gemeinsam): „UI-Tests mit verzögert antwortendem Fake“ — für Aktionen, die nachladen, mindestens ein Test mit verzögertem Fake.

## Optionen

### Option A: Vorläufig sofort zeigen, durch Antwort **oder** eigenes Ereignis bestätigen; ein Sendeauftrag zugleich je Tab; fremde Ereignisse je Frame bündeln
- Vorteile: die eigene Nachricht steht im selben Frame wie das Drücken von `Enter` (die Zustände des Designsystems sind abgebildet); Reihenfolge der eigenen Nachrichten eines Tabs gleich der im Log; kein Doppel, egal ob Antwort oder Ereignis zuerst kommt; ein Ausbruch fremder Nachrichten kostet eine Anker-Korrektur je Frame statt eine je Nachricht.
- Nachteile: zwei Bestätigungswege und eine Warteschlange im Tab — mehr Zustände, die eigene Node-Tests brauchen.

### Option B: Erst nach der Antwort zeigen (kein vorläufiger Zustand)
- Vorteile: nur ein Weg; nichts vorläufig.
- Nachteile: zwischen `Enter` und Anzeige liegt eine Netzrunde — im LAN unsichtbar, bei Last oder WLAN-Wackler ein leeres Feld ohne Rückmeldung; die Zustände „wird gesendet“ und „gepuffert“ des Designsystems entfallen.

### Option C: Nur über den Strom zeigen; die Antwort leert nur das Feld
- Vorteile: eine Quelle für alle Nachrichten, auch für die eigenen.
- Nachteile: bei getrenntem Strom erscheint die eigene Nachricht nie, obwohl sie gesendet ist; Fehlerzustände brauchen trotzdem einen lokalen Zustand.

### Option D: Wie A, aber Ereignisse einzeln zeichnen
- Vorteile: weniger Code (kein Sammeln).
- Nachteile: bei einem Ausbruch von 120 Nachrichten 120 Einfügungen samt Anker-Korrektur in einem Task; der Tab ist dafür blockiert, und am Ende des Verlaufs rutscht der Blick 120-mal.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass nur A die Zustände des Designsystems trägt und dabei kein Doppel zulässt, gleich in welcher Reihenfolge Antwort und Ereignis eintreffen, und dass das Bündeln je Frame die Arbeit eines Ausbruchs auf eine Einfügung je Bild begrenzt. In Kauf genommen: eine Warteschlange und zwei Bestätigungswege im Tab, und dass ein Tab, der viele eigene Nachrichten puffert, sie nacheinander statt parallel sendet.

Regeln (als reine Funktionen in `web/`, Node-Tests in `e2e/einheit/`):

1. **Vorläufig**: `Enter` (Regeln der Eingabe in [ADR-0038](0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md)) legt eine vorläufige Nachricht `{schluessel, channel, text, zustand: "wird gesendet"}` an, leert das Feld und zeichnet sie **unter** allen bestätigten Nachrichten. Steht der neueste Block nicht im Fenster (`neuer`), öffnet der Tab zuerst neu wie „Zum Neuesten“ (ADR-0026 Regel 6, aber der Fokus bleibt in der Eingabe); dann steht der Blick am Ende. Vorläufige stehen in der Reihenfolge des Absendens.
2. **Ein Sendeauftrag zugleich je Tab**: die vorläufigen Nachrichten eines Tabs bilden eine Warteschlange über alle Channels; gesendet wird immer nur die älteste, die nächste erst nach ihrer Antwort. Jeder Versuch trägt denselben `Idempotency-Key` (`crypto.randomUUID()` beim Anlegen).
3. **Bestätigen**: die erste von zwei Quellen gewinnt, die zweite findet nichts mehr zu tun —
   - Antwort 201/200: die vorläufige fällt weg; die Nachricht wird über ihre `zeit` eingeordnet, wenn sie noch nicht da ist (ADR-0028 Regel 5);
   - Ereignis `nachricht` mit der **eigenen Kennung**, dem Channel und dem **Text** der gerade gesendeten vorläufigen (Regel 2 macht sie eindeutig im Tab): sie wird durch die bestätigte ersetzt; die spätere Antwort findet die Zeit schon vor.
   Stimmt nichts überein (ein anderer Tab desselben Browsers sendet denselben Text), wird die Nachricht wie eine fremde eingeordnet und die eigene später über die Antwort — am Ende steht jede Zeit genau einmal.
4. **Fehler eines Sendeauftrags** (Gestalt in ADR-0038):
   - Strom gilt als getrennt (Leiste „Verbindung verloren“): die vorläufige bleibt „wird gesendet“; die Warteschlange ruht und läuft nach dem Wiederverbinden mit demselben Schlüssel weiter;
   - Netzfehler bei verbundenem Strom oder **10 s** ohne Antwort (`AbortController`): „fehlgeschlagen — Keine Antwort vom Server.“; 401: der vorhandene stille Wiederbeitritt (ADR-0012, Browser) und **ein** neuer Versuch mit demselben Schlüssel, scheitert er, fehlgeschlagen; 413/422: fehlgeschlagen mit dem Grund aus der Antwort; 429: fehlgeschlagen mit „Zu viele Nachrichten … — wieder möglich in n s“; 500: „Nicht gespeichert — der Server konnte nicht schreiben.“;
   - eine fehlgeschlagene Nachricht hält die Warteschlange **nicht** an (die nächste wird gesendet); sie bleibt an ihrer Stelle unter den bestätigten, bis „Erneut senden“ (stellt sie mit demselben Schlüssel ans Ende der Warteschlange) oder „Verwerfen“.
5. **Vorläufige und fehlgeschlagene über Wechsel**: sie gehören zu ihrem Channel und bleiben im Speicher des Tabs, wenn ein anderer Channel geöffnet wird; kommt der Tab zurück, stehen sie wieder unter dem Verlauf. Beim Neuladen des Tabs sind sie verloren (die Warteschlange ist kein Speicher; eine vorläufige, die schon angekommen ist, steht dann im Verlauf).
6. **Fremde Nachrichten im offenen Channel** (ADR-0028 Regel 7): Ereignisse werden gesammelt und **einmal je `requestAnimationFrame`** eingefügt — eine Anker-Korrektur, ein Abgleich des Fensters je Frame.
   - Blick am Ende (Abstand zum Ende ≤ 1 Zeile): anfügen, der Blick folgt;
   - sonst: anfügen (wenn der neueste Block im Fenster steht) oder nur zählen (wenn nicht); die Pille zeigt „n neue Nachrichten“; Klick oder `Ende`: steht die erste neue im Fenster, scrollt der Tab sie an den oberen Rand mit einer Zeile Kontext darüber, sonst öffnet er neu wie „Zum Neuesten“; die Zahl gilt bis der Blick das Ende erreicht;
   - eigene bestätigte Nachrichten zählen nie zur Pille.
   Nachrichten anderer Channels werden nicht gezeichnet und nicht gezählt (Ungelesen-Zähler sind nicht I0009).
7. **Lücken** (ADR-0028 Regel 6): nach jedem Wiederverbinden und nach `NeuerStand` holt der Tab für den offenen Channel `nach=<jüngste bestätigte Zeit>` in Blöcken zu 200, bis `neuer` falsch ist, höchstens **5** Blöcke; gilt danach noch `neuer`, öffnet er neu wie „Zum Neuesten“. Die Leiste zeigt „✓ Wieder verbunden · n nachgeladen“ (n = gezählte nachgeholte Nachrichten; bei Neuöffnen „· neueste geladen“), 4 s. Erst danach läuft die Warteschlange (Regel 4) weiter — so stehen nachgeholte fremde Nachrichten vor den eigenen gepufferten, wie es ihre Zeiten verlangen.
8. **Kein zweiter Weg für Agenten**: wer den Strom mit `curl -N` liest, sieht `nachricht` unverändert (ADR-0028 Regel 8); diese Regeln gelten nur im Tab.

**Kriterien** (für den `planer`; Form nach AI-Team/planer.md, Gelerntes (1)–(7); Leistungsgrenzen stehen einmal in der Anforderung, der Test übernimmt sie):

| # | Was | Art | Startpunkt der Uhr / Messweg | erwartet | Grenze | Zweck |
|---|---|---|---|---|---|---|
| Z1 | Fremde Nachricht erscheint im offenen Channel einer zweiten Sicht | Gate (E2E, zwei Kontexte) — **Zustellung** | A erhält die 201 (`Date.now()` im Fenster von A) → Nachricht mit Absender sichtbar in B (`waitForFunction`, `polling: 'raf'`, Zeitstempel im Fenster von B; gleiche Maschine) | ≈ 20–60 ms (gerechnet: Bus < 1 ms, Weg über den `SharedWorker`, ein bis zwei Frames — Probe 33 ms) | < 1 s (Q3) | „erscheint sofort“ |
| Z2 | Reihenfolge und Eindeutigkeit | Gate (E2E, deterministisch) — **Regel** | A sendet 5, ein Agent über die API 5 abwechselnd | B und A zeigen dieselben 10 Zeiten, streng steigend, je einmal | exakt | gleiche Ordnung überall |
| Z3 | Eigene Nachricht sofort vorläufig | Gate (E2E, Route um 2 s verzögert) | `Enter` → Element „wird gesendet …“ mit dem Text sichtbar **vor** der Antwort | im nächsten Frame | vor Eintreffen der Antwort | „sofort“ auch für den Absender |
| Z4 | Kein Doppel, gleich wer zuerst kommt | Gate (Node, rein) | Ereignis vor Antwort; Antwort vor Ereignis; zwei Tabs, gleicher Text | jede Zeit genau einmal, keine vorläufige übrig | exakt | Regel 3 |
| Z5 | Nicht am Ende: nichts scrollt | Gate (E2E) | B steht 2 Bildschirmhöhen über dem Ende; A sendet 3 | `scrollTop` von B unverändert (± 1 px), Pille „3 neue Nachrichten“ | exakt 3 | Designsystem `VerlaufsMarken` |
| Z6 | Lücke nach Abriss | Gate (E2E) | Strom von B getrennt (Route abgebrochen), 3 Nachrichten über die API, Strom wieder → ab „verbunden“ | „Wieder verbunden · 3 nachgeladen“, die 3 genau einmal | < 2 s ab Wiederverbinden (erwartet ≈ 50 ms + ein Abruf) | ADR-0028 Regel 6 |
| Z7 | Gepuffert senden | Gate (E2E) | Strom getrennt; A sendet 2 → beide „wird gesendet“; Strom wieder | beide genau einmal im Verlauf von A und B, in Sendereihenfolge | exakt | Designsystem „gepuffert“ |
| Z8 | Antwort verloren, Wiederholung | Gate (E2E, Route liefert die erste Antwort nicht aus) | „Erneut senden“ | genau ein Satz im Log, eine Nachricht im Verlauf | exakt | Wiederholschlüssel |
| Z9 | Ausbruch zeichnen | **Messwert** | Agent sendet 120 kleine Nachrichten so schnell wie erlaubt, B am Ende | längste Aufgabe in B ≈ wenige ms je Frame | keine Aufgabe über 250 ms, höchstens 2 % Frames über 50 ms (wie M1 in ADR-0026) | Lehre #5 |

Konsolenkriterium (Gelerntes (5)): erlaubt sind „Failed to load resource“ nur zu Antworten, die ein Szenario selbst herbeiführt — 401 (Wiederbeitritt nach Neustart), 413, 422, 429, 500 — und zu den abgebrochenen Routen in Z6–Z8.

## Konsequenzen

**Positiv:**
- Die eigene Nachricht steht im Frame des `Enter`; die Bestätigung tauscht sie aus, ohne dass etwas springt (sie steht schon an der richtigen Stelle, weil vorläufige immer unten stehen und der Blick am Ende ist).
- Ein Ausbruch von 120 fremden Nachrichten kostet einige Frames mit je einer Einfügung.
- Fehlerwege sind reine Zustandsübergänge — testbar ohne Browser.

**Negativ / Inkauf zu nehmen:**
- Eigene Nachrichten eines Tabs gehen nacheinander; wer offline 20 schreibt, wartet nach dem Wiederverbinden 20 Netzrunden (im LAN Millisekunden).
- Beim Neuladen gehen vorläufige und fehlgeschlagene Nachrichten verloren (Text weg); der Entwurf im Feld bleibt (ADR-0038).
- Zwei Tabs desselben Browsers mit gleichem Text können kurz die Bestätigung tauschen (Regel 3) — am Ende stimmt der Verlauf.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … die eigene Nachricht für einen Augenblick doppelt erscheint (bestätigt oben, vorläufig unten). Früh-Indikator: Node-Test Z4, E2E mit verzögerter Antwort **und** sofortigem Ereignis. Gegenmaßnahme: Regel 3, zweiter Weg.
2. … nach dem Wiederverbinden die eigenen gepufferten vor fremden, älteren Nachrichten stehen. Früh-Indikator: Z7 mit fremder Nachricht während der Trennung. Gegenmaßnahme: Regel 7 (erst Lücke holen, dann Warteschlange); die Einordnung über die Zeit korrigiert es ohnehin.
3. … ein Agent im Takt sendet und der Verlauf bei einem Leser oben trotzdem springt. Früh-Indikator: Z5 unter Ausbruch. Gegenmaßnahme: Regel 6 (nicht am Ende: anfügen nur unterhalb des Blicks, Anker-Korrektur wie ADR-0026 Regel 3 je Frame).

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Node-Test (rein, `e2e/einheit/`): Zustand `{bestaetigt: [Zeiten], vorlaeufig: [{schluessel, text}]}`; (a) senden „x“ → eine vorläufige; Antwort `zeit 5` → keine vorläufige, `5` bestätigt; (b) Ereignis eigene Kennung „x“ `zeit 5` vor der Antwort → ersetzt; Antwort `zeit 5` → unverändert; (c) Ereignis fremd `zeit 4` zwischen zwei eigenen → Folge 4, 5 ohne Doppel; (d) Fehler 429 → `fehlgeschlagen` mit Wartezeit, nächste vorläufige wird gesendet.
- [ ] Erster Node-Test Bündeln: 120 Ereignisse in einem Task → genau ein Einfügen beim nächsten Frame (gestellter `requestAnimationFrame`).
- [ ] Erster E2E-Test: Z1 und Z3 mit zwei Kontexten.

## Quellen / Bezuege

- [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md); Aspekt [X00004 Fehlerbehandlung](../Architektur/Aspekte/X00004-fehlerbehandlung.md)
- ADR-0012, ADR-0015, ADR-0025, ADR-0026, ADR-0028, ADR-0034, ADR-0036, ADR-0038; `web/strom.js`, `web/verbindung.js`
- Vision R00000 „Live überall“; WBS I0009; Designsystem `components/Nachricht/README.md`, `components/VerlaufsMarken/README.md`; `design-prompt.md` 4.2 („Viel Bewegung“, Zustände „wird gesendet / gesendet / fehlgeschlagen“)
- AI-Team/planer.md, Gelerntes (Zeit- und Konsolenkriterien); Factory `gemeinsam/gelerntes.md` („UI-Tests mit verzögert antwortendem Fake“)
