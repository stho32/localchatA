---
id: R00050
status: Neu
datum: 2026-10-30
---

# R00050: Direktnachricht senden — nur die zwei Beteiligten sehen das Gespräch

## Beschreibung

Ein Teilnehmer — Mensch im Browser oder Agent über die API, beide gleich — schickt einem anderen Teilnehmer aus der Teilnehmerliste eine Nachricht, die nur die zwei sehen. Das Gespräch ist ein Paar aus **zwei vollen Kennungen** (`name@herkunft`), entsteht beim ersten Senden, liegt wie ein Channel als Ordner mit Tageslogs unter `direkt/<paar>/` und entfällt nach 7 Tagen. Gesendet, gelesen, bearbeitet, gelöscht, beantwortet und mit Anhang versehen wird es über dieselben Wege wie im Channel; **auf keinem Weg erfährt ein Dritter etwas** — nicht Inhalt, nicht Zeitpunkt, nicht Größe, nicht Anhang-ID, nicht ob es das Gespräch gibt: nicht im Verlauf, im Ereignisstrom, bei `warten` (auch nicht über `seit`), bei den Anhängen, in der Liste der Gespräche und im Health-Check.

Wie privat „privat“ ist, steht fest (PO Zeile 168): **nur die zwei in Oberfläche und API**; der Ordner ist lesbar (keine Anmeldung, Vision), und wer nach dem `weg` einer Kennung mit demselben Namen von derselben Herkunft beitritt, liest deren Gespräche mit. Der Hinweis in der Oberfläche verspricht deshalb nicht mehr als „Direktnachricht — nur ihr beide im Chat; die Dateien liegen im Ordner der Instanz“; die Anleitung (I0019) nennt die drei Lesewege.

Zahlt ein auf: [Vision](R00000-vision.md) — „Direktnachrichten und Erwähnungen“ (Nachrichten an einzelne Teilnehmer), „Mensch und KI sind gleichberechtigt“, „Live überall“ und die Leitplanken „keine Anmeldung“ und „Lastprofil Team-LAN“.

Herkunft: Karte #13 (WBS I0012, Dialog D0004); Entscheidung des `architekt` in [ADR-0055](../Dokumentation/ADRs/0055-direktgespraeche-paar-aus-zwei-kennungen-gegenueber-im-pfad-anlegen-beim-ersten-senden-eine-sichtbarkeitsregel-sichtbare-marke.md) (Qualitätsziele QD1–QD6, Regeln 1–11, Bausteine D1–D15); die Richtungsfragen P1–P5 sind vom `product-owner` entschieden (`AI-Team/po-entscheidungen.md` Zeilen 166–170); Abnahme-Maßstab ist die [Angriffsmatrix](../Dokumentation/ADRs/0055-angriffsmatrix.md) des `security-reviewer` (42 Zeilen, Kopfzeile der Datei sagt 44; **K1 zuerst grün**).

Umfang (WBS `Dokumentation/Planung/localchata.md`, Dialog D0004 Direktnachrichten und Erwähnungen): **I0012** mit sieben Slices — **F0172** Direktnachricht senden und lesen, **F0173** Strom, Aktivität und Warten zeigen ein Direktgespräch nur den Beteiligten, **F0174** Anhänge eines Direktgesprächs bleiben Dritten verborgen, **F0175** Direktgespräche entfallen nach 7 Tagen und belegen begrenzt Platz (Ausbaustufe Frist 1), **F0176** Kein Weg gibt ein Direktgespräch ohne Sichtbarkeitsprüfung aus, **F0177** Health-Check verrät nichts über Direktgespräche, **F0178** Direktnachricht im Browser führen; 25 Bubbles B0764–B0788. Nicht hier: **F0179** Frist im Direktgespräch wählen (Ausbaustufe 2, nicht beauftragt, braucht eine eigene ADR), F0168 „Direktnachricht weckt“ (R00049, trägt die A1-Probe), Gruppen-Direktgespräche (P2), Archivieren (ADR-0055 E14), Zähler „ungelesen“ und „@n“ der Seitenleiste.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00004, R00011, R00029, R00047, R00049: Kennung (`name@herkunft`), Name, Herkunft (Quelladresse, kanonische `IpAddr` ohne Zone), Gespräch (Channel oder Direktgespräch), Verlauf, Zeit einer Nachricht, Sitzung, Teilnehmerliste, Gnadenfrist, Plattenquote. Neu und hier festgelegt: **Direktgespräch** (Paar aus zwei Kennungen), **Gegenüber** (die andere Kennung des Paars, im Pfad und in der Ausgabe als `direkt`), **Beteiligter** (eine der zwei Kennungen, ganze Kennung, nie Teilstück), **Dritter** (jeder andere, mit oder ohne Sitzung), **Sichtbarkeit** (eine Funktion `sichtbar_fuer`), **sichtbare Marke** (`seit` hängt nur von Zeiten sichtbarer Nachrichten ab), **Anleger** (Absender der ältesten Nachricht des Paars in der Frist).

## Geschäftlicher Nutzen

Wirkungskette: `anna` will mit `ben` etwas klären, das nicht in `# allgemein` gehört (ein Token, eine Rückfrage, ein Urteil über einen Agenten) → ohne Direktnachricht bleibt nur der Channel, den alle lesen, oder ein anderer Weg außerhalb des Chats → mit R00050 schreibt sie `ben` aus der Teilnehmerliste an, nur beide sehen es, und `ben` bekommt es binnen 1 s im Browser oder über `warten` → das Gespräch ist ein Ordner wie jeder Channel und entfällt nach 7 Tagen, ohne Datenbank und ohne Anmeldung. Dass **kein dritter Weg** etwas verrät, ist der eigentliche Wert: der Listener auf `0.0.0.0` ohne Anmeldung macht jede Route zur LAN-Angriffsfläche, und jede Zeile, die fremde Zeitpunkte, Größen oder IDs nennt, würde das Versprechen „nur ihr beide“ aushöhlen — darum die Matrix mit 42 Zeilen als Abnahme.

## Funktionale Anforderungen

Festgelegt in ADR-0055 (Nummern = Regeln der ADR). Die Wortlaute der deutschen Fehlertexte legt der Bau fest; Codes und `regel`-Werte sind hier fest.

### F0172 Direktnachricht senden und lesen — B0764–B0771

- **Adressierung** (Regel 3): `GET|POST /api/direkt/{kennung}/nachrichten` (Verlauf nach ADR-0025, Senden nach ADR-0034 mit `anhaenge`, `antwort_auf`, Wiederholschlüssel), `POST …/nachrichten/bearbeiten` und `…/loeschen` (ADR-0048), `PUT|DELETE|GET …/aktivitaet` (ADR-0051); `{kennung}` ist das **Gegenüber** als Pfadsegment (prozentdekodiert, `:` roh oder `%3A`). Das Paar bildet der Server aus der **Sitzung** und dem Pfad — ein fremdes Gespräch kann kein Aufrufer nennen. Die Handler sind dieselben wie im Channel; unbekannte Körperfelder werden wie dort mit 422 `feld` abgewiesen, `von`, `direkt`, `channel`, `an` im Körper sind wirkungslos, der Absender ist immer die Sitzung.
- **Kanonische Kennung** (Regel 2): die Kennung muss `name@herkunft` sein und die Herkunft als Adresse gelesen wieder **byte-gleich** ergeben; abgewiesen (422 `regel: "kennung"`, nicht umgeformt) werden u. a. `anna@::ffff:127.0.0.1`, `anna@fd00:0::7`, `anna@FD00::7`, `anna@[fd00::7]`, `anna@127.0.0.01`, `Anna@…`, Zonenangaben, Steuerzeichen, 200 Zeichen. Rechenbeispiel: `ben@10.0.0.8` und `anna@10.0.0.7` ergeben den Ordner `anna@10.0.0.7+ben@10.0.0.8` (aufsteigend nach Bytes); `claude@fd00::7` wird `claude@fd00%3A%3A7`; `anna@::1` mit `ben@127.0.0.1` wird `anna@%3A%3A1+ben@127.0.0.1`.
- **Prüfreihenfolge** (Regel 3): Grenzschutz 403 → Sitzung 401 (vor dem Pfad, B4: ohne Sitzung ist die Antwort für jede Pfadform dieselbe 401) → Kennung nicht kanonisch 422 `kennung` → eigene Kennung 422 `selbst` (P4) → Schreibkonto 429 → Körper wie im Channel → nur beim Senden ohne bestehendes Gespräch: Gegenüber nicht in der Teilnehmerliste 404 `regel: "empfaenger"` (P5) → Plattenquote 507 → Anlegen unter der Verzeichnissperre: Gesamtgrenze 429, Grenze je Herkunft 429 (F0175) → Ordner und Schreiben.
- **Anlegen beim ersten Senden** (Regel 4): ein `GET` legt nie an; gleichzeitiges erstes Senden beider Seiten ergibt **einen** Ordner mit beiden Nachrichten; ein bestehendes Gespräch geht immer weiter, auch wenn das Gegenüber weg ist (P5). Lesen ohne Gespräch ist 200 mit leerem Block (`nachrichten: []`, `aelter` und `neuer` `false`).
- **Liste der eigenen Gespräche** (Regel 3): `GET /api/direkt` → 200 `{direktgespraeche: [{kennung, letzte, frist_tage}]}` — nur die eigenen mit mindestens einer Nachricht in der Frist, `kennung` = Gegenüber, `letzte` = Zeit der jüngsten Nachricht, nach `letzte` absteigend, `frist_tage` fest 7, `Cache-Control: no-store`.
- **Laden** (Regel 1): beim Start wird jeder Ordner unter `direkt/` geprüft (genau ein `+`, beide Teile kanonisch, aufsteigend, verschieden, nur `:` als `%3A`); ein ungültiger Ordner wird übergangen mit **einer** `WARNING laden`-Zeile (Steuerzeichen ersetzt, `ordner` höchstens 128 Zeichen); ein Symlink als `direkt` ist Exit 2, als Paarordner oder als `anhaenge` im Paar wird er nicht gefolgt.
- **Eine Sichtbarkeitsfunktion** (Regel 5, B5): `sichtbar_fuer(gespraech, kennung, archiviert)`; `aktivitaet::sichtbar` (`src/aktivitaet/mod.rs:285`) entfällt ersatzlos.

