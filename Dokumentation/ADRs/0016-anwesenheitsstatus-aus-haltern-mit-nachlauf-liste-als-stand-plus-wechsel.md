# ADR-0016: Anwesenheitsstatus aus Haltern mit Nachlauf, Teilnehmerliste als Stand plus Wechsel

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #4 I0005)
**Kontext-Anforderung**: — (WBS I0005; prägt I0006, I0014, I0016)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (axum 0.8.9, hyper 1.12, Annahmeschleife wie `src/start/ausliefern.rs` mit `header_read_timeout` 10 s, `--release`), danach gelöscht: (1) ohne gültigen Header → 401; (2) Client schließt einen **ruhenden** SSE-Strom (kein Byte seit dem ersten Ereignis) → der Halter im Strom fällt nach 0,3 ms; (3) ein Strom ohne jedes Byte über 12 s wird von der Kopf-Zeitgrenze nicht abgeschnitten, der Halter lebt; (4) Ende-Signal bei offenem Strom (`take_until` auf den Ende-Kanal) → Client liest das Strom-Ende, die Schleife ist nach 0,3 ms fertig statt nach 5 s.

## Kontext

- Vision „Wer da ist, ist sichtbar“: Liste aller aktiven Teilnehmer mit Online-Status; Abbruch → sofort offline; ohne Rückkehr nach etwa einer Minute aus der Liste. Q4 in [A00001](../Architektur/A00001-localchata.md): Tab zu → offline in < 2 s.
- [ADR-0012](0012-identitaet-und-sitzung.md): eine Sitzung lebt, solange sie Halter hat oder ihr letztes Lebenszeichen höchstens die Gnadenfrist (60 s) zurückliegt; Lebenszeichen = jede Anfrage mit Token und das Ende des letzten Halters; „weg“ wird heute beim Zugriff berechnet (`src/anwesenheit/bestand.rs`, `lebende_sitzungen`). [A00010](../Architektur/A00010-anwesenheit.md) ist einziger Schreiber aller Sitzungen.
- [ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md): ein Strom je Browser; lädt der einzige Tab neu, endet der Strom und ein neuer beginnt Sekundenbruchteile später. Mit I0016 enden Warte-Aufrufe von Agenten laufend und beginnen neu.
- Designsystem `components/Teilnehmerliste`: Menschen zuerst, dann Agenten, ab 8 Agenten nach Herkunft gruppiert; online ● ohne Wort, offline ○ mit „offline“; „offline, gleich weg“ mit Restzeit; Zeilen springen nicht; Statuswechsel werden nicht angesagt.
- Ein Abonnent kann zwischen Liste und erstem Wechsel Ereignisse verpassen, wenn Liste und Strom getrennt geholt werden; Ereignisse, die nach dem Freigeben einer Sperre veröffentlicht werden ([X00008](../Architektur/Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)), können in anderer Reihenfolge ankommen, als die Wechsel geschahen.

## Optionen

### Option A: Status sofort aus der Halterzahl; Liste per `GET /api/teilnehmer`, danach Wechsel im Strom
- Vorteile: kürzeste Regel; kein Takt.
- Nachteile: jedes Neuladen und jeder Warte-Zyklus erzeugt offline → online bei allen Beobachtern (Flackern); zwischen `GET` und Abonnieren gehen Wechsel verloren; ein neuer Teilnehmer erscheint erst offline, Millisekunden später online.

### Option B: Status mit Anlauf und Nachlauf von 1 s, berechnet beim Zugriff und von einem Takt (`pflegen(jetzt)`) veröffentlicht; der Strom beginnt mit dem **Stand** der Liste, danach **Wechsel**; jeder Stand und jeder Wechsel trägt eine laufende Nummer `stand`
- Vorteile: kein Flackern bei Neuladen und Warte-Zyklen; keine Lücke zwischen Liste und Wechseln (erst abonnieren, dann Stand); Reihenfolgefehler werden am Empfänger verworfen (älterer `stand` gilt nicht); derselbe Takt meldet später „weg“ (I0006) und kennt die Uhr nur über den Provider — rein prüfbar.
- Nachteile: offline erscheint bis zu 1 s plus Takt später; ein Takt-Task läuft immer; der Empfänger muss nach `stand` zusammenführen.

