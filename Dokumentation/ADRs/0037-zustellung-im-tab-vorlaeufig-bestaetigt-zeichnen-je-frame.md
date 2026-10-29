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

## Ergänzungen

- **2026-10-28 (Karte #31; UX-Review #8 S5) — Platz der fehlgeschlagenen Nachricht:** Präzisiert Regel 4 („bleibt an ihrer Stelle unter den bestätigten“); ersetzt wird nichts. Anlass: Designsystem `components/Nachricht/README.md` Zeile 9 und R00011 F0043 („bleibt an ihrer Stelle“) gegen R00011 und Regel 1 („**unter** allen bestätigten“); gebaut wandert eine fehlgeschlagene ans Ende, sobald neue bestätigte kommen.
  - Option A: **an der Stelle des Absendens** zwischen den bestätigten, die vor ihr kamen, und denen danach. Stärke: der Bezug bleibt sichtbar (worauf sie antworten sollte). Schwäche: sie hat keine Zeit des Servers — die Ordnung im Verlauf gilt nur über die Nachrichtenzeit ([ADR-0035](0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md), [ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md) Regel 5), die Stelle wäre aus der Browseruhr geraten; neue Nachrichten schieben sie aus dem Blick, gerade wenn sie Handeln verlangt („Erneut senden“, F0046); der Fenster-Abwurf ([ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md)) müsste lokale Einträge zwischen Blöcken halten.
  - Option B: **immer unter allen bestätigten**, in der Reihenfolge des Absendens (gewählt, wie gebaut). Stärke: eine Regel für vorläufige und fehlgeschlagene (Regel 1); die Bestätigung tauscht ohne Sprung aus, weil vorläufige unten stehen (Regel 3); eine fehlgeschlagene bleibt am Ende sichtbar, wo der Blick nach dem Senden ist, und mit ihr ihr Ausweg. Schwäche: kommen danach bestätigte Nachrichten, verliert sie optisch den Bezug zu der Nachricht, auf die sie antwortete.
  - Option C: wie B, mit Zeitangabe „vor 3 min“ an der fehlgeschlagenen. Stärke: der zeitliche Bezug ist lesbar. Schwäche: eine laufend nachgeführte Zeit aus der Browseruhr an einem einzigen Zustand; verlangt hat sie niemand.
  - **Gewählt: B.** „An ihrer Stelle“ heißt: in der Reihenfolge der vorläufigen und fehlgeschlagenen **unter allen bestätigten**; sie springt nicht beim Wechsel von „wird gesendet“ zu „fehlgeschlagen“, und neue bestätigte Nachrichten erscheinen **über** ihr. In Kauf genommen: der optische Bezug zur Vorgängernachricht geht verloren, sobald andere schreiben. Die Vision ist nicht berührt (keine Frage an den Betreiber): es geht um die Darstellung eines lokalen Zustands, nicht um die Ordnung der Gespräche.
  - **Folgen** (für den `planer`, nicht hier geschrieben): Designsystem `components/Nachricht/README.md` Zeile 9 „bleibt an ihrer Stelle“ → „bleibt unter dem Verlauf, unter allen bestätigten, bis ‚Erneut senden‘ oder ‚Verwerfen‘“; R00011 F0043 „Die fehlgeschlagene Nachricht bleibt an ihrer Stelle und hält die Warteschlange nicht an“ (Zeile 49) liest sich mit dieser Bedeutung — Berichtigung über die nächste Anforderung, die F0043 berührt, oder `/anforderung review`. Kein Code: gebaut ist B. Beleg als Test (falls noch keiner): E2E — A sendet „x“, Route antwortet 500; danach sendet `beta` 2 → die beiden stehen **über** „x“, „x“ ist das letzte Element des Verlaufs und zeigt ⚠. Mutante „fehlgeschlagene nach Zeit einordnen“ → rot.
- **2026-10-28 (Karte #31; Code-Review #8 A4, Nacharbeit #8 B0329) — Auslauf unter dem Verlauf:** Ergänzt Regel 6 („Klick oder `Ende`: … scrollt der Tab sie an den oberen Rand mit einer Zeile Kontext darüber“). Der Mechanismus ist gebaut (`web/verlauf.js`, `auslaufFuer`, Element `.lc-auslauf`), stand aber in keiner ADR; sein Abbau wird mit B0329 gebaut. Die Regeln beschreiben das Ziel von B0329 (WBS, R00011 Kriterium „Lesestelle nach dem Sprung“); weicht der Bau in einer Einzelheit ab, die die Kriterien grün hält, wird diese Ergänzung nach dem Bau angeglichen, nicht der Bau.
  1. **Zweck**: der Sprung zur ersten neuen soll sie an ihr Ziel bringen (oberer Rand plus eine Zeile Kontext; ab R00020 F0067 unter dem klebenden Tagestrenner), auch wenn unter ihr weniger als eine Sichthöhe Inhalt steht. Dafür entsteht unter dem Inhalt ein leerer Raum, der **Auslauf**.
  2. **Entstehen** — nur beim Sprung: fehlt Scrollweg zum Ziel, wird der Auslauf `fehlt + eine Zeilenhöhe` hoch (die Zeile mehr, damit der Blick dabei nicht schon „am Ende“ gilt). Er ist damit höchstens eine Sichthöhe plus eine Zeile. Sonst wächst er nie — weder beim Anfügen noch bei einer Größenänderung des Fensters.
  3. **„Am Ende“ meint das Ende des Inhalts**, nicht das Ende des Scrollbereichs: Folgen (Regel 6), das Zurücksetzen der Zählung und das Verbergen der Pille richten sich nach dem Abstand der Unterkante der letzten Nachricht (bestätigt, vorläufig oder fehlgeschlagen) zur Unterkante des Fensters.
  4. **Abbau, ohne dass sich der sichtbare Inhalt bewegt** (B0329):
     - neuer Inhalt unten (fremde, eigene vorläufige, bestätigte) **nimmt den Auslauf ein**: Auslauf := max(0, Auslauf − angefügte Höhe) im selben Frame wie das Anfügen; die Scrollhöhe bleibt, solange Auslauf da war;
     - der Teil des Auslaufs **unterhalb** des Fensters fällt weg (beim Scrollen nach oben, beim Sprung „ans Ende des Inhalts“ aus R00020 F0067); der **sichtbare** Teil bleibt;
     - er fällt nie auf einen Schlag weg, solange er sichtbar ist (der Fehler aus UX-Review #8 S1: Sprung um 515–596 px).
  5. **Entfallen**: beim Öffnen eines Channels, beim Neuöffnen wie „Zum Neuesten“ und beim Channelwechsel ist der Auslauf 0.
  6. **Grenze**: der Auslauf ist kein Inhalt — leer, ohne Rolle, nicht fokussierbar, nie Ziel einer Ansage; er gehört nur dem offenen Channel. Er ändert nichts an Fenster und Abwurf ([ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md)): fällt der neueste Block aus dem Fenster, liegt der Auslauf unterhalb und fällt nach Punkt 4 weg.
  - Verworfen (rückblickend, für die Lesenden): Sprung ohne Auslauf (die erste neue bliebe tiefer als ihr Ziel, wenn wenig nachkommt — gegen Regel 6); Auslauf am Ende auf einmal weg (gebaut vor B0329, verletzt R00011 „Lesende werden nie aus ihrer Stelle gerissen“); Auslauf fest eine Sichthöhe (Leerraum ohne Grund, und das Ende des Inhalts läge für jeden Leser eine Sichthöhe tiefer).
  - **Erster Test**: die Probe aus R00011 „Lesestelle nach dem Sprung“ (zwei Fenstergrößen, (a)–(c)); dazu, falls B0329 die Rechnung als reine Funktion baut, Node-Test `auslaufNach({auslauf, angefuegt, unterhalbFenster})`: 300 px Auslauf, 120 px angefügt → 180 px; 300 px, davon 200 px unterhalb des Fensters → 100 px; 300 px, 400 px angefügt → 0. Mutante „Auslauf am Ende auf einmal weg“ → rot.
- **2026-10-29 (Nacharbeit Karte #22; Code-Review #22 B1, B2, A3; WBS Offene Fragen #22 Punkte 1 und 2) — Nachholen: Fehlerfall und Ereignisse während des Abrufs:** Präzisiert Regel 7; ersetzt wird nichts. Den Fehlerfall hat der `planer` entschieden (R00012 F0047 „Scheitert das Nachholen“, Z13), hier bestätigt und um die Punkte ergänzt, die R00012 offen lässt (Cursor, ein Lauf zugleich, Abbruch).
  - Option A: **stiller Neuversuch** — keine Meldung, die Leiste bleibt „⚠ Verbindung verloren“, die Warteschlange ruht, neuer Versuch nach dem Rückzug des Stroms, Zeitgrenze 10 s je Abruf (Empfehlung des `planer`, **gewählt**). Stärke: die Lücke bleibt sichtbar, bis sie geschlossen ist; „Fremde vor eigenen“ (Regel 7) bleibt wahr; kein neues Bedienelement; die Lücke schließt sich ohne Klick, sobald der Server wieder liefert. Schwäche: scheitert jeder Abruf (der Server liest den Verlauf nicht), bleiben eigene Nachrichten „wird gesendet …“ — sie gehen nicht verloren, aber sie gehen auch nicht hinaus.
  - Option B: Fehlerleiste „Verpasste Nachrichten nicht geladen“ mit Schalter, Warteschlange läuft weiter. Stärke: der Mensch sieht den Grund und entscheidet. Schwäche: verlangt einen Klick; eigene Nachrichten gingen vor fremde ältere hinaus.
  - Option C: Rückfall auf Neu-Öffnen wie „Zum Neuesten“. Stärke: ein bekannter Weg. Schwäche: das Neu-Öffnen ist selbst ein Abruf und scheitert unter derselben Störung; die Lesestelle ginge verloren.
  - **Gewählt: A.** In Kauf genommen: unter einer dauerhaften Störung des Verlaufs ruht die Warteschlange; eine Lücke nach `NeuerStand` (Strom verbunden) bleibt ohne Leiste, bis sie geschlossen ist (Regel 7e).
  - **Regeln** (rein, mit gestellter Zeit, wo es um Zeit geht):
    - **7a Zeitgrenze**: jeder Abruf `nach=` bricht **10 s** nach seinem Beginn ab (`AbortController`), wie Regel 4. Erwartet ist ein Block zu 200 in der Größenordnung von „Channel öffnen < 500 ms“ (Lastprofil); die Grenze liegt das 20-Fache darüber.
    - **7b Cursor**: ein Nachhol-Lauf beginnt bei der jüngsten bestätigten Zeit **vor** der Trennung bzw. vor `NeuerStand` und rückt nur mit gelungenen Blöcken vor (auf die jüngste Zeit des Blocks). Live angenommene Nachrichten (7f) rücken ihn **nie** vor — sonst übersprünge der nächste Versuch die Lücke. Ein Versuch nach einem Fehlschlag fragt ab dem Cursor, nicht ab der jüngsten gezeigten Zeit. Gelungene Blöcke eines später gescheiterten Versuchs bleiben im Verlauf.
    - **7c Fehlschlag**: Netzfehler, Zeitgrenze oder Status außer 200 (auch 401, 429, 500) lassen den Versuch scheitern. Dann keine Meldung „✓ Wieder verbunden“, die Leiste bleibt „⚠ Verbindung verloren“ (mit Countdown bis zum nächsten Nachhol-Versuch, Ergänzung F0084 in [ADR-0038](0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md)), die Warteschlange ruht. Neuversuch 1 s, 2 s, 5 s nach dem ersten, zweiten, dritten Fehlschlag, danach alle 10 s, gemessen ab dem Ende des gescheiterten Abrufs. Die Tabelle ist die des Stroms (`RUECKZUG_MS` in `web/verbindung.js`, exportiert und benutzt, nicht kopiert); der Zähler der Fehlversuche gehört dem Tab, der des Stroms bleibt im Worker. Eine 401 hier löst keinen eigenen Beitritt aus — den macht der Weg des Stroms ([ADR-0012](0012-identitaet-und-sitzung.md), Ergänzung Karte #36); der nächste Versuch trägt das dann gültige Token.
    - **7d Erfolg**: Meldung „✓ Wieder verbunden · n nachgeladen“ (n = alle in diesem Lauf nachgeholten Nachrichten, über alle Versuche), danach läuft die Warteschlange weiter.
    - **7e Ein Lauf zugleich je Tab**; er endet bei
      - Erfolg (7d);
      - **erneuter Trennung**: der laufende Abruf wird abgebrochen, ein wartender Neuversuch entfällt; nach dem nächsten Wiederverbinden beginnt ein neuer Lauf **am selben Cursor** mit sofortigem ersten Versuch;
      - **Channelwechsel**: Abruf abgebrochen, Neuversuch entfällt; der neue Channel wird vollständig geöffnet (er hat keine Lücke), die Leiste meldet „✓ Wieder verbunden“, sobald der Strom verbunden ist, die Warteschlange läuft weiter. Kommt der Tab später in den alten Channel zurück, öffnet er ihn neu.
      - Ein **`NeuerStand` während eines Laufs** startet keinen zweiten: der laufende holt, bis `neuer` falsch ist, und deckt ihn ab; wartet er auf einen Neuversuch, bleibt dessen Zeitpunkt.
      - **`NeuerStand` bei verbundenem Strom** startet einen Lauf wie das Wiederverbinden, aber ohne Leiste und ohne Ruhen der Warteschlange (R00012): der Strom ist da, und die Ordnung zwischen fremden und eigenen Nachrichten stellt die Einordnung über die Zeit her (7f). Früh-Indikator für eine solche stille Lücke: `WARNING live … Ereignisstrom überholt` im Betriebsprotokoll ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung Karte #31 Punkt 6).
    - **7f Ereignisse während des Nachholens** (nicht blockierend; Code-Review B1): Ereignisse `nachricht` des offenen Channels werden ab dem **ersten Ereignis des neuen Stroms** angenommen, nicht erst nach dem Nachholen. Nachgeholte und live angenommene Nachrichten werden über die Zeit zusammengeführt: jede Zeit genau einmal, in der Folge der Zeiten — auch Nachrichten der Lücke, die älter sind als eine schon gezeigte live angenommene (reine Operation neben `einordnen`, B0390; das heutige `einordnen` übernimmt nur Zeiten über der jüngsten). Die Mechanik — sofort einfügen (mit Anker und Fenster, [ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md)) oder bis zum Ende des Abrufs puffern — wählt der Entwickler in B0391; beide erfüllen die Regel, gepufferte werden aber bei einem gescheiterten Versuch **sofort** gezeichnet, nicht bis zum Erfolg zurückgehalten. Nach dem Bau wird hier eine Zeile mit der gebauten Mechanik nachgetragen.
  - **Pre-Mortem** (ein Monat nach der Nacharbeit gescheitert, weil …):
    1. … nach einem gescheiterten ersten Abruf und einer live angenommenen Nachricht der zweite Abruf ab deren Zeit fragt und die Lücke still bleibt. Früh-Indikator: der Node-Test unten. Gegenmaßnahme: 7b.
    2. … ein Neuversuch und ein neuer Lauf nach erneuter Trennung gleichzeitig laufen und die Meldung doppelt kommt. Früh-Indikator: Node-Test „Trennung während des Wartens“. Gegenmaßnahme: 7e, ein Lauf zugleich.
    3. … der hängende Abruf die Warteschlange für immer festhält. Früh-Indikator: Z13. Gegenmaßnahme: 7a.
  - **Erste Tests** (für den `planer`; ergänzen R00012 F0047, ersetzen nichts):
    - Node (rein, gestellte Zeit): bekannt 1–3, Trennung; live 6 angenommen; erster Abruf scheitert → der zweite fragt `nach=3`, nicht `nach=6`; Block 4, 5 → Folge 1–6 je einmal, Meldung „· 2 nachgeladen“. Mutante „Cursor = jüngste gezeigte Zeit“ → rot.
    - Node: Fehlschlag bei 0 ms, Trennung bei 500 ms, Wiederverbinden bei 700 ms → kein Versuch bei 1 000 ms aus dem alten Lauf, ein Versuch bei 700 ms ab demselben Cursor. Mutante „alter Zeitgeber läuft weiter“ → rot.
    - Die Kriterien „Neuversuch“, „Gescheitertes Nachholen“, „Nachhol-Abruf hängt“ (Z13) und „Während des Abrufs“ aus R00012 gelten unverändert.
- **2026-10-29 (Karte #36, R00025 F0085, B0400–B0402; UX-Review #22 F4; WBS Offene Fragen #22 Punkt 5) — Verworfen auf Zeit:** Ändert Regel 4 („bis ‚Erneut senden‘ … oder ‚Verwerfen‘“): „Verwerfen“ entfernt eine fehlgeschlagene Nachricht nicht mehr sofort, sondern nach 5 s; bis dahin steht „Verworfen · Rückgängig“ an ihrer Stelle (Regel 4, Ergänzung Karte #31: unter allen bestätigten). Regel 5 gilt für verworfene wie für fehlgeschlagene.
  - Option A: **Zustand in der Zustellung** — `fehlgeschlagen → verworfen{seit}` als reiner Übergang mit gestellter Zeit; der Channelwechsel behält ihn, das Neuladen verwirft endgültig (Empfehlung des `planer`, **gewählt**). Stärke: eine Quelle für Zeichnen, Warteschlange und Wechsel; exakt testbar (4.999 / 5.000 ms); verhält sich über Wechsel und Neuladen genau wie eine fehlgeschlagene (Regel 5) — der Mensch muss keine zweite Regel lernen.
  - Option B: nur im Zeichnen (DOM-Zeile mit eigenem `setTimeout`). Stärke: kein neuer Zustand. Schwäche: der Channelwechsel zeichnet den Verlauf neu, die Zeile und ihr Zeitgeber gehen verloren oder laufen ins Leere; die Zeit ist nur im Browser prüfbar.
  - Option C: im `sessionStorage`, damit auch das Neuladen sie zurückbringen kann. Stärke: Verklicken und Neuladen kosten nichts. Schwäche: eine verworfene überlebte das Neuladen, die fehlgeschlagene davor aber nicht (Regel 5) — verkehrte Welt; Nachrichtentexte im Speicher des Tabs außerhalb des Entwurfs.
  - Option D: wie A, aber die 5 s laufen nur, solange ihr Channel offen ist. Stärke: wer zurückkommt, findet sie noch. Schwäche: eine verworfene Nachricht kann unbegrenzt leben; „5 s“ verliert seine Bedeutung, die Zeit wird zustandsabhängig.
  - **Gewählt: A.** In Kauf genommen: wer innerhalb der 5 s neu lädt oder den Tab schließt, hat den Text verloren — wie bisher bei jeder fehlgeschlagenen.
  - **Regeln**:
    1. Übergänge (rein, `web/zustellung.js`): `fehlgeschlagen —Verwerfen→ verworfen{seit: jetzt}`; `verworfen —Rückgängig, jetzt − seit < 5.000→ fehlgeschlagen` mit Text, Grund und Wiederholschlüssel unverändert, **nichts gesendet**; `verworfen, jetzt − seit ≥ 5.000 → entfernt`. Rückgängig ab 5.000 ms tut nichts.
    2. Eine verworfene ist nicht in der Warteschlange, hält sie nicht an und wird nie bestätigt; ein Ereignis mit gleichem Text (Regel 3) trifft sie nicht.
    3. Die Zeit läuft weiter, auch während ein anderer Channel offen ist. Wer innerhalb der 5 s zurückkommt, findet „Verworfen · Rückgängig“ an ihrer Stelle; wer später kommt, nichts.
    4. Ein Zeitgeber je Tab für die nächste fällige verworfene ruft die Zustellung mit `jetzt` auf; gezeichnet wird nur im offenen Channel. Kein Zeitgeber im DOM.
    5. Neuladen, Schließen: endgültig weg (Regel 5). Ein Wechsel zum Beitreten-Bildschirm ist kein Neuladen — die Zustellung lebt im Tab weiter ([ADR-0012](0012-identitaet-und-sitzung.md), Ergänzung Karte #36), die Uhr entscheidet wie in Regel 3.
  - **Erste Tests** (für den `planer`): B0400 wie geplant (4.999 → zurück, 5.000 → entfernt; Mutanten 4 s, 6 s); dazu Node: verworfen bei 0, Channelwechsel bei 1.000, Rückkehr bei 3.000 → Zeile da, Rückgängig → fehlgeschlagen mit demselben Schlüssel; Rückkehr bei 6.000 → entfernt. Mutante „Wechsel verwirft“ → rot.