### F0173 Strom, Aktivität und Warten — B0772–B0774

- **Strom** (Regel 6): `nachricht`, `aenderung`, `anhaenge` und die Aktivitätssignale eines Direktgesprächs gehen nur an die zwei Beteiligten (auch an den zweiten Tab des Absenders); Ausgabe `{"art": …, "channel": null, "direkt": <Gegenüber>, …}`, in Channels `direkt: null`; Aktivität nur an das Gegenüber, nie an den Tippenden selbst. Ein neues Gespräch braucht kein eigenes Thema: die erste `nachricht` mit unbekanntem `direkt` legt den Eintrag an. Nach einem erzwungenen `Lagged` enthält das Nachladen keine Direktdaten fremder Gespräche.
- **Sichtbare Marke** (Regel 8, A1): `seit` in jeder Antwort von `warten` hängt nur von Zeiten sichtbarer Nachrichten ab; ein unsichtbares Bus-Ereignis rückt den verarbeiteten Stand nicht vor; fehlt `seit` im Eingang, antwortet der Aufruf mit der letzten **sichtbaren** Zeit. Verlustfreiheit bleibt (ein Wartender, der nicht beteiligt ist, verliert nichts; ein Beteiligter bekommt die Direktnachricht beim nächsten Aufruf — F0168).

### F0174 Anhänge — B0775–B0776

- Eine Anhang-ID eines Direktgesprächs steht in keiner Antwort an einen Dritten (Verlauf, Strom, Warten, Liste); mit dem Link öffnet er die Datei (gewollt, ADR-0047 B4). `POST /api/anhaenge/loeschen` wertet die Sichtbarkeit dort, wo die ID aufgelöst wird (`finden_fuer(id, aufrufer)`): für einen Dritten sind Vorschau, `schon_geloescht` und der Wettlauf-Zweig `unbekannt`, byte-gleich zu einer nie vergebenen ID; eine gemischte Anfrage zählt nur die eigenen. Das Binden einer fremden ID beim Senden bleibt 422 `fremd` wie im Channel.

### F0175 Entfallen und Grenzen — B0777–B0780

- **Frist** (Regel 10): Direktgespräche entfallen nach **7 Kalendertagen** nach ADR-0029 (heute zählt mit) beim Start und beim Tageswechsel samt Anhängen (ADR-0030), gepflegt wie Channels, Erlaubnisliste und Symlink-Regeln der ADR-0032 unverändert. Ein **leerer** Paarordner (keine Tagesdatei, kein Anhang-Tagordner mit Inhalt, keine Fremddatei) wird von innen nach außen **nur mit `rmdir`** entfernt, keinem Link folgend; scheitert ein `rmdir`, geschieht nichts. Fest 7 Tage (P1), bis F0179 sie wählbar macht.
- **Grenze neuer Paare** (Regel 4, B1): `DIREKT_GESAMT = 8.192`, `DIREKT_JE_HERKUNFT = 4.096` (Herleitung gerechnet: 128 Sitzungen je Herkunft × 32 Gegenüber; 2 Herkünfte); nur wenn ein Ordner **neu entstehen** würde: Gesamtgrenze zuerst, dann Herkunft des **Anlegers** (Absender der ältesten Nachricht des Paars in der Frist); erreicht → 429 `{fehler, grenze: "direktgespraeche", art: "gesamt" oder "herkunft", hoechstens}`, nichts angelegt; bestehende Paare senden weiter; liegen beim Start mehr Ordner vor, werden alle geladen, nur neue abgewiesen; ein leerer Ordner zählt zur Gesamtgrenze, nicht zur Herkunft, und das Senden in ihn ist keine Anlage. Benannte Schwäche: die Zuordnung zur Herkunft kann mit dem Entfallen der ältesten Nachrichten wandern — die Gesamtgrenze ist das harte Maß.
- **Protokoll** (Regel 11, S14): Zeilen mit Bezug auf ein Direktgespräch nennen nur `ordner=direkt/<ordnername>`, nie Text, Anhangnamen, volle Anhang-IDs oder Token; Anlegen, Entfallen von Tagen und Entfernen eines leeren Ordners stehen auf **DEBUG** (auf Stufe `information` schreibt ein Direktgespräch im Normalbetrieb keine Zeile und ändert `protokoll_bytes_heute` nicht); `WARNING laden` je übergangenem Ordner, `ERROR` bei Symlink oder Nicht-Ordner, neu `WARNING direkt` als Episode je Herkunft beim Erreichen einer Grenze.

### F0176 Gate — B0781

- Ein Architekturtest (`tests/architektur_sichtbarkeit.rs`, im normalen `cargo test`) prüft: (1) `Gespraech::Direkt` kommt nur in Dateien einer ausdrücklichen Liste vor, je Zeile mit Begründung; (2) jede Datei, die über alle Gespräche läuft, ruft `sichtbar_fuer(` auf oder steht mit Begründung in der Liste; (3) kein zweiter Name der Entscheidung (`fn sichtbar`, `beteiligt(` außerhalb von Sichtbarkeit und dem Aufbau des Paars); (4) kein `_ =>` in einem `match` über `Gespraech` in Strom, Warten, Anhänge löschen.

### F0177 Health — B0782–B0783

- `GET /api/gesundheit` (ohne Sitzung lesbar): `nachrichten`, `log_bytes`, `defekte_zeilen`, `nachholen.saetze` und `nachholen.kosten` zählen **nur Channels**; neues Feld `direkt_mib` = ⌊Bytes aller Tageslogs der Direktgespräche / 1 MiB⌋; kein Feld „Zahl der Paare“; die Plattenquote zählt weiter alles. Jedes Feld der Antwort hat im Inventar der ADR-0055 Regel 9 ein Urteil; die benannten Restorakel (a)–(i) sind hingenommen, keines nennt Gespräch, Beteiligte, Text oder Anhang.

### F0178 Direktnachricht im Browser führen — B0784–B0788

- **Beginnen** nur aus der Teilnehmerliste (P5): Klick oder Enter auf einen Teilnehmer öffnet ein Menü mit der Zeile „Direktnachricht“ (die Zeile „Erwähnen“ ergänzt I0013); am **eigenen** Teilnehmer gibt es keinen Eintrag „Direktnachricht“ (P4); eine Kennung frei einzutippen ist nicht vorgesehen.
- **Ansicht**: Adresse `#/d/<kennung>` (Gegenüber, prozentkodiert) neben `#/c/<name>`; keine eigene Seite, sondern die Hauptansicht im Privat-Modus (`lc-privat-bg`); Kopfzeile statt `# name`: die **volle Kennung** des Gegenübers, „7 Tage“ und der Hinweis „Direktnachricht — nur ihr beide im Chat; die Dateien liegen im Ordner der Instanz“ (Wortlaut `ux-reviewer`, PO 168; Feinschliff vor dem Bau). Verlauf, Eingabe, Bearbeiten, Löschen, Antworten, Anhang wie im Channel.
- **Seitenleiste**: Abschnitt „Direktnachrichten“ unter den Channels, ein Eintrag je Gegenüber mit der vollen Kennung und dem Status-Punkt des Gegenübers (offline und „gleich weg“ zusätzlich als Wort), Liste beim Laden aus `GET /api/direkt`, neue Einträge aus der ersten `nachricht` mit unbekanntem `direkt`.
- **Live**: Nachrichten, Änderungen, Anhänge und Tippen bzw. „denkt nach“ des Gegenübers erscheinen ohne Neuladen; Tippen des eigenen Tabs nie bei sich selbst.

