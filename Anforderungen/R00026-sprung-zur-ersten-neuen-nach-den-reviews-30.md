---
id: R00026
status: Neu
datum: 2026-10-29
---

# R00026: Sprung zur ersten neuen nach den Reviews #30

## Beschreibung

Der Sprung zur ersten neuen Nachricht aus R00020 und das Folgen am Ende aus R00011 schließen die Lücken, die die Auto-Reviews zu #30 gefunden haben und die nicht Muss waren. Danach gilt:

- **Folgen und Sprung sind belegt**: wer am Ende mitliest, sieht schnell nacheinander eintreffende Nachrichten desselben Absenders gruppiert, ohne eigenen Kopf. Der Sprung zur ersten neuen trifft sein Ziel auch dann, wenn vor ihr keine Nachricht im Verlauf steht. Beides prüft je ein Test, dessen Mutante rot wird.
- **Der Auslauf verschwindet mit dem Lesen**: hat die Mitleserin nach dem Sprung bis zum Ende des Inhalts gelesen, liegt unter dem Fenster kein leerer Raum mehr, und der Scrollbalken zeigt das Ende des Inhalts. Die letzte Nachricht bewegt sich dabei nur mit ihren eigenen Schritten.
- **Die Schreibweisen aus dem Rechtschreib-Review #30 sind nachgezogen**: kein unsichtbares Zeichen in `Dokumentation/Gelerntes.md`, der Platz der fehlgeschlagenen Nachricht im Designsystem heißt „am Ende des Verlaufs“, „Bildschirmleser“ und „Mutante“ stehen einheitlich.

Zahlt ein auf: [Vision](R00000-vision.md) — „Live überall“ (wer weiter oben liest, findet nach dem Sprung, wer schreibt, und der Verlauf zeigt ehrlich, wo das Gespräch endet). Kein Widerspruch zur Vision.

Herkunft: Karte #30 (R00020 Teil 1, I0009), Auto-Reviews und Entwicklertest vom 2026-10-29 — Code-Review Sollte A1 und Optional „Rückfallzweig“ aus der engen Wiederholung; UX-Review Optional O1 mit dem Nachtrag des Directors aus der UX-Wiederholung #8; Rechtschreib-Review Sollte und Begriffe. Was nicht hier steht und warum: Abschnitt „Notizen“.

Slices (WBS `Dokumentation/Planung/localchata.md`, beide unter I0009 Nachricht senden): F0089 Folgen und Sprung mit Tests belegt, F0090 Auslauf verschwindet mit dem Lesen.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00011 und R00020: Pille, erste neue, Sprung, klebender Tagestrenner, Kopf und Gruppierung (Designsystem `Nachricht`, R00007 F0030), Folgen, Ende des Inhalts, Auslauf (ADR-0037 Ergänzung „Auslauf unter dem Verlauf“).

## Geschäftlicher Nutzen

Die Reviews zu #30 zeigen zwei Stellen, an denen heute nichts falsch läuft, ein Rückschritt aber unbemerkt bliebe: der Wächter, der nur gezählten Nachrichten den Kopf der „ersten neuen“ gibt, und der Sprung, wenn vor der ersten neuen keine Nachricht steht. Fiele der Wächter weg, bekäme jede mitgelesene Nachricht am Ende ihren eigenen Kopf — der Verlauf würde unruhig, und kein Test schlüge an. Und wer nach dem Sprung mit `↓` bis ans Ende liest, sieht darunter bis zu 84 % der Sichthöhe leeren Raum (579 bzw. 591 px, gemessen im UX-Review #8); der Scrollbalken verspricht Inhalt, den es nicht gibt.

Mit R00026 sind beide Verhalten gegen Rückschritte geschützt, und das Ende des Verlaufs sieht nach dem Lesen aus wie ein Ende.

## Funktionale Anforderungen

### Folgen und Sprung mit Tests belegt (F0089; Code-Review #30 Sollte A1 und Optional)