### Option C: Status sofort; bei jedem Wechsel die **ganze** Liste als Ereignis
- Vorteile: Empfänger ersetzt einfach; keine Zusammenführung, kein Reihenfolgeproblem, wenn die Liste unter der Sperre gebildet wird.
- Nachteile: Last wächst mit Teilnehmern × Wechseln × Sichten (bis 512 Sitzungen); Flackern wie A; ohne Nummer bleibt das Reihenfolgeproblem beim Veröffentlichen nach der Sperre.

### Option D: Anwesenheit aus Lebenszeichen-Pings statt Haltern (Browser schickt alle 10 s `POST /api/lebenszeichen`)
- Vorteile: kein Halter-Mechanismus; erkennt auch stille Abrisse.
- Nachteile: offline erst nach 10–20 s statt < 2 s (Q4 verfehlt); widerspricht ADR-0007/0012 (Halter); dauerhafte Last je Tab.

| # | Q4 (< 2 s) | Flackern | Lücke/Reihenfolge | Testbarkeit | Last |
|---|---|---|---|---|---|
| A | ja | ja | Lücke, falsche Folge möglich | gut | gering |
| B | ja (≤ 1,25 s + Worker-Ende) | nein | geschlossen über `stand` | sehr gut (Uhr-Provider) | gering |
| C | ja | ja | Folge offen | gut | hoch |
| D | nein | nein | — | gut | mittel |

## Entscheidung

**Gewählt: Option B**

Es überwiegt, dass die Liste ohne Flackern und ohne Lücken stimmt, obwohl Halter ständig kommen und gehen, und dass dieselbe reine Funktion `pflegen(jetzt)` den späteren Wechsel nach „weg“ trägt. In Kauf genommen: „offline“ kommt bis zu gut einer Sekunde nach dem Abbruch, und ein Takt läuft dauernd.

Regeln:

1. **Status je Sitzung** (berechnet in der Anwesenheit, gegen den Uhr-Provider):
   - `online` ⇔ mindestens ein Halter **oder** seit `max(Beitritt, Ende des letzten Halters)` sind weniger als `NACHLAUF` = 1 s vergangen;
   - `offline` ⇔ kein Halter, Nachlauf vorbei, Sitzung lebt (letztes Lebenszeichen ≤ Gnadenfrist);
   - `weg` ⇔ Sitzung lebt nicht mehr (ADR-0012) — sie erscheint in keiner Liste.
   Eine Sitzung ohne je einen Halter (Agent, der nur sendet) ist nach dem Anlauf `offline` — ehrlich: live ist sie nicht erreichbar.
2. **Halter**: `verbunden(token) -> Halter` zählt die Halter der Sitzung hoch; sein `Drop` zählt herunter, setzt beim letzten das Halter-Ende und ist ein Lebenszeichen (ADR-0012). Mehrere Halter je Sitzung sind erlaubt (Rückfall je Tab, ADR-0015; später mehrere Warte-Aufrufe eines Agenten).
3. **Wechsel**: jede Operation, die einen Status ändert (`beitreten`, `verbunden`), und `pflegen(jetzt)` liefern die Wechsel als Rückgabewert — `Wechsel{stand, teilnehmer{kennung, name, herkunft, art}, status, seit}`; `stand` ist eine `u64`, unter der Sperre der Anwesenheit hochgezählt. Die Anwesenheit kennt den Bus nicht; veröffentlicht wird von der Integration (Handler bzw. Takt-Task).
4. **Takt**: ein Task des Starts ruft alle 250 ms `pflegen(uhr.jetzt())` und veröffentlicht die Wechsel. In I0005 erzeugt er `offline`; `weg` liefert dieselbe Funktion (Zuschnitt siehe Bericht an den `planer`: Empfehlung, „weg“ schon mit I0005 zu melden, damit keine Geisterzeilen stehen).
5. **Stand plus Wechsel im Strom**: der Handler von `/api/ereignisse` abonniert **zuerst** den Bus, holt **dann** `teilnehmer()` = `Stand{stand, liste}` und sendet ihn als erstes Ereignis `{"art":"teilnehmer", "stand", "liste":[{kennung, name, herkunft, art, status, seit}]}`; danach je Wechsel `{"art":"anwesenheit", "stand", "teilnehmer":{…, status, seit}}`. Der Empfänger übernimmt einen Wechsel nur, wenn sein `stand` größer ist als der des zuletzt übernommenen Stands bzw. des Eintrags; `weg` entfernt die Zeile. Wer zu langsam liest (`Lagged`), bekommt einen neuen Stand.
6. **Für alle sichtbar**: jeder Teilnehmer mit Sitzung sieht alle lebenden Sitzungen samt `art` und Herkunft (keine Rechte, [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md); die Herkunft ist Teil der Kennung und steht ohnehin an jeder Nachricht). Die eigene Sitzung ist in der Liste; die Oberfläche erkennt sie an der eigenen Kennung. Sortierung und Gruppierung (Menschen, Agenten, ab 8 Agenten nach Herkunft) macht die Oberfläche nach dem Designsystem; der Server liefert nach Kennung sortiert.
7. **`GET /api/teilnehmer`** (Sitzung nötig) liefert denselben `Stand` — für Agenten ohne Strom und für Tests.
8. **Ende**: der Strom hört das Ende-Signal des Programms und endet selbst (Probe 4); der Halter fällt dabei wie bei jedem Abbruch.