## Nicht-funktionale Anforderungen

- **Sicherheit (QD1, QD3)**: nichts aus einem Direktgespräch erreicht einen Dritten auf irgendeinem Weg; eine Funktion (`sichtbar_fuer`) entscheidet für alle verteilenden Wege, und das Abschalten dieser Funktion macht mindestens zwei Proben rot (Mutationsprobe S17 der Matrix, elf Mutanten, je einzeln, die Suite muss rot werden — fährt der `security-reviewer` im Gate). Die Abweisungen sind **byte-gleich** (Status, Kopfzeilen außer `Date`, Körper identisch zur Antwort auf eine nie vergebene Kennung, Zeit oder ID).
- **Sicherheit (QD4, QD6)**: kein zweiter Ordner für dasselbe Paar (Aliase einer Kennung werden abgewiesen); die Zahl der Paarordner übersteigt nie `max(8.192, beim Start vorgefundene)`.
- **Performance (QD2)**: Zustellung der Direktnachricht an das Gegenüber binnen 1 s nach der 201 (Startpunkt: die 201 des Sendens; Zweck: Lastprofil Team-LAN, Zustellung live < 1 s; Grenze und Strecke wie im Channel, R00011 — der Test übernimmt sie, nie schärfer); der Verlauf eines Direktgesprächs mit 100.000 Nachrichten öffnet in unter 500 ms (gleiche Abfrage wie im Channel, Leistungsziel der Vision); dieselben Grenzen, Fehlertexte und Codes wie im Channel.
- **Ohne localchatA lesbar (QD5)**: `ls direkt/` zeigt die Paare, `less direkt/<paar>/<tag>.log` liest wie ein Channel.
- **Benutzerfreundlichkeit (F0178)**: bedienbar ohne Maus (Menü: Enter, Escape; sichtbarer Fokus), Touch-Ziele mindestens 44 px im neuen Menü, kein Querscrollen bei 320 px. „Keine Konsolenfehler“ heißt: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer den Browser-Meldungen „Failed to load resource“ zu Antworten, die dieses Dokument selbst vorschreibt (401, 404, 422, 429).
- **Nicht abgedeckt (Matrix)**: L3 (kein vorgelagerter Proxy im Projekt); Last- und Flutproben gegen das Programm (die Grenzen werden mit kleinen Testkonstanten geprüft); die Timing-Restorakel (b), (c), (e) der ADR nur begründet; IPv6 mit echter `fd00`-Adresse (L2 über `::1`).

## Akzeptanzkriterien

Rahmen (wörtlich aus der Angriffsmatrix; alle Zeilen laufen zusammen mit dem Kontrollpaar K1 — eine Matrix, die K1 rot lässt, misst nichts):

> Schutzziel: Inhalt, Zeitpunkt, Größe, Anhang-IDs und Beteiligte eines Direktgesprächs gegenüber Dritten in Oberfläche und API (ADR-0055 QD1, PO 168); Verfügbarkeit gegen Anlege-Flut (S10).
> Akteure: K anna@127.0.0.2 ↔ ben@127.0.0.3 (Kontrolle) · A0 Dritter im LAN ohne Sitzung · A1 cora@127.0.0.1 mit Sitzung · A2 Namensübernahme nach `weg` (gleiche / andere Herkunft) · A3 IPv6-Alias (Kennungsformen, IPv6-Beteiligter `anna@::1` bei `--host ::`) · A4 fremder Origin / fremder Host (Browser, DNS-Rebinding)
> Linien: L1 Integration im Prozess (testserver) · L2 echtes Programm über die eigene Annahmeschleife, Wegwerf-Ordner, `--port 0` · L3 nicht abgedeckt (kein vorgelagerter Proxy im Projekt)
> Kontrollpaar: K1 — anna sendet ben, ben liest: heute 404 (Route fehlt, geprobt), danach 201/200. Jede Probe läuft zusammen mit K1; eine Matrix, die K1 rot lässt, misst nichts.
> „heute“: geprobt = Einzelprobe 2026-10-30 gegen eigene Instanz aus main 641d9f0; sonst „rot (Route fehlt)“ bzw. begründet aus dem Code.
> Byte-gleich = Status, Kopfzeilen außer Date, Körper identisch zur Antwort auf eine nie vergebene Kennung/Zeit/ID.

Zu jeder Zeile gehört ihre **Abnahme-Probe samt Mutante**; die Mutante muss die Probe rot machen (Mutationsprobe S17).

### F0172 Direktnachricht senden und lesen

