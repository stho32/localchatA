# ADR-0007: Live-Kanal — SSE für Browser, Long-Poll für Agenten, ein Ereignisbus

**Status**: Vorgeschlagen
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #1 I0001) — wartet auf eine Probe (stiller Abriss, siehe Aufgaben)
**Kontext-Anforderung**: — (WBS I0005, I0006, I0009, I0015, I0016)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (axum 0.8, `--release`): ein Client schließt eine offene SSE-Verbindung bzw. einen offenen Long-Poll — der Server lässt Strom bzw. Handler in beiden Fällen **sofort** fallen (innerhalb der Messgenauigkeit der Probe, < 100 ms); Probe danach gelöscht

## Kontext

Neue Nachrichten erscheinen sofort in allen offenen Sichten (Vision „Live überall“); ein Agent wartet blockierend auf Neues für sich, ohne Polling-Schleife (WBS I0016); bricht eine Verbindung ab, steht der Teilnehmer sofort als offline (Vision, I0006). Agenten laufen meist als Shell-Befehle (`curl`) in KI-Sessions: ein Aufruf, der blockiert und mit dem Ergebnis endet, passt besser als ein Strom, den man fortlaufend auswerten muss. kanbanD nutzt SSE mit Herzschlag (15 s) und einen Client-Wächter (40 s Stille → neu verbinden).

## Optionen

### Option A: SSE für Browser + Long-Poll `GET /api/warten` für Agenten, beide aus einem Ereignisbus (`tokio::sync::broadcast`)
- Vorteile: Browser-Teil ist die erprobte kanbanD-Antwort (`EventSource` verbindet selbst neu); Agent schreibt `curl "…/api/warten?seit=<zeit>&bis=600"` und bekommt genau dann eine Antwort, wenn etwas für ihn da ist; Senden bleibt normales `POST`; der offene SSE-Strom bzw. Warte-Aufruf ist zugleich das Anwesenheitssignal.
- Nachteile: zwei Auslieferungswege; ein Agent ist zwischen zwei Warte-Aufrufen ohne Verbindung (Anwesenheit braucht eine Gnadenfrist, ADR-0008).

### Option B: WebSocket für alle
- Vorteile: ein Kanal in beide Richtungen; Abbruch sofort sichtbar.
- Nachteile: für Agenten in der Shell unhandlich (kein `curl`-Einzeiler); eigenes Wiederverbinden im Browser; mehr Protokoll für eine Richtung, die kaum gebraucht wird.

### Option C: SSE für alle, auch Agenten
- Vorteile: ein Weg.
- Nachteile: der Agent muss einen Strom offen halten und auswerten — „blockierend warten und reagieren“ wird zur Schleife über Zeilen; Verbindungsende ist für ihn kein natürliches Ende eines Befehls.

### Option D: Nur Long-Poll für alle
- Vorteile: ein Weg, einfache Server-Logik.
- Nachteile: Browser hat nach jeder Antwort eine Lücke ohne Verbindung — Anwesenheit flackert, und Nachrichten kommen in Schüben.

## Entscheidung

**Gewählt (vorgeschlagen): Option A**

Jede Sorte Teilnehmer bekommt den Weg, der zu ihr passt, und beide speisen sich aus demselben Bus und demselben Index. In Kauf genommen: zwei Auslieferungswege und Anwesenheit über Gnadenfrist für Agenten. Regeln:

- `/api/ereignisse` (SSE): je Ereignis `data: {art, …}`; `event: herzschlag` alle 15 s (Testknopf `LOCALCHATA_HERZSCHLAG_MS`); Client: 40 s Stille → neu verbinden (Vorbild kanbanD `web/verbindung.js`). Nach dem Wiederverbinden holt der Browser per `seit` nach.
- `/api/warten?seit=<zeit>&bis=<sekunden>[&erwaehnung][&direkt][&channel=…]`: erst nachholen aus dem Index (alles nach `seit`, was der Filter durchlässt), sonst auf den Bus warten; antwortet mit der Liste oder nach `bis` (höchstens 3600 s) mit 200 und leerer Liste; die Antwort nennt die `seit`-Zeit für den nächsten Aufruf.
- Ein Halter der Anwesenheit lebt so lange wie Strom bzw. Warte-Aufruf (Drop → offline).

**Vorgeschlagen**, weil eine Annahme offen ist: Ein **stiller** Abriss (WLAN weg, Rechner schläft) wird serverseitig erst bemerkt, wenn ein Schreiben scheitert. Die Probe hat nur den sauberen Abbruch (Tab zu, `curl` abgebrochen) gezeigt.

## Konsequenzen

**Positiv:**
- „Sofort offline“ beim Schließen eines Tabs oder Abbrechen eines Agenten ist durch die Probe belegt.
- Kein Polling: ein wartender Agent kostet einen ruhenden Task.

**Negativ / Inkauf zu nehmen:**
- Für einen stillen Abriss gilt „offline nach höchstens Herzschlag + TCP-Zeitüberschreitung“, nicht „sofort“.

**Folgeentscheidungen / Aufgaben:**
- [ ] Probe als Test (I0006): Client verschwindet ohne FIN (z. B. Netz-Namespace oder Firewall-Regel im Test, sonst Herzschlag-Zeitüberschreitung im Server: kein erfolgreicher Herzschlag-Schreibvorgang binnen 2 Takten → Halter fallen lassen); messen, wann der Teilnehmer offline steht. Danach Status `Akzeptiert`.
- [ ] Antwortform von `warten` (eine Nachricht oder alle bis dahin) — `planer` bei I0016; Empfehlung: alle aufgelaufenen, höchstens 100.

## Quellen / Bezuege

- [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md); ADR-0005, ADR-0008
- kanbanD `src/web/mod.rs` (`ereignisstrom`), `web/verbindung.js` (Q04, nur gelesen); Vision „Live überall“, „Agenten können warten und reagieren“
