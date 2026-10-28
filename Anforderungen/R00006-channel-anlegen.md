---
id: R00006
status: Neu
datum: 2026-10-28
---

# R00006: Channel anlegen

## Beschreibung

Jeder Teilnehmer — Mensch im Browser oder Agent über die API — legt einen Channel mit einem Namen an, und der neue Channel steht sofort in der Liste aller Teilnehmer: in der Seitenleiste jedes offenen Browsers und im Ereignisstrom jedes Agenten. Ein Channel ist ein Ordner `channels/<name>/` mit einer lesbaren `einstellungen.toml` (wann, von wem); er übersteht jeden Neustart. Ungültige Namen werden mit einem Vorschlag abgewiesen, ein vorhandener Name mit dem vorhandenen Channel beantwortet, und Grenzen je Rechner und gesamt verhindern, dass ein entlaufener Agent den Ordner mit Channels füllt.

Zahlt ein auf: [Vision](R00000-vision.md) — „Mehrere Channels“ (ein Channel zwischen Security-Factory und Entwicklungs-Factory), „Ein Chat, der ein Ordner ist“, „Mensch und KI sind gleichberechtigt“, „Live überall“.

Entschieden vom Director nach Empfehlung (Vision nicht berührt, dem Betreiber gesammelt vorgelegt): **alle Teilnehmer sehen alle Channels**; ein frischer Ordner bekommt **keinen** automatischen Channel `allgemein`; der Namensvorschlag bildet **Buchstaben mit Akzent auf den Grundbuchstaben** ab (`José` → `jose`).

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0007 Channel anlegen): F0020 Channel über die API anlegen und listen, F0021 Channels live im Ereignisstrom, F0022 Grenzen beim Anlegen, F0023 Ordner von Hand und Plattenfehler, F0024 Seitenleiste im Browser, F0025 Channel im Browser anlegen. Nicht beauftragt: F0026 Beschreibung (Ausbaustufe 2).

## Geschäftlicher Nutzen

Bis R00005 gibt es nur die Channels, deren Ordner jemand von Hand angelegt hat, und niemand sieht sie. Zwei Factories, die sich einen gemeinsamen Ort einrichten wollen, brauchen dafür einen Menschen mit Zugriff auf den Ordner. R00006 macht das Anlegen zu einem Aufruf, den ein Agent selbst ausführt („anlegen, falls es fehlt“: 201 oder 409 mit dem vorhandenen Channel), und zeigt jedem Teilnehmer ohne Neuladen, welche Channels es gibt. Damit ist der Weg für I0008 (Channel wechseln, Verlauf lesen) und I0009 (Nachricht senden) frei.

## Funktionale Anforderungen

### Channel über die API anlegen und listen (F0020; ADR-0020, ADR-0021, ADR-0022)

- `POST /api/channels` mit Sitzung (`X-LocalchatA-Sitzung`) und JSON `{"name": …}` legt einen Channel an; Mensch und Agent gleich.
- **Name** = Ordnername: 1–32 Zeichen aus `a–z`, `0–9`, `-`, `-` nicht am Rand (Regel aus ADR-0013). **Reserviert** sind `con`, `prn`, `aux`, `nul`, `com0`–`com9`, `lpt0`–`lpt9`.
- **Abweisen statt umformen**: ein ungültiger Name → 422 `{fehler, regel, vorschlag?}`; `vorschlag` ist die Umformung aus ADR-0013, wenn sie nicht leer und nicht reserviert ist.
- **Vorschlag mit Akzenten** (gilt für Teilnehmer- und Channelnamen, Server und Browser gleich): Buchstaben mit diakritischem Zeichen aus U+00C0–U+017F werden ihr Grundbuchstabe (`é` → `e`, `ñ` → `n`, `å` → `a`, `ł` → `l`, `ø` → `o`, `æ` → `ae`, `œ` → `oe`); `ä`, `ö`, `ü`, `ß` bleiben `ae`, `oe`, `ue`, `ss`; alles andere wird wie bisher `-`.
- **Antwort 201** `{channel: {name, angelegt, angelegt_von, beschreibung}}`: `angelegt` Serverzeit RFC 3339 mit Millisekunden, `angelegt_von` die Kennung des Anlegers, `beschreibung` `null`.
- **Ordner** `channels/<name>/` entsteht beim Anlegen, darin sofort `einstellungen.toml` mit `angelegt` und `angelegt_von` (ohne `name`), geschrieben über `einstellungen.toml.neu` und Umbenennen.
- **Vorhanden** (Name Zeichen für Zeichen gleich) → 409 `{fehler, channel}` mit dem vorhandenen Channel.
- **Gleichzeitig**: von mehreren gleichzeitigen Anlagen desselben Namens bekommt genau eine 201.
- **Beschreibung** gehört zu Ausbaustufe 2 (F0026, nicht beauftragt): `beschreibung` ist in jeder Antwort, jedem Stand und jedem Ereignis `null`; ein Körper mit `beschreibung`, die weder fehlt noch `null` noch `""` ist, → 422 `{fehler, regel: "beschreibung"}` — kein stilles Verwerfen. Ein Schlüssel `beschreibung` in einer vorhandenen `einstellungen.toml` gilt bis dahin als unbekannter Schlüssel.
- `GET /api/channels` mit Sitzung → 200 `{stand, liste}`, `liste` nach Name sortiert, je Eintrag dieselbe Form wie in der 201.
- **Laden beim Start**: jeder gültige Ordner unter `channels/` ist ein Channel; `angelegt` und `angelegt_von` kommen aus seiner `einstellungen.toml`, fehlt sie, sind beide `null`. `stand` beginnt nach jedem Start bei 0.
- Schutz wie jede Schreibroute: ohne gültige Sitzung 401, fremder `Origin` 403, Körper zu groß 413, ohne JSON-`Content-Type` 415.

### Channels live im Ereignisstrom (F0021; ADR-0023)

