---
id: R00005
status: Neu
datum: 2026-10-28
---

# R00005: Verbindung bricht ab

## Beschreibung

Verliert ein Teilnehmer das Netz, ohne dass seine Verbindung sauber endet — WLAN weg, Deckel zu, Rechner schläft —, sehen ihn die anderen nach gut einer Viertelminute als offline statt erst nach Minuten. Wer offline ist und bald verschwindet, zeigt in der Liste, wann: ein gestrichelter Kreis und „weg in 0:40“, sekündlich herunter, richtig auch bei falsch gehender Uhr im Browser. Agenten lesen denselben Zeitpunkt als `weg_um` über die API und können ihren Namen am Ende ihrer Arbeit mit `POST /api/verlassen` sofort freigeben.

Zahlt ein auf: [Vision](R00000-vision.md) — „Wer da ist, ist sichtbar“: „Bricht die Verbindung ab, erscheint ein Teilnehmer sofort als offline; kehrt er nicht innerhalb etwa einer Minute zurück, verschwindet er aus der Liste.“

Lesart der Vision, angesprochen statt überschrieben: Beim **stillen** Abriss heißt „sofort“ hier „nach etwa 14 s, höchstens 22 s“ — ein Bruchteil der Minute bis „weg“. Schneller geht es nur mit einer kürzeren Abrissgrenze, und die kostet Menschen im WLAN häufige kurze Wechsel offline → online (ADR-0017, Pre-Mortem 1). Beim **sauberen** Abbruch (Tab zu) bleibt es bei < 2 s aus R00004. „Etwa eine Minute“ zählt ab dem letzten Lebenszeichen, das der Server kennt; nach einem stillen Abriss ist der Name also nach etwa 75 s frei. Offene Frage an den Betreiber, siehe unten; blockiert nicht.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0006 Verbindung bricht ab): F0016 Stiller Abriss wird erkannt, F0017 Restzeit über API und Strom, F0018 Restzeit in der Teilnehmerliste, F0019 Verlassen über die API.

## Geschäftlicher Nutzen

Nach R00004 steht ein Kollege, dessen Laptop eingeschlafen ist, bei allen noch rund 15 Minuten als online (ADR-0007: TCP gibt erst nach der Wiederholungsgrenze auf). Wer ihn in dieser Zeit fragt, schreibt ins Leere und hält ihn für erreichbar; sein Name bleibt so lange belegt. R00005 macht die Liste wieder zur Wahrheit: der Server erkennt die tote Verbindung selbst, die Liste sagt, wann der Name frei wird, und ein Agent, der fertig ist, muss nicht eine Minute warten, bis er unter demselben Namen von einem anderen Rechner neu beginnen kann.

## Funktionale Anforderungen

### Stiller Abriss wird erkannt (F0016, ADR-0017)

- **Herzschlag** im Ereignisstrom alle **5 s** (statt 15 s); der Testknopf `LOCALCHATA_HERZSCHLAG_MS` bleibt. Der Herzschlag erzeugt die Daten, deren fehlende Bestätigung den Abriss verrät.
- **Abrisspflege** je zugelassener Verbindung, einmal vor dem Bedienen, abgeleitet aus dem Herzschlag `H`: `TCP_USER_TIMEOUT` = 2H (nur Linux), `SO_KEEPALIVE` an, `TCP_KEEPIDLE` = H, `TCP_KEEPINTVL` = H, `TCP_KEEPCNT` = 2 (Sekunden aufgerundet, mindestens 1 s). Abgewiesene Verbindungen (Verbindungsgrenze) werden nicht gepflegt.
- Scheitert das Setzen, wird die Verbindung trotzdem bedient (wie heute, ohne Pflege); der erste Fehler je Programmlauf steht einmal auf stderr mit Option und Fehler, nie je Verbindung.
- Bricht der Kernel die Verbindung ab, fällt der Halter wie bei jedem Abbruch (R00004): `offline` nach dem Nachlauf, `weg` nach der Gnadenfrist ab dem Erkennen.
- `/api/gesundheit` nennt zusätzlich `herzschlag_ms` und `abrissgrenze_ms` (die wirksamen Werte).
- **Wächter im Browser**: mehr als 15 s (3 × H) ohne Byte → neu verbinden (statt 40 s).

### Restzeit über API und Strom (F0017, ADR-0019 Regeln 1–3)

- Jede Teilnehmerzeile — in `GET /api/teilnehmer`, im Stand `{art:"teilnehmer"}` und im Wechsel `{art:"anwesenheit"}` — trägt `weg_um`: bei `offline` den Zeitpunkt `letztes Lebenszeichen + Gnadenfrist` (RFC 3339 mit Millisekunden, Serverzeit), den letzten Augenblick, in dem die Sitzung noch lebt; bei `online` `null`.
- Die Sitzung ist weg, sobald `jetzt > weg_um` (dieselbe Regel wie bisher `lebt`).
- `jetzt` (Serverzeit, unter der Sperre gelesen, nie fallend) steht in der Antwort von `GET /api/teilnehmer` (`{stand, jetzt, liste}`), im Stand (`{art, stand, jetzt, liste}`), in jedem Wechsel (`{art, stand, jetzt, teilnehmer}`); der Herzschlag trägt `data: {"jetzt": …}`.
- **Verschobenes `weg_um`**: ändert ein Lebenszeichen ohne Halter (Agent, der ohne Strom Anfragen schickt) das `weg_um` um **mindestens 1 s** gegenüber dem zuletzt gemeldeten, entsteht ein Wechsel mit unverändertem `status: "offline"`, neuem `weg_um` und neuem `stand`. Kleinere Verschiebungen werden nicht gemeldet; ein gemeldetes `weg_um` ist damit höchstens 1 s zu früh, nie zu spät.