Nicht hier: Erkennen eines stillen Abrisses (Herzschlag-Schreiben scheitert erst nach TCP-Zeitüberschreitung) — offene Probe aus ADR-0007, I0006; `POST /api/verlassen` und die Restzeit „gleich weg“ (`weg_um`) — I0006.

## Konsequenzen

**Positiv:**
- Neuladen, kurze Netzaussetzer und Warte-Zyklen von Agenten sind für andere unsichtbar; ein echter Abbruch ist nach ≤ 1,25 s (Nachlauf + Takt) als offline sichtbar.
- Die Liste ist zu jedem Zeitpunkt aus Stand + Wechseln rekonstruierbar; Reihenfolgefehler beim Veröffentlichen schaden nicht.
- Statusregel und Takt sind reine Funktionen mit Uhr-Provider; der Bus bleibt aus der Anwesenheit heraus.

**Negativ / Inkauf zu nehmen:**
- „Sofort offline“ der Vision heißt hier: gut eine Sekunde.
- Ein Takt-Task läuft dauernd (bei ≤ 512 Sitzungen vernachlässigbar), und alle 250 ms wird die Sperre der Anwesenheit kurz genommen.
- Ein Agent ohne Warte-Aufruf steht als offline, obwohl er gerade sendet.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … die Liste nach Neustarts Geisterzeilen oder doppelte Namen zeigt. Früh-Indikator: Zeile „offline“ für einen Namen, der zugleich online ist. Gegenmaßnahme: `weg` als Wechsel (Regel 4), Neuer Stand nach Wiederverbinden (der Strom beginnt immer mit dem Stand).
2. … viele Agenten mit kurzen Warte-Aufrufen ein Dauerfeuer an Wechseln erzeugen. Früh-Indikator: Wechsel je Minute im Strom. Gegenmaßnahme: Nachlauf 1 s; zählt mit I0016 nach.
3. … ein Wechsel unter Last veraltet ankommt und einen Teilnehmer falsch zeigt. Früh-Indikator: Unit-Test mit vertauschter Veröffentlichung. Gegenmaßnahme: `stand` (Regel 3, 5), Test zuerst.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (Anwesenheit, gestellte Uhr): `beitreten` bei t0 → Wechsel `online`, `stand` 1; `pflegen(t0+999 ms)` → nichts; `pflegen(t0+1 s)` → `offline`, `stand` 2; `verbunden` → `online`, 3; zweiter Halter, einer fällt → nichts; letzter fällt bei t1, neuer Halter bei t1+500 ms → nichts, `pflegen(t1+2 s)` → nichts; letzter fällt bei t2, `pflegen(t2+1 s)` → `offline`; jeder `stand` streng steigend; `teilnehmer()` nennt den aktuellen `stand`.
- [ ] Erster Integrationstest (echtes Programm): A und B treten bei; B öffnet den Strom → erstes Ereignis `teilnehmer` mit A (offline) und B (online); A öffnet den Strom → B erhält `anwesenheit` A online; A schließt → B erhält A offline in < 2 s; SIGTERM bei offenem Strom → Exit 0 in < 1 s.
- [ ] Erster Node-Test (Oberfläche): Zusammenführung — Stand mit `stand` 5, Wechsel mit `stand` 4 wird verworfen, mit 6 übernommen, `weg` entfernt die Zeile; Gruppierung nach Designsystem (7 Agenten ungruppiert, 8 nach Herkunft).
- [ ] I0006: Probe stiller Abriss (ADR-0007), `verlassen`, `weg_um` für die Restzeit.

