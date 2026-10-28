# ADR-0019: Restzeit `weg_um` in Serverzeit mit Uhrenabgleich über `jetzt`; gestrichelt heißt „gleich weg“

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #5 I0006)
**Kontext-Anforderung**: — (WBS I0006; prägt I0012 Seitenleiste, I0014)
**Erprobt durch**: — (Rechnung aus dem Bestand: `src/anwesenheit/bestand.rs` `lebt`, `Halter::drop`; Designsystem `components/Teilnehmerliste`)

## Kontext

- Designsystem `components/Teilnehmerliste/README.md`: „offline, gleich weg: **gestrichelter Kreis** und Restzeit ‚weg in 0:40‘, sekundengenau nur für diese Zeile; die Zeile bleibt an ihrem Platz“; „Der Verbraucher liefert … bei ‚gleich weg‘ — Zeitpunkt des Verschwindens“. Dieselbe Form steht in `Seitenleiste`, `Hauptansicht`, `README.md` und `2-token-erweiterung.md` (`lc-offline`: „Offline ○ und ‚gleich weg‘ (gestrichelt)“).
- Ist-Stand (Nacharbeit 2 von #4, `web/lc-seite.css:77`, `web/teilnehmerliste.js` `onlineWort`): bei unterbrochener Verbindung („Stand unbekannt“) wird jeder ● **gestrichelt** gezeichnet und heißt „zuletzt online“. Das UX-Review von #4 merkt an: die Form ist damit doppelt belegt. Die Statusspalte ist fest 52 px breit (Platz für „○ offline“) und fasst „weg in 0:40“ nicht.
- Lebensdauer ([ADR-0012](0012-identitaet-und-sitzung.md), Code `lebt`): ohne Halter lebt eine Sitzung, solange `jetzt − letztes Lebenszeichen ≤ Gnadenfrist`. Das Ende des letzten Halters ist ein Lebenszeichen; jede Anfrage mit Token ebenso. Die Gnadenfrist ist heute 60 s für alle, ab I0014 je Agent bis 3600 s.
- Browser und Server laufen im LAN auf verschiedenen Rechnern — ihre Uhren können um Sekunden bis Minuten auseinanderliegen. Der `SharedWorker` merkt sich den letzten Stand und gibt ihn jedem neuen Tab, auch viel später ([ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md) Regel 2). Herzschläge (`event: herzschlag`) reicht der Leser heute nicht an die Tabs weiter (`web/live.js`, Filter `event === ''`).
- Testuhr `LOCALCHATA_JETZT` verschiebt die Serverzeit gegenüber der Rechneruhr (X00007).

## Optionen

### Option A: Absoluter Zeitpunkt `weg_um` (Serverzeit) je Zeile, dazu `jetzt` (Serverzeit) in Stand, Wechsel und Herzschlag; der Client rechnet mit dem Versatz zur eigenen Uhr
- Vorteile: der Wert bleibt gültig, egal wann er gelesen wird — auch im zwischengespeicherten Stand des Workers und in `GET /api/teilnehmer` für Agenten; Uhrenabweichung wird bei jedem Ereignis neu abgeglichen; erfüllt „Zeitpunkt des Verschwindens“ des Designsystems wörtlich; die Rechnung ist eine reine Funktion.
- Nachteile: zwei neue Felder und ein Herzschlag mit Inhalt; der Versatz muss neben dem gespeicherten Stand mitgeführt werden.

### Option B: Relative Restzeit `weg_in_ms` zum Zeitpunkt des Ereignisses
- Vorteile: keine Uhrenfrage; ein Feld.
- Nachteile: der Wert altert ab dem Senden — der zwischengespeicherte Stand im Worker gibt einem später geöffneten Tab eine falsche Restzeit; in `GET /api/teilnehmer` gilt er nur für den Augenblick der Antwort; jeder Neuversand ändert ihn, ohne dass sich etwas geändert hat.

### Option C: Der Server meldet die Restzeit sekündlich als Wechsel
- Vorteile: der Client zeigt nur an.
- Nachteile: ein Wechsel je Sekunde je abtretender Sitzung an jede Sicht; `stand` wächst ohne Statuswechsel; Takt müsste sekundengenau melden.

### Option D: Der Client rechnet selbst: `seit` des `offline` + Gnadenfrist aus der Beitrittsantwort
- Vorteile: keine Protokolländerung.
- Nachteile: der Client kennt die Gnadenfrist **anderer** Sitzungen nicht (I0014: je Agent verschieden) und sieht keine Lebenszeichen ohne Statuswechsel — ein offline sendender Agent liefe bei allen auf 0:00, obwohl er bleibt.

Zur Form:

| # | „gleich weg“ | „zuletzt online“ (Stand unbekannt) | Schwäche |
|---|---|---|---|
| F1 | gestrichelt (Designsystem) | gefüllter Punkt in `lc-offline` (grau statt grün), Liste mit Hinweis „Stand unbekannt“ | Nacharbeit 2 wird umgebaut; grau gefüllt vs. grün gefüllt unterscheidet sich nur in der Farbe — getragen wird die Bedeutung vom Hinweis der ganzen Liste und vom Namen „zuletzt online“ |
| F2 | hohler Kreis ○ wie offline, nur das Wort „weg in 0:40“ unterscheidet | gestrichelt (wie heute) | weicht vom Designsystem in vier Bausteinen ab; offline und gleich weg nur über das Wort getrennt |
| F3 | gestrichelt | gestrichelt (eine Form „unsicher“) | eine Form, zwei Bedeutungen — genau der Befund des UX-Reviews |

Zur Breite der Statusspalte: (S1) feste Spalte, breit genug für den längsten Statustext „weg in 1:00“ mit gleich breiten Ziffern, für alle Zeilen gleich; (S2) Spalte je Zeile nach Inhalt (`auto`) — eine Zeile, die „gleich weg“ wird, bricht ihre Kennung neu um und schiebt die Zeilen darunter (verletzt „springt nicht“); (S3) nur „0:40“ ohne Wort in 52 px — eine Zahl ohne Wort ist ohne Kenntnis des Designs unverständlich.

## Entscheidung

**Gewählt: Option A, Form F1, Spalte S1**

Es überwiegt, dass nur ein absoluter Zeitpunkt den zwischengespeicherten Stand im Worker und die Liste für Agenten richtig hält, und dass das Designsystem die Bedeutung der gestrichelten Form an vier Stellen festlegt — die Abweichung aus Nacharbeit 2 weicht, nicht das Designsystem. In Kauf genommen: zwei Felder mehr im Protokoll, der Umbau der Darstellung „zuletzt online“, und eine schmalere Kennungsspalte (lange Kennungen brechen öfter um, aber nie durch einen Wechsel).

Regeln:

1. **`weg_um`** je Teilnehmerzeile (in `Stand.liste`, in `Wechsel.teilnehmer`, in `GET /api/teilnehmer`): bei `offline` der Zeitpunkt `letztes Lebenszeichen + Gnadenfrist` der Sitzung (RFC 3339 mit Millisekunden, Serverzeit) — der letzte Augenblick, in dem die Sitzung noch lebt; bei `online` `null`. Die Sitzung ist weg, sobald `jetzt > weg_um` (gleich der heutigen Regel `lebt`).
2. **`jetzt`** (Serverzeit, unter der Sperre gelesen, monoton) im Stand (`{art:"teilnehmer", stand, jetzt, liste}`), in jedem Wechsel (`{art:"anwesenheit", stand, jetzt, teilnehmer}`) und in `GET /api/teilnehmer`; der Herzschlag trägt `data: {"jetzt": …}`.
3. **Verschobenes `weg_um` ist ein Wechsel**: ändert ein Lebenszeichen ohne Halter (z. B. ein offline sendender Agent) das `weg_um` um **mindestens 1 s** gegenüber dem zuletzt gemeldeten, entsteht unter der Sperre ein Wechsel mit unverändertem `status: "offline"`, neuem `weg_um` und neuem `stand` — höchstens einer je Sekunde und Sitzung. Kleinere Verschiebungen werden nicht gemeldet; die angezeigte Restzeit ist dann höchstens 1 s zu kurz, nie zu lang.
4. **Uhrenabgleich im Client**: wer ein Ereignis mit `jetzt` liest (Worker bzw. Tab im Rückfall), bildet `versatz = jetzt_server − Date.now()` beim Empfang und gibt ihn mit jedem weitergereichten Ereignis und mit dem zwischengespeicherten Stand an die Tabs; der Worker reicht dafür auch den Herzschlag (`jetzt`) weiter. Worker und Tabs teilen die Uhr des Rechners — der Versatz gilt für beide.
5. **Restzeit** (rein, `web/teilnehmerliste.js` oder eigene Datei): `rest_s = max(0, ⌈(weg_um − (Date.now() + versatz)) / 1000⌉)`. Anzeige: `rest_s ≤ 60` → gestrichelter Kreis und „weg in m:ss“ (Ziffern gleich breit), Name „offline, gleich weg“ (statisch, keine sekündliche Ansage); `rest_s > 60` → ○ „offline“ wie heute; bei 0 bleibt „weg in 0:00“ stehen, bis der Wechsel `weg` die Zeile entfernt — der Server entscheidet, nicht die Uhr des Browsers. Ein Sekundentakt läuft nur, solange mindestens eine Zeile „gleich weg“ ist, und zeichnet nur diese Zeilen neu.
6. **„Zuletzt online“** (Stand unbekannt): gefüllter Punkt in `lc-offline` statt gestrichelt; Name „zuletzt online“ bleibt; Zeilen „gleich weg“ zählen im unbekannten Stand weiter herunter (beste Kenntnis), der nächste Stand nach dem Wiederverbinden ersetzt sie.
7. **Statusspalte**: eine feste Breite für alle Zeilen, die „weg in 1:00“ in `lc-status` (12 px) samt Punkt fasst; ein Wechsel online ↔ offline ↔ gleich weg ändert weder Spaltenbreite noch Zeilenhöhe. Das Designsystem-README bekommt die Breite als Hinweis (Nachzug wie B0064).

## Konsequenzen

**Positiv:**
- Die Restzeit stimmt auf jedem Rechner im LAN, auch bei falsch gehender Uhr, und in jedem später geöffneten Tab.
- Agenten sehen in `GET /api/teilnehmer`, wann ein Name frei wird.
- Jede Regel ist rein prüfbar: Server mit gestellter Uhr, Client mit gestellter Uhr und gestelltem Versatz.

**Negativ / Inkauf zu nehmen:**
- Der Herzschlag trägt Inhalt, der Worker reicht ihn weiter (alle 5 s eine Nachricht je Tab).
- Ein offline sendender Agent erzeugt bis zu einen Wechsel je Sekunde.
- Die Kennungsspalte wird um etwa 25 px schmaler.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein neuer Tab Restzeiten zeigt, die um Minuten falsch sind, weil der Worker den Stand ohne frischen Versatz weitergibt. Früh-Indikator: Restzeit > Gnadenfrist oder Sprung beim nächsten Herzschlag. Gegenmaßnahme: Regel 4; Node-Test „Stand vor 40 s gespeichert, neuer Tab“.
2. … Zeilen bei 0:00 hängen bleiben, weil der Wechsel `weg` verloren ging. Früh-Indikator: „weg in 0:00“ länger als einen Takt plus Zustellung. Gegenmaßnahme: `Lagged` → neuer Stand (ADR-0016 Regel 5); Wiederverbinden beginnt mit Stand.
3. … Agenten mit vielen Anfragen ohne Strom die Sichten mit Wechseln fluten. Früh-Indikator: Wechsel je Minute im Strom. Gegenmaßnahme: Drossel aus Regel 3 (≥ 1 s).

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Test (Unit, Anwesenheit, gestellte Uhr, Gnadenfrist 60 s): `alpha` mit Halter; Halter fällt bei t → `pflegen(t + 1 s)` → `offline`, `seit` = t + 1 s, `weg_um` = t + 60 s; `pflegen(t + 60 s)` → nichts; `pflegen(t + 60 s + 1 ms)` → `weg`, `seit` > `weg_um`; Lebenszeichen ohne Halter bei t + 10,5 s → Wechsel `offline` mit `weg_um` = t + 70,5 s; weiteres bei t + 11 s → kein Wechsel (Verschiebung 0,5 s); bei t + 11,6 s → Wechsel.
- [ ] Erster Test (Node, rein): `restzeit({weg_um: T+40 000 ms, versatz: 0, jetzt: T})` → 40 → „weg in 0:40“; `T+39 001` → „0:40“; `T+60 000` → „weg in 1:00“; `T+60 001` → „offline“; abgelaufen → „weg in 0:00“; Client-Uhr 30 s vor (`versatz = −30 000`) → dieselbe Anzeige.
- [ ] Erster Test (Node, Worker-Verteiler): Stand empfangen, Herzschlag 40 s später, neuer Tab → bekommt den Stand mit dem Versatz aus dem Herzschlag.
- [ ] E2E (Gnadenfrist 5 000 ms über `LOCALCHATA_GNADENFRIST_MS`): Beobachter sieht nach dem Schließen des anderen Kontexts die Zeile gestrichelt mit „weg in 0:0s“, `s` ≤ 5, sieht danach einen kleineren Wert, dann verschwindet die Zeile; Zeilenhöhe und Spaltenbreite vor und nach dem Wechsel gleich; bei 320 px kein Querscrollen; im Stand unbekannt ist kein Punkt gestrichelt außer „gleich weg“.
- [ ] Designsystem-README: Statusspalte und „zuletzt online“ als Hinweis nachziehen (Gestaltung, nicht Architektur — über den Director).

## Quellen / Bezuege

- [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md) (primär), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md); Aspekt [X00007](../Architektur/Aspekte/X00007-zeit.md)
- ADR-0012, ADR-0015, ADR-0016, ADR-0017
- Designsystem `components/Teilnehmerliste/README.md`, `components/Seitenleiste/README.md`, `README.md` (Status nie nur über Farbe), `2-token-erweiterung.md`; `web/lc-seite.css`, `web/teilnehmerliste.js`, `web/live.js`
