# ADR-0018: `POST /api/verlassen` — sofort `weg`, Name frei; die Oberfläche ruft es nie beim Schließen

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #5 I0006)
**Kontext-Anforderung**: — (WBS I0006; prägt I0014, I0016)
**Erprobt durch**: — (folgt dem Muster der bestehenden Schreibrouten aus ADR-0012/0014; Probe 1 aus [ADR-0017](0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md) zeigt, dass ein beendeter Strom seinen Halter in < 1 ms fallen lässt)

## Kontext

- [ADR-0012](0012-identitaet-und-sitzung.md), Folgeaufgabe: „I0006: explizites `POST /api/verlassen` (Empfehlung)“. Einen Namenswechsel gibt es nicht (422); wer anders heißen will, muss die Sitzung beenden. Ein Agent, der seine Arbeit beendet, hält seinen Namen sonst die Gnadenfrist lang (60 s, mit I0014 bis 3600 s) — ein Neustart unter demselben Namen von anderer Herkunft scheitert mit 409.
- Vision „Wer da ist, ist sichtbar“: **bricht die Verbindung ab**, steht der Teilnehmer sofort als offline und verschwindet nach etwa einer Minute — das Schließen des letzten Tabs ist genau dieser Fall und mit I0005 gebaut (Halter + Nachlauf, [ADR-0016](0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)).
- [ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md): jede nicht lesende Methode unter Origin-/`Sec-Fetch-Site`-Prüfung; Sicherheitsheader auf jeder Antwort. [ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md): das Token geht **nur** im Header `X-LocalchatA-Sitzung`, nie in URL oder Körper — ein Nachweisweg.
- Browser: `navigator.sendBeacon` kann keine eigenen Header setzen (Beacon API); `fetch(…, {keepalive: true})` kann es. `pagehide` feuert beim Schließen **und** beim Neuladen und Wegnavigieren; ein Tab kann beides nicht verlässlich unterscheiden. Alle Tabs eines Browsers teilen eine Sitzung (ADR-0012), das Schließen eines von mehreren Tabs darf sie nicht beenden.
- Das Designsystem kennt keine Bedienung „Verlassen“ oder „Namen ändern“ (`Dokumentation/Wireframes/designsystem/`, Kopfzeile, ScreenFlow; `3-offene-fragen.md` Frage 10 meint Channels).

## Optionen

Die Route selbst ist unstrittig (Empfehlung aus ADR-0012); erwogen wurde, **wann die Oberfläche sie ruft**.

### Option A: Nie automatisch — nur Agenten und eine ausdrückliche Handlung des Menschen rufen `verlassen`; Schließen bleibt „Verbindung bricht ab“
- Vorteile: entspricht der Vision wörtlich (Abbruch → offline, eine Minute Rückkehrfrist); Neuladen verliert nie den Namen; keine Sonderwege beim Entladen; ein Nachweisweg.
- Nachteile: wer den Browser schließt, hält seinen Namen noch eine Minute; ohne eine Bedienung im Designsystem kann ein Mensch seinen Namen nicht sofort freigeben.

### Option B: Beim `pagehide` des letzten Tabs `fetch('/api/verlassen', {method:'POST', keepalive:true, headers:{X-LocalchatA-Sitzung}})`
- Vorteile: Header möglich, also kein zweiter Nachweisweg; Name nach dem Schließen sofort frei.
- Nachteile: `pagehide` kommt auch beim Neuladen — jeder Neuladen des letzten Tabs gibt den Namen frei, und ein anderer kann ihn in der Lücke übernehmen (Pre-Mortem 3 in ADR-0012); „letzter Tab“ ist ohne Web Locks (unsicherer Kontext, LAN-Normalfall) nicht verlässlich zu bestimmen; widerspricht der Rückkehrfrist der Vision.

### Option C: `navigator.sendBeacon('/api/verlassen', token)` beim Entladen
- Vorteile: zuverlässigste Zustellung beim Entladen.
- Nachteile: kein Header — das Token müsste in Körper oder URL: ein zweiter Nachweisweg an der Grenze, in der URL zudem gegen ADR-0015; dazu alle Nachteile von B.

### Option D: Der `SharedWorker` ruft `verlassen`, wenn sich der letzte Tab abmeldet
- Vorteile: der Worker kennt alle Tabs; Header möglich.
- Nachteile: ein `MessagePort` meldet das Schließen eines Tabs nicht verlässlich; Neuladen sieht für den Worker aus wie Schließen; Rückfall ohne Worker hat keinen solchen Ort.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass Schließen und Neuladen für den Browser nicht unterscheidbar sind und die Vision für den Abbruch ausdrücklich eine Rückkehrfrist will — ein automatisches `verlassen` würde jedes Neuladen zum Namensverlust machen. `sendBeacon` scheidet zusätzlich aus, weil es das Token aus dem Header zwingen würde. In Kauf genommen: ein Mensch, der den Browser schließt, belegt seinen Namen noch etwa eine Minute (so will es die Vision), und einen sofortigen Austritt im Browser gibt es erst, wenn das Designsystem eine Bedienung dafür hat.