- `GET /api/ereignisse` liefert als **erstes** Ereignis wie bisher `{"art":"teilnehmer",…}`, als **zweites** `{"art":"channels","stand":n,"liste":[…]}` (Form wie `GET /api/channels`).
- Jedes Anlegen erzeugt `{"art":"channel","stand":n,"channel":{…}}` an alle Ströme; `stand` des Themas Channels wächst mit jedem Anlegen um 1 und ist unabhängig vom `stand` der Teilnehmer.
- Ein Strom sendet einen Channel-Wechsel nur, wenn sein `stand` größer ist als der zuletzt gesendete des Themas Channels; ein neuer Stand nach Überlauf (`NeuerStand`) sendet beide Stände.

### Grenzen beim Anlegen (F0022; ADR-0021 Regeln 3–5)

- **Je Herkunft** höchstens **32** Channels, deren `angelegt_von` diese Herkunft trägt — gezählt aus den Einstellungsdateien, also über Neustarts hinweg. Channels ohne bekannten Anleger zählen nur gesamt.
- **Gesamt** höchstens **256** Channels einschließlich der beim Start geladenen; liegen beim Start mehr im Ordner, werden alle geladen und nur neue abgewiesen.
- Grenze erreicht → 429 `{fehler, grenze: "herkunft" | "gesamt", hoechstens}`; nichts wird angelegt.
- Prüfreihenfolge: Namensregel → vorhanden → gesamt → je Herkunft. Ein vorhandener Name ergibt 409 auch bei erreichter Grenze.
- `/api/gesundheit` nennt zusätzlich `channels` (Anzahl).

### Ordner von Hand und Plattenfehler (F0023; ADR-0020 Regel 6, ADR-0021 Regel 1, ADR-0022 Regeln 2–3)

Ausnahmewege — Kriterien nach ihrem Nutzen: nichts geht verloren, nichts wird überschrieben, der Betreiber erfährt, was los ist.

- Liegt `channels/<name>/` im Ordner, ist aber nicht geladen (von Hand angelegt, während das Programm lief) → 409 `{fehler, regel: "ordner-vorhanden"}` mit dem Text „`channels/<name>` besteht im Ordner schon; localchatA übernimmt ihn beim nächsten Start.“; am Ordner ändert sich nichts.
- Scheitert das Anlegen des Ordners oder das Schreiben der Einstellungen → 500 `{fehler}`; ein eben angelegter Ordner wird wieder entfernt; eine Zeile auf stderr nennt Pfad und Grund; kein Ereignis, `stand` unverändert.
- **Start**: ein Unterordner von `channels/` mit ungültigem Namen ist kein Channel; beginnt er mit `.`, wird er still übergangen, sonst steht eine Zeile auf stderr („`channels/<ordner>`: kein gültiger Channelname, übergangen“) und er zählt in `defekte_zeilen`. Er wird nie umbenannt oder gelöscht.
- **Start**: eine defekte `einstellungen.toml` (kein TOML, falscher Typ, kein UTF-8, `angelegt_von` keine gültige Kennung) → der Channel wird geladen mit `angelegt` und `angelegt_von` `null`, eine Zeile auf stderr nennt Datei und Zeile, `defekte_zeilen` zählt sie; die Datei bleibt unangetastet. Ein unbekannter Schlüssel ist kein Defekt.

### Seitenleiste im Browser (F0024; ADR-0024 Regeln 1–3, 6, 9; Designsystem `components/Seitenleiste`)

- Nach dem Beitritt steht links die Seitenleiste `<nav aria-label="Channels">`: Kopf „Channels“, rechts der Schalter `+` (zugänglicher Name „Channel anlegen“, `aria-keyshortcuts="Control+Alt+N"`), darunter je Channel ein Eintrag `# name`, nach Name sortiert.
- Leere Liste: „Noch keine Channels.“ unter dem Kopf.
- Ein von anderen angelegter Channel erscheint an seiner Sortierstelle, ohne Neuladen, ohne den Fokus zu verschieben und ohne Ansage.
- Jeder neue Tab zeigt die Liste, ohne dass jemand etwas anlegt (der Worker gibt beide Stände weiter; im Rückfall ohne `SharedWorker` der eigene Strom).
- Unter 720 px zeigt die Leiste je Eintrag nur `#` mit dem Namen als `title` und zugänglichem Namen; `+` bleibt sichtbar.
- Tab-Reihenfolge: Kopfzeile → Seitenleiste → Mitte → Teilnehmerliste.
- Einträge sind noch keine Links, es gibt keinen aktiven Eintrag und keine Zähler (I0008 und später).

### Channel im Browser anlegen (F0025; ADR-0024 Regeln 4, 5, 7)

- `+` oder `Strg Alt N` öffnet überall in der Hauptansicht den modalen Dialog „Channel anlegen“ mit dem Feld „Name“ (Fokus darin, Hinweis mit der Regel, Live-Prüfung wie beim Beitreten), den Schaltern „Channel anlegen“ (primär) und „Abbrechen“. `Strg Alt N` wirkt nicht, wenn `AltGraph` gedrückt ist.
- `Enter` im Feld legt an. 201 → Dialog zu, Statuszeile (`role="status"`) „Channel # name angelegt.“, der Eintrag steht **einmal** in der Liste, der Fokus kehrt zum Auslöser zurück.
- Fehler bleiben im offenen Dialog, die Eingabe bleibt stehen, Fehlerzeile in `kd-danger` mit ⚠, Feld `aria-invalid="true"`, Fehlertext über `aria-describedby`:
  - ungültiger Name → Regel und, wenn es einen Vorschlag gibt, Schalter „`<vorschlag>` übernehmen“, der den Vorschlag ins Feld setzt;
  - vorhanden → „# name gibt es schon.“;
  - Grenze → Meldung, die mit „Zu viele Channels“ beginnt und die Zahl aus `hoechstens` nennt;
  - Netzfehler → „Nicht angelegt — Verbindung prüfen.“
- `Esc` und „Abbrechen“ schließen ohne Anlegen; der Fokus kehrt zum Auslöser zurück (`+` oder das zuvor fokussierte Element).
- Die Mitte bleibt die leere Fläche aus I0004.

## Nicht-funktionale Anforderungen

