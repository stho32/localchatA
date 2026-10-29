---
id: R00028
status: Neu
datum: 2026-10-29
---

# R00028: Netznamen nach den Reviews #35 gehärtet

## Beschreibung

Das Öffnen über Netznamen aus R00024 schließt die Lücken, die die Auto-Reviews zu #35 gefunden haben und die nicht Muss waren. Danach gilt:

- **Die Abweisungsseite passt auf Telefon und Tablet**: wer einen nicht freigegebenen Namen auf dem Telefon öffnet, liest die Seite ohne Zoomen, im hellen wie im dunklen Farbschema, und kommt mit einem Tipp auf den Link zur Oberfläche.
- **Abweisung und Hilfe nennen nur, was hilft**: kein „du“, einheitliche Wörter („IP-Adresse“, „Host-Name“), `--erreichbar-als` nur dort, wo es wirkt, kein leerer oder falscher Name im Text, ein Text, der mit einem Zeilenumbruch endet.
- **Das Protokoll sagt, warum abgewiesen wurde**: die Betreiberin unterscheidet einen nicht freigegebenen Namen von einer belegten Auskunft, einem leeren Eimer, einer Zeitgrenze und einer ausbleibenden Antwort.
- **Die Namensauskunft ist mit gestellter Uhr belegt und aufgeräumt**.

Zahlt ein auf: [Vision](R00000-vision.md) — „Eine gemeinsame Instanz im LAN, die alle Teilnehmer ansprechen — auch über Rechnergrenzen“ (der Weg zur Instanz bleibt auch auf dem Telefon und bei einer Abweisung verständlich). Kein Widerspruch zur Vision.

Herkunft: Karte #35 (R00024, I0024), Auto-Reviews vom 2026-10-29 — UX-Review Sollte S1 und Optional O1–O3; Rechtschreib-Review Sollte und Optional; Code-Review Sollte A1, A4 und Optional A5–A7; Observability-Review Sollte S1–S3 und Optional O1–O3; Security-Review Anmerkungen 3 und 4. Die Muss-Befunde und Anmerkung 2 des Security-Reviews sind Nacharbeit in R00024 (B0417–B0423). Was nicht hier steht und warum: Abschnitt „Notizen“.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0024 Instanz über ihren Netznamen öffnen, Dialog D0001 Start und Ablage): F0093 Abweisungsseite auf Telefon und Tablet, F0094 Abweisung und Hilfe nennen nur, was hilft, F0095 Ursache der Abweisung im Protokoll, F0096 Namensauskunft mit gestellter Uhr belegt und aufgeräumt.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00024 und ADR-0045: Namensauskunft, lokaler Namensraum, Rückwärtsname, Normalform, Abweisungsseite, Erlaubnis, Eimer, Gnadenfrist.

## Geschäftlicher Nutzen