Regeln:

1. **Route**: `POST /api/verlassen` ohne Körper, unter `grenzschutz` (Origin-Prüfung wie jede Schreibroute). Ohne Header `X-LocalchatA-Sitzung` → 401 `{fehler}`.
2. **Wirkung**: Header mit gültigem Token **derselben Herkunft** → unter der Sperre der Anwesenheit wird die Sitzung entfernt, `stand` wächst, der Wechsel `{status: "weg", seit: jetzt}` wird unter der Sperre veröffentlicht — also **bevor** die Antwort geht; danach 204 mit Header-Satz und `Cache-Control: no-store`. Der Name ist ab der Antwort frei.
3. **Idempotent und ohne Orakel**: unbekanntes, abgelaufenes oder fremdes Token (andere Herkunft) → 204 **ohne Wirkung**. Eine fremde Herkunft beendet nie eine Sitzung, und die Antwort verrät nicht, ob das Token galt.
4. **Eigene Ströme enden**: ein Strom, der den Wechsel `weg` seiner **eigenen** Sitzung sieht, sendet ihn noch und endet dann. Halter, deren Sitzung entfernt ist, ändern beim `Drop` nichts (heutiges Verhalten: kein Eintrag, keine Wirkung) — eine verlassene Sitzung lebt nie wieder auf.
5. **Kein Lebenszeichen**: `verlassen` zählt nicht als Lebenszeichen und hat keinen Nachlauf — `online` → `weg` ohne Zwischenstand `offline`.
6. **Oberfläche**: kein Aufruf bei `pagehide`, `beforeunload`, `visibilitychange` oder aus dem Worker. Bekommt das Designsystem eine Bedienung („Verlassen“ an der eigenen Kennung in der Kopfzeile), gilt: der Tab entfernt zuerst das Token aus dem `localStorage` (der gemerkte Name bleibt als Vorschlag), ruft dann `verlassen` mit dem Header über `api.js`; ohne Token im `localStorage` tritt kein Tab still wieder bei, sondern zeigt den Beitreten-Bildschirm; der Strom verbindet nach dem eigenen `weg` nicht neu (sonst stünde ein 401 in der Konsole).
7. **Agenten**: `curl -X POST -H 'X-LocalchatA-Sitzung: <token>' <adresse>api/verlassen` → 204; die Hilfe bzw. OpenAPI-Beschreibung (`/api/openapi.json`, wenn es sie gibt) nennt die Route.

## Konsequenzen

**Positiv:**
- Ein Agent gibt seinen Namen am Ende seiner Arbeit sofort frei und kann unter demselben Namen von woanders neu beginnen.
- Neuladen und mehrere Tabs bleiben unberührt; der Abbruch folgt weiter der Vision.
- Die Route ist mit gestellter Uhr und ohne Wartezeit prüfbar: nach 204 sofort `beitreten` mit demselben Namen → 201.

**Negativ / Inkauf zu nehmen:**
- Im Browser kein sofortiger Austritt, solange das Designsystem keine Bedienung hat (Richtungsfrage an den Betreiber über den `planer`).
- 204 auch für ungültige Tokens: ein Agent mit falschem Token merkt seinen Fehler hier nicht (er merkt ihn bei jeder anderen Route als 401).

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … eine fremde Seite im Browser eines Teilnehmers dessen Sitzung beendet. Früh-Indikator: `weg` ohne Zutun. Gegenmaßnahme: Origin-Prüfung (ADR-0014 Regel 2) und der eigene Header, den ein fremder Ursprung nur nach einer nie beantworteten Vorabfrage senden darf (Regel 4); Durchsetzung: Integrationstest `POST /api/verlassen` mit `Origin: http://evil.example` → 403, Sitzung lebt.
2. … ein verlassener Browser sich still wieder anmeldet, weil der Worker auf 401 neu beitritt. Früh-Indikator: nach „Verlassen“ steht derselbe Name wieder online. Gegenmaßnahme: Regel 4 und 6 (kein Token → kein stiller Beitritt, kein Neuverbinden nach eigenem `weg`).
3. … ein Wechsel `weg` vor dem letzten `online` ankommt und eine Geisterzeile stehen bleibt. Früh-Indikator: Zeile eines Verlassenen bleibt. Gegenmaßnahme: Veröffentlichung unter der Sperre mit `stand` (ADR-0016, Ergänzung), Empfänger merkt sich `stand` entfernter Zeilen (`web/teilnehmerliste.js`, `entfernt`).

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Test (Unit, Anwesenheit, gestellte Uhr): `alpha` tritt bei (stand 1), `verlassen(token, herkunft)` → Wechsel `weg`, stand 2, `seit` = jetzt, veröffentlicht unter der Sperre; `teilnehmer()` ohne `alpha`; `beitreten("alpha")` von anderer Herkunft sofort ok; `verlassen` mit demselben Token erneut → kein Wechsel; mit fremder Herkunft → kein Wechsel, Sitzung lebt.
- [ ] Erster Test (Integration, echtes Programm): `alpha` (127.0.0.2) und Beobachter `beta` (127.0.0.3) mit Strom; `alpha` hält einen Strom; `POST /api/verlassen` von `alpha` → 204; `beta` erhält `weg` für `alpha` (Startpunkt: 204 erhalten; erwartet wenige ms, Grenze 1 s wie Q3); `alpha`s Strom liefert das eigene `weg` und endet; `beitreten("alpha")` von 127.0.0.4 → 201. Ohne Header → 401; fremder Origin → 403; fremde Herkunft → 204, `alpha` lebt.
- [ ] Bedienung „Verlassen“ im Browser: Richtungsfrage an den Betreiber (Designsystem), nicht Teil des Hauptwegs von I0006.

