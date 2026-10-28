---
id: R00004
status: Neu
datum: 2026-10-28
---

# R00004: Teilnehmer mit Online-Status sehen

## Beschreibung

Wer beigetreten ist, sieht alle aktiven Teilnehmer mit voller Kennung `name@herkunft`, ob Mensch oder Agent, und ob sie gerade online oder offline sind. Ein Mensch sieht die Liste im Browser rechts neben dem Verlauf und live: schließt jemand seinen Browser, steht er bei allen anderen in weniger als 2 s als offline; meldet er sich bis zum Ende der Gnadenfrist nicht zurück, verschwindet seine Zeile. Skripte und Agenten lesen dieselbe Liste über `GET /api/teilnehmer` oder den Ereignisstrom `GET /api/ereignisse`.

Mit R00004 entsteht der Live-Kanal der Instanz: ein Ereignisstrom je Browser, den alle Tabs teilen, der sich nur über den Sitzungs-Header ausweist, mit dem Ende des Programms selbst schließt und später (I0009, I0012, I0013) auch Nachrichten trägt.

Zahlt ein auf: [Vision](R00000-vision.md) — „Wer da ist, ist sichtbar“ (Liste aller aktiven Teilnehmer mit Online-Status; Abbruch → offline; ohne Rückkehr nach etwa einer Minute aus der Liste) und „Live überall“.

Lesart der Vision, angesprochen statt überschrieben: „sofort offline“ heißt hier „in weniger als 2 s“ (Qualitätsziel Q4 in A00001; Nachlauf 1 s gegen Flackern beim Neuladen, ADR-0016). Gilt nur für den sauberen Abbruch (Tab oder Browser zu, Strom geschlossen); der stille Abriss (WLAN weg, Rechner schläft) bleibt bei I0006.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0005 Teilnehmer mit Online-Status sehen): F0011 Teilnehmerliste über die API abfragen, F0012 Anwesenheit live im Ereignisstrom, F0013 Teilnehmerliste im Browser, F0014 Ein Strom je Browser, F0015 Teilnehmerliste ein- und ausblenden.

## Geschäftlicher Nutzen

In einem Chat, in dem Menschen und Agenten zusammenarbeiten, ist die erste Frage „Wer ist da?“: lohnt es, @claude zu fragen, oder ist die Factory gerade gar nicht verbunden? Ohne Liste schreibt man ins Leere. R00004 beantwortet die Frage für Menschen (Spalte im Browser) und für Agenten (API) aus derselben Quelle. Weil alle Tabs eines Browsers einen Strom teilen, bleibt die Instanz auch mit vielen offenen Tabs bedienbar — die 6 Verbindungen, die ein Browser je Rechner hält, wären sonst ab dem 7. Tab erschöpft (Probe in ADR-0015). Und weil verschwundene Teilnehmer nach der Gnadenfrist aus der Liste gehen, stehen dort keine Geisterzeilen.

## Funktionale Anforderungen

### Liste über die API (F0011, ADR-0016)

- `GET /api/teilnehmer` mit gültiger Sitzung (Header `X-LocalchatA-Sitzung`) → 200 `{stand, liste}`; `liste` enthält **alle lebenden Sitzungen** (auch die eigene), nach Kennung sortiert, je `{kennung, name, herkunft, art, status, seit}`; ohne, mit unbekanntem oder fremdem Token → 401 `{fehler}`.
- **Jeder sieht alle**, samt Herkunft und Art — es gibt keine Sichtbarkeitsregel für Anwesenheit (X00002); ohne Sitzung gibt es keine Liste.
- **Status**: `online`, solange die Sitzung mindestens einen Halter hat (offener Ereignisstrom) oder seit `max(Beitritt, Ende des letzten Halters)` weniger als 1 s vergangen ist (Anlauf bzw. Nachlauf); sonst `offline`. Eine Sitzung, die nie einen Strom öffnet (Agent, der nur sendet), steht nach 1 s `offline`.
- `seit` ist der Zeitpunkt (RFC 3339), ab dem der Status gilt: bei `online` der Beitritt bzw. der Beginn des Halters nach einem `offline`, bei `offline` das Ende des Nachlaufs (Halter-Ende bzw. Beitritt + 1 s).
- `stand` ist eine Zahl, die bei jedem Statuswechsel um mindestens 1 wächst; sie ordnet Liste und Wechsel im Strom.
- Eine Sitzung, deren letztes Lebenszeichen länger als die Gnadenfrist zurückliegt, ist **weg** und steht in keiner Liste (ADR-0012; Lebenszeichen sind jede Anfrage mit Token und das Ende des letzten Halters).

### Anwesenheit live im Ereignisstrom (F0012, ADR-0015, ADR-0016, ADR-0007)