- [ ] **K1** — Senden/Lesen, Akteur K: anna `POST /api/direkt/ben@127.0.0.3/nachrichten {"text":"k1"}`; ben `GET /api/direkt/anna@127.0.0.2/nachrichten` → erwartet (L1 = L2): 201 `{channel:null, direkt:"ben@127.0.0.3", nachricht}`; 200 mit k1; Ordner `direkt/anna@127.0.0.2+ben@127.0.0.3/<tag>.log`. Heute: 404 (geprobt). Abnahme-Probe und Mutante, die rot werden muss: L2 Rundweg · Paar ohne Sortierung (zweiter Ordner bei ben→anna).
- [ ] **R1** — Lesen, Akteur A0: `GET /api/direkt/{x}/nachrichten` ohne Sitzung, x ∈ {`ben@127.0.0.3`, `..%2F..`, `%FF`, `a+b`} → erwartet (L1 = L2): 401 für alle x (Sitzung vor Kennung, S16), byte-gleich. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L1+L2 · 422 vor 401 (Path-Extraktor vor Teilnehmer).
- [ ] **R2** — Lesen, Akteur A1: cora mit x ∈ {`anna@127.0.0.2+ben@127.0.0.3`, `anna@127.0.0.2%2Bben@127.0.0.3`, `anna`, `anna@`, `..%2Fdirekt%2Fanna@127.0.0.2+ben@127.0.0.3`, `%252F`, `Anna@127.0.0.2`, `anna@127.0.0.2%00`, 200 Zeichen} → erwartet (L1 = L2): 422 `{regel:"kennung"}`, Körper je Variante gleich; kein Ordner, kein Index-Eintrag. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L2 Tabelle; `ls direkt/` vorher = nachher · Dekodieren ohne Kanonprüfung.
- [ ] **R3** — Lesen, Akteur A1: cora `GET /api/direkt/anna@127.0.0.2/nachrichten` und `…/ben@127.0.0.3/…` → erwartet (L1 = L2): 200 leer (`nachrichten:[]`, `aelter/neuer:false`) — nie Inhalt von anna↔ben; **kein** Ordner `anna…+cora…`. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L2 · `GET` legt an; Paar aus Pfad statt aus Sitzung+Pfad.
- [ ] **R4** — Lesen über Channel, Akteur A1: `GET /api/channels/{n}/nachrichten`, n ∈ {`anna@127.0.0.2+ben@127.0.0.3`, `anna%40127.0.0.2`, `..%2Fdirekt%2Fanna@127.0.0.2+ben@127.0.0.3`, `direkt`} → erwartet (L1 = L2): 422/404 wie heute für ungültige bzw. unbekannte Channelnamen; nie Inhalt. Heute: wie erwartet (begründet: Erlaubnisliste ADR-0020). Abnahme-Probe und Mutante, die rot werden muss: L2 Regression.
- [ ] **R5** — Lesen/Senden, Akteur A3: x ∈ {`anna@::ffff:127.0.0.2`, `anna@0:0:0:0:0:ffff:7f00:2`, `anna@127.0.0.02`, `anna@fd00:0::7`, `anna@FD00::7`, `anna@[fd00::7]`, `anna@fd00::7%25eth0`}; cora liest; anna sendet an Alias von ben → erwartet (L1 = L2): 422 `kennung`; kein zweiter Ordner, kein Lesen unter anderem Namen. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L1 (D2) + L2 · nur `Kennung::bilden`.
- [ ] **R6** — Lesen, Akteur A2: anna verlässt, Gnadenfrist um; mallory tritt als `anna` bei (a) von 127.0.0.2 (b) von 127.0.0.9; (c) altes Token von 127.0.0.9 → erwartet (L1 = L2): (a) liest anna↔ben — **hingenommen** (PO 168), Zeile dokumentiert es; (b) Kennung `anna@127.0.0.9`, liest leer; (c) 401. Heute: (c) 401 begründet (teilnehmer.rs gemeldete_sitzung). Abnahme-Probe und Mutante, die rot werden muss: L2 · `beteiligt` vergleicht nur den Namen.
- [ ] **R7** — Lesen, Akteur A4: `Host: evil.example`; `Origin: http://evil.example` mit Sitzungskopf → erwartet (L1 = L2): 403 vor allem (Host); bei fremdem Origin auf GET kein `Access-Control-Allow-*`; `Cache-Control: no-store` auf `GET /api/direkt`. Heute: Host 403 begründet (grenzschutz.rs schicht). Abnahme-Probe und Mutante, die rot werden muss: L2 Regression.
- [ ] **W1** — Senden, Akteur A0/A4: `POST` ohne Sitzung mit fremdem Origin; ohne Sitzung ohne Origin mit ungültiger Kennung → erwartet (L1 = L2): 403 vor 401; 401 vor 422. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L2 · Reihenfolge vertauscht.
- [ ] **W2** — Senden, Akteur A1: cora → anna (lebend) mit Körper `{"text":"x","von":"anna@127.0.0.2","direkt":"ben@127.0.0.3","channel":"allgemein","an":"ben@127.0.0.3"}` → erwartet (L1 = L2): wie im Channel (unbekannte Felder 422 `feld`, sonst ignoriert); Absender immer Sitzung; landet nur in `anna…+cora…`. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L2; Log anna↔ben byte-gleich · Absender aus Körper.
- [ ] **W3** — Senden, Akteur A1/A3: cora → `cora@127.0.0.1`; cora → `cora@::ffff:127.0.0.1` → erwartet (L1 = L2): 422 `selbst`; 422 `kennung` (nicht `selbst`), kein Ordner. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L1 · Alias als selbst erkannt und umgeformt.
- [ ] **W4** — Senden, Akteur A1: cora → `zed@127.0.0.9` nie gesehen; → `ben@127.0.0.3` nach weg ohne Gespräch; → anna nach weg **mit** Gespräch → erwartet (L1 = L2): 404 `{regel:"empfaenger"}` byte-gleich für „nie“ und „weg“, kein Ordner; mit Gespräch 201. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L2 · Empfängerprüfung auch bei bestehendem Gespräch.
- [ ] **W5** — Senden, Akteur K: anna→ben und ben→anna gleichzeitig als erstes (Schranke) → erwartet (L1 = L2): ein Ordner, zwei Nachrichten. Heute: —. Abnahme-Probe und Mutante, die rot werden muss: L1 (D4) · Anlegen außerhalb der Verzeichnissperre.
- [ ] **W6** — Senden, Akteur A1: cora in `…/anna…` mit `antwort_auf` = Zeit+Kennung einer Nachricht aus anna↔ben → erwartet (L1 = L2): 422 `unbekannt`, byte-gleich zu nie vergebener Zeit; kein Zitat; Logs unverändert. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L2 · Bezug global aufgelöst.
- [ ] **W8** — Senden, Akteur A1/K: anna Schlüssel k an ben; anna Schlüssel k an cora; cora Schlüssel k an anna → erwartet (L1 = L2): wie Channel (Wiederholung „anders“ nur für dieselbe Kennung); cora erfährt nichts aus anna↔ben. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L1.
- [ ] **E1** — Bearbeiten/Löschen, Akteur A1: cora `POST /api/direkt/anna@127.0.0.2/nachrichten/bearbeiten` bzw. `…/loeschen` mit Zeit aus anna↔ben; dasselbe über `/api/channels/allgemein/…` → erwartet (L1 = L2): 404 wie „nie da“, byte-gleich zu Zufallszeit; Log anna↔ben unverändert. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L2 · Suche der Zeit über alle Gespräche.
- [ ] **E2** — Bearbeiten/Löschen, Akteur K: ben bearbeitet/löscht annas Nachricht im gemeinsamen Gespräch → erwartet (L1 = L2): wie Channel für fremde Nachrichten (ADR-0048). Heute: —. Abnahme-Probe und Mutante, die rot werden muss: L1.
- [ ] **E3** — Bearbeiten/Löschen, Akteur A1: cora ohne Gespräch mit anna → erwartet (L1 = L2): 404 wie nie da, kein Ordner. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L1.
- [ ] **O1** — Laden, Akteur —: `direkt` als Symlink; `direkt/<paar>` als Symlink; `anhaenge` im Paar als Symlink → erwartet (L1 = L2): Exit 2; übergangen (kein Folgen); nicht gefolgt. Heute: `direkt`-Symlink wird heute gefolgt (read_dir), Paar-Symlink übergangen (file_type, begründet). Abnahme-Probe und Mutante, die rot werden muss: L2 (D4, B0583).
- [ ] **O2** — Laden, Akteur —: Ordner `x`, `b@1.1.1.1+a@1.1.1.1`, `a@1.1.1.1+a@1.1.1.1`, drei Teile, `%3a`, `%2F`, Alias-Herkunft, `anna@fd00%3A%3A7+…` richtig kodiert (Kontrolle) → erwartet (L1 = L2): ungültige: `WARNING laden`, nicht im Index, Health/`seit` unberührt, keine Route erreicht sie; Kontrolle geladen. Heute: **rot geprobt**: `x` und `b+a` still geladen, zählen in Health, `x` bestimmt `seit`. Abnahme-Probe und Mutante, die rot werden muss: L1 (D1) + L2 · Laden ohne Prüfung (logablage.rs:116).
- [ ] **P2** — Protokoll, Akteur —: Ordner `direkt/evil⏎2026-… INFORMATION fake`, Ordnername 300 Zeichen → erwartet (L1 = L2): je ein `WARNING laden` in **einer** Zeile, Steuerzeichen ersetzt, `ordner` ≤ 128 Zeichen. Heute: **rot geprobt**: still geladen, keine Zeile. Abnahme-Probe und Mutante, die rot werden muss: L2.
- [ ] **Liste (D11)**: `GET /api/direkt` als `anna` → `[{kennung: ben@…, letzte, frist_tage: 7}]` (bei einem zweiten Gespräch mit `dora` zuerst das mit der jüngeren `letzte`), als `cora` → `[]`; `Cache-Control: no-store`; Mutante: fremde Gespräche in der Liste.
- [ ] **Rundweg**: Senden, Verlauf, Bearbeiten, Löschen, Antwort mit Zitat und Anhang im Direktgespräch verhalten sich wie im Channel (Statuscodes, `regel`-Werte, Grenzen: Länge 422, Schreibkonto 429, Plattenquote 507); nach einem Neustart derselben Ablage ist das Gespräch unverändert da.

### F0173 Strom, Aktivität und Warten zeigen ein Direktgespräch nur den Beteiligten

- [ ] **K2** — Strom/Warten, Akteur K: ben Strom offen, ben `warten?bis=5&direkt`; anna sendet → erwartet (L1 = L2): Strom ben < 1 s `{art:nachricht, channel:null, direkt:anna@…}`; warten Treffer `anlass:["direkt"]`. Heute: kein Ereignis (bus.rs:150). Abnahme-Probe und Mutante, die rot werden muss: L2 · `direkt` = Absender statt Gegenüber.
- [ ] **K3** — IPv6-Beteiligter, Akteur K/A3: Instanz `--host ::`; anna über `::1` (Kennung `anna@::1`), ben über 127.0.0.1; senden, lesen, Tippen → erwartet (L1 = L2): Ordner `anna@%3A%3A1+ben@127.0.0.1`; anna sieht Verlauf, Strom, Tippen von ben. Heute: Regel alt schließt IPv6 aus (aktivitaet/mod.rs:285, begründet). Abnahme-Probe und Mutante, die rot werden muss: L2 · alter Vergleich auf kodiertem Ordnernamen.
- [ ] **S1** — Strom, Akteur A1: cora Strom offen; anna↔ben: nachricht, aenderung, löschen, anhaenge, aktivitaet; zusätzlich cora mit 2 Tabs, Wiederverbinden, erzwungenes `Lagged` (kleiner Puffer im Test) → erwartet (L1 = L2): cora: kein Ereignis mit `direkt≠null`; Nachlade-Ereignis nach `Lagged` ohne Direktdaten. Heute: —. Abnahme-Probe und Mutante, die rot werden muss: L2 (D8) · Filter weg; `direkt` aus Absender.
- [ ] **S2** — Strom, Akteur K: anna 2. Tab → erwartet (L1 = L2): erhält eigene Nachricht mit `direkt:ben@…`. Heute: —. Abnahme-Probe und Mutante, die rot werden muss: L2.
- [ ] **X1** — Aktivität, Akteur K/A1: anna `PUT /api/direkt/ben@127.0.0.3/aktivitaet {"zustand":"tippt"}`; Ströme ben, cora, anna (2. Tab) → erwartet (L1 = L2): ben: `{art:aktivitaet, channel:null, direkt:anna@…}`; cora und anna: nichts in 3 s. Heute: Strom verwirft Direkt (ereignisse.rs:299). Abnahme-Probe und Mutante, die rot werden muss: L2 · Filter `true`; `_ =>` im match.
- [ ] **X2** — Aktivität, Akteur A1: cora `GET /api/direkt/anna@127.0.0.2/aktivitaet` während anna→ben tippt; cora `PUT` an anna ohne Gespräch → erwartet (L1 = L2): 404 ohne Gespräch bzw. nur cora↔anna-Zustand; nie annas Tippen an ben. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L2 · Tabelle nach Kennung statt nach Gespräch gelesen.
- [ ] **G1** — Warten, Akteur A1: cora `warten?bis=0` → s0; anna→ben; cora `warten?bis=0&seit=s0` ohne Filter, mit `direkt`, mit `erwaehnung` (Text `@cora`) → erwartet (L1 = L2): kein Treffer; `seit` = s0 unverändert, solange Channels ruhen; nach Channel-Nachricht = deren Zeit. Heute: —. Abnahme-Probe und Mutante, die rot werden muss: L2 (D7) · `seit.max(H)` (nachholen.rs:129).
- [ ] **G2** — Warten, Akteur A1: cora `warten?bis=0` ohne `seit`, letzte Nachricht im Ordner ist eine Direktnachricht → erwartet (L1 = L2): `seit` = letzte Channel-Zeit. Heute: **rot geprobt**: `seit` = Zeit der Direktnachricht (16:53:31.000 statt 16:53:02.879). Abnahme-Probe und Mutante, die rot werden muss: L2 · Marke global.
- [ ] **G3 in F0168**: `ben` mit `seit=s0` erhält die Direktnachricht (Verlustfreiheit) — die Probe liegt in B0749 (R00049), weil sie den Filter `direkt` und das Nachholen von Direktgesprächen braucht.