Die Abweisungsseite ist der Ausweg für jemanden, der einen falschen Namen bekommen hat. Auf dem Telefon ist sie heute auf ein Drittel verkleinert (Schrift 5,2 px, Link 48 × 6 px, gemessen im UX-Review #35) und damit genau dort unbrauchbar, wo man einen Link aus dem Chat antippt. Die Seite schlägt `--erreichbar-als` auch für Namen vor, die damit nie freigegeben werden (`a_b.example`), und sie spricht den Teilnehmer mit „du“ an, gegen die Sprachregel des Designsystems. Im Protokoll sieht eine Abweisung wegen belegter Auskunft aus wie ein Verbot; wer nach einem Ausfall sucht, liest die falsche Ursache. Mit R00028 führt die Seite auf jedem Gerät zur Instanz, ihre Ratschläge wirken, und das Protokoll nennt die wahre Ursache.

## Funktionale Anforderungen

### Abweisungsseite auf Telefon und Tablet (F0093; UX-Review #35 S1, O1)

- Die Seite trägt eine Viewport-Angabe für Gerätebreite und das Farbschema „hell und dunkel“; sonst bleibt sie ohne Stil und ohne Skript (ADR-0045 Regel 7, CSP unverändert — vorbehaltlich der Bestätigung des architekt, „Offene Fragen“ 1).
- Das gilt auch für die Seite ohne Namen (fehlender `Host`, E4).

### Abweisung und Hilfe nennen nur, was hilft (F0094; Rechtschreib-Review #35, UX-Review O2, O3, Code-Review A1, A5, Security-Review Anmerkung 4)

- Seite und Text sprechen niemanden mit „du“ an; die Seite nennt localchatA statt „Instanz“ und sagt nicht, die Instanz „antworte nicht“. Empfehlung für den Satz vor dem Link: „localchatA ist über die IP-Adresse erreichbar:“ (Wortlaut per Ergänzung zu ADR-0045 Regel 7, „Offene Fragen“ 2).
- Die Zahl-Adresse heißt überall „IP-Adresse“; „Adresse“ allein steht nur in der Überschrift aus ADR-0045 Regel 7 (gemeint ist dort die aufgerufene Adresse).
- Fehlt der Name, heißt es auf Seite und im Text gleich: „keinen Host-Namen“.
- Ein doppelter `Host` ergibt im Text keinen leeren Namen und kein `--erreichbar-als` ohne Namen; ein `Host`, der nicht gelesen werden kann (roher UTF-8-Name), heißt „nicht lesbar“, nicht „nennt keinen Namen“.
- `--erreichbar-als <name>` wird nur für Namen vorgeschlagen, die die Normalform erfüllen; was mit `--erreichbar-als` beim Start angegeben wird und nie angenommen werden kann, wird beim Start nicht still übergangen (Form nach der Entscheidung des architekt, „Offene Fragen“ 3).
- Der Text für Nicht-Browser endet mit einem Zeilenumbruch.
- `--hilfe` nennt „einteilige Namen“ statt „einteilig“.

### Ursache der Abweisung im Protokoll (F0095; Observability-Review #35 S1–S3, O1–O3)

- Jede Abweisung eines Host-Namens trägt in ihrer Episode `grenzschutz` einen festen Wert für die Ursache: nicht freigegeben, Auskunft belegt (keine Erlaubnis), Eimer leer (kein Zeichen), Zeitgrenze, ohne Antwort (Fehler oder Panik der Auflösung). Schlüssel und Werte legt der architekt fest („Offene Fragen“ 4).
- Ein Fehler der echten Namensauskunft steht auf Debug mit dem Fehlertext in `ursache` (Fremdtext nach ADR-0040).
- Läuft eine Instanz mit `--alle-namen` über Mitternacht, steht die Warnung aus ADR-0045 Regel 9 auch in der neuen Tagesdatei.
- Der Start-Eintrag nennt, dass `LOCALCHATA_TESTNAMEN` gesetzt ist (nur Debug-Build).
- `/api/gesundheit` zählt zu langsame Auflösungen und Abweisungen ohne Erlaubnis oder Zeichen (Feldnamen: architekt, ADR-0043).

### Namensauskunft mit gestellter Uhr belegt und aufgeräumt (F0096; Code-Review #35 A4, A6, A7, Security-Review Anmerkung 3)

- Die Tests der Zeitgrenze laufen mit gestellter Uhr, nicht in Echtzeit.
- `HostAbgewiesen` hat keinen ungenutzten `Display`.
- Die Testauskunft (`LOCALCHATA_TESTNAMEN`, Tabelle) wächst nicht mit der Zahl der Fragen.
- `cc` und `shlex` stehen im Lockfile nicht unter dem Stand vor 38eb6c4 (`cc` 1.10.0, `shlex` 2.0.1), oder der Rückschritt ist im Commit begründet.

## Nicht-funktionale Anforderungen

- **Benutzerfreundlichkeit**: auf Telefon und Tablet keine Verkleinerung, Schrift mindestens 16 px, kein waagrechtes Scrollen; der Link ist mit einem Tipp erreichbar (Prüfraster des ux-reviewer: Gerät emuliert, nicht verkleinertes Fenster).
- **Sicherheit**: die CSP und der Header-Satz aus ADR-0014 Regel 5 bleiben unverändert; die Seite bleibt ohne Skript und ohne Stil; der Name erscheint weiter nur maskiert.
- **Zeit- und Konsolenkriterien** nach `AI-Team/planer.md`: neue Zeitgrenzen gibt es nicht. Konsole wie R00024 F0079 „Im Browser“: erlaubt sind nur die Meldungen „Failed to load resource“ zu den 403 auf das Dokument und auf `/favicon.ico` unter dem abgewiesenen Namen.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, `<p>`, `heute.log`, „Testnamen“ und „Testbrowser“ wie in R00024.

### F0093 Abweisungsseite auf Telefon und Tablet

- [ ] **Telefon** (E2E, Playwright mit eigenem Kontext; Abnahme-Probe aus dem UX-Review #35, Sollte S1, wörtlich übernommen): „Probe: Chromium isMobile, DPR 2, 320/375/414, hell/dunkel → clientWidth = Breite, scale 1, <p> ≥ 16 px, scrollWidth ≤ innerWidth, Linkhöhe ≥ 17 px; auch ohne Host. Heute rot.“ Ausgeschrieben: Kontext mit `isMobile: true`, `hasTouch: true`, `deviceScaleFactor: 2`, Viewport 320 × 640, 375 × 812 und 414 × 896, je mit `colorScheme` `light` und `dark`; `http://boese.example:<p>/` im Testbrowser → `document.documentElement.clientWidth` gleich der Breite, `visualViewport.scale` = 1, jedes `<p>` mit berechneter Schriftgröße ≥ 16 px, `document.documentElement.scrollWidth` ≤ `innerWidth`, der Link mindestens 17 px hoch. „Auch ohne Host“: die Seite ohne Namen (roh über Socket geholt wie in R00024 „Ohne `Host`“) im selben Kontext mit `page.setContent` geladen → dieselben Werte.
- [ ] **Tablet** (E2E, wie oben; Prüfraster des ux-reviewer): 768 × 1024 und 1024 × 768 → dieselben Werte.
- [ ] **Tipp führt zur Instanz** (E2E, Telefon 375 × 812, nur Touch): `tap()` auf den Link → `http://127.0.0.1:<p>/` öffnet sich mit dem Titel `localchatA`.
- [ ] **Dunkles Farbschema** (E2E; UX-Review #35 O1): die Seite enthält `<meta name="color-scheme" content="light dark">`; mit `colorScheme: 'dark'` ist die berechnete Textfarbe von `<h1>` nicht `rgb(0, 0, 0)`, mit `light` schon.
- [ ] **Bestand** (Integration, Unit): R00024 F0079 „Seite“ und „Browser bekommt die Seite“ bleiben grün — kein `<style`, kein `style=`, kein `<script`, kein `on…=`; `Content-Security-Policy` unverändert; bei 1280 × 800 ohne `isMobile` ist jedes `<p>` mindestens 16 px groß und `scrollWidth` ≤ `innerWidth`.

### F0094 Abweisung und Hilfe nennen nur, was hilft

- [ ] **Keine Du-Form, einheitliche Wörter** (Unit, rein; Rechtschreib-Review #35 Sollte, UX-Review O2): Seite und Text — mit Namen, ohne Namen, mit IPv6 — enthalten keines der Wörter `Öffne`, `öffne`, `du`, `dich`, `dein` (ganze Wörter) und nicht „antwortet nicht“; der Satz vor dem Link nennt „localchatA“ und „IP-Adresse“; „Adresse“ ohne „IP-“ steht nur in der Überschrift „Diese Adresse ist für localchatA nicht freigegeben“.
- [ ] **Fehlender Name gleich benannt** (Unit, Integration roh über Socket): ohne `Host` enthalten Seite und Text „keinen Host-Namen“; keine von beiden enthält „keinen Namen“ oder „keinen Host —“.
- [ ] **Doppelter und unlesbarer `Host`** (Integration, roh über Socket; Code-Review A5, Security-Review Anmerkung 4): zwei `Host`-Köpfe `127.0.0.1:<p>` und `boese.example:<p>` → 403, der Text enthält weder „Namen  “ (Name leer) noch `--erreichbar-als` ohne folgenden Namen; `Host: bücher.local:<p>` roh in UTF-8 → 403, der Text enthält „nicht lesbar“ und nicht „keinen Host-Namen“.
- [ ] **`--erreichbar-als` nur, wo es wirkt** (Unit, Integration; Code-Review A1, Rechtschreib-Review Hinweis): `Host: a_b.example:<p>` und `Host: a"b.local:<p>` → Seite und Text ohne `--erreichbar-als`; `Host: boese.example:<p>` → mit `--erreichbar-als boese.example` wie bisher.
- [ ] **`--erreichbar-als` beim Start** (Integration; vorbehaltlich der Entscheidung des architekt, „Offene Fragen“ 3 — hier die Empfehlung): `localchatA <ordner> --erreichbar-als a_b.example --kein-browser` → Exit 2, eine Zeile auf stderr nennt `--erreichbar-als` und `a_b.example`, kein Listener; `--erreichbar-als Chat.Firma.DE.` → Start, `Host: chat.firma.de:<p>` erlaubt.
- [ ] **Text endet mit Zeilenumbruch** (Integration; UX-Review O3): der Körper jeder 403 als `text/plain` endet mit `\n`.
- [ ] **Hilfe** (Unit; Rechtschreib-Review Optional): `--hilfe` enthält „einteilige Namen“ und nicht „, einteilig)“; die Probe aus R00024 „Wortlaut der Risikosätze“ bleibt grün.
- [ ] Review-Punkt für den `rechtschreib-reviewer`: die Wörter aus den beiden ersten Kriterien gelten auch in den Tests und Kommentaren unter `src/http/` zur Abweisung.

### F0095 Ursache der Abweisung im Protokoll

- [ ] **Ursache bei belegter Auskunft** (Unit mit Speicher-Senke und blockierender Testauskunft; Abnahme-Probe aus dem Observability-Review #35, Sollte S1, wörtlich übernommen): „Probe: zwei hängende .local, dann nuc5.fritz.box über LAN-IP → Zeile nennt auskunft_belegt (heute rot).“ Ausgeschrieben: zwei Vorwärtsfragen für `a.local` und `b.local` hängen (beide Erlaubnisse belegt); dann `nuc5.fritz.box:7430` über die lokale Adresse `192.168.178.65`, nicht im Cache → abgewiesen, und die `WARNING grenzschutz`-Zeile der Episode nennt die Ursache „Auskunft belegt“ (Schlüssel und Wert nach „Offene Fragen“ 4; Empfehlung `grund=auskunft_belegt`).
- [ ] **Die übrigen Ursachen** (Unit wie oben): `boese.example` → „nicht freigegeben“; 21. neuer Name bei stehender Testuhr → „Eimer leer“; Frist abgelaufen → „Zeitgrenze“; Panik oder leere Antwort mit Fehler → „ohne Antwort“. Je Ursache genau ein fester Wert, kein Fremdtext im festen Schlüssel.
- [ ] **Fehler der echten Auskunft** (Unit über die Fehlerabbildung; Observability S2): ein Fehler von `lookup_addr` bzw. `ToSocketAddrs` ergibt auf Debug eine Zeile „Namensauskunft ohne Antwort“ mit `art`, `schluessel` und dem Fehlertext in `ursache`.
- [ ] **Warnung je Tagesdatei** (Integration, gestellte Uhr `LOCALCHATA_JETZT` kurz vor Mitternacht; Observability O1): mit `--alle-namen` enthält nach dem Tageswechsel auch `logs/<Folgetag>.log` die WARNING „--alle-namen ist aktiv: Jeder Host-Name wird angenommen …“ genau einmal (Form nach „Offene Fragen“ 4).
- [ ] **Testknopf im Start-Eintrag** (Integration, Debug-Build; Observability O3): mit Testnamen nennt der Start-Eintrag `LOCALCHATA_TESTNAMEN`; ohne nicht.
- [ ] **Zähler** (Integration bzw. Unit; Observability O2): nach einer Zeitgrenze steigt der Zähler für zu langsame Auflösungen um 1, nach einer Abweisung ohne Erlaubnis oder ohne Zeichen der Zähler dafür um 1; `/api/gesundheit` nennt beide (Feldnamen nach „Offene Fragen“ 4).

### F0096 Namensauskunft mit gestellter Uhr belegt und aufgeräumt

- [ ] **Gestellte Uhr** (Review-Punkt für den `code-reviewer`; Code-Review A4): die Tests der Zeitgrenze und der Frist laufen mit pausierter bzw. gestellter Zeit; die Mutante „Frist 1 s → 2 s“ wird rot, ohne dass ein Test eine Sekunde Echtzeit wartet.
- [ ] **Kein toter Code** (Review-Punkt; Code-Review A6): `Display` für `HostAbgewiesen` ist entweder entfernt oder von einem Test und einem Aufrufer benutzt; `cargo clippy --all-targets -- -D warnings` grün.
- [ ] **Testauskunft begrenzt** (Unit; Code-Review A7): 100.000 Fragen über 10 Schlüssel → die Testauskunft hält höchstens 10 Einträge (je Art und Schlüssel ein Zähler); `fragen(art, schluessel)` liefert weiter die Zahl.
- [ ] **Lockfile** (Review-Punkt für den `security-reviewer`; Anmerkung 3): `Cargo.lock` führt `cc` ≥ 1.10.0 und `shlex` ≥ 2.0.1, oder der Commit, der sie senkt, nennt den Grund.
- [ ] **Kommentar** (Review-Punkt für den `rechtschreib-reviewer`; Optional): der beanstandete Kommentar in `e2e/tests/r00024-netznamen.spec.js` trennt die beiden Hauptsätze mit Semikolon.

## Betroffene Verzeichnisstruktur

- `src/http/`: `abweisungsseite.rs` (Viewport, Farbschema, Wortlaut, Ausweg nur bei Normalform, Zeilenumbruch), `grenzschutz.rs` (Ursache der Abweisung, doppelter und unlesbarer Host), `auskunft.rs` (Zähler, Tests mit gestellter Uhr), `namensauskunft.rs` (Fehler mit Ursache, begrenzte Testauskunft), `gesundheit.rs` (Zähler).
- `src/start/`: `aufruf.rs` (Hilfe, Prüfung von `--erreichbar-als`), `mod.rs` (Start-Eintrag); `src/protokoll/` (Warnung je Tagesdatei).
- `tests/`: Integration über das echte Programm; `e2e/`: Abweisungsseite als emuliertes Telefon und Tablet.

## Technische Überlegungen

### Ablauf

1. **Start**: `--erreichbar-als` gegen die Normalform prüfen (Empfehlung) → Start-Eintrag mit Testknopf → bei `--alle-namen` WARNING, auch beim Tageswechsel.
2. **Anfrage im Grenzschutz** (wie R00024): abgewiesen → Ursache als fester Wert in der Episode → Zähler → Seite (mit Viewport und Farbschema) oder Text (mit Zeilenumbruch); Vorschlag `--erreichbar-als` nur bei Normalform.

### Grobentwurf (Vorplanung)

Die Slices sind noch nicht bis Bubble geplant (Rolling Wave, Architektur zuerst). Nach den Entscheidungen des architekt: `/planung verfeinern F0093 F0094 F0095 F0096 --bis Bubble`.

### Änderungen an bestehenden Klassen

- `abweisungsseite::seite` und `abweisungsseite::text`: Wortlaut, Meta-Angaben, Ausweg nur bei Normalform, Zeilenumbruch.
- `Grenzschutz`: Ursache je Abweisung, Zähler.
- `Startauftrag`/Aufruf: Prüfung von `--erreichbar-als`; `HILFE`.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests.

- **Unit (rein)**: Seite und Text (Wörter, Ausweg, Zeilenumbruch), Ursachen der Abweisung, Fehlerabbildung der echten Auskunft, begrenzte Testauskunft, Hilfe.
- **Integration (echtes Programm)**: doppelter und unlesbarer Host roh über Socket, `--erreichbar-als` beim Start, Start-Eintrag, Warnung je Tagesdatei, Zähler im Health-Check.
- **E2E**: Abweisungsseite als emuliertes Telefon und Tablet, hell und dunkel, Tipp auf den Link.

## Abhängigkeiten

- Abhängig von: R00024 (I0024) — F0079 Abweisung als verständliche Seite (grün) für F0093; F0079 und F0080 Schalter `--alle-namen` (gelb, Nacharbeit B0423) für F0094; F0077 Über den Rückwärtsnamen öffnen (gelb, Nacharbeit B0417–B0422) für F0095 und F0096.
- Blockiert: keine.

## Offene Fragen

An den architekt, vor dem Bau (Architektur zuerst); Empfehlungen des planer:

1. **Meta-Angaben auf der Seite** (F0093): passen `<meta name="viewport" content="width=device-width, initial-scale=1">` und `<meta name="color-scheme" content="light dark">` zu „ohne Stil“ aus ADR-0045 Regel 7? Empfehlung: ja — beides ist eine Angabe an den Browser, kein Stil; die CSP bleibt.
2. **Wortlaut der Seite** (F0094): ADR-0045 Regel 7 nennt die Sätze wörtlich; neuer Satz für den Teilnehmer ohne Du-Form und ohne „antwortet nicht“. Empfehlung: „localchatA ist über die IP-Adresse erreichbar: <Link>“; ohne Namen „Diese Anfrage nennt keinen Host-Namen. localchatA ist über die IP-Adresse erreichbar: <Link>“.
3. **`--erreichbar-als` und Normalform** (F0094, Code-Review A1): (a) beim Start prüfen, Exit 2 für Namen außerhalb der Normalform; (b) die feste Liste vor der Normalform vergleichen, dann wirkt `--erreichbar-als a_b.example` wieder. Empfehlung: (a) — Regel 1 bleibt vor jeder Liste, und der Betreiber erfährt beim Start, dass der Name nicht taugt, statt später eine Seite mit einem Ratschlag zu sehen, der nicht wirkt.
4. **Ursache, Tageswechsel, Zähler** (F0095): der Vorschlag des observability-reviewer heißt `ursache=…`; in diesem Projekt trägt `ursache` Fremdtext, feste Werte stehen in `grund` (ADR-0040, ADR-0042). Empfehlung: `grund` mit `nicht_freigegeben`, `auskunft_belegt`, `eimer_leer`, `zeitgrenze`, `ohne_antwort`; Katalog ADR-0042 und Feldliste ADR-0043 nachtragen (Observability S3: auch `alle_namen` und `schluessel`); Form des Hinweises je Tagesdatei; Zählernamen im Health-Check unter der Regel „keine Aufteilungen im Health-Check“.

## Manuelle Vorbereitungstätigkeiten

- architekt: Entscheidungen zu „Offene Fragen“ 1–4 als Ergänzung zu ADR-0045, ADR-0042 und ADR-0043; danach `/planung verfeinern F0093 F0094 F0095 F0096 --bis Bubble` (planer).

## Manuelle Nachbereitungstätigkeiten

- Keine.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser sind die Reviews #35: Auf dem Telefon ist die Abweisungsseite unlesbar klein (fehlende Viewport-Angabe), ihr Ratschlag `--erreichbar-als` wirkt für manche Namen nicht, und das Protokoll nennt bei einer belegten Auskunft dieselbe Ursache wie bei einem verbotenen Namen. Ziel ist, dass die Abweisung auf jedem Gerät zur Instanz führt und jede Spur die wahre Ursache zeigt.

- Die Viewport-Angabe lässt das Telefon die Seite in Gerätebreite zeichnen. Deshalb sind Schrift und Link groß genug, und der Teilnehmer tippt sich zur Instanz durch.
- Der Ratschlag erscheint nur, wo er wirkt, und ein unbrauchbarer Startwert fällt beim Start auf. Deshalb sucht der Betreiber nicht nach einem Fehler, den es nicht gibt.
- Die Ursache steht als fester Wert in der Episode. Deshalb sieht die Betreiberin bei einer Flut „Auskunft belegt“ statt „nicht freigegeben“ und sucht die Ursache im Netz, nicht in der Freigabe.

Der Hebel liegt an den Stellen, die R00024 gebaut hat (Seite, Text, Episode); eine neue Seite oder ein neuer Dialog wäre mehr, als die Befunde verlangen.

## Missing-Docs

- Kein Betriebshandbuch und kein README im Projekt; ADR-0045 nennt Hinweise „für das Betriebshandbuch“ (Restrisiko DNS über HTTPS, `ip_nonlocal_bind`), die heute keinen Ort haben.
- Glossar fehlt (`Dokumentation/Domaenenwissen/`).

## Notizen

### Zuordnung der Befunde aus den Reviews #35

- In R00024 als Nacharbeit (nicht hier): Code-Review B1 (Muss), A2, A3; Security-Review W1, Anmerkung 2; Rechtschreib-Review Muss und Sollte `RISIKO` „Jeder“; Hinweis des code-reviewer „Rate ausgehender Fragen“.
- Security-Review Anmerkung 1 (DNS über HTTPS, PTR unter öffentlicher TLD): Restrisiko in ADR-0045 „Negativ / Inkauf zu nehmen“; kein Knoten.

### Verworfene Alternativen

- UX-Review O4 (Risikosatz in `--hilfe` umbrechen): verworfen — die Abnahme-Probe des rechtschreib-reviewer (R00024, Nacharbeit) verlangt den Satz auf einer Zeile; Terminals brechen selbst um, und die Aufrufzeile der Hilfe ist schon länger.
- Observability-Review O4 (hyper-400 ohne Grund): nicht hier — nicht aus #35; Kandidat für eine Härtung unter I0021.
- Härtung als eigene Interaction: verworfen — dasselbe Verhalten an der Systemgrenze, reifer, wie F0059–F0062 unter I0021 und F0089, F0090 unter I0009.

### Umfang

Nicht bis Bubble geplant (`/planung zaehlen I0024`: F0093–F0096 ohne Bubbles). Hypothese zur Ordnung, nicht gezählt: F0093 etwa 2 Bubbles, F0094 etwa 4, F0095 etwa 4, F0096 etwa 3 — zur Untergrenze 0,4 h je Bubble rund 5 h, Faktor 0,5–2.