Die Zusagen stehen in R00007 F0030 (Gruppierung) und R00020 F0067 (Sprung) und halten; neu sind die Belege und der ausdrücklich genannte Fall „ohne Nachricht davor“.

- **Folgen gruppiert wie bisher**: steht der Blick am Ende, folgt er neuen Nachrichten (R00011). Eine neue Nachricht, die so angefügt wird, ist keine „erste neue“ — sie trägt ihren Kopf nur nach der Gruppierung (gleicher Absender wie die vorige, höchstens 5 Minuten, kein Trenner → ohne Kopf). Die Marke „erste neue“ bekommt nur eine Nachricht, die gezählt wird.
- **Sprung ohne Nachricht davor**: steht vor der ersten neuen keine Nachricht im Verlauf, kommt die Oberkante ihres Platzes (mit ihrem Abstand nach oben) eine Zeilenhöhe unter die Unterkante des klebenden Tagestrenners bzw. ohne klebenden Trenner unter den oberen Rand des Verlaufs. Mit Nachricht davor gilt R00020 F0067 unverändert.

### Auslauf verschwindet mit dem Lesen (F0090; UX-Review #30 O1, Nachtrag des Directors zu #8)

- **Unter dem Fenster fällt er weg**: der Teil des Auslaufs unterhalb des Fensters fällt weg, wenn die Mitleserin nach oben scrollt (ADR-0037 Ergänzung „Auslauf“ Punkt 4). Der sichtbare Teil bleibt; der sichtbare Inhalt bewegt sich dabei nicht.
- **Am Ende des Inhalts** (vorbehaltlich der Entscheidung des architekt, „Offene Fragen“ 1): ist nach dem Sprung das Ende des Inhalts sichtbar, liegt auch beim Scrollen nach unten kein Auslauf mehr unter dem Fenster; was davon sichtbar ist, bleibt als Raum für neue Nachrichten und wird von ihnen eingenommen (Punkt 4, erster Spiegelstrich).
- Unverändert: der Auslauf entsteht nur beim Sprung (Punkt 2), fällt beim Öffnen, Neuöffnen und Channelwechsel weg (Punkt 5) und reißt nie eine Lesestelle mit (R00011 „Lesestelle nach dem Sprung“).

### Schreibweisen nach dem Rechtschreib-Review #30 (F0089, Review-Punkte, kein Verhalten)