### F0174 Anhänge eines Direktgesprächs bleiben Dritten verborgen

- [ ] **A1** — Anhänge abrufen, Akteur A0: `GET /anhaenge/<id aus anna↔ben>` ohne Sitzung → erwartet (L1 = L2): 200 (B4, gewollt); die ID taucht in keiner Antwort an cora auf (Verlauf, Strom, Warten, Liste). Heute: —. Abnahme-Probe und Mutante, die rot werden muss: L2 (D9) Regression.
- [ ] **A2** — Anhänge löschen, Akteur A1: cora `POST /api/anhaenge/loeschen {ids:[<id anna↔ben>]}` (a) unbestätigt (b) bestätigt (c) nachdem anna sie gelöscht hat (d) Mischung mit eigener ID → erwartet (L1 = L2): (a)(b)(c) `unbekannt`, byte-gleich zu Zufalls-ID, **keine** 409-Vorschau, kein `schon_geloescht`, Datei/Quote unverändert; (d) Vorschau zählt nur eigene/sichtbare. Heute: heute ohne Sichtbarkeitsprüfung (loeschen.rs:93, begründet). Abnahme-Probe und Mutante, die rot werden muss: L2 · Prüfung nur im Vorschau-Zweig; `schon_geloescht` ungefiltert.
- [ ] **A3** — Anhänge anmelden, Akteur A1: `POST /api/anhaenge` → erwartet (L1 = L2): gesprächsfrei wie heute. Heute: wie erwartet. Abnahme-Probe und Mutante, die rot werden muss: L1 Regression.
- [ ] **W7** — Senden, Akteur A1: cora sendet mit `anhaenge:[<ID aus anna↔ben>]` (ID per Link bekannt) → erwartet (L1 = L2): wie im Channel für fremd gebundene ID (422 `fremd`), kein Bezug. Heute: 404. Abnahme-Probe und Mutante, die rot werden muss: L2.

### F0175 Direktgespräche entfallen nach 7 Tagen und belegen begrenzt Platz

- [ ] **O3** — Pflege, Akteur —: Testuhr, Tage 10-23/10-24, Fremddatei `notiz.txt`, Symlink-Tagesdatei → erwartet (L1 = L2): 10-23 samt Anhängen weg, 10-24 bleibt, Fremdes unberührt, Symlink nicht gefolgt; (bei Annahme B1) leerer Paarordner nach letztem Tag entfernt. Heute: Direkt nie gepflegt (pflege.rs:98). Abnahme-Probe und Mutante, die rot werden muss: L1 (D5).
- [ ] **F1** — Anlege-Flut (S10), Akteur A1: je Herkunft und gesamt: N+1-tes **neues** Paar → erwartet (L1 = L2): 429 `{grenze:"direktgespraeche", art, hoechstens}` wie Channel-Grenze; bestehende Gespräche senden weiter; nach Entfallen frei. Heute: keine Grenze (ADR Regel 4). Abnahme-Probe und Mutante, die rot werden muss: L1 mit kleiner Testkonstante (keine Flutprobe gegen L2) · Grenze weg.
- [ ] **F2** — Anlege-Flut, Akteur A1: Ordner nach Frist leer → erwartet (L1 = L2): wird entfernt bzw. zählt und lädt nicht. Heute: Regel 10: bleibt für immer. Abnahme-Probe und Mutante, die rot werden muss: L1 (D5).
- [ ] **P1** — Protokoll, Akteur —: voller Austausch anna↔ben bei `--protokoll-stufe debug`, dann `grep` in `logs/` → erwartet (L1 = L2): kein Text, kein Anhang-Anzeigename, keine volle Anhang-ID (nur Präfix), kein Token; `ordner=direkt/<paar>` erlaubt. Heute: —. Abnahme-Probe und Mutante, die rot werden muss: L2 · Text im Kontext.
- [ ] **Grenzen (D13), Rechenbeispiel mit kleinen Testkonstanten** (3 je Herkunft, 5 gesamt): ``anna@10.0.0.1` beginnt 3 Paare → 201, das 4. **neue** → 429 `{grenze: "direktgespraeche", art: "herkunft", hoechstens: 3}`; ein Paar von `10.0.0.2` → 201; bestehende Paare senden weiter; Gesamtgrenze 5 → 429 `gesamt` (Gesamtgrenze vor Herkunftsgrenze); nach Neustart derselben Ablage gleiche Zählung (Anleger aus der ältesten Nachricht in der Frist); beim Start mehr Ordner als die Grenze → alle geladen, nur neue abgewiesen; leerer Ordner zählt zur Gesamtgrenze, nicht zur Herkunft, und das Senden in ihn ist keine neue Anlage; nach Entfallen und Entfernen (D5) ist ein Platz frei; Wertprüfung der Konstanten als Spanne (4.096/8.192)`
- [ ] **Konstanten**: `DIREKT_JE_HERKUNFT` und `DIREKT_GESAMT` liegen in der Spanne der Wertprüfung (4.096 / 8.192; 4.095 und 4.097 sind rot).
- [ ] **Protokoll (D15)**: `voller Austausch `anna` ↔ `ben` bei `--protokoll-stufe debug`, dann `grep` in `logs/`: kein Text, kein Anzeigename, keine volle Anhang-ID, kein Token, `ordner=direkt/<paar>` erlaubt; bei Stufe `information` ändert Anlegen, Senden und Entfallen `protokoll_bytes_heute` nicht; Grenze erreicht → eine `WARNING direkt`-Episode je Herkunft`

### F0176 Kein Weg gibt ein Direktgespräch ohne Sichtbarkeitsprüfung aus

