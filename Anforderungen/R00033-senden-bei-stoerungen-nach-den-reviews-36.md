---
id: R00033
status: Neu
datum: 2026-10-29
---

# R00033: Senden bei Störungen nach den Reviews #36

## Beschreibung

Erste kleine Härtung aus den Sollte-Befunden zu R00025 (Karte #36, gesammelt auf Karte #50). Danach gilt:

- **Der Countdown ist auch am schmalen Telefon ganz zu lesen**: bei 320 px Breite schneidet die Verbindungsleiste nie mehr die Zahl oder das „s“ ab; wenn der Platz nicht reicht, kürzt sie den Anfang „Verbindung verloren“ mit „…“, nie „neuer Versuch in n s“ bzw. „wird verbunden …“. Ab 375 px steht der Text ungekürzt. Der Wortlaut bleibt wie in R00025.
- **Die Wartezeile ist für Bildschirmleser hörbar**: „n ungesendete Nachrichten warten auf den Beitritt — Neuladen verwirft sie.“ gehört zur Beschreibung des Namensfelds auf dem Beitreten-Bildschirm; wer dort mit Bildschirmleser landet, hört sie mit dem Feld.
- **Zusammengezogene Ansagen tragen keinen doppelten Punkt**: endet ein Text schon auf einem Satzzeichen, kommt beim Zusammenziehen kein weiterer Punkt dazu.
- **Der Wächter „Uhr zurück“ des Wiederholspeichers ist belegt**: ein Test zeigt, dass eine zurückgestellte Uhr keine Wiederholung doppelt ankommen lässt, und wird rot, wenn der Wächter fehlt.
- **Zahlen und Wörter in Tests und Kommentaren** folgen der Schreibweise des Projekts („15.000“, „Episodenbeginn“).

Zahlt ein auf: [Vision](R00000-vision.md) — „Mensch und KI sind gleichberechtigt“ und „eine gemeinsame LAN-Instanz“ über R00025: wer am Telefon oder mit Bildschirmleser arbeitet, liest bzw. hört dieselbe Warnung wie am Schreibtisch, und eine verstellte Uhr verdoppelt keine Nachricht. Kein Widerspruch zur Vision.

Herkunft: Karte #50 (Sollte/Optional ohne Muss aus den Reviews und dem Entwicklertest #36, 2026-10-29) — ux-reviewer #36 S1 und S3, code-reviewer #36 A2, rechtschreib-reviewer #36 Sollte und Optional (doppelter Punkt). Die übrigen Punkte der Karte bleiben auf Karte #50 (siehe „Notizen“, „Bewusst außerhalb“).

Weiche (`rollen/planer/befunde-einplanen.md`): **reifer** — R00025 hält seine Zusagen (Countdown-Text, Wartezeile, Ansage, Wiederholschlüssel); kein Kriterium verlangt 320 px, Bildschirmleser auf dem Beitreten-Bildschirm oder einen Test für den Wächter. Deshalb ein Feature unter I0020, kein `aus-bug`.

Slice (WBS `Dokumentation/Planung/localchata.md`, unter I0020 Nachricht senden bei Störungen und mit Komfort): F0124 Senden bei Störungen nach den Reviews #36.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00012 und R00025: Verbindungsleiste (Warnung), Countdown, Ansage-Region, Sofort-Ansage, Beitreten-Bildschirm, Wartezeile, Wiederholschlüssel, Wiederholspeicher, Verdrängung.

## Geschäftlicher Nutzen

Am Telefon mit 320 px Breite liest die Mitleserin heute „⚠ Verbindung verloren · neuer Versuch i…“ — die eine Zahl, die ihr sagt, ob sich Warten lohnt, fehlt. Wer mit Bildschirmleser nach einem gescheiterten Wiederbeitritt auf dem Beitreten-Bildschirm landet, erfährt nicht, dass seine Nachrichten warten und ein Neuladen sie verwirft — er lädt neu und verliert sie. Ein Bildschirmleser liest „entfernen.. Frist …“. Und der Wächter, der bei zurückgestellter Uhr eine doppelte Nachricht verhindert, kann heute unbemerkt verschwinden.

## Funktionale Anforderungen

### Senden bei Störungen nach den Reviews #36 (F0124)

- **Countdown bei 320 px** (ux-reviewer #36 S1): die Leiste besteht aus zwei Teilen — **Anlass** („⚠ Verbindung verloren“) und **Stand** („ · neuer Versuch in n s“ bzw. „ · wird verbunden …“). Der Stand wird nie gekürzt; reicht der Platz nicht, kürzt der Anlass mit „…“ am Ende. Ohne Stand („⚠ Verbindung verloren“ allein) und für „✓ Wieder verbunden …“ gilt die Kürzung wie bisher. Der Text als Ganzes (Ansage, `textContent` der Leiste) bleibt wörtlich wie R00025; die Leiste bleibt einzeilig und in allen Texten gleich hoch.
- **Wartezeile für Bildschirmleser** (ux-reviewer #36 S3): steht die Wartezeile auf dem Beitreten-Bildschirm, trägt sie eine `id`, und das Namensfeld nennt diese `id` in seinem `aria-describedby` (nach den bisherigen: `lc-name-zweck lc-name-hinweis lc-meldung`); ohne Wartezeile keine Referenz. Kein `role="alert"` — der Bildschirm setzt den Fokus ohnehin ins Namensfeld, und `lc-meldung` ist schon die Alert-Region.
- **Kein doppelter Punkt** (rechtschreib-reviewer #36 Optional, `web/ansage.js:84–86`): beim Zusammenziehen mehrerer Sofort-Ansagen eines Bildes folgt auf einen Text, der auf „.“, „!“, „?“ oder „…“ endet, nur ein Leerzeichen; sonst „. “ wie bisher. Reihenfolge und „gleiche einmal“ bleiben.
- **Wächter „Uhr zurück“ belegt** (code-reviewer #36 A2, `src/gespraeche/wiederholung.rs:181–183`): kein Verhalten neu, nur ein Test.
- **Schreibweisen** (rechtschreib-reviewer #36 Sollte): `e2e/tests/r00012-eingabe.spec.js:84–85` Zahlen mit Tausenderpunkt („14.990“, „16.390“, „15.000“); `tests/wiederholung_regressionen.rs:384` „Episodenbeginn und -ende“ statt „Episode-Beginn und -Ende“.

## Nicht-funktionale Anforderungen

- **Barrierefreiheit**: WCAG 2.2 SC 1.4.10 (Reflow, 320 CSS-px) für die Leiste; SC 4.1.2 (Name, Rolle, Wert) bzw. 1.3.1 für die Wartezeile als Beschreibung des Felds.
- Keine neue Zeitgrenze, keine Änderung an Server-Antworten.

## Akzeptanzkriterien

Gemeinsam: Start, Beitritte und Browser A wie in den User Stories zu R00025; „Telefon n px“ heißt Chromium mit Viewport n × 740, `hasTouch`, `isMobile`. „Getrennt mit Countdown“ wird hergestellt wie in R00025 F0084 (Instanz beenden, solange A offen ist; der Strom meldet den nächsten Versuch).

### F0124 Senden bei Störungen nach den Reviews #36

- [ ] **Countdown bei 320/375/414 px** (E2E; Abnahme-Probe ux-reviewer #36 S1, wörtlich: „Probe: Textbreite ≤ clientWidth − Polster bei 320/375/414.“). Ausgeschrieben, Lesart des planer (gemessen mit `Range.getBoundingClientRect()` über den Text des jeweiligen Teils, „Polster“ = `padding-left` + `padding-right` der Leiste): bei Telefon 320, 375 und 414 px zeigt A getrennt mit Countdown „neuer Versuch in 10 s“ (breitester Stand) und danach „wird verbunden …“; je Fall ist die Breite des **Stands** ≤ `clientWidth` der Leiste − Polster − Breite des sichtbaren Anlasses, und der Stand ist ungekürzt (`scrollWidth` ≤ `clientWidth` seines Elements). Bei 375 und 414 px ist auch der Anlass ungekürzt (Textbreite der ganzen Leiste ≤ `clientWidth` − Polster). Bei 320 px heute rot (275 > 248 px, „s“ fehlt). Die Mutante „Stand darf schrumpfen“ (`flex-shrink` des Stands ≠ 0) wird bei 320 px rot.
- [ ] **Text der Leiste unverändert** (Node-Test, rein): `warnungText` liefert dieselben Zeichenketten wie R00025 „Countdown-Text“ (Kriterium bleibt grün); die Leiste setzt Anlass und Stand so, dass `textContent` der Leiste genau diesen Text ergibt.
- [ ] **Wartezeile im `aria-describedby`** (Node-Test über den DOM des Beitreten-Bildschirms und E2E): mit 2 fehlgeschlagenen Nachrichten nennt `aria-describedby` des Namensfelds `lc-name-zweck lc-name-hinweis lc-meldung` und die `id` der Wartezeile, deren Text „2 ungesendete Nachrichten warten auf den Beitritt — Neuladen verwirft sie.“ ist; mit 0 fehlt die `id`, und es gibt kein leeres Element dafür. E2E (Chromium, Weg „Scheitern nach Neustart (a) Strom zuerst“ aus R00025 F0088): die berechnete Beschreibung des Felds (`page.getByLabel('Name')` → `toHaveAccessibleDescription`) enthält „warten auf den Beitritt — Neuladen verwirft sie.“. Heute rot. Die Mutante „Wartezeile ohne Referenz“ wird rot.
- [ ] **Kein doppelter Punkt** (Node-Test, rein, `ansageSchreiberAnlegen` mit gestelltem `imNaechstenBild`): im selben Bild `'a.'`, `'b'` → „a. b“; `'a'`, `'b'` → „a. b“; `'a…'`, `'b'` → „a… b“; `'a.'`, `'b.'` → „a. b.“; `'a'`, `'a'` → „a“. Die Mutante „immer `. ` verbinden“ wird rot („a.. b“).
- [ ] **Wächter „Uhr zurück“** (Unit, rein, gestellte Uhr; hergeleitet vom planer, siehe Offene Fragen): Speicher `klein(640, 100)`; `a` merkt `y` bei 300.000 ms; die Uhr springt zurück, `k` merkt `x` bei 0 ms; bei 600.001 ms ist `x` für `k` nicht bekannt (abgelaufen, steht aber noch hinter `y`), und `k` merkt `x` neu; bei 900.001 ms merkt `z` `w` (Abgelaufene fallen, darunter `y`); bei 900.002 ms ist `x` für `k` bekannt (`Wiederholt`), und `k` hat genau 1 Eintrag. Rechnung: der neue Eintrag `x` ist bei 900.002 ms 300.001 ms alt < 600.000 ms. Die Mutante „Zeilen 181–183 entfernt“ (alter Eintrag bleibt, sein späteres Entfernen löscht den Verweis auf den neuen) macht `x` unbekannt — rot.
- [ ] **Schreibweisen** (Durchsicht): `grep -n "15 000\|14 990\|16 390" e2e/tests/r00012-eingabe.spec.js` und `grep -n "Episode-Beginn" tests/wiederholung_regressionen.rs` finden nichts; beide Dateien laufen grün.
- [ ] Konsole bei A in den E2E-Kriterien: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zu Anfragen an die beendete Instanz.

## Betroffene Verzeichnisstruktur

- Oberfläche: `web/hauptansicht.js` (`leisteZeigen` setzt Anlass und Stand), `web/verbindungsleiste.js` (Teilung des Texts, falls dort), `web/lc-seite.css` (`.lc-leiste` und zwei Teile), `web/beitreten.js` (Wartezeile mit `id`, `aria-describedby`), `web/ansage.js` (Zusammenziehen).
- Server: kein Programmcode; Test in `src/gespraeche/wiederholung.rs` (Modul `tests`).
- Tests: `e2e/einheit/verbindungsleiste.test.mjs`, `e2e/einheit/beitreten.test.mjs`, `e2e/einheit/ansage.test.mjs`; E2E neu `e2e/tests/r00033-schmal-und-vorgelesen.spec.js`; Durchsicht `e2e/tests/r00012-eingabe.spec.js`, `tests/wiederholung_regressionen.rs`.

## Technische Überlegungen

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0124: B0550 Countdown bleibt bei 320 px ganz · B0551 Wartezeile beschreibt das Namensfeld · B0552 Zusammengezogene Ansage ohne doppelten Punkt · B0553 Wächter „Uhr zurück“ belegt · B0554 Schreibweisen in Tests und Kommentaren

Wireframe: kein Artboard `D0003.dc.html` nötig; Gestaltung wie gebaut (ADR-0038 Ergänzung Karte #36, Designsystem `Statuszeile`), nur die Kürzung wandert in den Anlass.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Unit/Node (rein)**: Text der Leiste, Wartezeile im DOM, kein doppelter Punkt, Wächter „Uhr zurück“.
- **E2E**: Countdown bei 320/375/414 px, Wartezeile vorgelesen.

## Abhängigkeiten

- Abhängig von: F0084, F0088 (gruen, R00025).
- Konflikte: keine Datei, die #9 (I0010 F0097) gerade ändert (`src/http/*`, `src/gespraeche/anhang*`). `e2e/tests/r00012-eingabe.spec.js` ändert #49 in Zeile 4 (`require('./rahmen')`); B0554 ändert Zeilen 84–85 — getrennte Stellen. Neue E2E-Datei nach dem Rahmen aus #49 (`require('./rahmen')`), sobald #49 auf main ist.
- Blockiert: —

## Offene Fragen

Keine an den Betreiber. Entschieden vom planer (Rückfragen mit Empfehlung beantwortet):

- **Countdown: Stand vor der Ellipse statt kürzerer Wortlaut** (ux S1 nennt beides). Gewählt: der Anlass kürzt, der Stand nie — der Wortlaut aus R00025 F0084, seinen User Stories, der Ansage und den Bestandstests bleibt, und am Telefon steht trotzdem die Zahl. Verworfen: kürzerer Wortlaut überall („⚠ Getrennt · neuer Versuch in n s“) — ändert eine wörtliche Zusage von R00025 und alle Tests daran; Wortlaut nur bei schmal — zwei Texte für denselben Zustand. Ein ux-reviewer kann am Ergebnis den Wortlaut kippen.
- **Wartezeile: `aria-describedby` statt `role="alert"`** (ux S3 nennt beides): der Fokus steht beim Wechsel im Namensfeld, die Beschreibung wird mit dem Feld gelesen; ein zweiter Alert neben `lc-meldung` würde mit der Meldung „Name vergeben“ um die Ansage konkurrieren.
- **Abnahme-Probe zu crev A2 fehlt auf der Karte**: der Kommentar „code-reviewer #36“ verweist auf „Probe im Bericht“; der Bericht liegt nicht vor. Das Kriterium „Wächter ‚Uhr zurück‘“ ist aus dem Code hergeleitet (`merken`, `abgelaufene_entfernen`, `entfernen`). Liegt die Probe des code-reviewer vor, ersetzt sie dieses Kriterium wörtlich.

## Manuelle Vorbereitungstätigkeiten

Keine.

## Manuelle Nachbereitungstätigkeiten

- Designsystem-README `Statuszeile` bzw. Verbindungsleiste: Satz „bei schmaler Breite kürzt der Anlass, nie der Stand“ (Entwickler oder ux-reviewer).

## Umfang

Gezählt aus der WBS (Bubbles); 0,4 h je klare Bubble ist die Untergrenze nach dem Skill `work-breakdown-structure` (Hypothese, ungemessen für diesen Typ); unklar als Spanne.

- F0124: 5 Bubbles · 4 klar (1,6 h) · 1 unklar (B0550, 2–4 h: Messung der Textbreite je Teil in drei Viewports, Layout mit zwei Teilen in einer einzeiligen Leiste)
- Zusammen: 5 Bubbles · 1,6 h klar + 2–4 h unklar

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: die Befunde ux-reviewer #36 S1 (275 > 248 px), S3 (Wartezeile stumm), code-reviewer #36 A2 (Wächter ungetestet), rechtschreib-reviewer #36.

- Der Stand schrumpft nicht, der Anlass schon. Deshalb sieht die Mitleserin am kleinsten Telefon, wann der nächste Versuch kommt, und das Warnsymbol mit dem Anfang des Anlasses bleibt stehen.
- Die Wartezeile beschreibt das Namensfeld. Deshalb hört eine Bildschirmleser-Nutzerin beim Fokus „… warten auf den Beitritt — Neuladen verwirft sie“ und tritt bei, statt neu zu laden und Nachrichten zu verlieren.
- Kein Punkt nach einem Satzzeichen. Deshalb liest der Bildschirmleser keine „Punkt Punkt“-Pause und kein Satzzeichen-Rauschen.
- Ein Test mit Mutante für den Wächter. Deshalb fällt sein Verschwinden im Gate auf statt als doppelte Nachricht nach einer Zeitumstellung.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).
- Abnahme-Probe code-reviewer #36 A2 (Bericht) — siehe Offene Fragen.

## Notizen

- Präzisiert R00025 F0084 „Ansage“ („mit „. “ verbunden“): nach einem Text mit Satzzeichen am Ende nur ein Leerzeichen. R00025 bleibt an dieser Stelle; R00033 nennt die Änderung.

### Bewusst außerhalb (Rest von Karte #50)

- ux S2 (Leiste verdeckt „Links einen Channel wählen.“ ohne offenen Channel, 52 < 68 px) — braucht den architekt (Ort des Hinweises, ADR-0038).
- obs S1 (`wiederholung.rs:224–259` „voll, aber kein Kandidat“ still → EXCEPTION nach ADR-0042 Regel 4) — die Meldung läuft über `src/gespraeche/mod.rs` und `src/start/mod.rs`, die #9 gerade ändert; dazu Katalogzeile (Text, Bereich) offen.
- srev: Mutant `cfg!(debug_assertions)` überlebt — Release-Test `--ignored`.
- ux O1/O2, crev O1–O4, obs O1–O3, Sekundentakt bis Neubeitritt.
- rsr: R00025 Z. 120 Ziffern („5 000 ms“, „4 001 ms“) — Text einer Anforderung, nur über `/anforderung review`; Optional „vergeben an zeigen“, „640-jüngste“.
- etst: Konvention `setOffline` in den Stories nachziehen; bei `online` sofort ein Verbindungsversuch (`web/hauptansicht.js:248`, Ausbau).