## Quellen / Bezuege

- [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md) (primär), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md); Aspekt [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md)
- ADR-0012, ADR-0014, ADR-0015, ADR-0016
- W3C Beacon (`sendBeacon` ohne eigene Header), WHATWG Fetch (`keepalive`), WHATWG HTML (`pagehide` bei Entladen und Navigation)

## Ergänzungen

- **2026-10-28 (Karte #5 I0006, Security-Review #5, Muss, CWE-770) — verlassene Plätze zählen bis zum Ende der Gnadenfrist:** Anlass: Regel 2 entfernte die Sitzung sofort und gab damit auch ihren Platz in den Grenzen aus [ADR-0012](0012-identitaet-und-sitzung.md) (`JE_HERKUNFT = 64`, `GESAMT = 512`) sofort frei. `beitreten`/`verlassen` ließen sich so ohne Bremse in Schleife fahren — gemessen 2.456 Wechsel/s je Zuschauer, ein Zuschauer-Tab war 28,7 s eingefroren. Vorher begrenzte die Gnadenfrist die Beitritte je Herkunft. Entscheidung des Directors nach Empfehlung des security-reviewer.
  - **Regel 2a (Wortlaut):** Ein durch `verlassen` entfernter Platz zählt für `JE_HERKUNFT` und `GESAMT` weiter, bis die Gnadenfrist der Sitzung, gerechnet ab dem `verlassen`, abgelaufen ist. Der Name ist sofort frei, das Token sofort ungültig, und der Platz erscheint in keinem Stand und keinem Wechsel. Eine Herkunft kann damit nicht mehr als 64 Beitritte je Gnadenfrist erzeugen.
  - Geprüft wird beim Beitreten gegen den Uhr-Provider, unter der Sperre der Anwesenheit, wie die lebenden Sitzungen; das Auslaufen eines reservierten Platzes ist kein Wechsel und erhöht `stand` nicht. Regel 3 (204 ohne Wirkung) und Regel 5 (`verlassen` ist kein Lebenszeichen) bleiben unverändert — ein idempotent wiederholtes `verlassen` reserviert keinen zweiten Platz.
  - **Positiv:** Ein Agent kann seinen Namen weiter sofort von woanders neu belegen, wie Regel 2 es zusichert. Die Beitrittsrate je Herkunft ist wieder durch die Gnadenfrist begrenzt (höchstens 64 je Gnadenfrist), wie vor `verlassen`; die Zuschauer bekommen höchstens so viele `online`/`weg`-Wechsel, wie diese Grenze zulässt. Mit gestellter Uhr prüfbar: 64 × (`beitreten`, `verlassen`) → der 65. Beitritt 429; nach Ablauf der Gnadenfrist (+ 1 ms) → 201.
  - **Negativ:** Ein Agent, der in einer Gnadenfrist mehr als 64-mal beitritt und verlässt, bekommt 429 (Text wie heute: höchstens 64 je Herkunft). Die Zahl der lebenden Sitzungen einer Herkunft kann darum kleiner als 64 sein, während die Grenze schon greift; ein Agent mit langer Gnadenfrist (`gnadenfrist_s` bis 3600, I0014) hält seinen verlassenen Platz entsprechend länger. Mehrere Agenten hinter einer Herkunft teilen sich die 64 Plätze einschließlich der reservierten.
  - **Erster Test (Unit, Anwesenheit, gestellte Uhr):** 64-mal `beitreten` + `verlassen` von 127.0.0.2 → der 65. `beitreten` von dort mit `Beitrittsfehler` „zu viele“, von 127.0.0.3 ok; derselbe Name sofort nach `verlassen` von 127.0.0.3 ok; Uhr + Gnadenfrist + 1 ms → Beitritt von 127.0.0.2 wieder ok; zweimal `verlassen` mit demselben Token belegt nur einen Platz.