- Performance (Zeitgrenzen mit Zweck, Startpunkt und erwartetem Wert; Grenze jeweils mehr als 30 % über dem Erwarteten):
  - **Zustellung an Agenten** (Zweck: „erscheint sofort bei allen“, `design-prompt.md` 4.3 — ein Agent, der auf neue Channels reagiert, wartet nicht; Start: Erhalt der 201 beim Anleger; Beobachter: ein anderer Strom): Grenze **2 s**, erwartet wenige Millisekunden — gerechnet, nicht gemessen: das Ereignis wird unter der Sperre vor der Antwort veröffentlicht (ADR-0023 Regel 2), kein Takt liegt im Weg.
  - **Zustellung an Browser** (Zweck wie oben, für Menschen; Start: Erhalt der 201 beim Anleger im Testprozess; Beobachter: die Seitenleiste eines anderen Browsers, ereignisgenau mit Prüfintervall ≤ 25 ms): Grenze **2 s**, erwartet unter 0,1 s — gerechnet: Strom → Worker → Tab → Zeichnen ohne Takt.
  - **Neuer Tab** (Zweck: wer einen Tab öffnet, sieht sofort, welche Channels es gibt; Start: Rückkehr der Navigation): Grenze **2 s**, erwartet unter 0,5 s — gerechnet: Seite lokal, `ich`, gemerkter Stand im Worker, keine Abfrage der Liste.
  - Kosten: jede (Wieder-)Verbindung überträgt die ganze Channel-Liste, bei 256 Channels ohne Beschreibung etwa 45 KB (gerechnet in ADR-0021).