- `GET /api/ereignisse` mit gültiger Sitzung → 200 `text/event-stream`; das Token zählt **nur im Header**, auch für diesen Strom — `?sitzung=<token>` ohne Header ergibt 401.
- Erstes Ereignis: `data: {"art":"teilnehmer","stand":…,"liste":[…]}` (derselbe Stand wie `GET /api/teilnehmer`); danach je Statuswechsel `data: {"art":"anwesenheit","stand":…,"teilnehmer":{kennung, name, herkunft, art, status, seit}}` mit `status` `online` | `offline` | `weg`; `stand` jedes Ereignisses ist größer als der des Stands davor. Zwischen Stand und erstem Wechsel geht kein Wechsel verloren (erst abonnieren, dann Stand).
- Der offene Strom ist ein **Halter** seiner Sitzung: solange er offen ist, ist sie `online`. Schließt der Client ihn, bekommen die anderen Ströme den Wechsel `offline` nach dem Nachlauf (1 s) und spätestens beim nächsten Takt (250 ms).
- **weg**: ein Takt meldet `weg`, sobald die Gnadenfrist ohne Lebenszeichen vorbei ist; der Name ist dann frei.
- Wer zu langsam liest, bekommt einen neuen Stand (`art: "teilnehmer"`) statt einer stillen Lücke.
- **Herzschlag**: `event: herzschlag` alle 15 s; Testknopf `LOCALCHATA_HERZSCHLAG_MS` (Millisekunden), ein unlesbarer Wert beendet den Start mit Exit 2 und einer stderr-Zeile, die die Variable nennt.
- **Ende**: hört der Strom das Ende-Signal des Programms (SIGTERM, Strg+C), endet er selbst; das Programm braucht dafür nicht die 5-s-Frist der Annahmeschleife.
- `/api/gesundheit` nennt zusätzlich `sitzungen` (lebende Sitzungen) und `halter` (offene Ströme).

### Teilnehmerliste im Browser (F0013, Designsystem `components/Teilnehmerliste`, Bildschirm 4.4)

- Nach dem Beitritt zeigt die Seite neben der Kopfzeile eine Region „Teilnehmer“ mit allen Teilnehmern: zuerst **Menschen**, dann **Agenten**; ab 8 Agenten sind diese **nach Herkunft** gruppiert (Unterkopf mit der Herkunft); innerhalb jeder Gruppe alphabetisch.
- Je Zeile die volle Kennung in Mono; online: gefüllter Kreis ● ohne Wort; offline: hohler Kreis ○ mit dem Wort „offline“, Kennung gedämpft; Agenten tragen das Wort „Agent“. Die **eigene Zeile** ist markiert („du“). Status nie nur über Farbe.
- Wechsel erscheinen ohne Neuladen und ohne Animation; Zeilen springen nicht; `weg` entfernt die Zeile, die darunter rücken nach; die Liste sagt dem Screenreader nichts an.
- Der Browser hält den Strom mit dem Sitzungs-Header (`fetch`, nicht `EventSource`) und verbindet nach einem Abbruch selbst neu (nach 1 s, 2 s, 5 s, dann alle 10 s; 40 s ohne Byte gilt als Abbruch). Antwortet der Strom 401 (Neustart, Gnadenfrist), tritt die Seite still mit dem gemerkten Namen wieder bei (wie R00003) und verbindet neu.
- Während die Verbindung fehlt, zeigt die Seite einen Hinweis in Fehlergestalt (⚠ und Wort, `kd-danger`); nach dem Wiederverbinden verschwindet er, und die Liste ist wieder aktuell.
- Text vom Server nur als Text, nie als HTML.

### Ein Strom je Browser (F0014, ADR-0015)

- Alle Tabs desselben Browsers teilen **einen** Strom über einen `SharedWorker`; ein neu geöffneter Tab zeigt die Liste sofort, ohne dass sich etwas ändern muss (der Worker gibt ihm den letzten Stand).
- Schließt der letzte Tab des Browsers, endet der Strom; die anderen sehen den Teilnehmer in weniger als 2 s `offline`.
- Fehlt `SharedWorker` (z. B. Chrome auf Android), öffnet jeder Tab seinen eigenen Strom (Rückfall); die Liste stimmt trotzdem, ab 6 Tabs ist die Instanz in diesem Browser nicht mehr bedienbar (hingenommen, ADR-0015).
- Die Adresse des Workers trägt die Programmversion (`/web/strom.js?v=<version>`), damit nach einem Update kein alter Worker weiterläuft; `/api/gesundheit` nennt `version`.
- Meldet der Worker 401, tritt genau ein Tab still wieder bei und gibt das neue Token an den Worker; es entsteht kein zweiter Beitritt.

### Ein- und ausblenden (F0015, Designsystem README, `1-tastatur.md`)

- `Strg .` (überall) und ein Schalter „Teilnehmer“ in der Kopfzeile blenden die Liste ein und aus; der Schalter trägt `aria-expanded`.
- Ohne Wunsch des Menschen ist die Liste ab 1100 px Fensterbreite sichtbar und darunter eingeklappt; der Wunsch gilt bis zum Neuladen.

## Nicht-funktionale Anforderungen