- [ ] **U1** — Suche, Laufwerk, ZIP (künftig), Akteur A1: je neuem Weg: cora sucht Wort aus anna↔ben, listet Laufwerk, lädt ZIP → erwartet (L1 = L2): nichts aus anna↔ben, Trefferzahl/Größe wie ohne das Gespräch. Heute: —. Abnahme-Probe und Mutante, die rot werden muss: Abnahme-Kriterium jeder künftigen Anforderung; Mutante `sichtbar_fuer → true` rot.
- [ ] **Architekturtest (D14)**: `Regel 5, Satz „Genau eine Funktion“: Allowlist für `Gespraech::Direkt`, Aufrufpflicht `sichtbar_fuer(` in Dateien, die über alle Gespräche laufen, kein zweiter Name, kein `_ =>` in den verteilenden Wegen` Mutanten, die rot werden müssen: ein Probemodul `suche.rs` mit `Gespraech::Direkt` ohne Listeneintrag; `fn sichtbar` neu eingeführt; `_ =>` im Strom-Filter.
- [ ] **Mutationsprobe S17 (Matrix)**, je einzeln, Suite muss rot: `sichtbar_fuer → true` (S1, X1, G1, A2) · Paar unsortiert (K1, W5) · `beteiligt` als Teilstück/Name (R6b, K3) · Marke global (G1, G2) · Health ohne Trennung (H1) · `_ =>` im Strom-Filter (S1) · Löschen ohne Sichtbarkeit (A2) · Laden ohne Prüfung (O2) · Path vor Teilnehmer (R1) · `GET` legt an (R3) · Flutgrenze weg (F1) · `remove_dir_all` statt `rmdir` (O3).

### F0177 Health-Check verrät nichts über Direktgespräche

- [ ] **H1** — Health, Akteur A0: Health-Schnappschuss vor/nach Direktnachricht 1.000 B ohne Anhang (Stufe information) → erwartet (L1 = L2): JSON gleich bis auf die in Regel 9 benannten Felder (bei Text ohne Anhang: keines außer `direkt_mib` an MiB-Grenze); `nachrichten`, `log_bytes` nur Channels. Heute: **rot geprobt**: `nachrichten` 3, `log_bytes` 223 zählen Direktgespräche mit. Abnahme-Probe und Mutante, die rot werden muss: L2 (D10) · Felder zählen Direkt mit.
- [ ] **H2** — Health, Akteur A0: 1,1 MiB in Direktgesprächen → erwartet (L1 = L2): `direkt_mib:1`. Heute: —. Abnahme-Probe und Mutante, die rot werden muss: L1.
- [ ] **H3** — Health, Akteur A0: Direktnachricht mit Anhang; Löschen einer Direktnachricht mit Anhang → erwartet (L1 = L2): ändern dürfen sich nur die als Restorakel (f) benannten Felder (s. S5). Heute: —. Abnahme-Probe und Mutante, die rot werden muss: L2 Diff gegen Erlaubnisliste der Felder.
- [ ] **Schlüsselliste (D10)**: `Direktnachricht von 1.000 Bytes (Text, Stufe `information`) → Schnappschuss vor/nach gleich, `nachrichten`, `log_bytes`, `defekte_zeilen`, `nachholen.saetze` unverändert, `direkt_mib` 0; 1,1 MiB → `direkt_mib` 1; mit Anhang: Diff nur in den Feldern des Restorakels (f); **Schlüsselliste:** die Schlüssel der Antwort werden gegen das Inventar aus Regel 9 (im Test als Konstante) verglichen — ein Feld ohne Zeile ist rot`

### F0178 Direktnachricht im Browser führen

- [ ] **Beginnen**: `anna` klickt in der Teilnehmerliste auf `ben` → das Menü hat die Zeile „Direktnachricht“; auf ihre eigene Zeile geklickt gibt es keine Zeile „Direktnachricht“; die Auswahl öffnet `#/d/ben%40127.0.0.3` (bzw. die kodierte Form der Kennung); die Seite bietet keine Eingabe einer freien Kennung. Mutanten: Menüzeile am eigenen Teilnehmer; Kennung frei eintippbar.
- [ ] **Privat-Modus und Kopfzeile**: die Hauptansicht trägt `lc-privat-bg`, die Kopfzeile nennt die volle Kennung des Gegenübers statt `# name`, „7 Tage“ und den Hinweis wörtlich „Direktnachricht — nur ihr beide im Chat; die Dateien liegen im Ordner der Instanz“ (Feinschliff beim `ux-reviewer` vor dem Bau; geändert wird der Wortlaut dann hier und in B0784); bei 320 px bricht die Kopfzeile um, ohne den Hinweis zu verdecken oder die Seite quer scrollen zu lassen.
- [ ] **Live und Seitenleiste (D12)**: `Teilnehmerliste → „Direktnachricht“ öffnet `#/d/<kennung>` im Privat-Modus (`lc-privat-bg`, Kopfzeile mit voller Kennung und dem Hinweis „Direktnachricht — nur ihr beide im Chat; die Dateien liegen im Ordner der Instanz“); Seitenleiste „Direktnachrichten“ mit Status-Punkt; zweiter Browserkontext `ben` sieht die Nachricht live und den Eintrag in der Seitenleiste; dritter Kontext `cora` sieht weder Eintrag noch Nachricht, auch nach Neuladen; Tippen im Direktgespräch nur beim Gegenüber` Mutanten, die rot werden müssen: Seitenleiste listet fremde Gespräche; Hash eines fremden Paars öffnet Inhalt.
- [ ] **Adresse eines fremden Paars**: `cora` öffnet `#/d/anna%40127.0.0.2` → ein leeres Gespräch `cora`↔`anna` (R3), nie der Inhalt von `anna`↔`ben`; ohne Senden entsteht kein Ordner.
- [ ] **Tippen**: `anna` tippt im Direktgespräch → die Tippzeile bei `ben` erscheint binnen 1 s nach dem Signal (Grenze und Startpunkt wie R00045), bei `cora` und im zweiten Tab von `anna` nie.
- [ ] **Ohne Konsolenfehler** über die LAN-Adresse (Definition unter „Nicht-funktionale Anforderungen“).

## Betroffene Verzeichnisstruktur

Folgt der ADR (`architekt`): reine Werte `Direktpaar`, kanonische `Kennung` und `sichtbar_fuer` in `src/gespraeche/` (Baustein [Sichtbarkeit](../Dokumentation/Architektur/A00020-sichtbarkeit.md)); Anlegen, Laden, Grenzen in `src/gespraeche/verzeichnis.rs` und `logablage.rs`, Pflege in `pflege.rs`, Anhänge in `loeschen.rs` und `anhangverzeichnis.rs`, Marke in `nachholen.rs`; Routen in `src/http/` (Auflösung „Gespräch im Pfad“, A00004), Bus und Strom in `src/live/bus.rs` und `src/http/ereignisse.rs` (A00011), Health in `src/http/gesundheit.rs`, Protokoll und Episodenkatalog in `src/protokoll/`; Oberfläche in `web/` (`auswahl.js`, `hauptansicht.js`, `kopfzeile.js`, `seitenleiste.js`, `teilnehmerliste.js`, `verteiler.js`, `live.js`; Designsystem `components/Kopfzeile`, `Seitenleiste`, `Teilnehmerliste` nachziehen); Tests spiegeln die Themenordner, Architekturtest `tests/architektur_sichtbarkeit.rs`, E2E in `e2e/`.

## Technische Überlegungen

### Ablauf (Vorplanung, Stand nach ADR-0055; die Bubbles sind Entwurf, der Entwickler entscheidet)

1. **Werte und Laden** (F0172): `Direktpaar` und kanonische `Kennung` rein → `sichtbar_fuer` (ersetzt `aktivitaet::sichtbar`) → Anlegen und geprüftes Laden.
2. **K1 zuerst** (B0768): Senden und Lesen mit dem Paar aus Sitzung und Pfad; danach Zugang (B0769) und die übrigen Wege (B0770), Liste (B0771).
3. **Verteilende Wege** (F0173, F0174): Bus trägt `Gespraech`, jeder Abonnent und `warten` fragen `sichtbar_fuer`; Anhänge löschen über `finden_fuer`.
4. **Platz** (F0175): Pflege, leere Ordner, Grenzen, Protokollstufen.
5. **Gate und Health** (F0176, F0177), **Oberfläche** (F0178) zuletzt.

| Bubble | Datenfluss (Vorplanung) | Aufwand |
|---|---|---|
| B0764 Direktpaar als reiner Wert | zwei Kennungen → `Direktpaar` (sortiert, mit Ordnername) oder `Selbst`; Ordnername → `Option<Direktpaar>` | 2 |
| B0765 Kanonische Kennung | Text → `Kennung` (byte-gleich kanonisch) oder Abweisung `kennung` | 2 |
| B0766 Sichtbarkeit: eine Funktion, Typ Direktpaar im Gespräch | `Gespraech`, `Kennung`, archiviert → sichtbar ja/nein; `aktivitaet::sichtbar` entfällt | 2 |
| B0767 Direktgespräche anlegen und laden | erstes Senden → Ordner `direkt/<paar>/` unter der Verzeichnissperre; Ordner beim Start → geprüftes Paar im Index oder `WARNING laden` | 2-4 |
| B0768 Rundweg: Senden und Lesen im Direktgespräch (K1) | `POST/GET /api/direkt/{kennung}/nachrichten` mit Sitzung → 201 `{channel: null, direkt, nachricht}` bzw. Verlaufsblock | 2-4 |
| B0769 Zugang: Sitzung vor Pfad, Kanonprüfung, Lesen legt nichts an | Aufruf mit beliebiger Pfadform ohne oder mit Sitzung → 401 bzw. 422 `kennung`/`selbst`, 200 leer; kein Ordner | 2 |
| B0770 Bearbeiten, Löschen, Antworten, Anhang und Schlüssel im Direktgespräch | `POST …/nachrichten/bearbeiten/loeschen`, Senden mit `antwort_auf`, `anhaenge`, Wiederholschlüssel → wie im Channel, Dritte 404 wie „nie da“ | 2-4 |
| B0771 Liste der eigenen Direktgespräche | `GET /api/direkt` mit Sitzung → `{direktgespraeche: [{kennung, letzte, frist_tage}]}` nur eigene mit Nachricht in der Frist | 2 |

| Bubble | Datenfluss (Vorplanung) | Aufwand |
|---|---|---|
| B0772 Strom: Ereignisse tragen das Gespräch und gehen nur an Beteiligte | `nachricht/aenderung/anhaenge` im Bus mit `Gespraech` → Ausgabe `{art, channel: null, direkt: <Gegenüber>}` nur an die zwei Beteiligten | 2-4 |
| B0773 Aktivität im Direktgespräch nur beim Gegenüber | `PUT/DELETE/GET /api/direkt/{kennung}/aktivitaet` → Signal `{art: aktivitaet, channel: null, direkt}` nur an das Gegenüber | 2 |
| B0774 Sichtbare Marke: `seit` verrät keine fremde Direktnachricht | `warten` ohne `seit` oder mit `seit`, Wartender → `seit` hängt nur von Zeiten sichtbarer Nachrichten ab | 2-4 |

| Bubble | Datenfluss (Vorplanung) | Aufwand |
|---|---|---|
| B0775 Anhang-IDs eines Direktgesprächs bleiben Dritten verborgen | Verlauf, Strom, Warten, Liste für `cora` → keine Anhang-ID aus `anna`↔`ben`; Senden mit fremder ID → 422 `fremd` | 2 |
| B0776 Anhänge löschen: Sichtbarkeit an der Auflösung der ID | `POST /api/anhaenge/loeschen {ids}` mit Sitzung → für nicht sichtbare IDs `unbekannt`, byte-gleich zu einer nie vergebenen ID | 2-4 |

| Bubble | Datenfluss (Vorplanung) | Aufwand |
|---|---|---|
| B0777 Pflege: Direktgespräche entfallen samt Anhängen | Testuhr, Tageslogs eines Paars → Tage älter als 7 Kalendertage (heute zählt mit) samt Anhängen entfernt | 2 |
| B0778 Pflege: leere Paarordner entfernen (nur rmdir) | Paarordner nach dem letzten Tag, leer → entfernt von innen nach außen mit `rmdir`; mit Fremddatei oder Symlink → bleibt | 2-4 |
| B0779 Grenze neuer Paare je Herkunft und gesamt | erstes Senden an ein neues Paar → 201 oder 429 `{grenze: "direktgespraeche", art, hoechstens}`; bestehende Paare senden weiter | 2-4 |
| B0780 Protokoll: nichts Inhaltliches, Anlegen und Entfallen auf DEBUG, Episode bei Grenze | Austausch `anna`↔`ben` bei Stufe `debug` → `logs/` ohne Text, Anzeigenamen, volle Anhang-ID, Token; Grenze erreicht → eine `WARNING direkt`-Episode je Herkunft | 2 |

| Bubble | Datenfluss (Vorplanung) | Aufwand |
|---|---|---|
| B0781 Architekturtest Sichtbarkeit | Quelltext `src/**/*.rs` → rot, wenn `Gespraech::Direkt` außerhalb der Liste vorkommt, ein zweiter Name der Entscheidung existiert oder ein verteilender Weg `_ =>` über `Gespraech` trägt | 2 |

| Bubble | Datenfluss (Vorplanung) | Aufwand |
|---|---|---|
| B0782 Health zählt Direktgespräche nur als `direkt_mib` | `GET /api/gesundheit` ohne Sitzung → `nachrichten`, `log_bytes`, `defekte_zeilen`, `nachholen.saetze` und `kosten` nur Channels; neues Feld `direkt_mib` | 2 |
| B0783 Health: Schlüsselliste und Anhangzähler als benanntes Restorakel | Antwort von `GET /api/gesundheit` → jeder Schlüssel steht im Inventar aus ADR-0055 Regel 9; bei Anhang ändern sich nur die Felder des Restorakels (f) | 2 |

| Bubble | Datenfluss (Vorplanung) | Aufwand |
|---|---|---|
| B0784 Adresse `#/d/<kennung>` und Privat-Modus der Hauptansicht | Adressleiste `#/d/<kennung>` → Hauptansicht im Privat-Modus (`lc-privat-bg`) mit Verlauf und Eingabe gegen `/api/direkt/{kennung}/nachrichten`; Kopfzeile mit voller Kennung des Gegenübers | 2-4 |
| B0785 Teilnehmerliste: Menü „Direktnachricht“ | Klick oder Enter auf einen anderen Teilnehmer → Menü mit der Zeile „Direktnachricht“ → `#/d/<kennung>` | 2-4 |
| B0786 Seitenleiste „Direktnachrichten“ mit Status-Punkt | `GET /api/direkt` beim Laden und `nachricht` mit unbekanntem `direkt` im Strom → Abschnitt „Direktnachrichten“ mit Eintrag je Gegenüber (voller Kennung, Status-Punkt) | 2-4 |
| B0787 Live im Direktgespräch: Verlauf, Bearbeiten, Löschen, Tippen | Ereignisse `nachricht/aenderung/anhaenge/aktivitaet` mit `direkt` → Verlauf, Zitate, Anhänge und Tippzeile des offenen Direktgesprächs ändern sich ohne Neuladen | 2-4 |
| B0788 Drei Browserkontexte: der Dritte sieht nichts (E2E D12) | drei Kontexte `anna`, `ben`, `cora` → `cora` sieht weder Eintrag noch Nachricht, auch nach Neuladen | 2-4 |

### Grobentwurf

**Wichtige Einstiegsstellen:** Route `/api/direkt/{kennung}/…` (neue Ressource, gleiche Handler), Bus-Ereignis `Gespraech` statt `channel: String` (`src/live/bus.rs`), `warten`-Marke (`src/gespraeche/nachholen.rs`), Auflösung einer Anhang-ID für den Aufrufer (`src/gespraeche/loeschen.rs`), Health-Zählung (`src/http/gesundheit.rs`), Pflege (`src/gespraeche/pflege.rs`), Hauptansicht und Seitenleiste im Browser.

**Elemente** (maximal die Signaturen aus der ADR):
- `Direktpaar` (Wert, rein) — `bilden(a, b) -> Result<Direktpaar, Selbst>`, `beteiligt(&Kennung) -> bool`, `gegenueber(&Kennung) -> Option<&Kennung>`, `ordnername()`, `aus_ordnername(&str) -> Option<Direktpaar>`
- `Kennung::kanonisch(text)` (Operation, rein) — Abweisung statt Umformung
- `sichtbar_fuer(gespraech, kennung, archiviert) -> bool` (Operation, rein, **die einzige** Entscheidung)
- `Gespraech::Direkt(Direktpaar)` (Variante des Gesprächstyps)
- `anhangverzeichnis.finden_fuer(id, aufrufer)` (Operation) — `None` für nicht sichtbare Einträge
- `DIREKT_GESAMT`, `DIREKT_JE_HERKUNFT` (Konstanten mit Wertprüfung als Spanne)

### Änderungen an bestehenden Klassen

`Gespraech::Direkt(String)` → `Direktpaar`; `Ereignis::Nachricht|Aenderung|Anhaenge` tragen `gespraech: Gespraech` statt `channel: String`; `nachrichten_veroeffentlichen` veröffentlicht Direktgespräche (heute übergeht es sie, `bus.rs:150`); Strom-Filter (`ereignisse.rs:299`); `nachholen` (heute übergeht Direktgespräche, `:142`; Marke `:129`); `pflege` (heute übergeht `direkt/`, `:98`); `Dateiablage::gespraeche` (Laden mit Prüfung, `logablage.rs:116`); `anhaenge_loeschen` (`loeschen.rs:93`); `gesundheit.rs:105`.

## Tests

Nach `test-pyramide`: Test zuerst je Bubble, jede Probe läuft zusammen mit dem Kontrollpaar K1.

**Unit (rein):** `Direktpaar`, kanonische `Kennung`, `sichtbar_fuer`, Konstanten-Spanne, Architekturtest (Textprüfung).
**Integration:** Laden und Anlegen mit Speicherablage und `tempfile` (Schranke wie `src/gespraeche/schranke.rs`), Pflege mit Testuhr `LOCALCHATA_JETZT`, Grenzen mit kleinen Testkonstanten, Linie L1 der Matrix (Testserver im Prozess).
**Echtes Programm (Linie L2):** Routen, Strom, Warten, Anhänge, Health, Protokoll über die eigene Annahmeschleife mit Wegwerf-Ordner, `--port 0`, Quelladressen `127.0.0.2/3/9` (`curl --interface`), `--host ::` für K3.
**E2E (Playwright):** D12 mit drei Browserkontexten `anna`, `ben`, `cora`.
Mutationsprobe S17 im Gate des `security-reviewer`. Verzögert antwortender Fake für die Oberflächen-Aktionen, die neu laden (Seitenleiste, Wechsel zwischen Gesprächen).

## Abhängigkeiten

- Abhängig von: I0009 (Senden; Status gelb) mit [R00011](R00011-nachricht-senden.md), [R00039](R00039-eigene-nachricht-ueber-die-api-bearbeiten.md) und [R00040](R00040-eigene-nachricht-ueber-die-api-loeschen-anhaenge-mit.md) (Zeit als Adresse), [R00029](R00029-anhaenge-ueber-die-api-ablegen-senden-und-ausliefern.md) (Anhänge), [R00023](R00023-plattenquote.md) (Quote), [R00045](R00045-tippanzeige-und-denkt-nach-status.md) (Aktivität), [R00047](R00047-mit-zitat-antworten-api-und-browser.md) F0157 (Bezug), [R00049](R00049-auf-nachrichten-warten.md) F0166 (`warten`), R00008 und R00019 (Frist), R00015 (Grenzen je Herkunft); Entscheidungen ADR-0003, 0012, 0021, 0025, 0029, 0030, 0032, 0034, 0044, 0046, 0047, 0048, 0051, 0053, 0054, 0055.
- Blockiert: F0168 „Direktnachricht weckt“ (R00049, braucht F0172 und F0173), I0015 (Nachweis „Agent sendet direkt, Browser zeigt live“, braucht F0178), I0019 (Anleitung nennt die drei Lesewege, den 429 `direktgespraeche` und IPv6-Link-Local), F0179 (Ausbaustufe, braucht F0175 und eine eigene ADR); I0011, I0026, I0027 übernehmen die Matrix-Zeile U1 als Abnahme-Kriterium.
- Reihenfolge (WBS-Notiz „Karte #13 Detailplanung“): F0172 zuerst; danach F0173 ∥ F0174 ∥ F0175 ∥ F0177; F0176 nach F0173 und F0174; F0178 nach F0173 (UX-Vorprüfung vorher). B0774 und B0748 erst nach dem Merge der Karte #17 (`nachholen.rs`).

## Offene Fragen

Richtungsfragen gibt es keine: P1–P5 sind entschieden (Zeilen 166–170). Offen, nicht blockierend:

1. **Wortlaut und Platz des Hinweises** in der Kopfzeile — Feinschliff beim `ux-reviewer` (Vorprüfung vor F0178); `components/Kopfzeile` nennt noch „Privat · nur ihr beide“ und wird nachgezogen.
2. **Zähler „ungelesen“ und „@n“** der Seitenleiste (Designsystem) haben keine Interaction in der WBS — F0178 baut sie nicht; Empfehlung: Frage an den Betreiber, ob eine Interaction „Ungelesen sehen“ gewollt ist.
3. **Menü der Teilnehmerliste**: es gibt es im Code noch nicht; F0178 (B0785) baut es mit der Zeile „Direktnachricht“, I0013 ergänzt „Erwähnen“ — Empfehlung bestätigt, solange I0013 nicht davor gebaut wird.
4. **R00049** trägt die A1-Probe (sichtbare Marke) nicht; Nachzug über `/anforderung review R00049`.
5. **Matrix-Zeilenzahl**: die Kopfzeile der Angriffsmatrix nennt 44 Zeilen, gezählt sind 42 (Beleg: `grep` über die Zeilenköpfe K, R, W, E, X, S, G, A, H, U, P, O, F); alle 42 stehen oben in den Kriterien.
6. **Konstanten 4.096 / 8.192**: gerechnet, nicht gemessen (Herleitung in ADR-0055 Regel 4); jede Zahl ändert die ADR nicht, solange sie die Spanne der Wertprüfung trifft.
7. **Kein Artboard** zu D0004: gestaltet wird nach den Komponenten des Designsystems; `/wireframe verfeinern D0004` bleibt ein Angebot.

## Manuelle Vorbereitungstätigkeiten

- Entwicklertest (F0172–F0177 per `curl`, F0178 im Browser): die Quelladressen `127.0.0.2/3/9` stehen auf Linux lokal zur Verfügung; für K3 braucht die Instanz `--host ::`.

## Manuelle Nachbereitungstätigkeiten

- Designsystem nachziehen: `components/Kopfzeile` (Hinweistext), `components/Seitenleiste` (Abschnitt „Direktnachrichten“), `components/Teilnehmerliste` (Menü mit einer Zeile bis I0013).
- Betreiber-Anleitung I0019 (Lesewege, 429 `direktgespraeche`, Link-Local) und OpenAPI (I0015) nachziehen.

## Warum löst diese Anforderung das Problem?

Das Problem ist, dass es in localchatA nur Gespräche gibt, die jeder mit Sitzung liest, und dass jede Einführung eines privaten Gesprächs neue Wege schafft, auf denen das Private durchsickert: der Ereignisstrom, `warten`, die Anhänge, der Health-Check ohne Sitzung, sogar der Zeitstempel `seit`. Wenn die Zugriffsgrenze auf den Routen durch den Bau gilt (Paar aus Sitzung und Pfad, kein fremdes Gespräch nennbar) und alle übrigen Wege an **einer** Funktion hängen, die ein Architekturtest und elf Mutanten absichern, dann kann ein Weg das Private nicht mehr aus Versehen ausgeben — und jeder künftige Weg (Suche, Laufwerk, Reaktionen) erbt die Grenze oder macht das Gate rot. Dass die Matrix vor dem Bau existiert und K1 zuerst grün sein muss, macht aus der Zusage „nur ihr beide“ eine Reihe von Proben, die fehlschlagen können; und dass der Anlege-Flut eine Grenze gesetzt wird (4.096 / 8.192) und leere Ordner verschwinden, hält das Versprechen auch gegen den, der es durch Menge statt durch Einblick aushöhlt. Hebel ist die Reihenfolge: erst Routen und Sichtbarkeit, dann die verteilenden Wege, dann die Oberfläche — so zeigt die Oberfläche nur, was die API ohnehin schon schützt.

## Missing-Docs

Keine. Die Entscheidungen stehen in ADR-0055 und der Angriffsmatrix; für IPv6-Link-Local-Zonen gibt es keine eigene Quelle im Projekt (die ADR nennt die Annahme „Herkunft ist die `IpAddr` ohne Zone“) — die Anleitung (I0019) trägt den Hinweis.

## Notizen

- **Optionen**: entschieden in ADR-0055 (Gesamtschnitt V1 gewählt; verworfen V2 Paar im Pfad — ein fremdes Paar wäre nennbar; V3 Postfach je Kennung — zweiter Verteilweg und Ordnung; V4 Zufalls-ID — nicht lesbar ohne localchatA); die Einzelfragen E1–E18 stehen dort mit verworfenen Varianten. Verzicht auf ein neues Optionen-Tableau: die Richtung ist entschieden (P1–P5) und die Alternativen sind in der ADR dokumentiert.
- **Schnitt** (planer, 2026-10-30): nach Verhalten statt nach Bausteinen. Der API-Block des Auftrags (D1–D9, D11, D13–D15) hätte 17 Bubbles; er ist in fünf Slices geschnitten, weil die Hypothesen-Zählung über acht Bubbles ein Schnittsignal ist und weil F0173 (Strom, Warten), F0174 (Anhänge) und F0175 (Platz) für sich prüfbar und parallel baubar sind. Health und Oberfläche sind je ein Slice.
- **Bewusst ausgenommen**: Gruppen-Direktgespräche (P2: Vision „an einzelne Teilnehmer“, für Gruppen gibt es Channels); an sich selbst schreiben (P4: 422 `selbst`; `a+a` bliebe ungültig und wäre additiv nachrüstbar); Archivieren von Direktgesprächen (E14); Zusammenführen bei neuer Herkunft (E15); Frist wählbar (F0179, eigene ADR; Randbedingung: Ordner mit nur `einstellungen.toml`).
- **Vorprüfung `ux-reviewer`** (Pflicht vor F0178): Wortlaut und Platz des Hinweises, Kopfzeile schmal, Touch ≥ 44 px, Reihenfolge der Seitenleiste.
- **Bestandsbefunde** (latent, `security-reviewer`, nach I0012 „Wichtig“), mit dieser Anforderung behoben: `src/aktivitaet/mod.rs:285` (B0766), `src/gespraeche/logablage.rs:116` (B0767), `src/gespraeche/nachholen.rs:129` (B0774), `src/gespraeche/loeschen.rs:93` (B0776).