- Sicherheit: nur mit Sitzung, Grenzschutz wie jede Schreibroute (ADR-0014); der Name ist eine Erlaubnisliste und kann keinen Pfad verlassen (`..`, `/`, `\` sind ungültig); Grenzen ohne Gegenzug, der sie aufhebt (es gibt kein Löschen); die Grenze je Herkunft übersteht den Neustart.
- Benutzerfreundlichkeit: Fokus nie im Nichts (`document.activeElement` ist nach dem Schließen der Auslöser); kein Querscrollen bei 320 px; Fehler nie nur über Farbe (⚠ und Wort); fremd angelegte Channels ohne Ansage.
- Testbarkeit: Regeln (Reihenfolge, `stand`, Grenzen, Inhalt der Datei) exakt über Antworten, Dateien und die gestellte Uhr; Zeitgrenzen nur als obere Grenze zwischen zwei Augenblicken beim selben Beobachter (Projektregel #4). Tests sind nie schärfer als diese Anforderung: sie übernehmen die Grenzen von oben wörtlich.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem frischen Ordner, `<p>` ist der Port aus der Startzeile; weitere Absender `127.0.0.2` … `127.0.0.9` (`curl --interface …`, Linux-Loopback). Beitritte per `curl` ohne `art` sind Agenten. `S(<name>)` ist der Header `X-LocalchatA-Sitzung` mit dem Token aus dem Beitritt von `<name>`. „Neustart“ heißt: Programm mit Strg+C beenden und auf demselben Ordner neu starten; Sitzungen leben nur im Speicher, Beteiligte treten danach neu bei.

### F0020 Channel über die API anlegen und listen

- [ ] Anlegen (Integration, echtes Programm): `alpha` (127.0.0.2) sendet `{"name":"allgemein"}` → 201 `{channel:{name:"allgemein", angelegt, angelegt_von:"alpha@127.0.0.2", beschreibung:null}}`, `angelegt` RFC 3339 mit Millisekunden; danach liegen `<ordner>/channels/allgemein/einstellungen.toml` und keine `einstellungen.toml.neu`; die Datei enthält `angelegt` und `angelegt_von` mit denselben Werten wie die 201 und keinen Schlüssel `name`.
- [ ] Serverzeit (Unit, Verzeichnis mit Speicherablage, gestellte Uhr auf `2026-10-28T10:15:00.123+01:00`): `channel_anlegen("allgemein", alpha@127.0.0.2)` → `angelegt` = `2026-10-28T10:15:00.123+01:00`, `stand` 1; ein zweites Anlegen desselben Namens → `Vorhanden` ohne Schreibvorgang in der Ablage und ohne neuen `stand`.
- [ ] Liste: `GET /api/channels` mit `S(beta)` nach den Anlagen `security-entwicklung` und `allgemein` → 200 `{stand:2, liste:[allgemein, security-entwicklung]}` (nach Name, nicht nach Anlage); ohne Header → 401.
- [ ] Über den Neustart: nach einem Neustart liefert `GET /api/channels` dieselben zwei Channels mit denselben `angelegt` und `angelegt_von`, `stand` 0.
- [ ] Ohne Einstellungsdatei: ein vor dem Start von Hand angelegter Ordner `channels/notizen/` ohne Datei erscheint in der Liste mit `angelegt: null`, `angelegt_von: null`, ohne Ladedefekt.
- [ ] Namensregel als Spanne (Unit, `Channelname`): 32 Zeichen `a` → gültig, 33 → ungültig; `a`, `a-1`, `com10`, `console`, `lpt` gültig; `-a`, `a-`, `a_b`, `a b`, `..`, `a/b`, `""` ungültig; `con`, `nul`, `com0`, `com9`, `lpt0`, `lpt9` → `reserviert` ohne Vorschlag; `CON` → ungültig ohne Vorschlag (die Umformung `con` ist reserviert).
- [ ] Vorschlag (Unit, Server und Node-Test Browser mit derselben Tabelle): `Allgemein` → `allgemein`; `Prüfung` → `pruefung`; `José` → `jose`; `Café Crème` → `cafe-creme`; `Ñandú` → `nandu`; `Łódź` → `lodz`; `Ærø` → `aero`; `Straße` → `strasse`; `日本` → kein Vorschlag. Dieselbe Tabelle gilt für Teilnehmernamen (`POST /api/beitreten {"name":"José"}` → 422 mit `vorschlag: "jose"`).
- [ ] Abweisen (Integration): `Allgemein` → 422 mit `regel` und `vorschlag: "allgemein"`; `con` → 422 `regel: "reserviert"` ohne `vorschlag`; danach liegt unter `channels/` kein neuer Ordner.
- [ ] Vorhanden: `beta` (127.0.0.3) sendet `{"name":"allgemein"}` nach `alpha` → 409 `{fehler, channel}`, `channel.angelegt_von` = `alpha@127.0.0.2`; `einstellungen.toml` ist byte-gleich wie vorher.
- [ ] Gleichzeitig (Integration): 20 gleichzeitige `POST {"name":"sync"}` von `alpha` → genau eine 201 und 19 × 409, jede 409 mit demselben `channel.angelegt` wie die 201; unter `channels/` genau ein Ordner `sync`.
- [ ] Beschreibung noch nicht: `{"name":"x1","beschreibung":"Absprachen"}` → 422 `regel: "beschreibung"`, kein Ordner `x1`; `{"name":"x2","beschreibung":null}` und `{"name":"x3","beschreibung":""}` → 201 mit `beschreibung: null`.
- [ ] Schutz: ohne Header → 401; mit `S(alpha)` und `Origin: http://evil.example` → 403; ohne `Content-Type: application/json` → 415; Körper über der Körpergrenze (`src/http/koerper.rs`) → 413; in keinem Fall entsteht ein Ordner.

### F0021 Channels live im Ereignisstrom

- [ ] Reihenfolge: `beta` öffnet den Strom auf einem frischen Ordner → Ereignis 1 `{"art":"teilnehmer",…}`, Ereignis 2 `{"art":"channels","stand":0,"liste":[]}`.
- [ ] Live (Integration): `beta` liest den Strom; `alpha` legt `allgemein` an → `beta` liest `{"art":"channel","stand":1,"channel":{name:"allgemein",…}}` mit demselben `channel` wie in der 201, **spätestens 2 s nach Erhalt der 201** durch `alpha` (erwartet wenige ms).
- [ ] Gleichzeitig (Integration): 30 Anlagen verschiedener Namen durch `alpha` laufen parallel zu 10 Strom-Öffnungen durch `beta` → jeder der 10 Ströme kennt **spätestens 2 s nach der letzten 201** genau die 30 Channels (Vereinigung aus seinem Stand `channels` und seinen Wechseln `channel`); je Strom kommt jede `stand`-Nummer des Themas Channels höchstens einmal vor, und die Folge (Stand, dann Wechsel) steigt streng.
- [ ] Unter der Sperre (Unit, aufzeichnender Rückruf): drei Anlagen → der Rückruf erhält `stand` 1, 2, 3 in dieser Reihenfolge, jeweils während das Verzeichnis gesperrt ist (`try_lock` im Rückruf scheitert); `channels()` liefert danach `stand` 3 und drei Einträge nach Name.
- [ ] Testhilfe: `staende()` in `tests/gemeinsam/strom.rs` liefert die Stände eines Themas; die bestehenden Prüfungen „streng steigend“ aus R00004/R00005 (`tests/ereignisse.rs`) laufen gegen das Thema Teilnehmer und bleiben grün.
- [ ] Mutationsprobe: wird die Veröffentlichung aus der Sperre genommen (nach dem Entsperren aufgerufen), wird mindestens ein Test rot; der Entwickler nennt das Ergebnis.

### F0022 Grenzen beim Anlegen

Wertprüfungen gegen eine Grenze als Spanne: der Wert an der Grenze geht, der nächste nicht, und die Mutanten „Grenze zu klein“ und „Grenze zu groß“ werden rot.

- [ ] Je Herkunft (Unit, Speicherablage): 32 Anlagen von `10.0.0.1` → 32 × Erfolg, die 33. → `Grenze {herkunft, 32}`; eine Anlage von `10.0.0.2` danach → Erfolg; nach erneutem Laden derselben Ablage bleibt die nächste von `10.0.0.1` bei `Grenze {herkunft, 32}`.
- [ ] Gesamt (Unit, Speicherablage): 8 Herkünfte × 32 = 256 → Erfolg, die 257. von einer neunten Herkunft → `Grenze {gesamt, 256}`; ein vorhandener Name in diesem Zustand → `Vorhanden`, nicht `Grenze`.
- [ ] Ohne Anleger (Unit): 10 Channels mit `angelegt_von: null` in der Ablage → `10.0.0.1` kann weiter 32 anlegen; die Gesamtgrenze zählt die 10 mit (also noch 246 frei).
- [ ] Integration (echtes Programm): `alpha` legt `c01` … `c32` an → 32 × 201, `c33` → 429 `{fehler, grenze:"herkunft", hoechstens:32}`; `beta` legt danach `d01` an → 201; nach einem Neustart und neuem Beitritt von `alpha` → `c33` wieder 429 `herkunft`.
- [ ] Integration gesamt: 8 Agenten von `127.0.0.2` … `127.0.0.9` legen je 32 Channels an → 256 × 201; ein Agent von `127.0.0.1` legt einen weiteren an → 429 `{grenze:"gesamt", hoechstens:256}`; `POST {"name":"c01"}` (vorhanden) → 409; `/api/gesundheit` → `"channels":256`.
- [ ] Gesundheit: frischer Ordner → `"channels":0`; nach einer Anlage → `1`.
- [ ] Mutationsprobe: `JE_HERKUNFT` auf 31 und auf 33 sowie `GESAMT` auf 255 und auf 257 gesetzt → je mindestens ein Test rot; der Entwickler nennt die vier Ergebnisse.

### F0023 Ordner von Hand und Plattenfehler

Ausnahmewege — Kriterien nach Nutzen.

- [ ] Ordner vorhanden (Integration): während das Programm läuft, wird `<ordner>/channels/notizen/` mit einer Datei `zettel.txt` von Hand angelegt; `POST {"name":"notizen"}` → 409 `{fehler, regel:"ordner-vorhanden"}`; der Ordner enthält danach genau `zettel.txt`, unverändert; `GET /api/channels` enthält `notizen` nicht; nach einem Neustart enthält sie `notizen` mit `angelegt_von: null`.
- [ ] Plattenfehler (Unit, Speicherablage „Einstellungen schreiben scheitert“): Anlegen → `Ablage`-Fehler; der Ordner ist zurückgebaut, `stand` unverändert, kein Rückruf.
- [ ] Plattenfehler (Integration, Linux, nicht als root): `channels/` ist nicht beschreibbar (`chmod a-w`) → `POST {"name":"neu"}` → 500 `{fehler}`; stderr enthält eine Zeile mit `channels/neu` und dem Grund; `channels/neu` existiert nicht; kein Strom erhält ein Ereignis `channel`; nach `chmod u+w` → derselbe Aufruf 201.
- [ ] Ungültige Ordner beim Start (Integration): vor dem Start liegen `channels/allgemein/` (mit gültiger Datei), `channels/Test/` und `channels/.neu-x/` → `GET /api/channels` liefert genau `allgemein`; stderr enthält genau eine Zeile zu `channels/Test`, keine zu `.neu-x`; beide Ordner bestehen danach unverändert.
- [ ] Defekte Einstellungen beim Start (Integration): `channels/kaputt/einstellungen.toml` enthält `angelegt = "` (offene Zeichenkette) → `kaputt` steht in der Liste mit `angelegt: null`, `angelegt_von: null`; stderr nennt `channels/kaputt/einstellungen.toml` mit Zeile 1; die Datei ist byte-gleich wie vorher.
- [ ] Zählung: mit `channels/Test/` und der defekten Datei aus den zwei Kriterien davor in einem Ordner → `defekte_zeilen` in `/api/gesundheit` ist 2.
- [ ] Format (Unit, A00013, rein): Rundweg `Einstellungen` → Text → `Einstellungen` gleich; offene Zeichenkette, `angelegt_von = "Claude@x"`, `angelegt = 5`, ungültiges UTF-8 → je `Defekt` mit Zeile; `farbe = "rot"` → gelesen ohne Defekt und beim Zurückschreiben erhalten.

### F0024 Seitenleiste im Browser

Rein (`node --test`) und Playwright mit Chromium gegen das echte Programm. Browser A und B sind getrennte Browser-Kontexte; Agenten legen über die API an, damit die 201 im Testprozess ankommt.

- [ ] Zusammenführen (Node-Test, rein): Liste `{stand 5, [a, c]}`; Wechsel `stand 4` → unverändert; Wechsel `stand 6` mit `b` → `[a, b, c]`, `stand 6`; Wechsel `stand 7` mit `b` → `b` ersetzt, kein zweites `b`; Stand `stand 2` mit `[x]` (nach Neustart) → `[x]`; Ereignis mit unbekannter `art` → unverändert.
- [ ] Worker (Node-Test): Worker empfängt `teilnehmer` und `channels`, danach verbindet ein neuer Tab → der Tab erhält beide Stände.
- [ ] Leer (E2E): A tritt auf einem frischen Ordner bei → Seitenleiste mit Überschrift „Channels“, Schalter mit zugänglichem Namen „Channel anlegen“ und dem Text „Noch keine Channels.“.
- [ ] Live (E2E): A und B sind beigetreten; der Agent `alpha` legt `allgemein` über die API an → B zeigt den Eintrag `# allgemein` **spätestens 2 s nach Erhalt der 201** (erwartet unter 0,1 s), ohne Neuladen; der Fokus in B ist danach derselbe wie davor; danach legt `alpha` `zeta` und `beta` an → Reihenfolge `# allgemein`, `# beta`, `# zeta`.
- [ ] Zweiter Tab (E2E): `allgemein` existiert, A Tab 1 zeigt ihn; A öffnet Tab 2 → Tab 2 zeigt `# allgemein` **spätestens 2 s nach der Rückkehr der Navigation** (erwartet unter 0,5 s), ohne dass etwas angelegt wird.
- [ ] Rückfall (E2E, Kriterium nach Nutzen): A ohne `SharedWorker` (Init-Skript) → A zeigt die vorhandenen Channels und einen danach angelegten.
- [ ] Schmal (E2E): bei 320 px mit einem Channel, dessen Name 32 Zeichen hat → `scrollWidth <= innerWidth`; der Eintrag zeigt `#`, sein `title` und sein zugänglicher Name sind der Name; `+` ist sichtbar.
- [ ] Tab-Reihenfolge (E2E): von der Kopfzeile aus erreicht `Tab` den Schalter `+` vor der Teilnehmerliste.
- [ ] Konsole der beobachtenden Seite (in „Leer“, „Zweiter Tab“, „Schmal“ A, in „Live“ B): keine unbehandelte Ausnahme (`pageerror`) und kein CSP-Verstoß. Konsolenfehler: in „Leer“, „Live“, „Zweiter Tab“ und „Schmal“ keine — dort schreibt diese Anforderung keine Fehlerantwort vor. Für „Rückfall“ gilt das Kriterium nach Nutzen, keine weitere Vorgabe zur Konsole.

### F0025 Channel im Browser anlegen

- [ ] Kürzel (Node-Test, rein): `{ctrlKey, altKey, key:'n'}` ohne `AltGraph` → öffnen; dasselbe mit `getModifierState('AltGraph')` wahr → nichts; `{ctrlKey, key:'n'}` → nichts; `{ctrlKey, altKey, key:'N'}` (Umschalt) → öffnen.
- [ ] Antwort deuten (Node-Test, rein): 201 → angelegt; 409 mit `channel` → „# allgemein gibt es schon.“; 422 mit `vorschlag` → Regel und Vorschlag; 422 ohne `vorschlag` (`reserviert`) → Regel ohne Schalter; 429 `{hoechstens:32}` → Text beginnt mit „Zu viele Channels“ und enthält „32“; `ServerWeg` → „Nicht angelegt — Verbindung prüfen.“
- [ ] Anlegen per Kürzel (E2E): A und B beigetreten, der Fokus in A liegt auf dem Schalter „Teilnehmer“; A drückt `Strg Alt N` → Dialog „Channel anlegen“ offen, Fokus im Feld „Name“; A tippt `security-entwicklung` und `Enter` → Dialog geschlossen, Statuszeile „Channel # security-entwicklung angelegt.“, `# security-entwicklung` steht genau einmal in A's Liste, der Fokus liegt wieder auf „Teilnehmer“; B zeigt den Eintrag ohne Neuladen.
- [ ] Abbrechen (E2E): Klick auf `+` → Dialog offen; `Esc` → Dialog zu, Fokus auf `+`, `GET /api/channels` unverändert; dasselbe mit „Abbrechen“.
- [ ] Ungültig und vorhanden (E2E): `allgemein` existiert; A gibt `Allgemein` ein und drückt `Enter` → Fehlerzeile mit ⚠ in `kd-danger`, Feld `aria-invalid="true"`, Schalter „allgemein übernehmen“; Klick → Feld enthält `allgemein`; `Enter` → Fehlerzeile „# allgemein gibt es schon.“, Dialog offen, Feld enthält weiter `allgemein`.
- [ ] Grenze (E2E): ein Agent von `127.0.0.1` hat 32 Channels angelegt; A (ebenfalls `127.0.0.1`) legt `noch-einer` an → Fehlerzeile beginnt mit „Zu viele Channels“ und nennt 32; Dialog offen.
- [ ] Netzfehler (E2E, Ausnahmeweg, Kriterium nach Nutzen): Dialog offen mit `spaeter`, das Programm wird beendet, A drückt `Enter` → Fehlerzeile „Nicht angelegt — Verbindung prüfen.“, Feld enthält weiter `spaeter`.
- [ ] Schmal (E2E): bei 320 px ist der Dialog ohne Querscrollen bedienbar (`scrollWidth <= innerWidth`, Schalter „Channel anlegen“ sichtbar), auch mit Fehlerzeile und Vorschlagsschalter.
- [ ] Konsole bei A: keine unbehandelte Ausnahme und kein CSP-Verstoß. Konsolenfehler: in „Anlegen per Kürzel“, „Abbrechen“ und „Schmal“ keine; in „Ungültig und vorhanden“ höchstens „Failed to load resource“ zu `POST /api/channels` mit 422 und 409; in „Grenze“ höchstens „Failed to load resource“ zu `POST /api/channels` mit 429. Für „Netzfehler“ gilt das Kriterium nach Nutzen, keine weitere Vorgabe zur Konsole.

## Betroffene Verzeichnisstruktur

- Namensregel: `src/name.rs` (`Channelname`, reservierte Namen, Akzente im Vorschlag); Browser `web/namensregel.js`.
- Gespräche: `src/gespraeche/mod.rs` (Verzeichnis unter `RwLock`, `channel_anlegen`, `channels`, Rückruf), neu z. B. `src/gespraeche/verzeichnis.rs`, `src/gespraeche/einstellungen.rs` (A00013), `src/gespraeche/logablage.rs` und `speicherablage.rs` (Ordner anlegen und entfernen, Einstellungen schreiben, ungültige Ordner).
- HTTP: neu `src/http/channels.rs` (`POST`, `GET`), `src/http/mod.rs` (Routen unter `grenzschutz`), `src/http/ereignisse.rs` (zweites Thema), `src/http/gesundheit.rs` (`channels`); Bus `src/live/bus.rs` (`Ereignis::Channel`); Start `src/start/mod.rs` (Rückruf an den Bus).
- Oberfläche: neu `web/seitenleiste.js`, `web/channel-anlegen.js` (Dialog, Kürzel, Antwort deuten), `web/channels.js` (Zusammenführen); `web/hauptansicht.js`, `web/verteiler.js`, `web/lc-seite.css`.
- Tests: Unit im jeweiligen Modul; Integration neu `tests/channels.rs`, `tests/channels_strom.rs`, angepasst `tests/gemeinsam/strom.rs`, `tests/ereignisse.rs`; Oberfläche rein `e2e/einheit/*.test.mjs`; Abläufe `e2e/tests/r00006-*.spec.js`.
- Abhängigkeit: `toml = "1"` (ADR-0022, neun Crates).

## Technische Überlegungen

### Ablauf

1. **Start**: … → `Gespraeche::laden` liest `channels/*` (ungültige übergangen, Einstellungen gelesen, fehlend oder defekt → `null`) → Verzeichnis mit `stand` 0 → Rückruf an den Bus gehängt → Binden.
2. **`POST /api/channels`**: Grenzschutz → Teilnehmer → Körper → `spawn_blocking(channel_anlegen)`:
   - 2.1 unter der Sperre des Verzeichnisses: Namensregel → vorhanden (409) → gesamt → je Herkunft (429) → `create_dir` (existiert → 409 `ordner-vorhanden`) → Einstellungen schreiben (Fehler → Ordner entfernen, 500) → eintragen, `stand` + 1 → Rückruf veröffentlicht `ChannelWechsel`;
   - 2.2 Antwort 201.
3. **Strom öffnen**: wie bisher abonnieren und Teilnehmer-Stand senden, danach Channel-Stand unter der Sperre des Verzeichnisses; Wechsel je Thema über `zuletzt_*` gefiltert.
4. **Oberfläche**: Worker merkt sich beide Listen → Tab führt zusammen → Seitenleiste zeichnet; `+`/`Strg Alt N` → Dialog → `POST` → Antwort deuten → Statuszeile oder Fehlerzeile.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0020: B0116 Channelname prüfen (rein, mit Akzenten) · B0117 Einstellungen schreiben und gültige lesen (rein) · B0118 Channel-Ordner und Einstellungsdatei ablegen · B0119 Channel im Verzeichnis anlegen (unklar) · B0120 Channels mit Einstellungen laden · B0121 Anlegen beantworten · B0122 Channels auflisten beantworten
- F0021: B0123 Channel-Wechsel unter der Sperre veröffentlichen · B0124 Channels im Strom senden (unklar) · B0125 Testhilfe Stände je Thema
- F0022: B0126 Grenzen prüfen (rein) · B0127 Grenzen im Anlegen und 429 · B0128 Gesundheit nennt Channels
- F0023: B0129 Ungültige Channel-Ordner beim Laden übergehen · B0130 Defekte Einstellungen erkennen (rein) · B0131 Ordner vorhanden, aber nicht geladen · B0132 Rückbau bei Plattenfehler
- F0024: B0133 Channels zusammenführen (rein) · B0134 Worker gibt beide Stände · B0135 Seitenleiste zeichnen (unklar) · B0136 Seitenleiste in die Hauptansicht
- F0025: B0137 Tastenkürzel ohne AltGr (rein) · B0138 Namensregel im Browser mit reservierten Namen (rein) · B0139 Antwort im Dialog deuten (rein) · B0140 Dialog zeichnen und bedienen (unklar)

**Elemente** (nur öffentliche Signaturen, nach A00012, A00013, A00004, A00011, A00005):

- `Channelname` (Wert, unveränderlich) — `fn bilden(text: &str) -> Result<Channelname, Namensfehler>`; `Namensfehler { regel, vorschlag: Option<…> }` wie bei `Name`.
- `Gespraeche` (Aggregat) — zusätzlich `fn channel_anlegen(&self, name: &str, von: &Kennung) -> Result<Channel, Anlegefehler>`, `fn channels(&self) -> ChannelStand`, `fn mit_veroeffentlichen(&self, rueckruf: …)`.
- `Channel { name: Channelname, angelegt: Option<Zeit>, angelegt_von: Option<Kennung>, beschreibung: Option<String> }` (DTO); `ChannelStand { stand, liste }`; `ChannelWechsel { stand, channel }`.
- `Anlegefehler` = `Vorhanden(Channel)` | `OrdnerVorhanden` | `Regel(Namensfehler)` | `Beschreibung` | `Grenze { art, hoechstens }` | `Ablage(String)`.
- `einstellungen::lesen(bytes) -> Result<Einstellungen, Defekt>`, `einstellungen::schreiben(&Einstellungen, vorher: Option<&[u8]>) -> String` (rein).
- Logablage (Port): zusätzlich `channel_ordner_anlegen(name)`, `einstellungen_schreiben(gespraech, bytes)`, `ordner_entfernen(gespraech)`.
- Oberfläche: `channelsZusammenfuehren(liste, ereignis)`, `kuerzelOeffnet(ereignis)`, `anlegeAntwort({status, daten} | ServerWeg) -> {art, text, vorschlag?}`.

Wireframe: die WBS führt `wireframes: Dokumentation/Wireframes/`, ein Artboard `D0003.dc.html` fehlt (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle bis dahin: Designsystem `components/Seitenleiste/README.md`, `components/Hauptansicht/README.md`, `1-tastatur.md`, `design-prompt.md` 4.3 — Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `name::umformen`: Akzente auf den Grundbuchstaben (wirkt auch auf den Vorschlag beim Beitreten; R00003 „`José` → `jos`“ wird `jose`).
- `gespraeche::Gespraeche`: Menge der Gespräche zur Laufzeit veränderlich (`RwLock<Verzeichnis>`); `laden` liest Einstellungen und übergeht ungültige Ordner.
- `gespraeche::logablage` (`Dateiablage`, `Speicherablage`): neue Port-Operationen; heute wird jeder Unterordner von `channels/` ein Channel (`logablage.rs:80`).
- `http::ereignisse`: zweites Thema, `zuletzt_channels`; `http::gesundheit`: `channels`; `live::bus::Ereignis`: `Channel`.
- `web/verteiler.js` (`standAus`), `web/hauptansicht.js` (dritte Spalte, Tab-Reihenfolge), `web/namensregel.js` (Art Channel, reservierte Namen, Akzente).

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests. Tests sind nie schärfer als diese Anforderung: sie übernehmen die Grenzen von oben wörtlich.

**Unit (rein, gestellte Uhr, Speicherablage):** `Channelname` (Spanne 32/33, reserviert, Vorschlagstabelle); Einstellungsformat (Rundweg, Defekte, unbekannter Schlüssel); Verzeichnis (Anlegen, Vorhanden, `stand` unter der Sperre, Grenzen 32/33 und 256/257 samt Neuladen, ohne Anleger, Rückbau).

**Integration (`tests/`, echtes Programm):** Anlegen mit Ordner und Datei, Liste und Neustart, 422/409/401/403/413/415, 20 gleichzeitige, Beschreibung abgewiesen; Strom mit zweitem Ereignis, Zustellung ≤ 2 s, 30 Anlagen zu 10 Öffnungen; Grenzen über das Programm und den Neustart, Gesundheit; Ordner vorhanden, nicht beschreibbar, ungültige Ordner und defekte Datei beim Start. Mutationsproben: Veröffentlichung außerhalb der Sperre; Grenzen 31/33 und 255/257.

**Oberfläche rein (`e2e/einheit/`):** Zusammenführen, Worker mit beiden Ständen, Kürzel ohne AltGr, Antwort deuten, Namensregel mit Akzenten und reservierten Namen.

**E2E (Playwright):** die Szenarien aus F0024 und F0025. Dauern ereignisgenau (Prüfintervall ≤ 25 ms), nicht im Standardraster von `expect`; die 201 wird im Testprozess empfangen, die Ankunft dort beobachtet. Mindestens ein Test mit verzögert antwortendem `POST /api/channels` (1 s): der Dialog bleibt bis zur Antwort offen und schließt danach mit Statuszeile und Fokus beim Auslöser.

## Abhängigkeiten

- Abhängig von: I0004 Namen wählen und beitreten (R00003, grün) — Sitzung, Namensregel, Grenzschutz; I0005 Teilnehmer mit Online-Status sehen (R00004, grün) — Strom, Worker, Hauptansicht; I0002 Nachricht landet im Log (R00002, grün) — Laden und Logablage.
- Innerhalb (WBS `Braucht`): F0020 Welle 1; F0021, F0022, F0023 brauchen F0020 und sind untereinander parallel (Welle 2); F0024 braucht F0021 (Welle 3); F0025 braucht F0024 und F0022 (Welle 4).
- Blockiert: I0008 Channel wechseln und Verlauf lesen, I0018 Aufbewahrungsfrist je Channel festlegen (Einstellungsdatei und Zurückschreiben); prägt I0019 (Anleitung: „ein Channel je Zusammenarbeit, nicht je Auftrag“).

## Offene Fragen

Keine, die R00006 blockieren. An den Betreiber (über den Director):

- **Beschreibung erst als Ausbaustufe 2** (F0026): bis zur Aktivierung lehnt die API eine Beschreibung mit 422 ab. Empfehlung: so lassen und F0026 frühestens mit I0008 aktivieren — dort hat die Beschreibung ihren sichtbaren Ort (Kopf des Channels, ADR-0024 Regel 8).
- **ADR-0005 (nur das Programm schreibt) bleibt offen**: R00006 hängt nicht davon ab — `einstellungen.toml` wird nur beim Start gelesen, Hand-Änderungen gelten ab dem nächsten Start (ADR-0022 Regel 4); ein von Hand angelegter Ordner während des Laufs ergibt 409 `ordner-vorhanden`.

Entschieden vom planer für R00006:

- Ein Requirement für alle sechs Slices (siehe Notizen).
- Akzente im Vorschlag gelten für Teilnehmer- und Channelnamen gleich, Server und Browser mit derselben Tabelle — ein Vorschlag, der je Ort anders ausfällt, wäre ein Widerspruch in der Oberfläche.
- Abgrenzung der Akzente: Buchstaben aus U+00C0–U+017F; Zeichen außerhalb (z. B. `日本`) werden wie bisher `-`.

## Manuelle Vorbereitungstätigkeiten

- Für die Plattenfehler-Probe: ein Benutzer ohne root (unter root wirkt `chmod a-w` nicht).

## Manuelle Nachbereitungstätigkeiten

- ADR-0013: das Beispiel „`José` → `jos`“ (Ergänzung 2026-10-28) auf `jose` nachziehen und die Akzentregel als Ergänzung eintragen — architekt über den Director.
- ADR-0021 Regel 1 und ADR-0024 Regeln 4 und 8: Hinweis, dass die Beschreibung mit F0026 kommt — architekt über den Director.
- A00012: die zwei offenen Fragen sind entschieden (alle sehen alle; kein automatischer Channel) — architekt über den Director.
- Designsystem `3-offene-fragen.md` Frage 10: Antwort „alle sehen alle; ein Ausblenden je Browser wäre eine eigene Interaction“ — über den Director.

## Umfang

Aus der WBS gezählt (`/planung zaehlen I0007`); Standard-Bubbles mit der Untergrenze 0,4h (belegt aus `Schaetzungen/_ist-zeiten.md`, R00001–R00005, Typ Standard), unklare als Richtwert 2–4h:

- F0020 Channel über die API anlegen und listen = 7 Bubbles: 6 Standard (0,4h), 1 unklar (2–4h). Rest: 2,4h klar + 2–4h unklar · 6 von 7 Werten belegt, Rest Richtwert (ungemessen).
- F0021 Channels live im Ereignisstrom = 3 Bubbles: 2 Standard (0,4h), 1 unklar (2–4h). Rest: 0,8h klar + 2–4h unklar · 2 von 3 Werten belegt, Rest Richtwert (ungemessen).
- F0022 Grenzen beim Anlegen = 3 Bubbles: 3 Standard (0,4h). Rest: 1,2h klar · 3 von 3 Werten belegt.
- F0023 Ordner von Hand und Plattenfehler = 4 Bubbles: 4 Standard (0,4h). Rest: 1,6h klar · 4 von 4 Werten belegt.
- F0024 Seitenleiste im Browser = 4 Bubbles: 3 Standard (0,4h), 1 unklar (2–4h). Rest: 1,2h klar + 2–4h unklar · 3 von 4 Werten belegt, Rest Richtwert (ungemessen).
- F0025 Channel im Browser anlegen = 4 Bubbles: 3 Standard (0,4h), 1 unklar (2–4h). Rest: 1,2h klar + 2–4h unklar · 3 von 4 Werten belegt, Rest Richtwert (ungemessen).
- I0007 gesamt = 25 Bubbles. Rest: 8,4h klar + 8–16h unklar · 21 von 25 Werten belegt, Rest Richtwerte (ungemessen).
- Nicht enthalten: F0026 Beschreibung — Ausbaustufe 2, nicht beauftragt.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Die Security-Factory will mit der Entwicklungs-Factory einen gemeinsamen Ort `security-entwicklung` — heute braucht sie dafür einen Menschen, der von Hand einen Ordner anlegt, und auch dann sieht ihn niemand. Wenn ein Agent mit einem `POST /api/channels` den Channel anlegt und bei einem vorhandenen Namen den vorhandenen Channel zurückbekommt, dann kann jede Factory „anlegen, falls es fehlt“ ohne Absprache ausführen. Wenn das Anlegen unter einer Sperre geprüft, als Ordner mit Einstellungsdatei geschrieben und noch unter der Sperre veröffentlicht wird, dann gibt es den Channel genau einmal, er übersteht jeden Neustart, und jeder Strom erhält ihn in der richtigen Reihenfolge — der andere Agent und die Seitenleiste jedes Menschen zeigen ihn sofort. Wenn die Grenzen aus den Einstellungsdateien gezählt werden, dann kann ein Agent in einer Schleife höchstens 32 Channels anlegen und alle zusammen höchstens 256, auch über Neustarts — der Ordner bleibt überschaubar und die Seitenleiste lesbar.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe wie Channel, Channelname, Herkunft, Anleger, Stand, Wechsel sind aus ADR-0012, ADR-0020 bis ADR-0023 und A00012 übernommen.
- Kein Artboard für D0003 und für Bildschirm 4.3.

## Notizen

### Verworfene Alternativen

- **Je Slice eine eigene Anforderung**: verworfen — alle sechs haben ein Ziel (ein Channel entsteht und steht bei allen in der Liste) und durchlaufen als Arbeitspaket I0007 gemeinsam Entwicklertest und Abnahme.
- **Grenzen und Ausnahmewege in einem Slice** (Zuschnitt des architekt, S3): geteilt in F0022 und F0023 — beide für sich nützlich, parallel baubar, zusammen über acht Bubbles; F0023 nimmt zusätzlich das Laden ungültiger Ordner und defekter Einstellungen auf, das im Zuschnitt fehlte.
- **Beschreibung im ersten Durchgang** (API in F0020, Feld in F0025): verworfen — ihr sichtbarer Ort entsteht erst mit I0008; als Ausbaustufe 2 bleibt R00006 schlank, und die Aktivierung ändert nur das Verhalten bei gesetzter Beschreibung (422 → 201).
- **Beschreibung still verwerfen statt 422**: verworfen — ein Agent hielte sie für gespeichert; widerspricht „abweisen statt umformen“ (ADR-0013).
- **Akzente nur für Channelnamen**: verworfen — Beitreten und Anlegen zeigten für `José` verschiedene Vorschläge.

### Bewusst außerhalb

Channel wechseln, aktiver Eintrag, Kopf und Leerzustand der Mitte (I0008); Beschreibung (F0026); Frist je Channel (I0018); Ungelesen- und Erwähnungszähler; Channel löschen oder umbenennen (nicht in Vision und WBS — ADR-0021 Regel 6 verlangt dann eine neue Entscheidung zu den Grenzen); Ausblenden von Channels je Browser.