### Restzeit in der Teilnehmerliste (F0018, ADR-0019 Regeln 4–7, Designsystem `components/Teilnehmerliste`)

- **Uhrenabgleich**: wer Ereignisse liest (der Worker, im Rückfall der Tab), bildet beim Empfang `versatz = jetzt − Date.now()` und gibt ihn mit jedem weitergereichten Ereignis und mit dem gespeicherten Stand weiter; der Worker reicht auch den Herzschlag an die Tabs.
- **Restzeit**: `rest_s = max(0, ⌈(weg_um − (Date.now() + versatz)) / 1000⌉)`.
  - `rest_s ≤ 60` → **gestrichelter Kreis** und „weg in m:ss“; zugänglicher Name „offline, gleich weg“ (statisch, keine sekündliche Ansage);
  - `rest_s > 60` → ○ „offline“ wie in R00004;
  - bei 0 bleibt „weg in 0:00“ stehen, bis der Wechsel `weg` die Zeile entfernt — der Server entscheidet, nicht die Uhr des Browsers.
- Ein Sekundentakt läuft nur, solange mindestens eine Zeile „gleich weg“ ist, und zeichnet nur diese Zeilen neu.
- **„Zuletzt online“** (Stand unbekannt, Verbindung unterbrochen): gefüllter Punkt in der Farbe `lc-offline` statt gestrichelt; der Name „zuletzt online“ bleibt. Zeilen „gleich weg“ zählen im unbekannten Stand weiter herunter; der nächste Stand nach dem Wiederverbinden ersetzt sie.
- **Statusspalte**: eine feste Breite für alle Zeilen, die „weg in 1:00“ samt Punkt fasst; ein Wechsel online ↔ offline ↔ gleich weg ändert weder die Spaltenbreite noch die Zeilenhöhe; Zeilen springen nicht.

### Verlassen über die API (F0019, ADR-0018)

- `POST /api/verlassen` ohne Körper, mit Header `X-LocalchatA-Sitzung`:
  - gültiges Token **derselben Herkunft** → die Sitzung ist beendet, der Wechsel `{status: "weg", seit: jetzt}` geht an alle Ströme, **bevor** die Antwort geht; danach 204 mit Header-Satz und `Cache-Control: no-store`. Der Name ist ab der 204 frei.
  - unbekanntes, abgelaufenes oder fremdes Token (andere Herkunft), oder ein zweites `verlassen` → 204 **ohne Wirkung** (kein Orakel, keine Fremdbeendigung).
  - ohne Header → 401 `{fehler}`; fremder `Origin` → 403 (Grenzschutz wie jede Schreibroute, ADR-0014).
- `verlassen` ist kein Lebenszeichen und hat keinen Nachlauf: `online` → `weg` ohne `offline` dazwischen.
- Ein Strom, der das `weg` seiner **eigenen** Sitzung sieht, sendet es und endet. Halter einer verlassenen Sitzung ändern beim Schließen nichts mehr — eine verlassene Sitzung lebt nie wieder auf.
- Die Oberfläche ruft `verlassen` nie — nicht beim Schließen, Neuladen oder Wegnavigieren, nicht aus dem Worker (ADR-0018 Regel 6). Eine Bedienung „Verlassen“ im Browser gehört nicht zu R00005.

### Änderungen gegenüber R00004

- Herzschlag 5 s statt 15 s, Wächter 15 s statt 40 s (R00004, Funktionale Anforderungen F0012/F0013).
- Mit der Gnadenfrist 60 s ist jede Zeile ab dem `offline` schon „gleich weg“ (`weg_um − seit` = 59 s ≤ 60 s): wo R00004 ○ „offline“ zeigt, zeigt R00005 den gestrichelten Kreis und „weg in 0:59“. ○ „offline“ ohne Restzeit erscheint erst bei Gnadenfristen über 61 s (I0014). Tests von R00004, die nach dem Schließen ○ „offline“ erwarten, prüfen künftig „nicht mehr online“ (○ „offline“ oder gestrichelt „weg in …“).
- „Zuletzt online“ ist gefüllt grau statt gestrichelt (Nacharbeit 2 von R00004 umgebaut).

## Nicht-funktionale Anforderungen

- Performance (Zeitgrenzen mit Zweck, Startpunkt, erwartetem Wert; Rechnungen unter „Akzeptanzkriterien“):
  - **Stiller Abriss** (Zweck: die anderen sehen deutlich vor dem „weg“, dass jemand nicht mehr antwortet; Start: der Moment, ab dem die Gegenstelle keine Pakete mehr erreicht): erwartet ≈ 13,6 s, schlechtester Fall 16,25 s, Grenze 22 s — **Messwert im Entwicklertest, kein Gate** (ein echter Abriss braucht Paketverlust, den der Testrechner ohne Rechte nicht erzeugt).
  - **Harter Abbruch** (RST; Zweck: die Kette Socket-Fehler → Halter → `offline` ist im Gate belegt; Start: Rückkehr von `close`): erwartet ≤ 1,25 s, Grenze 2 s.
  - **Verlassen** (Zweck: ein Agent übergibt seinen Namen ohne Wartezeit; Start: Erhalt der 204): `weg` bei den anderen ≤ 1 s, erwartet wenige Millisekunden.
  - Kosten: ein Herzschlag (≈ 30 Byte) je Browser alle 5 s; ein Sekundentakt nur, solange eine Zeile „gleich weg“ ist.