## Quellen / Bezuege

- [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md) (primär), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00003 Start](../Architektur/A00003-start.md) (Takt, Ende-Signal); Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00007](../Architektur/Aspekte/X00007-zeit.md), [X00008](../Architektur/Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)
- ADR-0007, ADR-0010, ADR-0012, ADR-0015
- Vision R00000, „Wer da ist, ist sichtbar“; Designsystem `components/Teilnehmerliste/README.md`; kanbanD `src/web/mod.rs` (`ereignisstrom`, Q04, nur gelesen)

## Ergänzungen

- **2026-10-28 (Karte #5, Nachzug aus #4) — Regel 3 und 4, wer veröffentlicht:** Veröffentlicht wird **von der Anwesenheit unter ihrer Sperre**, nicht mehr vom Handler oder vom Takt nach dem Freigeben. Die Integration gibt der Anwesenheit beim Bau eine Funktion mit (`Anwesenheit::mit_veroeffentlichen`, im Programm `live::bus::anwesenheit_veroeffentlichen` → `broadcast::send`, synchron, ohne `await`); jede Operation, die Wechsel erzeugt (`beitreten`, `verbunden`, `pflegen`, ab I0006 `verlassen` und verschobenes `weg_um`), ruft sie für jeden Wechsel, solange sie die Sperre hält (`melden_unter_der_sperre`). Damit ist die Reihenfolge auf dem Bus die Reihenfolge von `stand`; der Empfänger-Schutz über `stand` (Regel 5) bleibt als zweite Linie. Der Takt ruft nur noch `pflegen(uhr.jetzt())`; Rückgabewerte sind weiter die Wechsel (für Tests). Abonnieren und Stand geschehen unter **einer** Sperre (`stand_mit`), und vor jedem Stand pflegt die Anwesenheit unter derselben Sperre (Abruf aktuell, Nacharbeit 2, US-4). Grund: Code-Review #4 B1/B2 — zwei Veröffentlichungen nach dem Freigeben konnten sich überholen, ein Strom konnte einen Wechsel mit kleinerem `stand` nach dem Stand sehen. Belege: Commits b3645cf (Nacharbeit 1), f6e446f (Nacharbeit 3); `src/anwesenheit/bestand.rs`, `src/live/takt.rs`, `src/http/ereignisse.rs`.
- **2026-10-28 (Karte #5, Nachzug aus #4) — „jetzt“ unter der Sperre, monoton:** Jede Operation liest „jetzt“ erst, **nachdem** sie die Sperre hat, und nie früher als die zuletzt unter der Sperre verwendete Zeit (`Bestand::monoton`, `bestand_und_jetzt`); auch `pflegen(jetzt)` hebt ein übergebenes, älteres „jetzt“ auf diese Zeit an. Ein Abruf, der die Sperre nach dem Takt bekommt, oder eine zurückspringende Uhr dreht so keinen Wechsel zurück (Code-Review #4 Nacharbeit 2, B1; Commit f6e446f). Die Wanduhr bestimmt weiter nur, **wann** gefragt wird ([X00007](../Architektur/Aspekte/X00007-zeit.md)).
- **2026-10-28 (Karte #5, Nachzug aus #4) — Wortlaut von `seit`:** `seit` ist der Zeitpunkt (Serverzeit, RFC 3339 mit Millisekunden), **ab dem der gemeldete Status gilt**: bei `online` der Beitritt bzw. der Beginn des ersten Halters nach einem `offline`; bei `offline` das Ende des Nachlaufs, also `max(Beitritt, Ende des letzten Halters) + 1 s`; bei `weg` der Zeitpunkt, zu dem die Anwesenheit die Sitzung entfernt hat (Takt nach Ablauf der Gnadenfrist — dann ist `seit` > `weg_um` — oder `verlassen`). So steht es in R00004 und im Code (`Haltung::status`, `abgelaufene_melden`).
- **2026-10-28 (Karte #5, I0006):** „Nicht hier“ ist entschieden — stiller Abriss in [ADR-0017](0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md), `POST /api/verlassen` in [ADR-0018](0018-verlassen-sofort-weg-nie-beim-schliessen.md), Restzeit `weg_um` und `jetzt` in Stand, Wechsel und Herzschlag in [ADR-0019](0019-restzeit-weg-um-in-serverzeit-und-statusformen.md).
