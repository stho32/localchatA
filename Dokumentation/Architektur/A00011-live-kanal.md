---
id: A00011
name: Live-Kanal
code: live
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/live/
verwandt: [A00006, A00010]
domaene: []
blackbox: Ereignisbus mit Filter; Mechanik in ADR-0007, Vorbild kanbanD ereignisstrom
---

# Live-Kanal

## Verantwortung

Verteilt jedes Ereignis an genau die offenen Sichten und wartenden Agenten, für die es sichtbar ist.

## Schnittstellen

- `veroeffentlichen(Ereignis)` — `tokio::sync::broadcast`; Ereignisarten `Nachricht{gespraech, zeit, von, erwaehnt}`, `Channel`, `Frist`, `Anwesenheit(Wechsel)` (ab I0005, mit `stand`, [ADR-0016](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)).
- `abonnieren(fuer: Kennung, filter) -> Strom` — Filter: sichtbar für (Direktgespräche nur für Beteiligte), optional nur `erwaehnung`, `direkt`, `channel=<…>`. Ab I0005: erst abonnieren, dann den `Stand` der Teilnehmerliste als erstes Ereignis, danach die Wechsel; der Strom endet mit dem Ende-Signal des Programms.
- `warten(fuer, filter, seit: Zeit, bis: Dauer) -> Vec<Nachricht>` — erst nachholen (Index, alles nach `seit`), dann auf den Bus warten; antwortet beim ersten Treffer oder nach `bis` leer.
- Wer zu langsam liest (`Lagged`), bekommt ein Ereignis „neu laden“ statt stiller Lücke (wie kanbanD); für die Anwesenheit ist das ein neuer `Stand`.

## Abhaengigkeiten

[Gespräche](A00006-gespraeche.md) (Nachholen über den Index); Uhr-Provider für Herzschlag und Zeitüberschreitung.

## Entscheidungen

- [ADR-0007: Live-Kanal — SSE für Browser, Long-Poll für Agenten, ein Ereignisbus](../ADRs/0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md) — Vorgeschlagen, 2026-10-27
- [ADR-0010: Eigene Annahmeschleife mit hyper http1 statt `axum::serve`](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md) — Akzeptiert, 2026-10-27
- [ADR-0015: Ein Ereignisstrom je Browser — SharedWorker mit `fetch`-Strom und Sitzungs-Header](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md) — Akzeptiert, 2026-10-28
- [ADR-0016: Anwesenheitsstatus aus Haltern mit Nachlauf, Teilnehmerliste als Stand plus Wechsel](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md) — Akzeptiert, 2026-10-28
- [ADR-0017: Stiller Abriss — der Kernel erkennt tote Verbindungen (Abrissgrenze und Keepalive je Verbindung), Herzschlag 5 s](../ADRs/0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md) — Vorgeschlagen, 2026-10-28
- [ADR-0018: `POST /api/verlassen` — sofort `weg`, Name frei; die Oberfläche ruft es nie beim Schließen](../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md) — Akzeptiert, 2026-10-28
- [ADR-0019: Restzeit `weg_um` in Serverzeit mit Uhrenabgleich über `jetzt`; gestrichelt heißt „gleich weg“](../ADRs/0019-restzeit-weg-um-in-serverzeit-und-statusformen.md) — Akzeptiert, 2026-10-28
- [ADR-0023: Channels als zweites Thema im vorhandenen Ereignisstrom](../ADRs/0023-channels-als-zweites-thema-im-ereignisstrom.md) — Akzeptiert, 2026-10-28
- [ADR-0028: Nachrichten live als drittes Thema im Ereignisstrom — Ordnung über die Nachrichtenzeit je Channel, kein Stand im Strom](../ADRs/0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md) — Akzeptiert, 2026-10-28 (gebaut mit I0009)
- [ADR-0031: Frist setzen über `PUT /api/channels/{name}/frist` — jeder mit Sitzung, Verkürzen nur mit Bestätigung, verteilt als Channel-Wechsel](../ADRs/0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md) — Akzeptiert, 2026-10-28
- [ADR-0035: Nachrichtenzeit und Ordnung — eine Schreibsperre über alle Gespräche, Vorlauf sichtbar](../ADRs/0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md) — Akzeptiert, 2026-10-28
- [ADR-0037: Zustellung im Tab — vorläufig, bestätigt, je Frame gebündelt](../ADRs/0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md) — Akzeptiert, 2026-10-28
- [ADR-0048: Eigene Nachrichten bearbeiten und löschen — Änderungssatz im Tageslog der Änderung, Fassung je Nachricht im Index, zwei Routen, Thema `aenderung`, der Löschsatz ist der Commit-Punkt für die Anhänge](../ADRs/0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md)
- [ADR-0051: Aktivität „tippt“ / „denkt nach“ — flüchtig im Speicher mit Ablauf, eigener Signalbus, eine Route je Gespräch, eigene Bremse](../ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) — Akzeptiert, 2026-10-30
- [ADR-0053: Antworten mit Zitat — Bezug als Metazeile `+ antwort_auf: <zeit> <kennung>`, aufgelöst beim Lesen, Feld `antwort_auf`, `zitat` im Thema `aenderung`, Sprung über den Verlauf, Wecken über die Kennung](../ADRs/0053-antworten-mit-zitat-bezug-als-metazeile-zeit-und-kennung-aufgeloest-beim-lesen-feld-antwort-auf-zitat-im-thema-aenderung.md) — Akzeptiert, 2026-10-30

