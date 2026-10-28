---
id: R00021
status: Neu
datum: 2026-10-28
---

# R00021: Frist bucht am Schreibkonto

## Beschreibung

Das Setzen der Aufbewahrungsfrist (`PUT /api/channels/{name}/frist`, R00008) bucht am Schreibkonto aus R00011 — demselben Konto, das das Senden bremst. Danach gilt:

- **Eine Friständerung kostet 10 Einheiten**: 1 Einheit vorab nach der Sitzungsprüfung, 9 nach den Prüfungen von Körper und Wert. Eine Kennung kann die Frist dreimal in Folge setzen, danach einmal je 10 s; alle Kennungen einer Adresse zusammen zwölfmal in Folge, danach rund einmal je 1,25 s.
- **Gebremst heißt 429 und nichts geändert**: die Antwort nennt „Zu viele Änderungen von beta@127.0.0.1 — wieder möglich in 10 s.“ bzw. „Zu viele Änderungen von 127.0.0.1 (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in 2 s.“, dazu `grenze`, `wieder_in_ms` und `Retry-After`. Die Einstellungsdatei bleibt, kein Strom bekommt ein Ereignis.
- **Die Betreiberin sieht es**: eine eigene `WARNING schreibkonto`-Episode „Frist gebremst: zu viele Änderungen“ je Kennung, getrennt von der Episode des Sendens; `gebremst` in `/api/gesundheit` zählt auch diese 429.
- **Der Mensch merkt nichts davon**: Verkürzen im Dialog sind zwei Aufrufe (409-Vorschau, bestätigt) = 20 Einheiten ≤ 30. Läuft er doch in die Bremse, zeigt der Dialog „Channel-Einstellungen“ den Grund in der Fehlerzeile.

Zahlt ein auf: [Vision](R00000-vision.md) — „eine gemeinsame LAN-Instanz“ (ein entlaufenes Skript friert nicht die Tabs aller anderen ein) und „Mensch und KI sind gleichberechtigt“ (dieselbe Grenze für Agent und Mensch, kein Recht wird genommen, nur die Häufigkeit begrenzt). Kein Widerspruch zur Vision; vom architekt als keine Richtungsfrage eingeordnet (ADR-0036, Ergänzung Karte #31 (b)).

Herkunft: Karte #31 — Security-Review #8 Optional (958 Wechsel/s ohne Grenze, je Wechsel eine `INFORMATION frist`-Zeile und ein Ereignis an jeden Tab), Wechsel-Flut M3 aus R00008 (Lehre #5: 2.456 Wechsel/s froren einen Tab ein). Entschieden vom architekt in ADR-0036, Ergänzung Karte #31 (b), Option A; der Betreiber kann die Entscheidung kippen (Offene Fragen).

Slice (WBS `Dokumentation/Planung/localchata.md`, unter I0018 Aufbewahrungsfrist je Channel festlegen): F0072 Frist bucht am Schreibkonto.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00008, R00011 und R00016: Frist, Verkürzen, Bestätigung (409-Vorschau), Schreibkonto, Eimer, Einheit, Vorab, Rest, Grenze (`kennung` | `herkunft`), Episode, Kennung, Herkunft.

## Geschäftlicher Nutzen

Heute setzt jede Sitzung die Frist eines Channels so oft sie will. Ein Agent in einer Schleife — absichtlich oder versehentlich — schafft rund 958 Wechsel je Sekunde. Jeder Wechsel schreibt die Einstellungsdatei, eine `INFORMATION frist`-Zeile ins Betriebsprotokoll und schickt ein Ereignis an jeden offenen Tab. Die Tabs aller anderen frieren ein, und das Tagesbudget für Information und Warning (32 MiB) wäre in rund 3 Minuten erschöpft — danach gingen die Warnungen des Tages verloren.

Mit R00021 sinkt die Flut auf 3 Wechsel in Folge und dann einen je 10 s je Kennung, je Adresse auf 0,8 je Sekunde — ein Faktor über 1.000. Wer die Frist einmal im Monat ändert, merkt nichts. Die Betreiberin sieht im Protokoll, welche Kennung es versucht hat.

## Funktionale Anforderungen

### Frist bucht am Schreibkonto (F0072; ADR-0036 Ergänzung Karte #31 (b), Regeln 1–4)

- **Buchung in zwei Schritten**: `PUT /api/channels/{name}/frist` bucht nach der Sitzungsprüfung und **vor** dem Lesen des Körpers 1 Einheit vorab; nach den Prüfungen von Körper (400, 413, 415) und Wert (422) und **vor** dem Setzen die übrigen 9. Wie beim Senden wird nichts erstattet.
- **Was eine Anfrage kostet**:

  | Antwort | Kosten | Warum |
  |---|---|---|
  | 401 (ohne Sitzung), 403 (Grenzschutz) | 0 | vor der Buchung |
  | 400, 413, 415, 408 (Körper), 422 (Wert) | 1 | nur der Vorab-Schritt |
  | 404 (Channel unbekannt), 409 (Vorschau beim Verkürzen), 200 (auch derselbe Wert), 500 | 10 | Vorab und Rest |
  | 429 | 0 bzw. 1 | gebremst im Vorab-Schritt: 0; gebremst im Rest-Schritt: die Vorab-Einheit bleibt verbraucht |

  Die 404 bleibt vor der 422 (Reihenfolge „Channel vor Wert“, R00008): ist der Channel unbekannt, bucht die Route den Rest und antwortet 404.
- **Volle Kosten im Vorab-Schritt**: die Kosten stehen vor dem Lesen fest. Wird schon der Vorab-Schritt gebremst, rechnet `wieder_in_ms` bis die **vollen** 10 Einheiten tragen. Das Senden nennt im Vorab-Schritt weiter 1 (R00011 unverändert).
- **Antwort 429**: `{"fehler": …, "grenze": "kennung" | "herkunft", "wieder_in_ms": …}` und `Retry-After` in ganzen Sekunden, aufgerundet, mindestens 1. Texte:
  - `kennung`: „Zu viele Änderungen von beta@127.0.0.1 — wieder möglich in 10 s.“
  - `herkunft`: „Zu viele Änderungen von 127.0.0.1 (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in 2 s.“
  Die Sekunden im Text sind `Retry-After`. Die Texte des Sendens („Zu viele Nachrichten von …“) bleiben wörtlich.
- **Nichts geändert**: eine 429 schreibt keine Einstellungsdatei, löscht keinen Tag, schickt kein Ereignis `channel` und schreibt keine `INFORMATION frist`-Zeile.
- **Betriebsprotokoll**: jede 429 der Frist ist ein Auftreten der Episode `WARNING schreibkonto` „Frist gebremst: zu viele Änderungen“ je Kennung. Der Schlüssel enthält den Text, deshalb ist sie getrennt von der Episode „Senden gebremst: zu viele Nachrichten“ derselben Kennung. Beginn-Zeile mit `kennung=` und `grenze=` des ersten Auftretens; Ende-Zeile nach R00016 und — sobald R00020 F0070 gebaut ist — mit der Zählung je Grenze und nur dem Kontext der Quelle.
- **Kennzahl**: `gebremst` in `/api/gesundheit` zählt jede 429, auch die der Frist. Kein neues Feld (ADR-0043, Ergänzung Karte #31).
- **Oberfläche**: der Dialog „Channel-Einstellungen“ (R00008) zeigt bei einer 429 den `fehler` der Antwort in der Fehlerzeile mit ⚠. Der Dialog bleibt offen, die Frist in der Kopfzeile unverändert. Ob das Feld dabei `aria-invalid` trägt, regelt R00019 F0066 (Server- gegen Eingabefehler).

### Rechenbeispiele

Eimer aus R00011: je Kennung Vorrat 30, Nachschub 1 je Sekunde; je Herkunft Vorrat 120, Nachschub 8 je Sekunde.

- **Kennung allein**: 3 Änderungen × 10 = 30 → Eimer leer; die 4. Anfrage wird im Vorab-Schritt gebremst, `wieder_in_ms` = 10 Einheiten ÷ 1 je s = 10.000 → „wieder möglich in 10 s“, `Retry-After: 10`.
- **Herkunft**: 4 Kennungen von `127.0.0.1` × 3 Änderungen × 10 = 120 → Eimer der Herkunft leer; die erste Änderung einer 5. Kennung (eigener Eimer voll) → `grenze: "herkunft"`, `wieder_in_ms` = 10 ÷ 8 je s = 1.250 → „wieder möglich in 2 s“ (aufgerundet).
- **Mensch verkürzt**: 409-Vorschau (10) + bestätigt (10) = 20 ≤ 30 → nie gebremst; danach geht noch eine Änderung (30), die nächste ist gebremst.
- **Wechselwirkung mit dem Senden** (in Kauf genommen, ADR-0036): ein Agent sendet 20 kleine Nachrichten (20 Einheiten, 10 übrig), setzt die Frist (10, 0 übrig) und gleich noch einmal → 429, „wieder möglich in 10 s“.
- **Viele 422**: 25 ungültige Werte in Folge kosten 25 (je 1, alle 422); die nächste gültige Änderung bucht vorab 1 (4 übrig, dazu höchstens rund 2 Nachschub, wenn die Folge bis 2 s dauert) und scheitert am Rest 9 → 429.

## Nicht-funktionale Anforderungen

- **Sicherheit** (CWE-770): die Wechsel-Flut einer Kennung sinkt von rund 958 je Sekunde auf höchstens 3 in Folge und dann einen je 10 s; je Herkunft auf höchstens 0,8 je Sekunde dauerhaft. Prüfreihenfolge und Grenzen aus R00008 ändern sich nicht, außer dass vor dem Körper die Vorab-Buchung steht.
- **Betrieb**: die Episode zeigt, welche Kennung flutet; `gebremst` zeigt dem Monitoring, dass gebremst wird.
- **Benutzerfreundlichkeit**: ein Mensch, der im Dialog verlängert oder verkürzt, wird nie gebremst; wird er es doch, liest er den Grund und die Wartezeit.
- **Zeitkriterien** nach `AI-Team/planer.md`: neue Zeitgrenzen gibt es nicht. `wieder_in_ms` und `Retry-After` folgen exakt aus der gestellten Uhr (Unit); im echten Programm nur als Untergrenze (`Retry-After` ≥ 9 nach drei schnellen Änderungen).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, `S(<name>)`, Browser A und B wie in R00008 und R00011; `heute.log` ist `logs/<heute>.log`. „Eine Änderung“ ist ein `PUT /api/channels/allgemein/frist` mit einem gültigen Wert, der sich vom geltenden unterscheidet. Jedes Integrationskriterium startet eine eigene Instanz (alle Kennungen kommen von `127.0.0.1` und teilen den Eimer der Herkunft). „Die Zeile `X`“ meint die Zeile ab der Stufe, ohne Zeitstempel.

### F0072 Frist bucht am Schreibkonto

- [ ] **Vorab mit den vollen Kosten** (Unit, rein, gestellte Uhr; erster Test aus ADR-0036 Ergänzung Karte #31 (b), wörtlich): „Kennung mit vollem Eimer bucht 3 × (vorab 10 → ok, Rest 9 → ok); der 4. `vorab` → `Gebremst { kennung, 10.000 ms }`; nach 9.999 ms gebremst, nach 10.000 ms ok. Mutanten „vorab nennt 1“ (→ 1.000 ms) und „Rest 0“ werden rot.“
- [ ] **Herkunft** (Unit, rein, gestellte Uhr): vier Kennungen von `10.0.0.1` buchen je 3 × (vorab 10, Rest 9) → ok; der Vorab-Schritt einer fünften Kennung von `10.0.0.1` → `Gebremst { herkunft, 1.250 ms }`; eine Kennung von `10.0.0.2` → ok. Die Mutante „Herkunft bucht nur die Vorab-Einheit“ wird rot.
- [ ] **Senden unverändert** (Unit): das Senden ruft den Vorab-Schritt mit den Kosten 1; die Kriterien „Zwei Schritte und volle Kosten“ und „Eimer je Kennung“ aus R00011 bleiben grün.
- [ ] **Kennung im echten Programm** (Integration; erster Integrationstest aus ADR-0036 Ergänzung Karte #31 (b), wörtlich): „eine Kennung setzt die Frist abwechselnd auf 8 und 9 so schnell wie möglich → drei 200, die vierte Anfrage 429 mit `grenze: "kennung"` und `Retry-After` ≥ 9; die Einstellungsdatei trägt den Wert der dritten; danach kein weiteres Ereignis `channel` an einen zweiten Strom; das Protokoll hat genau eine `WARNING schreibkonto Frist gebremst: zu viele Änderungen — Episode beginnt`. Mutante „Frist bucht nicht“ → rot.“ Ausgeschrieben: `beta` setzt 8, 9, 8, 9 → 200, 200, 200, 429; `channels/allgemein/einstellungen.toml` trägt `frist_tage = 8`; der zweite Strom (`gamma`) bekommt genau drei Ereignisse `channel` für `allgemein`; die 429 hat `fehler` „Zu viele Änderungen von beta@127.0.0.1 — wieder möglich in <s> s.“ mit `<s>` = `Retry-After`; die Beginn-Zeile ist `WARNING schreibkonto Frist gebremst: zu viele Änderungen — Episode beginnt | kennung=beta@127.0.0.1 grenze=kennung`, auf stderr dieselbe Zeile.
- [ ] **Text je Grenze** (Unit): `Gebremst { kennung, 10.000 ms }` für die Frist → `fehler` „Zu viele Änderungen von beta@127.0.0.1 — wieder möglich in 10 s.“, `Retry-After: 10`; `Gebremst { herkunft, 1.250 ms }` → „Zu viele Änderungen von 127.0.0.1 (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in 2 s.“, `Retry-After: 2`, `wieder_in_ms: 1250`. Die Texte des Sendens bleiben wörtlich (Unit aus R00011 grün). Die Mutante „Frist sagt Nachrichten“ wird rot.
- [ ] **Kosten je Antwort** (Integration): (a) `beta` schickt 25 ungültige Werte (`{"tage":0}`) so schnell wie möglich → 25 × 422; die folgende Änderung → 429. (b) `delta` setzt die Frist von `gibtsnicht` dreimal → 3 × 404; die vierte Anfrage (an `allgemein`) → 429. (c) `epsilon` setzt denselben Wert dreimal (7, 7, 7) → 3 × 200 ohne Ereignis; die vierte → 429. Rechnung: (a) 25 + 1 vorab = 26 von 30, Rest 9 fehlt, solange die Folge unter 5 s bleibt; (b) und (c) je 3 × 10 = 30. Die Mutanten „422 kostet 10“ (a: die 4. Anfrage wäre 429) und „404 kostet 1“ (b: die 4. wäre 200) werden rot.
- [ ] **Verkürzen bleibt ungebremst** (Integration): `allgemein` hat Tage, die beim Verkürzen auf 1 entfielen; `beta` setzt 1 → 409; bestätigt → 200; setzt dann 5 → 200; setzt 6 → 429. Die Integrations- und E2E-Tests aus R00008, R00017 und R00019 laufen grün — wo einer mehr als drei Änderungen je Kennung in rund 10 s macht, verteilt er sie auf Kennungen oder wartet `Retry-After` ab (Teständerung, keine Grenze im Test schärfer als hier).
- [ ] **Episode getrennt vom Senden** (Integration, `LOCALCHATA_EPISODE_RUHE_MS=2000`): `beta` sendet 31 kleine Nachrichten so schnell wie möglich (die 31. → 429, Eimer leer) und setzt gleich danach die Frist → 429 „Zu viele Änderungen von beta@127.0.0.1 — wieder möglich in 10 s.“ (`Retry-After` 9 oder 10). `heute.log` hat für `beta` genau eine Beginn-Zeile „Senden gebremst: zu viele Nachrichten“ und genau eine Beginn-Zeile „Frist gebremst: zu viele Änderungen“, obwohl die Episode des Sendens noch offen ist; nach der Ruhe je eine Ende-Zeile, die der Frist mit `Episode endet: 1 in 0 s`. Die Mutante „Frist im selben Schlüssel wie das Senden“ (dann nur eine Beginn-Zeile) wird rot.
- [ ] **`gebremst` zählt mit** (Integration): `/api/gesundheit` → `gebremst` vor dem Kriterium „Kennung im echten Programm“ = g; danach g + 1.
- [ ] **Bremse im Dialog** (E2E): A hat „# allgemein — Einstellungen“ offen; über die Sitzung von A drei Änderungen per `PUT` (8, 9, 8); A setzt im Feld 9 und klickt „Speichern“ → die Fehlerzeile zeigt „⚠ Zu viele Änderungen von <Kennung von A> — wieder möglich in <s> s.“ mit `<s>` 9 oder 10; der Dialog bleibt offen; die Kopfzeile zeigt weiter `· 8 Tage`. Die Mutante „Fehlerzeile zeigt bei 429 den Status statt `fehler`“ wird rot.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zu der einen 429, die das Kriterium „Bremse im Dialog“ herbeiführt.

## Betroffene Verzeichnisstruktur

- Server: `src/schreibkonto/mod.rs` (`vorab` mit den vollen Kosten), `src/http/frist.rs` (Buchung in zwei Schritten, Episode, 429), `src/http/senden.rs` (`gebremst_beantworten` an einen gemeinsamen Ort, Aufruf `vorab(…, 1)`), Ort der gemeinsamen 429-Antwort nach Wahl des Entwicklers (z. B. `src/http/schreibkonto.rs`); Tests `tests/frist*.rs`, `tests/schreibkonto.rs` bzw. neu `tests/frist_schreibkonto.rs`.
- Oberfläche: kein Programmcode (`web/channel-einstellungen.js` `fehlertext` zeigt `daten.fehler` jeder Antwort); Test neu `e2e/tests/r00021-frist-bremse.spec.js`; bestehende `e2e/tests/r00008-frist.spec.js` zieht mit, falls er mehr als drei Änderungen je Kennung in Folge macht.

## Technische Überlegungen

### Ablauf

1. `PUT …/frist` → Grenzschutz → Sitzung → **`vorab(kennung, herkunft, 10)`** → gebremst? → Episode + 429.
2. Körper lesen (400, 413, 415, 408) → Channel bekannt? nein → **`rest(…, 10)`** → gebremst? → Episode + 429 · sonst 404.
3. Wert prüfen (422) → **`rest(…, 10)`** → gebremst? → Episode + 429.
4. `frist_setzen` → 200 | 409 | 500 wie bisher (R00008, R00019).

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0072: B0341 Vorab mit den vollen Kosten · B0342 Bremse antwortet je Route · B0343 Frist bucht 1 + 9 · B0344 Bremse im Dialog belegt

Elemente: `Schreibkonto::vorab(&self, kennung, herkunft, volle_kosten: u32)`; eine 429-Antwort mit dem Gegenstand („Nachrichten“ | „Änderungen“) als Parameter; in der Frist-Route ein `gebremst_melden` mit dem Text „Frist gebremst: zu viele Änderungen“ (Namen entscheidet der Entwickler).

Wireframe: kein Artboard `D0003.dc.html`. Gestaltungsquelle: Dialog „Channel-Einstellungen“ wie gebaut (R00008, ADR-0033); keine neue Gestaltung.

### Änderungen an bestehenden Klassen

- `schreibkonto::Schreibkonto::vorab`: neues Argument `volle_kosten`; `wieder_in` rechnet bis zu diesen Kosten.
- `http::senden::gebremst_beantworten` → gemeinsam für Senden und Frist, mit Gegenstand.
- `http::frist`: Buchung vor dem Körper und vor dem Setzen; der 404-Zweig bucht den Rest.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Unit (rein, gestellte Uhr)**: Vorab mit den vollen Kosten, Herkunft, Senden unverändert, Text je Grenze.
- **Integration (echtes Programm)**: Kennung im echten Programm, Kosten je Antwort, Verkürzen bleibt ungebremst, Episode getrennt vom Senden, `gebremst` zählt mit.
- **E2E**: Bremse im Dialog.

## Abhängigkeiten

- Abhängig von: F0033 Frist über die API setzen und lesen (gruen), F0057 Senden im Betriebsprotokoll (gruen; Episoden-Mechanik des Schreibkontos). Fachlich frei.
- Nicht parallel, sondern nacheinander mit: F0064 und F0066 (R00019, Karte #27) — dieselbe Route `src/http/frist.rs` und derselbe Dialog `web/channel-einstellungen.js`. Ist F0066 vor F0072 gebaut, trägt das Feld an der 429 kein `aria-invalid`; sonst danach.
- Ende-Zeile mit Zählung je Grenze: kommt mit R00020 F0070 (Karte #31); bis dahin endet die Episode wie in R00016.
- Blockiert: —

## Offene Fragen

- **An den Betreiber (nicht blockierend)**: der architekt hat die Buchung am **selben** Konto gewählt (Option A). Folge: ein Agent, der eben 20 Nachrichten gesendet hat, wartet beim zweiten Frist-Aufruf rund 10 s; eine Frist-Schleife bremst das Senden derselben Kennung. Will der Betreiber das nicht, gilt Option B (eigener Eimer nur für die Frist, z. B. 3 je Kennung, 1 je 10 s) — dann wird diese Anforderung vor dem Bau angepasst.

Entschieden vom planer (Rückfragen mit Empfehlung beantwortet):

- **404 kostet 10, ohne die Reihenfolge „Channel vor Wert“ zu ändern**: ADR-0036 nennt die 404 bei den vollen Kosten, ADR-0031 prüft den Channel vor dem Wert. Beides hält, wenn der 404-Zweig den Rest bucht, bevor er antwortet.
- **Herkunft als Unit, nicht als Integration**: im echten Programm füllt der Nachschub (8 je Sekunde) den Eimer der Herkunft schneller, als 13 Anfragen sicher laufen; mit gestellter Uhr ist das Ergebnis exakt.
- **Oberfläche ohne Programmcode**: die Fehlerzeile zeigt den `fehler` jeder Antwort schon; nur der Beleg fehlt.

## Manuelle Vorbereitungstätigkeiten

- Keine.

## Manuelle Nachbereitungstätigkeiten

- Anleitung für Agenten (I0019, noch nicht gebaut): die Frist kostet 10 Einheiten am selben Konto wie das Senden — beim Schreiben der Anleitung aufnehmen.

## Umfang

Gezählt aus der WBS (Bubbles). 0,4 h ist die Untergrenze je Bubble nach dem Skill `work-breakdown-structure`; Richtwert, ungemessen für diese Bubble-Typen.

- F0072 Frist bucht am Schreibkonto: 4 Bubbles · 4 klar (1,6 h), keine unklaren

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist der Security-Review #8 (Optional): 958 Wechsel je Sekunde ohne Grenze, je Wechsel eine Protokollzeile und ein Ereignis an jeden Tab; dazu die Wechsel-Flut M3 aus R00008 und Lehre #5 (2.456 Wechsel je Sekunde froren einen Tab ein).

Die Wirkungskette:

- Jede Friständerung bucht 10 Einheiten am Schreibkonto. Deshalb sind nach drei Änderungen einer Kennung die Einheiten aufgebraucht, und der Nachschub erlaubt nur noch eine je 10 s.
- Die gebremste Anfrage ändert nichts und schickt kein Ereignis. Deshalb erreichen die Tabs der anderen höchstens drei Wechsel in Folge statt Tausender, und das Protokoll bekommt eine Episode statt einer Zeile je Wechsel.
- Das Konto ist schon gebaut und getestet. Deshalb kommt keine zweite Mechanik mit eigenen Konstanten dazu, und die Grenzen gelten für Agent und Mensch gleich.
- Zwei Aufrufe beim Verkürzen kosten 20 von 30. Deshalb bemerkt ein Mensch die Bremse nicht.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus R00008, R00011 und R00016.

## Notizen

- **Änderung von Zusagen** (R00008 und R00011 bleiben an diesen Stellen unverändert, R00021 nennt die Änderung):
  - R00008 „Prüfreihenfolge“ der Frist-Route: nach der Sitzung steht die Vorab-Buchung, vor dem Setzen die Rest-Buchung; eine Anfrage kann 429 antworten.
  - R00008 M3 „Wechsel-Flut“ (Messwert im Entwicklertest): misst danach höchstens 3 Wechsel in Folge und dann einen je 10 s je Kennung; die Testhilfe B0192 verteilt ihre Schleife auf Kennungen, wenn sie die alte Last messen soll.
  - R00011 „Schreibkonto“: das Konto bucht auch die Frist (ADR-0036 Regel 8 liest sich: Senden in Channels, Direktnachrichten und Setzen der Frist).
- Verworfen (Optionen des architekt in ADR-0036, Ergänzung Karte #31 (b)): eigener Eimer nur für die Frist (zweites Eimerpaar mit eigenen Konstanten, Tests und Zählung für eine Route, die ein Mensch einmal im Monat nutzt); 1 Einheit wie eine kleine Nachricht (8 Wechsel je Sekunde je Herkunft, Tagesbudget in rund 6 h erschöpft); keine Rate (Flut trifft alle anderen).

### Bewusst außerhalb

- Plattenquote für die Gesprächslogs (ADR-0036 Ergänzung Karte #31 (a)): Frage an den Betreiber, nicht hier.
- Frist der Direktgespräche (I0012).