- Performance: Tab bzw. Browser zu → bei den anderen `offline` in < 2 s (Q4; erwartet ≤ 1,25 s); SIGTERM bei offenem Strom → Exit 0 in < 1 s; ein Takt alle 250 ms, bei ≤ 512 Sitzungen ohne messbare Last.
- Sicherheit: genau ein Nachweisweg (Header); kein Token in einer URL, auch nicht für den Strom; `GET /api/ereignisse` erzeugt Halter und Lebenszeichen nur mit eigenem Header (Lesart ADR-0014, Ergänzung); Mutationsprobe für die Wächter dieser Routen (Sitzung nötig, Query ignoriert).
- Benutzerfreundlichkeit: bedienbar ohne Maus, Fokus sichtbar (≥ 3:1 gegen die Fläche daneben), kein Querscrollen bei 320 px, Fehler in Fehlergestalt, Kennungen in Mono, keine Konsolenfehler — auch über die LAN-Adresse und mit mehreren Tabs (`AI-Team/entwickler.md`, #3).
- Testbarkeit: Nachlauf und Gnadenfrist werden gegen den Uhr-Provider gerechnet (X00007); Testknöpfe `LOCALCHATA_GNADENFRIST_MS`, `LOCALCHATA_HERZSCHLAG_MS`; Mehr-Tab-Verhalten nur per Playwright gegen das echte Programm (X00009).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser`, `<p>` ist der Port aus der Startzeile; zweiter und dritter Absender `127.0.0.2`, `127.0.0.3` (`curl --interface …`); `S` heißt Header `X-LocalchatA-Sitzung: <token>`. Beitritte per `curl` ohne `art` sind Agenten. „Strom lesen“ heißt `curl -N -H 'X-LocalchatA-Sitzung: …' http://127.0.0.1:<p>/api/ereignisse`.

### F0011 Teilnehmerliste über die API abfragen

- [ ] `alpha` von `127.0.0.2` und `beta` von `127.0.0.3` beigetreten; `GET /api/teilnehmer` mit dem Token von `beta` → 200, `liste` enthält genau `alpha@127.0.0.2` und `beta@127.0.0.3` in dieser Reihenfolge, je mit `name`, `herkunft`, `"art":"agent"`, `status` und `seit`; Header-Satz mit `Cache-Control: no-store`.
- [ ] Anlauf und Nachlauf (Rechenbeispiel, gestellte Uhr im Unit-Test und echte Zeit im Integrationstest): `alpha` tritt bei t0 bei → bei t0 + 0,5 s `"status":"online"`, `seit` = t0; bei t0 + 1,5 s `"status":"offline"`, `seit` = t0 + 1 s; `stand` bei t0 + 1,5 s ist größer als bei t0 + 0,5 s.
- [ ] Ohne Header → 401 `{fehler}`; mit `X-LocalchatA-Sitzung: 00000000000000000000000000000000` → 401; das Token von `alpha` (127.0.0.2) von `127.0.0.3` → 401.
- [ ] Weg (`LOCALCHATA_GNADENFRIST_MS=3000`): `alpha` beigetreten bei 0 s, danach keine Anfrage mit seinem Token; `beta` (beigetreten bei 0 s) fragt bei 1 s, bei 2 s und bei 4 s → bei 1 s steht `alpha` `offline` in der Liste; bei 4 s antwortet die Instanz 200 (letztes Lebenszeichen von `beta` bei 2 s: 4 s − 2 s = 2 s ≤ 3 s), `alpha` fehlt (4 s − 0 s = 4 s > 3 s), und `{"name":"alpha"}` von `127.0.0.3` → 201.

### F0012 Anwesenheit live im Ereignisstrom

- [ ] Strom ohne Header → 401; `GET /api/ereignisse?sitzung=<gültiges token>` ohne Header → 401; mit `S` → 200 `Content-Type: text/event-stream`.
- [ ] Stand zuerst: `alpha` und `beta` beigetreten, nach 1,5 s liest `beta` den Strom → erstes Ereignis `{"art":"teilnehmer",…}` mit `alpha` `offline` und `beta` `online`.
- [ ] Online und offline live (Rechenbeispiel): `beta` liest den Strom; `alpha` öffnet seinen Strom → `beta` erhält `{"art":"anwesenheit"…}` mit `alpha` `online` in < 1 s; `alpha` schließt seinen Strom zum Zeitpunkt t1 → `beta` erhält `alpha` `offline` frühestens bei t1 + 1 s und vor t1 + 2 s.
- [ ] Neuladen flackert nicht: `alpha` schließt seinen Strom und öffnet ihn binnen 0,5 s neu → `beta` erhält für `alpha` keinen Wechsel.
- [ ] Weg im Strom (`LOCALCHATA_GNADENFRIST_MS=3000`): `gamma` tritt bei 0 s von `127.0.0.2` bei und schweigt; `beta` liest den Strom → `gamma` `offline` zwischen 1 s und 2 s, `gamma` `weg` zwischen 3 s und 4 s (Rechnung: letztes Lebenszeichen 0 s + Gnadenfrist 3 s, plus höchstens ein Takt 0,25 s).
- [ ] Reihenfolge: über alle Ereignisse eines Stroms ist `stand` streng steigend; Unit-Test der Anwesenheit nach ADR-0016 („Folgeentscheidungen“, erster Unit-Test) ist grün.
- [ ] Herzschlag: mit `LOCALCHATA_HERZSCHLAG_MS=500` liest ein Strom in 2 s mindestens 3 Zeilen `event: herzschlag` (Rechnung: 2000 / 500 = 4, eine Toleranz); `LOCALCHATA_HERZSCHLAG_MS=bald` → Exit 2, stderr nennt `LOCALCHATA_HERZSCHLAG_MS`.
- [ ] Gesundheit: zwei Sitzungen, eine mit offenem Strom → `/api/gesundheit` meldet `"sitzungen":2`, `"halter":1`; nach Schließen des Stroms `"halter":0`.
- [ ] Ende: bei offenem Strom SIGTERM an das Programm → der Client liest das Ende des Stroms, das Programm endet mit Exit 0 in < 1 s.
- [ ] Mutationsprobe: wird die Sitzungsprüfung von `/api/ereignisse` oder `/api/teilnehmer` abgeschaltet oder das Token aus dem Query gelesen, wird mindestens ein Test rot; der Entwickler nennt das Ergebnis je Wächter in seinem Bericht.

### F0013 Teilnehmerliste im Browser

Playwright mit Chromium gegen das echte Programm auf einem freien, festen Port.

- [ ] Kontext A tritt als `shoff` bei, Kontext B als `mia` → beide sehen in der Region „Teilnehmer“ unter „Menschen“ `mia@127.0.0.1` und `shoff@127.0.0.1`, je mit ●; die eigene Zeile ist als „du“ markiert.
- [ ] Kontext B wird geschlossen → A zeigt `mia@127.0.0.1` mit ○ und „offline“ in < 2 s, ohne Neuladen; mit `LOCALCHATA_GNADENFRIST_MS=3000` verschwindet die Zeile bei A in < 5 s nach dem Schließen.
- [ ] Ordnung: per API treten 7 Agenten `a1` … `a7` bei → A zeigt sie unter „Agenten“ ohne Herkunfts-Unterkopf, jede Zeile mit dem Wort „Agent“; ein 8. Agent `a8` von `127.0.0.2` → Agenten erscheinen nach Herkunft gruppiert (`127.0.0.1`: 7, `127.0.0.2`: 1), alphabetisch innerhalb.
- [ ] Zusammenführung (Node-Test, rein): Stand mit `stand` 5, danach Wechsel mit `stand` 4 → verworfen, mit `stand` 6 → übernommen; `weg` entfernt die Zeile.
- [ ] SSE-Leser (Node-Test, rein): ein Ereignis über zwei Teilstücke, `\r\n`-Zeilenenden, Kommentarzeile `: x`, zwei Ereignisse in einem Stück → je richtig zerlegt.
- [ ] Neustart: Programm beenden → A zeigt ⚠ mit Wort (Verbindung unterbrochen) in `kd-danger`; auf demselben Port neu starten → A verbindet ohne Neuladen neu, tritt still wieder bei, die Liste zeigt `shoff@127.0.0.1` ● „du“ und der Hinweis ist weg.
- [ ] Ohne Maus: die Region „Teilnehmer“ ist per `Tab` erreichbar und zeigt einen sichtbaren Fokusrahmen (≥ 3:1 gegen die Fläche daneben, beide Themen).
- [ ] Schmal: bei 320, 360 und 375 px Fensterbreite mit einem Agenten, dessen Name 32 Zeichen hat, gilt `scrollWidth <= innerWidth`; die volle Kennung steht im Text der Zeile.
- [ ] Kennungen in der Liste in Mono (`lc-id`, berechnete Schriftfamilie wie bei der Kennung in der Kopfzeile).
- [ ] Über die LAN-Adresse (Nicht-Loopback-IPv4 mit Default-Route) laden Beitreten und Liste ohne Konsolenfehler. In allen Szenarien dieses Abschnitts und in F0014 gibt es keine unbehandelte Ausnahme (`pageerror`) und keinen CSP-Verstoß; Konsolenfehler sind dort nur die Meldungen „Failed to load resource“, die der Browser selbst zu Antworten schreibt, die R00003 oder diese Anforderung vorschreiben (401 von `/api/ereignisse` und `/api/ich` nach Neustart oder Gnadenfrist, 409 und 422 von `/api/beitreten`), sowie `net::ERR_CONNECTION_REFUSED` zu `/api/…`, solange die Instanz beendet ist.

### F0014 Ein Strom je Browser

- [ ] Ein Kontext öffnet nacheinander 7 Tabs mit `/` → alle 7 zeigen die Liste; ein `fetch('/api/gesundheit')` aus Tab 7 antwortet in < 1 s; `/api/gesundheit` meldet `"halter":1` (nur dieser Browser verbunden).
- [ ] Neuer Tab ohne Änderung: Tab 1 zeigt die Liste, danach wird Tab 2 geöffnet, ohne dass sich jemand an- oder abmeldet → Tab 2 zeigt dieselbe Liste in < 2 s.
- [ ] Letzter Tab zu: Kontext B (`mia`) mit 3 Tabs; 2 Tabs zu → A zeigt `mia` weiterhin ●; den letzten zu → A zeigt `mia` „offline“ in < 2 s.
- [ ] Rückfall: Kontext C ohne `SharedWorker` (per Init-Skript entfernt) mit 3 Tabs → alle 3 zeigen die Liste, `/api/gesundheit` meldet für C 3 Halter mehr als vorher; keine Konsolenfehler.
- [ ] Version: die ausgelieferte Oberfläche legt den Worker unter `/web/strom.js?v=<version>` an, `<version>` gleich dem Feld `version` von `/api/gesundheit` (Integrationstest).
- [ ] Neustart mit 3 Tabs: Programm neu gestartet → genau ein `POST /api/beitreten` aus diesem Kontext, alle 3 Tabs zeigen danach die eigene Kennung und die Liste, `shoff` ist genau einmal belegt.

### F0015 Teilnehmerliste ein- und ausblenden

- [ ] Bei 1280 px ist die Liste sichtbar, bei 1024 px eingeklappt (Grenze 1100 px: 1099 eingeklappt, 1100 sichtbar — Node-Test).
- [ ] `Strg .` blendet die Liste aus und wieder ein; der Schalter „Teilnehmer“ in der Kopfzeile tut dasselbe mit `Enter`; `aria-expanded` folgt; der Fokus bleibt sichtbar und springt nicht an den Seitenanfang.
- [ ] Bei 320 px: Liste eingeklappt, Schalter erreichbar; eingeblendet ohne Querscrollen.

## Betroffene Verzeichnisstruktur

- Anwesenheit: `src/anwesenheit/` (Status, Halter, `pflegen`, `teilnehmer()`, `stand`; Datei je Unterthema nach Entscheidung des Entwicklers, z. B. `status.rs`, `halter.rs`).
- Live-Kanal: neu `src/live/` (A00011: Bus, Ereignisarten).
- Start: `src/start/mod.rs` (Ende-Signal als `watch`, Takt-Task, Herzschlag aus der Umgebung), `src/start/ausliefern.rs` (Ende-Signal von außen).
- HTTP: neu `src/http/teilnehmerliste.rs` (`GET /api/teilnehmer`), `src/http/ereignisse.rs` (`GET /api/ereignisse`); geändert `src/http/gesundheit.rs` (`sitzungen`, `halter`, `version`), `src/http/mod.rs` (Routen, Zustand), `src/http/eingebettet.rs` (neue Dateien, Query ignorieren), `src/http/beitreten.rs` (Wechsel veröffentlichen).
- Oberfläche: neu `web/sse.js`, `web/verbindung.js`, `web/teilnehmerliste.js`, `web/live.js`, `web/strom.js` (Modul-`SharedWorker`); geändert `web/start.js`, `web/kopfzeile.js` (Schalter), `web/lc-seite.css`.
- Tests: Unit im jeweiligen Modul; Integration `tests/teilnehmer.rs`, `tests/ereignisse.rs`; Oberfläche rein `e2e/einheit/*.test.mjs`; Abläufe `e2e/tests/r00004-*.spec.js`.

## Technische Überlegungen

### Ablauf

1. **Start** (A00003): … → Anwesenheit → Bus → Herzschlag aus `LOCALCHATA_HERZSCHLAG_MS` (unlesbar → Exit 2) → Ende-Signal als `watch` → Takt-Task (250 ms) → binden → Signale → Startzeile → ausliefern bis zum Ende-Signal.
2. **Beitreten** (wie R00003) → Wechsel `online` unter der Sperre → nach dem Freigeben veröffentlichen.
3. **`GET /api/teilnehmer`**: Extraktor `Teilnehmer` (401) → `teilnehmer()` → 200.
4. **`GET /api/ereignisse`**: Extraktor `Teilnehmer` (401) → `verbunden(token)` → Halter + Wechsel → veröffentlichen → abonnieren → `teilnehmer()` → SSE: Stand, dann Wechsel und Herzschlag bis Client-Abbruch oder Ende-Signal → Halter fällt (Halter-Ende, Lebenszeichen).
5. **Takt**: alle 250 ms `pflegen(uhr.jetzt())` → Wechsel `offline` (Nachlauf vorbei) bzw. `weg` (Gnadenfrist vorbei) → veröffentlichen.
6. **Oberfläche**: Beitritt fertig → `SharedWorker` vorhanden? → Port an `/web/strom.js?v=<version>`, Token übergeben | sonst eigener Strom → Ereignisse → Zusammenführung nach `stand` → ordnen → zeichnen; 401 → still beitreten → neues Token an den Worker.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0011: B0075 Status je Sitzung (Sitzung, jetzt → `online` | `offline` mit `seit`) · B0076 Teilnehmerstand bilden (→ `Stand {stand, liste}`, `beitreten` liefert Wechsel) · B0077 Teilnehmer beantworten (Route)
- F0012: B0078 Ende-Signal für alle (`watch` aus dem Start) · B0079 Herzschlag aus der Umgebung · B0080 Halter (`verbunden` → `Halter`, `Drop`) · B0081 Anwesenheit pflegen (jetzt → Wechsel `offline` | `weg`) · B0082 Ereignisbus · B0083 Takt und Veröffentlichen verdrahten · B0084 Ereignisse beantworten (SSE-Route) · B0085 Gesundheit nennt Sitzungen und Halter
- F0013: B0086 SSE zerlegen · B0087 Wächter entscheiden · B0088 Liste zusammenführen · B0089 Liste ordnen · B0090 Teilnehmerspalte zeichnen · B0091 Strom im Tab verbinden · B0092 Hauptansicht verdrahten
- F0014: B0093 Worker-Adresse mit Programmversion · B0094 Strom im SharedWorker halten · B0095 Worker oder Rückfall wählen · B0096 Neues Token an den Worker
- F0015: B0097 Sichtbarkeit wählen · B0098 Ein- und Ausblenden bedienen

**Elemente** (nur öffentliche Signaturen, nach A00004, A00005, A00010, A00011):

- `Anwesenheit` (einziger Schreiber, ein `Mutex`): zusätzlich `fn verbunden(&self, token: &Token) -> Option<(Halter, Vec<Wechsel>)>`, `fn pflegen(&self, jetzt: Zeit) -> Vec<Wechsel>`, `fn teilnehmer(&self) -> Stand`; `beitreten` liefert zusätzlich die Wechsel.
- `Halter` (RAII, hält `Arc` auf die Anwesenheit; `Drop` ohne `await`).
- `Wechsel { stand: u64, teilnehmer: Teilnehmerzeile, status: Status, seit: Zeit }`, `Stand { stand: u64, liste: Teilnehmerzeilen }` (unveränderlich); `Status::{Online, Offline, Weg}`.
- `Bus` (A00011): `fn veroeffentlichen(&self, ereignis: Ereignis)`, `fn abonnieren(&self) -> Abonnement`; `Ereignis::Anwesenheit(Wechsel)`.
- Oberfläche: `sse.zerlegen(rest, stueck) -> {ereignisse, rest}`; `verbindung.entscheiden(zustand, jetzt) -> {aktion, warten_ms}`; `teilnehmerliste.zusammenfuehren(liste, ereignis) -> liste`, `teilnehmerliste.ordnen(liste, eigeneKennung) -> gruppen`; `live.verbinden(token, beiEreignis, beiFehler)`.

Wireframe: die WBS führt `wireframes: Dokumentation/Wireframes/`, ein Artboard `D0002.dc.html` fehlt (Hinweis `/wireframe verfeinern D0002`). Gestaltungsquelle bis dahin: `design-prompt.md` Bildschirm 4.4 und 4.2 (Zone rechts), `Dokumentation/Wireframes/designsystem/components/Teilnehmerliste/` (README, `preview.html`), `components/Hauptansicht/README.md`, README (1100 px, Status nie nur über Farbe), `1-tastatur.md` (`Strg .`), Token `lc-online`, `lc-offline` — als Vorgabe für die Gestaltung, nicht als Akzeptanzkriterium. Der gestrichelte Zustand „offline, gleich weg“ mit Restzeit gehört zu I0006.

### Änderungen an bestehenden Klassen

- `anwesenheit::Anwesenheit`: Halterzahl, Halter-Ende und zuletzt gemeldeter Status je Sitzung; `stand` im Ganzen.
- `start::ausliefern`: bekommt das Ende-Signal von außen statt es selbst anzulegen.
- `http::Zustand`: trägt Bus, Herzschlag-Takt und Ende-Signal; `http::beitreten` veröffentlicht den Wechsel.
- `http::gesundheit::Gesundheit`: `sitzungen`, `halter`, `version`.
- `web/start.js`: nach dem Beitritt Liste und Strom; `web/kopfzeile.js`: Schalter „Teilnehmer“.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests.

**Unit (rein, gestellte Uhr):** Status mit Anlauf/Nachlauf (999 ms / 1 s); erster Unit-Test aus ADR-0016 (zweiter Halter, Wiederkehr im Nachlauf ohne Wechsel, `stand` streng steigend); `pflegen` → `weg` bei Gnadenfrist + 1 ms, Name frei; Herzschlag aus der Umgebung.

**Integration (`tests/`, echtes Programm):** 401-Fälle inklusive Query-Token; Stand zuerst; online/offline in < 2 s; kein Flackern beim Neuverbinden binnen 0,5 s; `weg` mit `LOCALCHATA_GNADENFRIST_MS=3000`; Herzschlag mit `LOCALCHATA_HERZSCHLAG_MS=500`; Gesundheit `sitzungen`/`halter`/`version`; SIGTERM bei offenem Strom → Exit 0 in < 1 s. Mutationsprobe je Wächter.

**Oberfläche rein (`e2e/einheit/`, `node --test`):** `sse.js`, `verbindung.js` (Rückzug 1/2/5/10 s, 40 s Stille), `teilnehmerliste.js` (Zusammenführung nach `stand`, Gruppierung 7/8 Agenten, eigene Zeile), Sichtbarkeit 1099/1100 px.

**E2E (Playwright):** die Szenarien aus F0013, F0014, F0015 — zwei Kontexte, Kontext zu → offline < 2 s, 7 Tabs, Rückfall ohne `SharedWorker`, Neustart, LAN-Adresse, 320 px; mindestens ein Test mit verzögert antwortendem Strom (erstes Ereignis 1 s verzögert), damit Zeichnen vor dem Stand sichtbar wird (`gemeinsam/gelerntes.md`, UI-Tests mit verzögertem Fake). Zeitkriterien am beobachtbaren Zustand der Liste festmachen, nicht an festen Wartezeiten. Dauern im Browser werden ereignisgenau gemessen (Prüfintervall ≤ 25 ms oder Animationsrahmen), nicht im Standardraster von `expect`.

## Abhängigkeiten

- Abhängig von: I0004 Namen wählen und beitreten (R00003, grün) — Sitzung, Extraktor `Teilnehmer`, Gnadenfrist, Beitreten-Bildschirm, stiller Wiederbeitritt; I0001 (R00001, grün) — Annahmeschleife, Signale.
- Innerhalb (WBS `Braucht`): F0011 frei; F0012 braucht F0011 (Status, Stand); F0013 braucht F0012 (Strom); F0014 und F0015 brauchen F0013 und sind untereinander parallel.
- Blockiert: I0006 Verbindung bricht ab (stiller Abriss, `verlassen`, `weg_um`); trägt den Live-Kanal für I0009 Nachricht senden, I0012, I0013 und I0016.

## Offene Fragen

Keine, die R00004 blockieren. Entschieden mit den Empfehlungen des architekt (Karte #4):

- Jeder Teilnehmer mit Sitzung sieht alle Teilnehmer samt Herkunft und Art (ADR-0016 Regel 6, X00002).
- Die eigene Zeile steht in der Liste und ist markiert.
- Ein Agent, der nur sendet und nie einen Strom oder Warte-Aufruf offen hat, steht als `offline`.
- „weg“ (Zeile verschwindet nach der Gnadenfrist) kommt schon mit R00004; die Restzeit „offline, gleich weg“ (`weg_um`) mit I0006.

Entschieden vom planer für R00004:

- `seit` ist der Zeitpunkt, ab dem der Status gilt (siehe Funktionale Anforderungen); ADR-0016 nennt das Feld ohne Definition — der architekt gleicht den Wortlaut an.
- Der Wunsch „Liste ein/aus“ wird nicht über ein Neuladen gemerkt; Rückmeldung aus der Abnahme abwarten.
- `/api/gesundheit` nennt `sitzungen`, `halter` und `version` (Vorschlag A00010 „Betrieb“, macht „ein Strom je Browser“ prüfbar). Die Zahlen sind ohne Sitzung lesbar; sie verraten keine Namen.

Weiter offen, ohne R00004 zu blockieren:

- **architekt**: Ein Klick auf einen Teilnehmer öffnet laut Designsystem ein Menü „Direktnachricht“ / „Erwähnen“ — gebaut mit I0012/I0013, in R00004 sind die Zeilen nicht anklickbar.

## Manuelle Vorbereitungstätigkeiten

- Keine über `AI-Team/umgebung.md` hinaus. Der LAN-Test braucht eine Nicht-Loopback-IPv4 mit Default-Route; die Absender `127.0.0.2`/`127.0.0.3` brauchen Linux-Loopback.

## Manuelle Nachbereitungstätigkeiten

- `AI-Team/umgebung.md` um den Testknopf `LOCALCHATA_HERZSCHLAG_MS` ergänzen (Director).
- Erwartung für die Abnahme bis I0006: wer den Rechner schlafen legt oder das WLAN trennt, bleibt bei den anderen `online`, bis die TCP-Verbindung serverseitig abläuft (Minuten); das ist der stille Abriss aus ADR-0007.

## Umfang

Aus der WBS gezählt (`/planung zaehlen I0005`); Standard-Bubbles mit der Untergrenze 0,4h (belegt aus `Schaetzungen/_ist-zeiten.md`, R00001–R00003), unklare als Richtwert 2–4h:

- F0011 Teilnehmerliste über die API abfragen = 3 Bubbles: 3 Standard (0,4h). Rest: 1,2h klar · 3 von 3 Werten belegt.
- F0012 Anwesenheit live im Ereignisstrom = 8 Bubbles: 7 Standard (0,4h), 1 unklar (2–4h). Rest: 2,8h klar + 2–4h unklar · 7 von 8 Werten belegt, Rest Richtwert (ungemessen).
- F0013 Teilnehmerliste im Browser = 7 Bubbles: 5 Standard (0,4h), 2 unklar (2–4h). Rest: 2,0h klar + 4–8h unklar · 5 von 7 Werten belegt, Rest Richtwerte (ungemessen).
- F0014 Ein Strom je Browser = 4 Bubbles: 3 Standard (0,4h), 1 unklar (2–4h). Rest: 1,2h klar + 2–4h unklar · 3 von 4 Werten belegt, Rest Richtwert (ungemessen).
- F0015 Teilnehmerliste ein- und ausblenden = 2 Bubbles: 2 Standard (0,4h). Rest: 0,8h klar · 2 von 2 Werten belegt.
- I0005 gesamt = 24 Bubbles. Rest: 8,0h klar + 8–16h unklar · 20 von 24 Werten belegt, Rest Richtwerte (ungemessen).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Nach R00003 kann man beitreten, sieht aber niemanden — weder ob die Gegenseite da ist noch ob ein Agent überhaupt verbunden ist; die Vision verspricht „Wer da ist, ist sichtbar“. Wenn jeder offene Strom ein Halter seiner Sitzung ist, dann ist „online“ eine beobachtbare Tatsache (dieser Browser hat localchatA offen) statt einer Vermutung aus Pings; wenn ein Nachlauf von 1 s und ein Takt von 250 ms die Wechsel erzeugen, dann sehen andere einen echten Abbruch in unter 2 s, ein Neuladen aber gar nicht; und wenn der Strom mit dem Stand beginnt und jeder Wechsel eine laufende Nummer trägt, dann stimmt die Liste in jedem Tab, auch wenn Ereignisse sich überholen. Der Hebel liegt beim Strom und nicht bei der Oberfläche, weil derselbe Strom später Nachrichten, Direktnachrichten und Erwähnungen trägt (I0009, I0012, I0013) — ein Strom je Browser mit Sitzungs-Header, einmal richtig gebaut, hält die Instanz auch mit vielen Tabs bedienbar und das Token aus jeder URL heraus.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe wie Halter, Nachlauf, Stand, Wechsel, Gnadenfrist, weg sind aus ADR-0012, ADR-0016 und A00010 übernommen.
- Verhalten von `SharedWorker` in Firefox und Safari ist im Projekt nicht erprobt (ADR-0015 probte nur Chromium).

## Notizen

### Verworfene Alternativen

- **Je Slice eine eigene Anforderung**: verworfen — alle fünf haben ein Ziel (Teilnehmer mit Online-Status sehen) und durchlaufen als Arbeitspaket I0005 gemeinsam Entwicklertest und Abnahme.
- **Nur API und Strom in R00004, Oberfläche als eigene Anforderung**: verworfen — das Fertig-Kriterium der Interaction („Liste zeigt …“) wäre für den Menschen erst mit der zweiten Anforderung erfüllt; die Abnahme prüft die Liste im Browser.
- **„weg“ erst mit I0006**: verworfen (Empfehlung architekt) — ohne `weg` blieben Zeilen von Teilnehmern stehen, deren Name längst frei ist (Geisterzeilen, Pre-Mortem 1 in ADR-0016).
- **Ende-Signal, Bus, Takt als eigene Slices** (Zuschnitt-Liste des architekt): verworfen — für sich nicht von außen prüfbar; als Bubbles in F0012 werden sie über Strom-Ende, Wechsel und Exit-Zeit geprüft.
- **Ein Strom je Browser als Ausbaustufe nach I0005**: verworfen — ADR-0015 macht ihn zur Regel, und mit einem Strom je Tab steht die Instanz ab dem 7. Tab; als eigener Slice F0014 bleibt er von außen prüfbar und nach F0013 baubar.

### Änderungen nach Prüfung

- 2026-10-28 (Karte #4, Entwicklertest): US-4 und das Kriterium „Weg“ unter F0011 — `beta` fragt zusätzlich bei 2 s ab, bei 4 s wird 200 erwartet. Grund: der alte Zeitplan widersprach der eigenen Gnadenfrist; mit dem letzten Abruf bei 1 s war `beta` bei 4 s selbst abgelaufen (401 in 2 von 8 Läufen). Geprüftes Verhalten (`alpha` bei 1 s `offline`, bei 4 s weg, Name frei) unverändert.
- 2026-10-28 (Karte #4, realist-consultant): letztes Kriterium unter F0013 — Konsole eingegrenzt: das Laden über die LAN-Adresse bleibt ohne Konsolenfehler; in den Szenarien von F0013 und F0014 gilt keine unbehandelte Ausnahme (`pageerror`), kein CSP-Verstoß, und als Konsolenfehler nur „Failed to load resource“ zu vorgeschriebenen Antworten (401 von `/api/ereignisse` und `/api/ich`, 409 und 422 von `/api/beitreten`) sowie `net::ERR_CONNECTION_REFUSED` zu `/api/…` bei beendeter Instanz. Grund: Urteil realist-consultant 2026-10-28: Kriterium widersprach vorgeschriebenen Antworten bzw. nannte den Startpunkt der Uhr nicht; vom Betreiber genehmigte Anpassung. US-19 und US-23 unverändert.
- 2026-10-28 (Karte #4, realist-consultant): US-15 und US-22 — die 2 s bis „offline“ zählen ab dem abgeschlossenen Schließen (von B bzw. des dritten Tabs); Abschnitt Tests/E2E: Dauern im Browser ereignisgenau messen (Prüfintervall ≤ 25 ms oder Animationsrahmen), nicht im Standardraster von `expect`. Grund: Urteil realist-consultant 2026-10-28: Kriterium widersprach vorgeschriebenen Antworten bzw. nannte den Startpunkt der Uhr nicht; vom Betreiber genehmigte Anpassung.

### Bewusst außerhalb

Stiller Abriss, `POST /api/verlassen`, Restzeit „offline, gleich weg“ (`weg_um`) (I0006); längere Gnadenfrist für Agenten `gnadenfrist_s` (I0014); `GET /api/warten` (I0016); Nachrichten im Strom (I0009); Menü „Direktnachricht“ / „Erwähnen“ an der Zeile (I0012, I0013); Umformung `é` → `e` im Namensvorschlag (gehört zur Namensregel aus R00003, nicht zu R00004).