- Sicherheit: `verlassen` beendet nie eine fremde Sitzung und verrät nicht, ob ein Token galt; das Token bleibt im Header (ein Nachweisweg, ADR-0015); Mutationsprobe „Herkunftsvergleich in `verlassen` entfernt“ → rot.
- Benutzerfreundlichkeit: Status nie nur über Farbe (gestrichelt + Wort, gefüllt grau + Name „zuletzt online“); keine sekündliche Ansage; kein Querscrollen bei 320 px; keine unbehandelte Ausnahme und kein CSP-Verstoß.
- Testbarkeit: Regeln über die gestellte Uhr bzw. über Serverzeiten im Ergebnis (`seit`, `weg_um`, `jetzt`), nie Serverzeit gegen die Uhr des Testprozesses; Zustellung als obere Grenze zwischen zwei Ankünften beim selben Beobachter; Untergrenzen nur über `seit` (Projektregel #4, X00007). Die Gesamtzeit des stillen Abrisses ist Messwert (X00009).
- Plattform: die Abrissgrenze für laufende Ströme wirkt nur unter Linux (`TCP_USER_TIMEOUT`); unter macOS und Windows erkennt nur Keepalive ruhende Verbindungen (ADR-0017, hingenommen; Projektumgebung ist Linux).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser`, `<p>` ist der Port aus der Startzeile, Gnadenfrist und Herzschlag Standard (60 s, 5 s), wenn nicht anders genannt; weitere Absender `127.0.0.2`, `127.0.0.3`, `127.0.0.4` (`curl --interface …`). Beitritte per `curl` ohne `art` sind Agenten. „Strom lesen“ heißt `curl -N -H 'X-LocalchatA-Sitzung: …' http://127.0.0.1:<p>/api/ereignisse`. Zeitgrenzen, die eine Ankunft messen, zählen ereignisgenau beim genannten Beobachter.

### F0016 Stiller Abriss wird erkannt

- [ ] Ableitung (Unit, rein): `pflege_aus(5 s)` → `TCP_USER_TIMEOUT` 10 000 ms, Keepidle 5 s, Keepintvl 5 s, Keepcnt 2; `pflege_aus(500 ms)` → 1 000 ms, 1 s, 1 s, 2 (aufgerundet, mindestens 1 s).
- [ ] Am echten Socket (Unit, Linux, Loopback, kein Netzabriss): die Kernel-Pflege auf einer angenommenen Verbindung → `getsockopt` liest genau die Werte aus `pflege_aus(5 s)` zurück.
- [ ] Je Verbindung genau einmal (Unit, `ausliefern` mit aufzeichnender Pflege): zwei Verbindungen mit je einer Anfrage → genau zwei Pflegeaufrufe; zwei Anfragen auf derselben Verbindung → ein Aufruf; eine über der Verbindungsgrenze abgewiesene Verbindung → kein Aufruf.
- [ ] Fehlerpfad (Unit, Pflege liefert Fehler): beide Verbindungen werden bedient (200), stderr enthält genau eine Zeile mit Option und Fehler.
- [ ] Mutationsprobe: wird der Pflegeaufruf in `ausliefern` entfernt, wird mindestens ein Test rot; der Entwickler nennt das Ergebnis in seinem Bericht.
- [ ] Harter Abbruch (Integration, echtes Programm): `beta` (127.0.0.3) liest den Strom; `alpha` (127.0.0.2) hält einen Strom über einen rohen Socket, `beta` hat `alpha` `online` gelesen; `alpha` bricht hart ab (`SO_LINGER 0`, RST) → `beta` liest `alpha` `offline` spätestens **2 s nach der Rückkehr von `close`** (erwartet ≤ 1,25 s = Nachlauf 1 s + höchstens ein Takt 0,25 s; Reserve 60 %).
- [ ] Gesundheit: ohne Knopf → `"herzschlag_ms":5000`, `"abrissgrenze_ms":10000`; mit `LOCALCHATA_HERZSCHLAG_MS=500` → `500` und `1000`.
- [ ] Herzschlag ohne Knopf: ein Strom liest nach dem Stand spätestens 7 s später ein `event: herzschlag` (erwartet ≤ 5 s; Reserve 40 %). `LOCALCHATA_HERZSCHLAG_MS=bald` → Exit 2 wie bisher.
- [ ] Wächter (Node-Test, rein, gestellte Uhr): 15 000 ms ohne Byte → weiter; 15 001 ms → neu verbinden.
- [ ] **Messwert stiller Abriss — Prüfpunkt im Entwicklertest, kein Gate** (US-3): Ergebnis min/Median/max über mindestens 3 Läufe steht in ADR-0017 unter „Erprobt durch“; erwartet ≈ 13,6 s, schlechtester Fall 16,25 s, Grenze 22 s (35 % über dem schlechtesten Fall). Über 22 s ist ein Befund an ADR-0017 (neue Entscheidung, Option C oder D), kein roter Test.

Rechnung stiller Abriss (ADR-0017): bis zum nächsten Herzschlag im Mittel H/2 = 2,5 s, schlechtestens H = 5 s; Abrissgrenze 2H = 10 s; Socket-Fehler → Halter < 1 ms; Nachlauf 1 s; Takt im Mittel 0,125 s, schlechtestens 0,25 s → Mittel 13,6 s, schlechtester Fall 16,25 s; 22 / 16,25 = 1,35.

Fristen gegeneinander: Wächter 15 s = 3 Herzschläge → ein verspäteter Herzschlag löst kein Neuverbinden aus. Ein Test, der `LOCALCHATA_HERZSCHLAG_MS` über 5 000 ms setzt, lässt den Wächter (fest 15 s) bei Herzschlägen über 15 s neu verbinden — E2E-Tests setzen den Knopf nicht über 5 000 ms.

### F0017 Restzeit über API und Strom

- [ ] Liste (Integration): `alpha` (127.0.0.2) tritt bei und schweigt, `beta` (127.0.0.3) ist beigetreten; `beta` ruft `GET /api/teilnehmer` 2 s nach dem Beitritt von `alpha` → Antwort mit `jetzt`; `alpha` `offline` mit `weg_um`, und `weg_um − seit` = **59 000 ms genau** (Gnadenfrist 60 s − Nachlauf 1 s; Rechnung: `seit` = Beitritt + 1 s, `weg_um` = Beitritt + 60 s); jede Zeile mit `"status":"online"` trägt `"weg_um":null`.
- [ ] Strom (Integration): `beta` liest den Strom; `alpha` schließt seinen Strom → der Wechsel `offline` für `alpha` trägt `jetzt` und `weg_um`, `weg_um − seit` = 59 000 ms genau; Stand und jeder Wechsel tragen `jetzt`.
- [ ] Regel weg (Unit, gestellte Uhr, Gnadenfrist 60 s): Halter fällt bei t → `pflegen(t + 1 s)` → `offline`, `seit` = t + 1 s, `weg_um` = t + 60 s; `pflegen(t + 60 s)` → kein Wechsel; `pflegen(t + 60 s + 1 ms)` → `weg` mit `seit` > `weg_um`.
- [ ] Weg nach der Restzeit (Integration, `LOCALCHATA_GNADENFRIST_MS=3000`): `beta` liest den Strom, `alpha` schließt seinen Strom → `beta` liest für `alpha` `offline` und danach `weg`; Regel über Serverzeiten: `weg_um − seit(offline)` = 2 000 ms, `seit(weg) > weg_um`; Zustellung: Ankunft(`weg`) − Ankunft(`offline`) bei `beta` ≤ (`weg_um − seit`) + 1 s = **3 s** (erwartet 2 s + höchstens ein Takt 0,25 s; der Zustellanteil 0,25 s hat 1 s Spielraum).
- [ ] Drossel (Unit, gestellte Uhr): Lebenszeichen ohne Halter bei t + 10,5 s → Wechsel `offline` mit `weg_um` = t + 70,5 s; weiteres bei t + 11 s → kein Wechsel (Verschiebung 0,5 s); bei t + 11,6 s → Wechsel (Verschiebung 1,1 s); jeder Wechsel wird unter der Sperre veröffentlicht (aufzeichnende Veröffentlichung, `try_lock` scheitert).
- [ ] Drossel (Integration): `beta` liest den Strom, `alpha` ist ohne Strom beigetreten und `beta` hat `alpha` `offline` gelesen; `alpha` ruft 2 s lang alle 0,25 s `GET /api/ich` → `beta` liest mindestens einen weiteren Wechsel `offline` für `alpha`; je zwei nacheinander gemeldete `weg_um` von `alpha` liegen ≥ 1 000 ms auseinander; `GET /api/teilnehmer` durch `beta` nach dem letzten Aufruf zeigt für `alpha` ein `weg_um`, das um 0 bis < 1 000 ms nach dem letzten im Strom gemeldeten `weg_um` von `alpha` mit `stand` ≤ dem `stand` dieser Antwort liegt (Zuordnung über `stand`, nicht über die Ankunftszeit).
- [ ] Herzschlag (Integration, `LOCALCHATA_HERZSCHLAG_MS=500`): ein Strom liest in 2 s mindestens 2 Herzschläge (erwartet 4), jeder mit `data: {"jetzt": …}`, kein `jetzt` kleiner als das vorige.

### F0018 Restzeit in der Teilnehmerliste

Rein (`node --test`, gestellte Uhr) und Playwright mit Chromium gegen das echte Programm.

- [ ] Restzeit (Node-Test, rein, `versatz` 0, jetzt = T): `weg_um` = T + 40 000 ms → „weg in 0:40“; T + 39 001 ms → „weg in 0:40“; T + 60 000 ms → „weg in 1:00“; T + 60 001 ms → „offline“; T − 1 ms → „weg in 0:00“; Uhr des Browsers 30 s vor (`versatz` = −30 000 ms) → dieselben Anzeigen.
- [ ] Versatz im Worker (Node-Test): Stand empfangen, Herzschlag 40 s später, danach neuer Tab → der Tab bekommt den Stand mit dem Versatz aus dem Herzschlag.
- [ ] Sekundentakt (Node-Test, gestellter Zeitgeber): eine Zeile „gleich weg“ → Takt läuft und zeichnet nur sie neu; keine → kein Takt.
- [ ] Gleich weg im Browser (E2E, `LOCALCHATA_GNADENFRIST_MS=5000`): A (`shoff`) und B (`mia`) online; B wird geschlossen → A zeigt `mia@127.0.0.1` **spätestens 2 s nach dem abgeschlossenen Schließen** (erwartet ≤ 1,25 s) mit gestricheltem Kreis und „weg in 0:0s“, 1 ≤ s ≤ 4 (Regel: höchstens Gnadenfrist − Nachlauf = 4 s); ein kleinerer Wert folgt **spätestens 3 s** nach dem ersten (erwartet ≤ 2 s: bis zur nächsten vollen Sekunde plus ein Takt von 1 s); die Zeile verschwindet **spätestens 5 s** nach dem Erscheinen der gestrichelten Form (Regel `weg_um − seit` = 4 s plus 1 s Zustellung; erwartet ≤ 4,25 s). Zeilenhöhe von `mia` und Breite der Statusspalte sind online und „gleich weg“ gleich; die Zeilen darüber bleiben an ihrem Platz.
- [ ] Zugänglich: die Zeile „gleich weg“ trägt den Namen „offline, gleich weg“; die Liste hat keine Live-Region, die die Restzeit ansagt.
- [ ] Neuer Tab (E2E, Gnadenfrist 60 s): B geschlossen, A Tab 1 zeigt „weg in 0:ss“; 20 s nach dem Schließen öffnet A Tab 2 → Tab 2 zeigt binnen 2 s ebenfalls „weg in 0:ss“; im selben Augenblick gelesen weichen die Werte beider Tabs um höchstens 1 ab (gleiche Rechnung, nur der Takt verschieden).
- [ ] Falsch gehende Uhr (E2E, Gnadenfrist 60 s): die Uhr von A geht 30 s vor — im Test `Date.now` in der Seite **und** im Worker um +30 000 ms verschoben (beide teilen die Uhr des Rechners; nur die Seite zu verschieben stellt einen Zustand her, den es nicht gibt); B geschlossen → A zeigt beim Erscheinen „weg in 0:ss“ mit 55 ≤ ss ≤ 59 (erwartet 59; ohne Abgleich stünde 0:29).
- [ ] Rückfall (E2E, Kriterium nach Nutzen): A ohne `SharedWorker` (Init-Skript) → nach dem Schließen von B zeigt A die Zeile `mia` gestrichelt mit „weg in …“.
- [ ] Zuletzt online (E2E): A und B online, das Programm wird beendet → A zeigt den Hinweis „Verbindung unterbrochen“, die Zeilen von `shoff` und `mia` tragen einen gefüllten Punkt in `lc-offline` (berechnete Hintergrundfarbe gleich dem Token, Rahmen nicht `dashed`) mit dem Namen „zuletzt online“; keine Zeile ist gestrichelt.
- [ ] Gleich weg im unbekannten Stand (Node-Test, rein): Stand unbekannt, eine Zeile „gleich weg“ → sie bleibt gestrichelt und zählt herunter; online-Zeilen → „zuletzt online“, gefüllt.
- [ ] Schmal (E2E): bei 320 px, Liste eingeblendet (`Strg .`), eine Zeile „weg in 0:ss“ mit einem Namen von 32 Zeichen → `scrollWidth <= innerWidth`.
- [ ] Konsole bei A (dem Beobachter): keine unbehandelte Ausnahme (`pageerror`) und kein CSP-Verstoß. Konsolenfehler: in „Gleich weg im Browser“, „Neuer Tab“, „Falsch gehende Uhr“ und „Schmal“ keine — dort schreibt diese Anforderung keine Fehlerantwort vor; in „Zuletzt online“ nur `net::ERR_CONNECTION_REFUSED` bzw. „Failed to load resource“ zu `/api/…`, solange die Instanz beendet ist. Für „Rückfall“ gilt das Kriterium nach Nutzen oben, keine weitere Vorgabe zur Konsole.

### F0019 Verlassen über die API

Ausnahmeweg (ein Agent beendet seine Arbeit) — Kriterien nach Nutzen: Name frei, die anderen sehen `weg`, der eigene Strom endet.

- [ ] Verlassen (Integration): `alpha` (127.0.0.2) hält einen Strom, `beta` (127.0.0.3) liest den Strom und hat `alpha` `online` gelesen; `alpha` ruft `POST /api/verlassen` mit seinem Header → 204 mit Header-Satz und `Cache-Control: no-store`; `beta` liest `weg` für `alpha` **spätestens 1 s nach dem Erhalt der 204** (erwartet wenige ms: veröffentlicht vor der Antwort), ohne `offline` davor; `alpha`s Strom liefert `weg` für `alpha` und endet spätestens 1 s nach der 204 (erwartet wenige ms); ab dem Ende von `alpha`s Strom liest `beta` für `alpha` 2 s lang keinen weiteren Wechsel (ein fälschliches `offline` käme nach Nachlauf 1 s + Takt 0,25 s; Reserve 60 %).
- [ ] Name sofort frei: nach der 204 ergibt `{"name":"alpha"}` von `127.0.0.4` sofort 201 (ohne Wartezeit dazwischen).
- [ ] Ohne Header → 401 `{fehler}`; mit dem Header von `alpha` und `Origin: http://evil.example` → 403, `alpha` steht danach in `GET /api/teilnehmer`.
- [ ] Ohne Wirkung: Token von `alpha` von `127.0.0.3` → 204, `alpha` lebt; Token `00000000000000000000000000000000` → 204; nach einem erfolgreichen `verlassen` ein zweites mit demselben Token → 204, kein weiterer Wechsel.
- [ ] Unit (Anwesenheit, gestellte Uhr): `alpha` tritt bei (stand 1); `verlassen(token, herkunft)` → Wechsel `weg`, stand 2, `seit` = jetzt, unter der Sperre veröffentlicht; `teilnehmer()` ohne `alpha`; `beitreten("alpha")` von anderer Herkunft sofort ok; der `Drop` eines Halters von `alpha` danach → kein Wechsel.
- [ ] Mutationsprobe: wird der Herkunftsvergleich in `verlassen` entfernt, wird mindestens ein Test rot; der Entwickler nennt das Ergebnis.
- [ ] Die Oberfläche ruft `/api/verlassen` nie: in den E2E-Szenarien von R00004 und R00005 (Schließen, Neuladen, neuer Tab) geht kein `POST /api/verlassen` hinaus.

## Betroffene Verzeichnisstruktur

- Start: `src/start/ausliefern.rs` (Pflege als Naht, Aufruf je zugelassener Verbindung), neu z. B. `src/start/abrisspflege.rs` (Ableitung, Kernel-Fassung), `src/start/mod.rs` (Herzschlag an Strom und Annahmeschleife); `src/live/herzschlag.rs` (Standard 5 s).
- Anwesenheit: `src/anwesenheit/` (`weg_um`, `jetzt` in Stand und Wechsel, Drossel, `verlassen`).
- HTTP: `src/http/gesundheit.rs` (`herzschlag_ms`, `abrissgrenze_ms`), `src/http/teilnehmerliste.rs` (`jetzt`), `src/http/ereignisse.rs` (Herzschlag mit `jetzt`, Ende nach eigenem `weg`), neu `src/http/verlassen.rs`, `src/http/mod.rs` (Route).
- Oberfläche: `web/verbindung.js` (15 s), `web/teilnehmerliste.js` oder neu `web/restzeit.js`, `web/verteiler.js`, `web/live.js` (Herzschlag, Versatz), `web/lc-seite.css` (Formen, Statusspalte).
- Tests: Unit im jeweiligen Modul; Integration `tests/ereignisse.rs`, `tests/teilnehmer.rs`, neu `tests/abriss.rs`, `tests/verlassen.rs`; Oberfläche rein `e2e/einheit/*.test.mjs`; Abläufe `e2e/tests/r00005-*.spec.js`; angepasst die R00004-Tests mit ○ „offline“ nach dem Schließen und „zuletzt online“ gestrichelt.
- Abhängigkeit: `rustix` Feature `net` (schon direkte Abhängigkeit).

## Technische Überlegungen

### Ablauf

1. **Start** (A00003): … → Herzschlag aus `LOCALCHATA_HERZSCHLAG_MS` (Standard 5 s) → `pflege_aus(herzschlag)` → `ausliefern(…, pflege)`; derselbe Herzschlag an den Strom.
2. **Annahme**: Verbindung angenommen → zugelassen? → Pflege einmal → bedienen. Kernel bricht eine tote Verbindung ab → hyper beendet die Antwort → Halter fällt → `offline` nach Nachlauf und Takt.
3. **Anwesenheit**: jede Zeile mit `weg_um`, Stand und Wechsel mit `jetzt`; Lebenszeichen ohne Halter → bei Verschiebung ≥ 1 s Wechsel; `verlassen` → Sitzung entfernt, `weg` unter der Sperre.
4. **`POST /api/verlassen`**: Header lesen (fehlt → 401) → `verlassen(token, herkunft aus ConnectInfo)` → 204. Bewusst nicht über den Extraktor `Teilnehmer` (ungültiges Token → 204, nicht 401).
5. **Strom**: Herzschlag mit `jetzt`; eigenes `weg` → senden, enden.
6. **Oberfläche**: Leser bildet `versatz`, Worker reicht Ereignisse, Herzschlag und Versatz; Restzeit rein; Sekundentakt nur für „gleich weg“; Wächter 15 s.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0016: B0099 Abrisspflege aus dem Herzschlag (rein) · B0100 Kernel-Pflege am Socket · B0101 Pflege in der Annahmeschleife (Naht) · B0102 Herzschlag 5 s und Pflege im Start verdrahten (RST-Integration) · B0103 Gesundheit nennt Herzschlag und Abrissgrenze · B0104 Wächter 15 s
- F0017: B0105 Wann weg je Sitzung · B0106 Serverzeit in Stand, Wechsel und Liste · B0107 Herzschlag mit Serverzeit · B0108 Verschobenes `weg_um` melden
- F0018: B0109 Restzeit rechnen · B0110 Versatz mitführen · B0111 Statusformen zeichnen · B0112 Sekundentakt für „gleich weg“
- F0019: B0113 Verlassen in der Anwesenheit · B0114 Verlassen beantworten · B0115 Eigener Strom endet nach eigenem `weg`

**Elemente** (nur öffentliche Signaturen, nach A00003, A00004, A00010, A00011, A00005):

- `fn pflege_aus(herzschlag: Duration) -> Verbindungspflege` (unveränderlich: `user_timeout`, `keepidle`, `keepintvl`, `keepcnt`); Pflege als Naht für `ausliefern` (Funktion oder kleiner Trait mit Kernel- und aufzeichnender Fassung).
- `Anwesenheit`: zusätzlich `fn verlassen(&self, token: &Token, herkunft: IpAddr) -> Vec<Wechsel>`; `Stand { stand, jetzt, liste }`, `Wechsel { stand, jetzt, teilnehmer, status, seit }`, Teilnehmerzeile mit `weg_um: Option<Zeit>`.
- Oberfläche: `restzeit({weg_um, versatz, jetzt}) -> {art: 'gleich-weg' | 'offline', text}`; Verteiler gibt `{ereignis, versatz}` weiter.

Wireframe: die WBS führt `wireframes: Dokumentation/Wireframes/`, ein Artboard `D0002.dc.html` fehlt (Hinweis `/wireframe verfeinern D0002`). Gestaltungsquelle bis dahin: Designsystem `components/Teilnehmerliste/README.md` („offline, gleich weg“), `2-token-erweiterung.md` (`lc-offline`), README (Status nie nur über Farbe) — Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `start::ausliefern`: bekommt die Pflege als Parameter; `live::herzschlag`: Standard 5 s (Test „ohne Wert 15 s“ wird „5 s“).
- `anwesenheit::Anwesenheit`: zuletzt gemeldetes `weg_um` je Sitzung (Drossel), `verlassen`; Stand und Wechsel mit `jetzt`.
- `http::gesundheit::Gesundheit`: `herzschlag_ms`, `abrissgrenze_ms`; `http::ereignisse`: kennt die eigene Kennung, Herzschlag mit Inhalt.
- `web/verbindung.js`: `STILLE_MS` 15 000; `web/live.js`: reicht `event: herzschlag` weiter (heute gefiltert); `web/verteiler.js`: Versatz; `web/teilnehmerliste.js`, `web/lc-seite.css`: Formen und Statusspalte (heute fest 52 px).

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests. Tests sind nie schärfer als diese Anforderung: sie übernehmen die Grenzen von oben wörtlich.

**Unit (rein, gestellte Uhr):** `pflege_aus`; `getsockopt` am echten Socket; aufzeichnende Pflege in `ausliefern` (je Verbindung einmal, abgewiesene keine, Fehlerpfad); `weg_um`, weg bei +60 s +1 ms; Drossel 10,5 / 11 / 11,6 s; `verlassen` (eigene, fremde, unbekannte Herkunft, zweimal, Halter danach).

**Integration (`tests/`, echtes Programm):** RST → `offline` ≤ 2 s ab `close`; `weg_um − seit` exakt; weg nach Restzeit mit Zustellgrenze; Drossel über `/api/ich`; Herzschlag mit `jetzt`; Gesundheit; `verlassen` (204, `weg` ≤ 1 s, eigener Strom endet, Name frei, 401, 403, ohne Wirkung). Mutationsproben: Pflegeaufruf entfernt, Herkunftsvergleich entfernt.

**Oberfläche rein (`e2e/einheit/`):** Restzeit (Tabelle oben), Versatz im Worker, Sekundentakt, Wächter 15 000 / 15 001 ms, unbekannter Stand.

**E2E (Playwright):** die Szenarien aus F0018. Dauern im Browser ereignisgenau (Prüfintervall ≤ 25 ms oder Animationsrahmen), nicht im Standardraster von `expect`; Zeitgrenzen am beobachtbaren Zustand der Liste festmachen. Mindestens ein Test mit verzögert antwortendem Strom (erstes Ereignis 1 s verzögert) bleibt aus R00004.

**Entwicklertest, Messwert (kein Gate):** US-3 stiller Abriss mit zweitem Gerät.

## Abhängigkeiten

- Abhängig von: I0005 Teilnehmer mit Online-Status sehen (R00004, grün) — Halter, Nachlauf, Takt, Strom, Worker, Liste; I0004 (R00003, grün) — Sitzung, Gnadenfrist, Grenzschutz; I0001 (R00001, grün) — Annahmeschleife.
- Innerhalb (WBS `Braucht`): F0016, F0017 und F0019 frei und untereinander parallel; F0018 braucht F0017 (`weg_um`, `jetzt`).
- Blockiert: keine Interaction fachlich; prägt I0014 (Gnadenfrist je Agent → ○ „offline“ über 60 s) und I0016 (Keepalive deckt ruhende Warte-Aufrufe).

## Offene Fragen

Keine, die R00005 blockieren. An den Betreiber (über den Director):

- **Bedienung „Verlassen“ im Browser**: das Designsystem hat keine; ADR-0018 legt fest, wie sie sich verhalten müsste (Token zuerst aus dem `localStorage`, kein stiller Wiederbeitritt, kein Neuverbinden). Empfehlung: nicht jetzt — erst, wenn die Abnahme danach fragt; dann eine neue Interaction in D0002 und eine Ergänzung des Designsystems.
- **Lesart „sofort offline“ beim stillen Abriss** (≈ 14 s, Grenze 22 s): Empfehlung: so annehmen — kürzer erkennt der Server einen stillen Abriss nur, wenn er WLAN-Aussetzer über wenige Sekunden ebenfalls als Abbruch wertet.

Entschieden vom planer für R00005:

- Ein Requirement für alle vier Slices (siehe Notizen).
- Der stille Abriss ist im Gate in seine Glieder zerlegt (Ableitung, Socket-Werte, Aufruf je Verbindung, RST-Kette); die Gesamtzeit ist Prüfpunkt im Entwicklertest (US-3), kein Gate.
- Der R00004-Test „○ offline nach dem Schließen“ wird auf „nicht mehr online“ gelockert, weil die Restzeit die Form im Normalfall ersetzt.

## Manuelle Vorbereitungstätigkeiten

- Für US-3 (Messwert): ein zweites Gerät im selben LAN mit Browser; die Instanz auf dem Entwicklungsrechner mit `--host 0.0.0.0` (LAN-Adresse aus der Startzeile). Alternative mit Rechten: Programm und Client in einem Netz-Namespace (`sudo unshare -n`), nach dem ersten Ereignis eine `nft`-Regel „drop“ für den Client-Port.
- Für US-2 von Hand: Python 3 (Standardbibliothek) für den harten Abbruch, Befehl in der User Story.

## Manuelle Nachbereitungstätigkeiten

- ADR-0017: Messwert unter „Erprobt durch“ eintragen; ≤ 22 s → Status `Akzeptiert` (ADR-0007 ebenso) — architekt über den Director.
- `AI-Team/umgebung.md`: Herzschlag-Standard 5 s nennen (Director).
- Designsystem-README: Statusspalte und „zuletzt online“ als Hinweis nachziehen (ADR-0019, über den Director).

## Umfang

Aus der WBS gezählt (`/planung zaehlen I0006`); Standard-Bubbles mit der Untergrenze 0,4h (belegt aus `Schaetzungen/_ist-zeiten.md`, R00001–R00004, Typ Standard), unklare als Richtwert 2–4h:

- F0016 Stiller Abriss wird erkannt = 6 Bubbles: 5 Standard (0,4h), 1 unklar (2–4h). Rest: 2,0h klar + 2–4h unklar · 5 von 6 Werten belegt, Rest Richtwert (ungemessen).
- F0017 Restzeit über API und Strom = 4 Bubbles: 4 Standard (0,4h). Rest: 1,6h klar · 4 von 4 Werten belegt.
- F0018 Restzeit in der Teilnehmerliste = 4 Bubbles: 3 Standard (0,4h), 1 unklar (2–4h). Rest: 1,2h klar + 2–4h unklar · 3 von 4 Werten belegt, Rest Richtwert (ungemessen).
- F0019 Verlassen über die API = 3 Bubbles: 3 Standard (0,4h). Rest: 1,2h klar · 3 von 3 Werten belegt.
- I0006 gesamt = 17 Bubbles. Rest: 6,0h klar + 4–8h unklar · 15 von 17 Werten belegt, Rest Richtwerte (ungemessen).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Ein Laptop schläft ein, während sein Browser einen Ereignisstrom hält; der Server schreibt weiter Herzschläge in den Sendepuffer, TCP wiederholt sie still, und bis die Wiederholungsgrenze greift (rund 15 Minuten), steht der Teilnehmer bei allen als online. Wenn jede Verbindung eine Abrissgrenze von zwei Herzschlägen trägt und der Herzschlag alle 5 s unbestätigte Daten erzeugt, dann bricht der Kernel die tote Verbindung nach höchstens 15 s ab; dann fällt der Halter über denselben Weg wie bei jedem Abbruch (in R00004 gebaut und geprobt), und die anderen sehen offline nach dem Nachlauf. Wenn jede Zeile den Zeitpunkt `weg_um` in Serverzeit trägt und der Browser ihn über `jetzt` gegen seine eigene Uhr abgleicht, dann stimmt die Restzeit auf jedem Rechner im LAN und in jedem später geöffneten Tab — man sieht, ob es lohnt zu warten. Und wenn ein Agent mit `verlassen` seinen Namen selbst freigibt, dann kann er ohne Minute Wartezeit unter demselben Namen neu beginnen, ohne dass Schließen oder Neuladen im Browser je den Namen kosten.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe wie Halter, Nachlauf, Gnadenfrist, Lebenszeichen, Abrissgrenze, Restzeit, „gleich weg“ sind aus ADR-0012, ADR-0016, ADR-0017, ADR-0019 und A00010 übernommen.
- Echter stiller Abriss im Projekt nicht geprobt (ADR-0017: unprivilegierte Netz-Namespaces gesperrt) — deshalb Messwert.

## Notizen

### Verworfene Alternativen

- **Je Slice eine eigene Anforderung**: verworfen — alle vier haben ein Ziel (Verbindung bricht ab: offline, Restzeit, Name frei) und durchlaufen als Arbeitspaket I0006 gemeinsam Entwicklertest und Abnahme.
- **Messwert stiller Abriss als Gate** (z. B. Test mit `sudo`): verworfen — das Gate liefe auf dem Testrechner nicht ohne Rechte; die Glieder sind einzeln deterministisch prüfbar (ADR-0017, X00009).
- **Verlassen als Ausbaustufe nach I0006**: verworfen — für sich nützlich, klein und parallel baubar; ADR-0012 hat die Route für I0006 vorgesehen.
- **Pflege, Herzschlag, Wächter als eigene Slices** (Zuschnitt-Liste des architekt, Punkt 1): als Bubbles in F0016 — für sich nicht von außen prüfbar.
- **Bedienung „Verlassen“ im Browser**: nicht in R00005 (Richtungsfrage, siehe Offene Fragen).

### Bewusst außerhalb

Bedienung „Verlassen“ im Browser; längere Gnadenfrist für Agenten `gnadenfrist_s` (I0014); `GET /api/warten` und die Messung von Keepalive bei ruhenden Warte-Aufrufen hinter NAT (I0016, ADR-0017 Pre-Mortem 3); Abrissgrenze unter macOS und Windows.