- `Dokumentation/Gelerntes.md` Zeile 37 enthält kein U+FEFF mehr (das Zeichen steht dort als Beispiel selbst im Text; geschrieben wird es als `U+FEFF`).
- Designsystem `components/Nachricht/README.md` Zeile 9: „Eine fehlgeschlagene Nachricht bleibt am Ende des Verlaufs, unter allen bestätigten, bis „Erneut senden“ oder „Verwerfen“ gewählt wird.“ Die gleichlautende Stelle in ADR-0037 (Ergänzung Karte #31, „Folgen“) zieht der architekt nach.
- `e2e/tests/r00020-zusagen.spec.js` Zeile 89: Zitat im Zitat mit einfachen Anführungszeichen („A sendet ‚x‘, …“).
- „Bildschirmleser“ statt „Screenreader“ in Kommentaren unter `web/` (gezählt 2026-10-29: 4 Stellen in `web/hauptansicht.js`, `web/lc-seite.css`, `web/teilnehmerliste.js`).
- „Mutante“ (Mehrzahl „Mutanten“) statt „Mutant“ in Code, Tests und `Dokumentation/Gelerntes.md` — wie in den Akzeptanzkriterien aller Anforderungen.

## Nicht-funktionale Anforderungen

- **Benutzerfreundlichkeit**: der Scrollbalken zeigt nach dem Lesen das Ende des Inhalts; Lesende werden nie aus ihrer Stelle gerissen (R00011, gilt weiter).
- **Zeitkriterien** nach `AI-Team/planer.md`: neue Zeitgrenzen gibt es nicht.
- **Konsole**: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile (die Szenarien rufen keine Fehlerantwort hervor).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, `S(<name>)`, `POST`, Browser A und B, Testdaten-Erzeuger und Grenzen je Kennung wie in R00011; „die Pille“ und „klebender Tagestrenner“ wie in R00020. „Abstand unter dem Fenster“ ist `scrollHeight − scrollTop − clientHeight` des Verlaufs.

### F0089 Folgen und Sprung mit Tests belegt

- [ ] **Folgen ohne „erste neue“** (E2E; Abnahme-Probe aus dem Code-Review #30, Sollte A1, wörtlich übernommen): „B am Ende, beta „a“, dann „b“, „c“ in getrennten Frames < 1 min → „b“, „c“ ohne .lc-meta.“ Ausgeschrieben: `allgemein` mit 300 Nachrichten; `alpha` sendet `x`; B steht am Ende; `beta` sendet `a`, dann `b`, dann `c`, jede erst, nachdem die vorige in B gezeichnet ist (getrennte Frames), alle drei innerhalb einer Minute → `a` trägt den Kopf mit `beta@127.0.0.1`, `b` und `c` haben kein `.lc-meta`; der Abstand zum Ende des Inhalts ist danach ≤ 1 px, keine Pille. Die Mutante „Wächter `lage === 'zaehlen'` entfernt“ (heute `web/verlauf.js:315`) wird rot.
- [ ] **Sprung ohne Nachricht davor** (Node, rein; Code-Review #30 Optional „Rückfallzweig“; die Form der reinen Funktion entscheidet der Entwickler): Ziel des Sprungs aus Scrollstand, Lage der ersten neuen, Lage der Nachricht davor (oder keiner), Unterkante des klebenden Trenners und Zeilenhöhe, alle in px vom oberen Rand des Verlaufs. Rechenbeispiel ohne Nachricht davor: Scrollstand 1.000, der Platz der ersten neuen beginnt (mit Abstand nach oben) 400 px unter dem oberen Rand, der klebende Trenner endet bei 20 px, Zeilenhöhe 20 px → Ziel 1.000 + 400 − 20 − 20 = 1.360. Ohne klebenden Trenner (Unterkante 0) → 1.380. Mit Nachricht davor, deren Unterkante bei 380 px liegt → 1.000 + 380 − 20 − 20 = 1.340 (R00020 F0067). Die Mutante „Zeilenhöhe im Rückfall nicht abgezogen“ wird rot.
- [ ] **Bestand bleibt grün** (E2E): „Kontext unter dem Trenner“, „Erste neue mit Kopf“, „Zweiter Klick ans Ende“ (R00020 F0067) und „Folgen knapp über dem Ende“ (R00020 F0071) laufen unverändert.
- [ ] **Schreibweisen** (Review-Punkt für `rechtschreib-reviewer`; Probe aus dem Rechtschreib-Review #30 wörtlich: „git grep -c -P '\x{FEFF}' Dokumentation/Gelerntes.md = 0“): `git grep -c -P '\x{FEFF}' Dokumentation/Gelerntes.md` findet nichts; `grep -n 'unter dem Verlauf' Dokumentation/Wireframes/designsystem/components/Nachricht/README.md` findet nichts und Zeile 9 trägt den Satz aus „Funktionale Anforderungen“; `git grep -n 'Screenreader' -- web e2e src tests` findet nichts; `git grep -nw 'Mutant' -- web e2e src tests Dokumentation/Gelerntes.md` findet nichts; `e2e/tests/r00020-zusagen.spec.js` Zeile 89 setzt das innere Zitat in einfache Anführungszeichen (‚x‘).
- [ ] Konsole bei B: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile.

### F0090 Auslauf verschwindet mit dem Lesen

- [ ] **Hochscrollen nimmt den Auslauf unter dem Fenster weg** (E2E, bei 1280 × 800 und 375 × 812; Abnahme-Probe aus dem UX-Review #30, O1, wörtlich übernommen): „… ↓ bis Pille verborgen, dann 2× ↑ → scrollHeight−scrollTop−clientHeight ≤ 1, letzte Nachricht bewegt sich ≤ 1 px“. Ausgeschrieben mit dem Gegeben aus R00011 „Lesestelle nach dem Sprung“: `allgemein` mit 300 Nachrichten; B steht 2 Bildschirmhöhen über dem Ende; `beta` sendet 3; Klick auf die Pille; `↓`, bis die Pille verborgen ist; dann 2 × `↑` → der Abstand unter dem Fenster ist ≤ 1 px; je `↑` bewegt sich die letzte Nachricht um die Scrollweite dieses Schritts nach unten, Abweichung ≤ 1 px (Lesart planer: „bewegt sich ≤ 1 px“ meint über den eigenen Schritt hinaus). Heute rot (der Auslauf fällt nur weg, wenn er ganz unter dem Fenster liegt, `web/live-verlauf.js:59–66`). Die Mutante „nur ganz unsichtbarer Auslauf fällt weg“ wird rot.
- [ ] **Am Ende des Inhalts** (E2E, bei 1280 × 800 und 375 × 812; Probe aus dem Nachtrag des Directors zu #30: „nach Sprung und ↓ ist der Auslauf ≤ eine Zeilenhöhe oder als Raum für neue Nachrichten erkennbar“; Form nach der Entscheidung des architekt, „Offene Fragen“ 1 — hier die Empfehlung): wie oben bis „`↓`, bis die Pille verborgen ist“ → der Abstand unter dem Fenster ist ≤ 1 px; die letzte Nachricht hat sich mit dem letzten `↓` um höchstens dessen Scrollweite + 1 px nach oben bewegt, nie nach unten; danach sendet `beta` eine 4. → sie steht unter der 3., die 3. bewegt sich um höchstens 1 px.
- [ ] **Bestand bleibt grün** (E2E): „Lesestelle nach dem Sprung“ (R00011, (a)–(c)) und „Zweiter Klick ans Ende“ (R00020) laufen unverändert; der Node-Test zu `auslaufNach` (ADR-0037 Ergänzung „Auslauf“, „Erster Test“) wird um die neuen Fälle ergänzt, die bestehenden Fälle gelten weiter.
- [ ] Konsole bei B: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile.

## Betroffene Verzeichnisstruktur

- Oberfläche: `web/verlauf.js` (`nachrichtenEinfuegen`, `sprungZiel`, Scrollen), `web/live-verlauf.js` (`auslaufNach`); Tests `e2e/einheit/` (Sprungziel, `auslaufNach`), `e2e/tests/r00020-sprung.spec.js` bzw. neu `e2e/tests/r00026-*.spec.js`.
- Schreibweisen: `Dokumentation/Gelerntes.md`, `Dokumentation/Wireframes/designsystem/components/Nachricht/README.md`, `e2e/tests/r00020-zusagen.spec.js`, Kommentare in `web/hauptansicht.js`, `web/lc-seite.css`, `web/teilnehmerliste.js`.

## Technische Überlegungen

### Ablauf

1. **Anfügen**: neue Nachrichten → Lage `folgen` | `zaehlen` | `nurZaehlen` → nur bei `zaehlen` wird die erste neue fremde zur „ersten neuen“ → Gliederung (Kopf nach Gruppierung, die erste neue immer mit Kopf).
2. **Sprung**: erste neue im Fenster → Nachricht davor? → ja: Unterkante des Kontexts eine Zeile unter den Trenner \| nein: Oberkante des Platzes der ersten neuen eine Zeile unter den Trenner → Auslauf → scrollen.
3. **Scrollen** (F0090): Scrollereignis → Teil des Auslaufs unter dem Fenster messen → nach oben gescrollt (bzw. nach der Entscheidung: Ende des Inhalts sichtbar) → dieser Teil fällt weg → der sichtbare Teil bleibt.

Optionen: für F0090 liegen sie beim architekt (Offene Frage 1, zwei Teilfragen mit Empfehlung); F0089 hat keine Wahl — es belegt, was gebaut ist, und schreibt den Rückfall so fest, wie er gebaut ist.

### Grobentwurf (Vorplanung)

Noch keine Bubbles (Rolling Wave; `/planung verfeinern F0089 F0090 --bis Bubble`, wenn die Karte dran ist, F0090 nach der Entscheidung des architekt). Elemente: eine reine Funktion für das Ziel des Sprungs (heute `sprungZiel` im Verlauf, mit DOM-Messung) — Name und Schnitt entscheidet der Entwickler; `auslaufNach` erhält, was es für „nach oben gescrollt“ bzw. „Ende des Inhalts sichtbar“ braucht.

Wireframe: kein Artboard `D0003.dc.html` (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle: Designsystem `components/VerlaufsMarken/README.md`, `components/Nachricht/README.md` — Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `web/verlauf.js`: Ziel des Sprungs als reine Funktion herausgelöst (falls der Entwickler den Node-Weg wählt); Scrollen gibt den Teil des Auslaufs unter dem Fenster frei.
- `web/live-verlauf.js` `auslaufNach`: neue Fälle nach ADR-0037 Ergänzung „Auslauf“ (nach der Entscheidung des architekt).

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Node (rein)**: Ziel des Sprungs mit und ohne Nachricht davor, mit und ohne klebenden Trenner; `auslaufNach` mit den neuen Fällen.
- **E2E**: Folgen ohne „erste neue“; Hochscrollen und Ende des Inhalts bei zwei Fenstergrößen; Bestand aus R00011 und R00020.
- **Review-Punkt**: die `grep`-Proben der Schreibweisen.

## Abhängigkeiten

- Abhängig von: F0067 Sprung zur ersten neuen Nachricht lesbar (gruen, R00020), F0071 Zusagen aus R00011 mit Tests belegt (gruen, R00020), F0044 Fremde Nachrichten im offenen Verlauf (gruen, R00011; Auslauf mit B0329). F0090 zusätzlich: Ergänzung des architekt zu ADR-0037 („Offene Fragen“ 1).
- Innerhalb: F0089 und F0090 parallel baubar (beide in `web/verlauf.js`, verschiedene Stellen).
- Blockiert: nichts.

## Offene Fragen

1. **An den architekt, blockiert F0090** (ADR-0037 Ergänzung „Auslauf unter dem Verlauf“): (a) Fällt der Teil unterhalb des Fensters beim Hochscrollen weg, wie Punkt 4 sagt (gebaut ist „nur, wenn er ganz unsichtbar ist“; nach der Ergänzung selbst würde bei grünen Kriterien die ADR angeglichen, nicht der Bau)? Empfehlung planer: ja, wie Punkt 4. (b) Was geschieht, wenn nach dem Sprung das Ende des Inhalts erreicht ist — bleibt der Teil unter dem Fenster, bis hochgescrollt wird, oder fällt er auch beim Scrollen nach unten weg? Empfehlung planer: er fällt weg, sobald das Ende des Inhalts sichtbar ist; der sichtbare Teil bleibt als Raum für neue Nachrichten. Zu prüfen: Verträglichkeit mit Punkt 2 („eine Zeile mehr, damit der Blick dabei nicht schon ‚am Ende‘ gilt“) und Punkt 3 („am Ende“ meint das Ende des Inhalts). Das Kriterium „Am Ende des Inhalts“ folgt der Entscheidung.
2. **An den Director, nicht blockierend**: Code-Review #30 Optional A3–A5 sind auf der Karte nicht ausgeschrieben und deshalb nicht eingeplant; für den Rückfallzweig gibt es keine Abnahme-Probe — das Kriterium „Sprung ohne Nachricht davor“ ist vom planer aus dem Code abgeleitet.

## Manuelle Vorbereitungstätigkeiten

- Für F0090: die Ergänzung des architekt zu ADR-0037 (Offene Frage 1).

## Manuelle Nachbereitungstätigkeiten

- architekt: ADR-0037 Ergänzung Karte #31, „Folgen“ — „bleibt unter dem Verlauf“ → „bleibt am Ende des Verlaufs“ wie im Designsystem.

## Umfang

Gezählt aus der WBS (2026-10-29):

- F0089 Folgen und Sprung mit Tests belegt: nicht bis Bubble geplant.
- F0090 Auslauf verschwindet mit dem Lesen: nicht bis Bubble geplant.
- Hypothese zur Ordnung, nicht gezählt: F0089 etwa 3 Bubbles (Folgen belegt, Rückfall belegt, Schreibweisen), F0090 etwa 2 (Abbau beim Hochscrollen, Abbau am Ende des Inhalts); zur Untergrenze 0,4 h je Bubble nach dem Skill `work-breakdown-structure` rund 2 h. Richtwert, ungemessen.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser sind die Befunde der Auto-Reviews zu #30, die nicht Muss waren: Code-Review A1 und Optional „Rückfallzweig“, UX-Review O1 mit dem Nachtrag des Directors, Rechtschreib-Review Sollte und Begriffe.

Die Wirkungskette:

- Ein Test fährt das Folgen am Ende mit drei schnell eintreffenden Nachrichten, und die Mutante „Wächter entfernt“ wird rot. Deshalb fällt ein Rückschritt, der jeder mitgelesenen Nachricht einen Kopf gibt, im Gate auf statt bei der Mitleserin.
- Das Ziel des Sprungs wird für den Fall ohne Nachricht davor festgeschrieben und gerechnet. Deshalb hat der Rückfallzweig eine Zusage und einen Beleg statt nur eines Kommentars.
- Der Teil des Auslaufs unter dem Fenster fällt beim Lesen weg, ohne den sichtbaren Inhalt zu bewegen. Deshalb zeigt der Scrollbalken nach dem Lesen das Ende des Inhalts, und die Mitleserin sucht nicht nach Nachrichten, die es nicht gibt.
- Die Schreibweisen werden mit `grep`-Proben nachgezogen. Deshalb sieht der nächste Review dieselben Funde nicht wieder.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus R00007, R00011, R00020 und dem Designsystem.

## Notizen

- **Nicht eingeplant, mit Grund** (Karte #30, Zuordnung in der WBS unter „Offene Fragen“):
  - UX-Review #30 O2 und Entwicklertest #8 Anmerkung 2 (Pille nennt nach dem Sprung weiter „3 neue“): so zugesagt — ADR-0037 Regel 6, R00011 „die Zahl gilt, bis der Blick das Ende erreicht“, entschieden in R00020.
  - Security-Review #30 Optional „unsichtbare Nachrichten aus Formatzeichen“: entschieden in ADR-0038 Ergänzung Karte #31 und belegt in R00020 F0071 („Leerraum wie beim Server“).
  - Security-Review #30 Optional „`bestaetigt` hält nur die Zeit ohne Channel“: ADR-0035 Regel 2 vergibt jede Zeit eindeutig über alle Channels; der Fall „gleiche ms in zwei Channels“ entsteht im Tab nicht.
  - Entwicklertest #8 Anmerkung 3 (Neuladen, während eine Nachricht unterwegs ist): bewusst hingenommen — R00011 F0043, ADR-0037 Regel 5 und Konsequenzen, R00012 US-9; Frage an den Betreiber, ob der Tab vorher warnen soll (WBS „Offene Fragen“, Karte #30, Punkt 8).
  - Code-Review #30 A2 (Wortlaut „Oberkante“) und der Testzettel „kein Schreibrecht“: Wortlaut in R00020 bzw. R00011 per `/anforderung review` nachgezogen, kein Umfang.
  - Rechtschreib-Review #30 Optional (Testnamen „bestätigen“, „die Taste Ende“) nicht aufgenommen; „Mutant“ in R00011, R00013, R00018 und ADR-0029 bleibt (Anforderungen werden nicht umgeschrieben, ADRs gehören dem architekt).