## Offene Fragen

## Notizen / Quellen

- `seit` ist eine Nachrichtenzeit; weil Zeiten global streng steigen ([ADR-0005](../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md)), verliert ein Agent zwischen zwei Warte-Aufrufen nichts — auch nicht über einen Neustart des Programms.
- Erster Test (Unit, tokio): Abonnent `shoff` mit Filter `erwaehnung`; Ereignis ohne `@shoff` → nichts, mit `@shoff` → genau eins; Direktnachricht `claude→mia` erreicht `shoff` nie.
- **Ergänzung 2026-10-27 (Karte #2, Nachzug aus #1):** Die Auslieferung ([ADR-0010](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md)) bestimmt drei Randbedingungen: (1) laufende Antworten werden von der Kopf-Zeitgrenze (10 s) nicht abgeschnitten; (2) jeder Strom und jeder wartende Aufruf belegt einen der 64 Plätze seiner Gegenstelle — Tabs und Agenten eines Rechners teilen sie sich; (3) beim Ende werden offene Antworten nach 5 s abgebrochen. Deshalb hören `abonnieren` und `warten` ein Ende-Signal (wie `ende` in `src/start/ausliefern.rs`) und enden selbst: der Strom schließt, `warten` antwortet leer. Erster Test dazu (Integration): offener SSE-Strom, SIGTERM → Strom endet und Prozess endet mit Exit 0 in deutlich unter 5 s.
- **Ergänzung 2026-10-28 (Karte #4, I0005 — erster Strom):** Der Live-Kanal entsteht mit I0005, nicht erst mit I0009: Bus, `/api/ereignisse` und das Ende-Signal kommen jetzt, Nachrichten-Ereignisse und `warten` später. Der Strom ist ein Halter der [Anwesenheit](A00010-anwesenheit.md) (`verbunden` beim Start des Stroms, `Drop` beim Ende — geprobt über die eigene Annahmeschleife: 0,3 ms nach dem Schließen, auch ohne laufende Daten; die Kopf-Zeitgrenze schneidet ihn nicht ab). Im Browser hält ihn ein `SharedWorker` je Browser ([ADR-0015](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)), damit belegt ein Mensch mit vielen Tabs einen Platz statt einen je Tab. Das Ende-Signal (bisher nur intern in `ausliefern`) braucht der Strom als Empfänger aus dem Start (`watch::Receiver`, vgl. kanbanD `ende`), die Folgeaufgabe aus ADR-0010 wird mit I0005 fällig.
  - **Erster Test (Integration, echtes Programm):** siehe ADR-0016 — zwei Sitzungen, Stand als erstes Ereignis, Wechsel online/offline in < 2 s, SIGTERM bei offenem Strom → Exit 0 in < 1 s; dazu ADR-0015 — ohne Header 401, Token im Query ohne Header 401.
- **Ergänzung 2026-10-28 (Karte #5, I0006):** (1) Herzschlag 5 s statt 15 s, mit Inhalt `{"jetzt": …}` ([ADR-0017](../ADRs/0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md), [ADR-0019](../ADRs/0019-restzeit-weg-um-in-serverzeit-und-statusformen.md)); er erzeugt die unbestätigten Daten, an denen die Abrissgrenze der Verbindung läuft. (2) Ein Strom, der das `weg` seiner **eigenen** Sitzung sieht, sendet es und endet ([ADR-0018](../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md) Regel 4); dafür kennt der Strom die Kennung seiner Sitzung. (3) Wechsel kommen seit #4 aus der Anwesenheit unter deren Sperre auf den Bus (ADR-0016, Ergänzungen) — die Reihenfolge auf dem Bus ist die von `stand`.
  - **Erster Test (Integration, echtes Programm):** siehe ADR-0017 (RST-Abbruch → `offline` bei anderen < 2 s, `weg_um − seit = Gnadenfrist − 1 s`) und ADR-0018 (verlassen → eigener Strom liefert `weg` und endet).
- **Ergänzung 2026-10-28 (Karte #6, I0007):** Der Bus trägt ein zweites Thema: `Ereignis::Channel(ChannelWechsel)`. Je Thema gilt die eigene Nummer `stand` mit Veröffentlichung unter der Sperre des Bestands, der sie vergibt (Anwesenheit bzw. [Channelverzeichnis](A00012-channelverzeichnis.md)); einen gemeinsamen Zähler gibt es bewusst nicht ([ADR-0023](../ADRs/0023-channels-als-zweites-thema-im-ereignisstrom.md), Option B verworfen). Der Strom beginnt mit zwei Ständen (`teilnehmer`, `channels`); `NeuerStand` sendet beide. Jede künftige Ereignisart nennt ihr Thema und dessen Nummer.
- **Ergänzung 2026-10-28 (Karte #7, I0008 — vorbereitet für I0009):** Drittes Thema `nachricht` ([ADR-0028](../ADRs/0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md)): Ordnungsmerkmal ist die Nachrichtenzeit je Channel, veröffentlicht unter der Gesprächssperre; **kein** Stand beim Öffnen und kein `zuletzt`-Filter im Strom für dieses Thema — der Stand eines Channels ist sein Verlauf über die API ([ADR-0025](../ADRs/0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md)), Lücken nach `NeuerStand` oder Wiederverbinden holt der Tab mit `nach=<zeit>`. Jeder Browser erhält Nachrichten aller Channels; Direktgespräche filtert „sichtbar für“ (I0012).
- **Ergänzung 2026-10-28 (Karte #12, I0018):** Kein neues Thema: eine geänderte Frist ist ein Channel-Wechsel (`stand + 1`, ersetzt nach `name`), der Channel trägt `frist_tage`; derselbe Wert erzeugt kein Ereignis; entfallene Tage sind kein Ereignis ([ADR-0031](../ADRs/0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md) Regel 4, ADR-0030 Regel 5).
- **Ergänzung 2026-10-28 (Karte #8, I0009):** `Ereignis::Nachricht { channel, nachricht }` wird unter der **Schreibsperre über alle Gespräche** veröffentlicht ([ADR-0035](../ADRs/0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md)) — die Reihenfolge auf dem Bus ist damit über alle Channels die Zeitfolge, und `warten` (I0016) braucht nur **einen** Zeiger `seit`. Das berichtigt ADR-0028 Regel 8 (dort war die Busreihenfolge über Channels als belanglos angenommen; mit Sperre je Gespräch konnte ein Agent eine ältere Nachricht eines anderen Channels verlieren). Bus-Kapazität 1.024 genügt: das Schreibkonto erlaubt je Herkunft Ausbrüche bis 120. Im Tab werden Nachrichten je Frame gebündelt gezeichnet ([ADR-0037](../ADRs/0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md)); der Strom selbst ändert sich nicht.
- **Ergänzung 2026-10-28 (R00015, F0056):** Randbedingung (2) der Ergänzung vom 2026-10-27 liest sich ab F0056 mit **160** Plätzen je Gegenstelle ([ADR-0010](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md), Ergänzung R00015) — ausgelegt auf 50 wartende Agenten, ihre Sendeverbindungen in der Spitze und einen Browser auf einem Rechner.
- **Ergänzung 2026-10-28 (Karte #31; Observability-Review #8 O4):** fällt ein Abonnent um mehr als die Kapazität des Busses zurück (`Lagged(n)`), trägt `Empfang::NeuerStand` die Zahl `n`, und der Strom schreibt eine `WARNING live`-Episode je Kennung „Ereignisstrom überholt, neuer Stand gesendet“; kein Feld im Health-Check. Quelle: [ADR-0042](../ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung Karte #31, Punkt 6.
- **Ergänzung 2026-10-29 (Karte #36, R00025 F0084):** im Browser trägt der Zustand `unterbrochen` den Zeitpunkt des nächsten Versuchs `naechsterVersuchUm` (`Date.now()` des Halters); der Verteiler reicht ihn neuen Tabs nach. Quelle: [ADR-0015](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md), Ergänzung Karte #36. Soll ab F0084; Ist: Zustand ohne Zeitpunkt.
- **Ergänzung 2026-10-29 (Karte #9 Betreiber F1′), Soll:** neues Thema `anhaenge` — `{"art":"anhaenge","channel","geloescht":[{id, zeit}]}` je Gespräch und Löschanfrage, ohne Ordnungszahl und ohne Stand (endgültiger, idempotenter Zustand); Direktgespräche nur an Beteiligte (Filter „sichtbar für“); nach `NeuerStand` holt der Tab nach und übernimmt `zustand` bekannter Nachrichten ([ADR-0047](../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „Karte #9 Betreiber F1′“ Regel 3).
- **Ergänzung 2026-10-29 (Karte #48, I0032/I0033), Soll:** neues Thema `aenderung` `{art, channel, nachricht}` mit der ganzen Nachricht in neuer Fassung und `geaendert`; ohne Stand, idempotent über `geaendert`; veröffentlicht unter der Schreibsperre; weckt `warten` nicht (Empfehlung für I0016); Nachholen über die Blöcke des Fensters, Route `aenderungen?nach=` als Folgeoption (ADR-0048 Regel 11, Frage F).
- **Ergänzung 2026-10-30 (Karte #77, I0040/I0041), Soll:** Zweiter Bus **Signalbus** (`broadcast`, Kapazität 1.024) für flüchtige Signale neben dem Ereignisbus; erstes Signal `Signal::Aktivitaet{channel, kennung, aktiv}` aus [Aktivität](A00019-aktivitaet.md), im Strom `{"art":"aktivitaet","channel","kennung","zustand","bis","jetzt"}` bzw. `"zustand":null`. Der Strom abonniert beide und wählt `biased` in der Folge Ende → Ereignisbus → Signalbus → Herzschlag; er sendet nie Signale der eigenen Kennung, filtert „sichtbar für“ wie bei Nachrichten; `Lagged` auf dem Signalbus → ein `{"art":"aktivitaet_luecke"}`, kein `NeuerStand`, keine Episode. Signale stehen nie im Stand beim Öffnen, nie im Nachholen; `warten` (I0016) abonniert den Signalbus nie. `KAPAZITAET` des Ereignisbusses und seine Rechnung in A00001 bleiben. Quelle: [ADR-0051](../ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) Regeln 5, 6.
- **Ergänzung 2026-10-30 (Karte #79, I0043/I0044), Soll:** Thema `nachricht` trägt `antwort_auf` (aufgelöst unter der Schreibsperre beim Veröffentlichen); Thema `aenderung` trägt zusätzlich `zitat` — das Bezugsobjekt der geänderten Nachricht selbst —, damit Tabs Zitate ohne Rückwärtsverzeichnis nachführen; kein neues Thema. `warten` (I0016) weckt zusätzlich die Sitzung, deren Kennung `antwort_auf.von` ist (nicht die eigene Antwort), gelesen aus der Metazeile ohne Nachschlagen. Regeln in ADR-0053 Regeln 3, 4, 9.
