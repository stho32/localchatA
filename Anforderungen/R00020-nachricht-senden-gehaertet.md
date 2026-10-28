---
id: R00020
status: Neu
datum: 2026-10-28
---

# R00020: Nachricht senden gehärtet

## Beschreibung

Das Senden aus R00011 und seine Spur im Betriebsprotokoll aus R00016 schließen die Lücken, die die Auto-Reviews zu #8 gefunden haben und die nicht Muss waren. Danach gilt:

- **Der Sprung zur ersten neuen Nachricht ist lesbar**: nach Klick auf die Pille oder `Ende` steht die erste neue Nachricht mit Kopf (Absender und Zeit) unter dem klebenden Tagestrenner, und die Zeile Kontext darüber ist zu sehen. Ein weiterer Klick oder `Ende` bringt den Blick ans Ende.
- **Die Pille ist beschriftet wie im Designsystem**: „↓ 3 neue Nachrichten“ mit Abstand vor der Taste `Ende`. Ein Bildschirmleser liest nur „3 neue Nachrichten“ vor, die Taste nennt `aria-keyshortcuts`.
- **Der zu große Körper spricht wie die Oberfläche**: die 413 der Senderoute sagt „höchstens 131.072 Byte; längere Texte teilen.“ Alle JSON-Routen bilden ihre Körperfehler über eine gemeinsame Antwort.
- **Die Bremse zählt je Grenze**: die Ende-Zeile einer Schreibkonto-Episode nennt, wie oft die Kennung an ihrem eigenen Eimer und wie oft am Eimer ihrer Adresse scheiterte. Bremse und Vorlauf stehen ohne doppelte Satzzeichen im Protokoll.
- **Zusagen aus R00011 sind belegt**: dass nur ein Ereignis desselben Channels eine eigene Nachricht bestätigt, dass der Blick knapp über dem Ende folgt und dass Markup im Text als Text erscheint, prüft je ein Test, dessen Mutante rot wird.

Zahlt ein auf: [Vision](R00000-vision.md) — „Live überall“ (wer weiter oben liest, findet die neuen Nachrichten mit Absender und verliert seine Stelle nicht), „Mensch und KI sind gleichberechtigt“ (Agent und Mensch lesen Grenzen in derselben Schreibweise) und „eine gemeinsame LAN-Instanz“ (die Betreiberin sieht, an welcher Grenze ein Rechner flutet). Kein Widerspruch zur Vision.

Herkunft: Karte #8 (I0009), Auto-Reviews vom 2026-10-28 — UX-Review Sollte S2, S3, S4 und Optional O2; Rechtschreib-Review Sollte; Code-Review Sollte A1, A2, A3; Observability-Review Sollte S1; Security-Review Optional 5. Schnitt des Directors: die Muss-Befunde (Code-Review B1–B3) und UX S1 sind Nacharbeit #8 an R00011 (B0326–B0329), der Rest ist diese Anforderung. UX S5 ist offen (siehe Offene Fragen).

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0009 Nachricht senden): F0067 Sprung zur ersten neuen Nachricht lesbar, F0068 Pille wie im Designsystem beschriftet, F0069 Körperfehler des Sendens in einer Schreibweise, F0070 Senden im Betriebsprotokoll je Grenze, F0071 Zusagen aus R00011 mit Tests belegt.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00011 und R00016: Pille, erste neue, Sprung, klebender Tagestrenner, Kopf und Gruppierung (Designsystem `Nachricht`), vorläufig, bestätigt, Schreibkonto, Eimer, Grenze (`kennung` | `herkunft`), Episode, Vorlauf.

## Geschäftlicher Nutzen

Die Reviews zu #8 zeigen, wo das Senden an seinen Rändern stolpert. Wer weiter oben liest und auf „3 neue Nachrichten“ klickt, sieht die erste neue Nachricht oft ohne Absender, weil sie mit der Nachricht davor gruppiert ist, und die Zeile Kontext darüber liegt unter dem klebenden Tagestrenner. Ein zweiter Klick auf die Pille tut nichts. Ein Bildschirmleser liest „3 neue NachrichtenEnde“ vor. Ein Agent, der einen zu großen Körper schickt, bekommt die Grenze als „131072 Bytes“ mit einem Beispiel, das ihm nicht hilft, während die Oberfläche „16.384 Byte“ schreibt. Flutet ein ganzer Rechner, zeigt die Ende-Zeile der Episode nur die Grenze vom ersten Auftreten — die Betreiberin sieht nicht, dass ihr Agent an der Adresse und nicht an seinem eigenen Eimer scheiterte. Und drei Zusagen aus R00011 halten heute, aber kein Test würde es merken, wenn sie brechen.

Mit R00020 findet die Mitleserin nach dem Sprung, wer schreibt und worauf geantwortet wird, und kommt mit einem zweiten Klick ans Ende. Der Agent liest die Grenze so wie der Mensch. Die Betreiberin misst die Bremse je Grenze, und die Zusagen sind gegen Rückschritte geschützt.

## Funktionale Anforderungen

### Sprung zur ersten neuen Nachricht lesbar (F0067; UX-Review #8 S2, S3, S4)

- **Kontext unter dem Trenner**: der Sprung zur ersten neuen (Klick auf die Pille oder `Ende`, R00011 F0044) setzt ihre Oberkante eine Zeilenhöhe unter die Unterkante des klebenden Tagestrenners. Ohne klebenden Trenner gilt wie bisher eine Zeilenhöhe unter dem oberen Rand des Verlaufs. Die Zeile Kontext darüber ist nicht vom Trenner verdeckt.
- **Erste neue mit Kopf**: die erste gezählte neue Nachricht trägt immer ihren Kopf mit Absender und Uhrzeit, auch wenn die Gruppierung (gleicher Absender, weniger als 5 Minuten, kein Trenner, Designsystem `Nachricht`) ihn weglassen würde. Die folgenden neuen gruppieren wie bisher.
- **Zweiter Klick ans Ende**: steht die erste neue schon am Ziel des Sprungs (± 1 px), bringen Klick auf die Pille oder `Ende` den Blick ans Ende des Inhalts. Steht der neueste Block nicht im Fenster, öffnet der Tab neu wie „Zum Neuesten“. Am Ende verschwindet die Pille. Die Zahl gilt weiter, bis der Blick das Ende erreicht (R00011).

### Pille wie im Designsystem beschriftet (F0068; Rechtschreib-Review #8 Sollte, UX-Review #8 O2)

- Sichtbar steht „↓ n neue Nachrichten“ (Einzahl „↓ 1 neue Nachricht“) bzw. „↓ Zum Neuesten“, danach mit Leerraum die Taste `Ende` als `<kbd>`.
- Der zugängliche Name ist genau „n neue Nachrichten“ bzw. „Zum Neuesten“: Pfeil und `<kbd>` sind für Bildschirmleser verborgen. Die Taste nennt weiter `aria-keyshortcuts="End"`.

### Körperfehler des Sendens in einer Schreibweise (F0069; Rechtschreib-Review #8 Sollte, Code-Review #8 A3)

- **413 der Senderoute**: ein Körper über 131 072 Bytes ergibt 413 `{"fehler":"Der Körper ist zu groß — höchstens 131.072 Byte; längere Texte teilen."}`. Tausenderpunkt und „Byte“ stehen wie in der 422 `laenge`, ohne Beispiel. Prüfreihenfolge, Grenze und `Connection: close` bleiben wie in R00011.
- **Eine Antwort für Körperfehler**: Beitreten, Anlegen, Frist und Senden bilden 413, 415, 400 und 408 über dieselbe Antwort. Jede Route gibt ihren Satz zur Grenze und ihr Beispiel hinein. Die Texte von Beitreten, Anlegen und Frist bleiben wörtlich (R00003, R00006, R00008, R00010).

### Senden im Betriebsprotokoll je Grenze (F0070; Observability-Review #8 S1, Rechtschreib-Review #8 Sollte)

- **Ende-Zeile je Grenze**: die Schreibkonto-Episode einer Kennung zählt ihre 429 je Grenze. Die Ende-Zeile lautet `… — Episode endet: <n> in <d> s (herkunft <h>; kennung <k>)` mit `h + k = n`. Ein Teil mit 0 entfällt, die Reihenfolge ist alphabetisch wie bei den Körperfehlern (R00014). Die Beginn-Zeile bleibt mit `grenze=` des ersten Auftretens.
- **Text der Bremse**: „Senden gebremst: zu viele Nachrichten“. Die Zeile hat damit genau einen Gedankenstrich, den der Episode: `WARNING schreibkonto Senden gebremst: zu viele Nachrichten — Episode beginnt | kennung=… grenze=…`.
- **Text des Vorlaufs** ohne Punkt am Ende: „Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt“. Die Episode hängt „ — Episode …“ an.
- Kommentar in `src/schreibkonto/mod.rs`: „Bei mehr als 1.024 Einträgen …“ statt „Über 1.024 Einträgen …“ (Review-Punkt, kein Verhalten).

### Zusagen aus R00011 mit Tests belegt (F0071; Code-Review #8 A1, A2, Security-Review #8 Optional 5)

Die Zusagen stehen in R00011 und halten; neu ist der Beleg.

- **Bestätigen nur im eigenen Channel**: ein Ereignis der eigenen Kennung mit dem Text der laufenden vorläufigen, aber aus einem anderen Channel, bestätigt nicht. Die Nachricht geht als fremde weiter, die vorläufige bleibt.
- **Folgen knapp über dem Ende**: wer höchstens eine Zeilenhöhe über dem Ende steht, folgt einer neuen Nachricht im echten Browser. Wer weiter oben steht, folgt nicht.
- **Markup bleibt Text**: ein Text mit HTML erscheint wörtlich. Kein Element aus dem Text entsteht, kein Skript läuft. Das muss auch nach I0017 (Markdown) gelten.

## Nicht-funktionale Anforderungen

- **Benutzerfreundlichkeit**: nach dem Sprung sind Absender und Kontext der ersten neuen sichtbar, und die Pille führt immer weiter. Ein Bildschirmleser hört den Namen der Pille ohne Tastenbezeichnung. Lesende werden nie aus ihrer Stelle gerissen (R00011, Nacharbeit #8 gilt weiter).
- **Sicherheit** (CWE-79): Text erscheint nur über `textContent` (R00011); der Beleg ist ein Test mit Mutante. Die gemeinsame Antwort für Körperfehler ändert keine Grenze und keine Prüfreihenfolge.
- **Betrieb**: die Ende-Zeile je Grenze ist die Messung, nach der das Schreibkonto je Herkunft ausgelegt wird (Entscheidung des Directors 2026-10-28, R00016).
- **Zeitkriterien** nach `AI-Team/planer.md`: neue Zeitgrenzen gibt es nicht. `LOCALCHATA_EPISODE_RUHE_MS` steht wie in R00016 nur als Testknopf.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, `S(<name>)`, `POST`, Browser A und B, Testdaten-Erzeuger und Grenzen je Kennung wie in R00011. „Die Pille“ ist der Schalter unten mittig im Verlauf von B. „Klebender Tagestrenner“ ist das `.lc-day`, das am oberen Rand des Verlaufs steht. `heute.log` ist `logs/<heute>.log`. „Die Zeile `X`“ meint die Zeile ab der Stufe, ohne Zeitstempel.

### F0067 Sprung zur ersten neuen Nachricht lesbar

- [ ] **Kontext unter dem Trenner** (E2E, bei 1280 × 800 und 375 × 812): `allgemein` mit 300 Nachrichten; B steht 2 Bildschirmhöhen über dem Ende; `beta` sendet 3; Klick auf die Pille → die Oberkante der ersten der drei minus die Unterkante des klebenden Tagestrenners liegt zwischen Zeilenhöhe − 1 px und Zeilenhöhe + 1 px; `document.elementFromPoint` eine halbe Zeilenhöhe über der Oberkante der ersten neuen trifft eine Nachricht, nicht den Tagestrenner. Die Mutante „Trennerhöhe nicht abgezogen“ wird rot.
- [ ] **Erste neue mit Kopf** (Node und E2E): Node — Gliederung einer Folge, deren erste Nachricht als „erste neue“ markiert ist, vom selben Absender 1 min nach der vorigen → Kopf ja; die zweite → Kopf nein. E2E — `beta` sendet 1; B scrollt 2 Bildschirmhöhen nach oben; `beta` sendet 3 innerhalb einer Minute; Klick auf die Pille → die erste der drei zeigt `beta@127.0.0.1` und ihre Uhrzeit im Kopf, die zweite und dritte keinen Kopf. Die Mutante „erste neue gruppiert“ wird rot.
- [ ] **Zweiter Klick ans Ende** (E2E): wie „Kontext unter dem Trenner“, nach dem Klick ein zweiter Klick auf die Pille → der Abstand zum Ende des Inhalts ist ≤ 1 px, die Pille ist verborgen, alle drei neuen sind sichtbar. Dasselbe mit `Ende` im Verlauf statt Klick. Die Mutante „zweiter Klick springt erneut zur ersten neuen“ wird rot.
- [ ] **Lesestelle** (E2E): das Kriterium „Lesestelle nach dem Sprung“ aus R00011 (Nacharbeit #8) bleibt mit dem neuen Ziel des Sprungs grün.
- [ ] Konsole bei B: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler.

### F0068 Pille wie im Designsystem beschriftet

- [ ] **Beschriftung** (E2E): `allgemein` mit 300 Nachrichten; B steht 2 Bildschirmhöhen über dem Ende; `beta` sendet 3 → `getByRole('button', {name: '3 neue Nachrichten', exact: true})` ist sichtbar; der sichtbare Text der Pille (`innerText`) passt auf `/^↓ 3 neue Nachrichten\s+Ende$/`; `aria-keyshortcuts` ist `End`. Ohne neue Nachrichten, mehr als eine Bildschirmhöhe über dem Ende: `getByRole('button', {name: 'Zum Neuesten', exact: true})`, Text passt auf `/^↓ Zum Neuesten\s+Ende$/`. Einzahl: nach 1 neuen `{name: '1 neue Nachricht', exact: true}`. Die Mutante „`<kbd>` ohne `aria-hidden`“ wird rot.
- [ ] Konsole bei B: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler.

### F0069 Körperfehler des Sendens in einer Schreibweise

- [ ] **413 der Senderoute** (Integration; Text aus dem Rechtschreib-Review #8 wörtlich): `beta` sendet einen Körper von 131 073 Bytes (`{` und 131 072 × `a`) → 413 mit `{"fehler":"Der Körper ist zu groß — höchstens 131.072 Byte; längere Texte teilen."}` und `Connection: close`; mit `Content-Type: text/plain` → dieselbe 413. Ein gültiger Körper von genau 131 072 Bytes → 201 (Spanne aus R00011 unverändert). `grep -rn 'höchstens 131072 Bytes' src tests e2e` findet nichts. Die Mutante „alter Text“ wird rot.
- [ ] **Eine Antwort für Körperfehler** (Unit und Integration, Review-Punkt aus dem Code-Review #8 A3): eine Funktion bildet 413, 415, 400 und 408 aus Fehlerart, Satz zur Grenze und Beispiel (Unit je Fehlerart: Status, `fehler`, `Connection: close` nur bei 413 und 408); `grep -rn 'fn kein_json' src` findet nichts; die Integrationstests zu den Körperfehlern von Beitreten, Anlegen und Frist (R00003, R00006, R00008, R00010) laufen unverändert grün — ihre Texte, etwa „höchstens 4096 Bytes, nur Name und Beschreibung schicken, zum Beispiel …“, bleiben wörtlich.

### F0070 Senden im Betriebsprotokoll je Grenze

- [ ] **Ende-Zeile je Grenze** (Integration; Abnahme-Probe aus dem Observability-Review #8, S1, wörtlich übernommen): „RUHE_MS=5000, beta 35× schnell, dann alle 1,2 s, 8 andere Kennungen derselben IP fluten 4 s → Ende-Zeile für beta nennt herkunft k = Zahl der 429 mit grenze=herkunft.“ Ausgeschrieben: `LOCALCHATA_EPISODE_RUHE_MS=5000`; `beta` sendet 35 kleine Nachrichten so schnell wie möglich, danach alle 1,2 s eine; erst dann senden 8 andere Kennungen von `127.0.0.1` 4 s lang so schnell wie möglich; der Test zählt die 429 an `beta` je `grenze` (h mit `herkunft`, k mit `kennung`) → nach der Ruhe enthält `heute.log` für `beta` genau eine Zeile `WARNING schreibkonto Senden gebremst: zu viele Nachrichten — Episode endet: <h + k> in <d> s (herkunft <h>; kennung <k>) | kennung=beta@127.0.0.1 …`. Rechnung: die ersten 30 gehen durch (Vorrat je Kennung 30), die 31. bis 35. scheitern an `kennung` (der Eimer der Herkunft hat noch rund 90), also k ≥ 5; danach reicht der Eimer der Kennung bei 1 Nachricht je 1,2 s (Nachschub 1/s), und `beta` scheitert nur am Eimer der Herkunft, den die 8 Kennungen leeren (Nachschub 8/s) — bei rund 3 Nachrichten in den 4 s also h ≥ 1. Ist h = 0 oder k = 0, ist der Lauf ungültig (wiederholen, nicht rot). Die Mutante „grenze nur vom ersten Auftreten“ wird rot.
- [ ] **Ende-Zeile rein** (Unit, Speicher-Senke): 6 Auftreten mit `grenze` `kennung` und 2 mit `herkunft` in einer Episode → „Episode endet: 8 in <d> s (herkunft 2; kennung 6)“; nur `kennung` → „(kennung 6)“ ohne `herkunft 0`.
- [ ] **Bremse mit einem Gedankenstrich** (Integration, R00016 „Bremse“ mit dem neuen Text): `beta` läuft in die Bremse → die Beginn-Zeile ist `WARNING schreibkonto Senden gebremst: zu viele Nachrichten — Episode beginnt | kennung=beta@127.0.0.1 grenze=kennung`, auf stderr dieselbe Zeile.
- [ ] **Vorlauf ohne Punkt** (Unit und Integration, R00011 „Vorlauf im echten Programm“): `vorlauf_text(7_410_002)` → „Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt“; die Episode-Zeile endet vor ` — Episode beginnt` auf `aufholt`. `grep -rnE 'aufholt\. —|Nachrichten — gebremst' src tests` findet nichts.
- [ ] **Kommentar** (Review-Punkt): `src/schreibkonto/mod.rs` schreibt „Bei mehr als 1.024 Einträgen“.

### F0071 Zusagen aus R00011 mit Tests belegt

- [ ] **Bestätigen nur im eigenen Channel** (Node-Test, rein; Code-Review #8 A1): vorläufige `x` in `allgemein` im Sendeauftrag; Ereignis der eigenen Kennung aus `technik` mit Text `x` → keine Bestätigung, `fremd` ist gesetzt, `x` bleibt vorläufig; danach dasselbe Ereignis aus `allgemein` → bestätigt, keine vorläufige übrig. Die Mutante J4 „Kanalvergleich entfernt“ wird rot.
- [ ] **Folgen knapp über dem Ende** (E2E; Code-Review #8 A2): `allgemein` mit 300 Nachrichten; B scrollt eine halbe Zeilenhöhe über das Ende; `beta` sendet 1 → der Abstand zum Ende ist ≤ 1 px, die Nachricht ist sichtbar, keine Pille. Gegenprobe: B steht eine Zeilenhöhe + 5 px über dem Ende; `beta` sendet 1 → `scrollTop` ändert sich um höchstens 1 px, die Pille zeigt „1 neue Nachricht“. Die Mutante „folgen nur bei ≤ 1 px“ wird rot.
- [ ] **Markup bleibt Text** (E2E; Security-Review #8 Optional 5): `beta` sendet `<img src=x onerror="window.__xss=1"><b>fett</b><script>window.__xss=2</script>` → B zeigt genau diesen Text in der Nachricht; die Nachricht enthält 0 `img`, 0 `b` und 0 `script`; `window.__xss` ist `undefined`. Die Mutante „`innerHTML` statt `textContent`“ wird rot.
- [ ] Konsole bei B: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler.

## Betroffene Verzeichnisstruktur

- Oberfläche: `web/verlauf.js` (`zumNeuesten`, Pille, `anfuegen`), `web/gliederung.js` (Kopf der ersten neuen), `web/lc-seite.css` (nur falls die Pille Abstand braucht); Tests `e2e/einheit/zustellung.test.mjs`, `e2e/einheit/gliederung.test.mjs`, neu `e2e/tests/r00020-*.spec.js`.
- Server: `src/http/koerper.rs` (gemeinsame Antwort für Körperfehler), `src/http/senden.rs`, `src/http/beitreten.rs`, `src/http/channels.rs`, `src/http/frist.rs` (Aufrufe; `kein_json` entfällt), `src/http/senden.rs` `gebremst_melden` (Text, `art` je Grenze), `src/uhr.rs` (`vorlauf_text`), `src/schreibkonto/mod.rs` (Kommentar); Tests `tests/senden.rs:159`, `tests/schreibkonto.rs` bzw. `tests/senden_protokoll.rs`.

## Technische Überlegungen

### Ablauf

1. **Sprung**: Klick oder `Ende` → steht die erste neue schon am Ziel (± 1 px)? → ans Ende des Inhalts (Nacharbeit #8, B0329) bzw. neu öffnen \| sonst Ziel = Oberkante der ersten neuen − Höhe des klebenden Trenners − Zeilenhöhe → Auslauf (B0329) → scrollen.
2. **Zeichnen**: angefügte neue → die erste gezählte bekommt den Kopf, unabhängig von der Gruppierung.
3. **Körperfehler**: Route → `json_lesen` → Fehler → gemeinsame Antwort (Status, `fehler` aus Satz der Route und Beispiel, `Connection: close` bei 413 und 408).
4. **Bremse**: 429 → `gebremst_melden` → Auftreten mit `art = grenze` → Episode zählt je Art → Ende-Zeile mit Klammer.

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0067: B0330 Kontext unter dem klebenden Tagestrenner · B0331 Erste neue mit Kopf · B0332 Pille nach dem Sprung führt ans Ende
- F0068: B0333 Pille beschriften
- F0069: B0334 Eine Antwort für Körperfehler · B0335 413 des Sendens in der Schreibweise der Oberfläche
- F0070: B0336 Schreibkonto-Episode zählt je Grenze · B0337 Meldetexte des Sendens mit einem Gedankenstrich
- F0071: B0338 Bestätigung nur im eigenen Channel belegt · B0339 Folgen knapp über dem Ende im Zusammenspiel · B0340 Markup im Nachrichtentext bleibt Text

Elemente: `fn koerper_abgewiesen(fehler: Koerperfehler, zu_gross: &str, beispiel: &str) -> Response` (Name entscheidet der Entwickler) in `src/http/koerper.rs`; `Auftreten { art: Some("kennung" | "herkunft"), … }` in `gebremst_melden`; in der Oberfläche eine Marke „erste neue“ für die Gliederung.

Wireframe: kein Artboard `D0003.dc.html` (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle: Designsystem `README` (Zeichen „↓ zum Neuesten“), `components/VerlaufsMarken/README.md` (Pille, Tagestrenner klebt), `components/Nachricht/README.md` (Gruppierung) — Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `web/verlauf.js` `zumNeuesten`: Ziel mit Trennerhöhe; zweiter Klick ans Ende. Pille: Pfeil und `<kbd>` mit `aria-hidden`, Leerraum.
- `web/gliederung.js` `gliederung`: Kopf für die erste neue.
- `http::senden::kein_json`, `http::beitreten::kein_json`, `http::channels::kein_json`, `http::frist::kein_json` → eine Funktion in `http::koerper`.
- `http::senden::gebremst_melden`: Text und `art` des Auftretens.
- `uhr::vorlauf_text`: ohne Punkt.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Unit (rein)**: gemeinsame Antwort je Fehlerart, Ende-Zeile je Grenze mit Speicher-Senke, `vorlauf_text`.
- **Node (rein)**: Gliederung mit erster neuer, Zustellung mit Ereignis aus anderem Channel.
- **Integration (echtes Programm)**: 413 der Senderoute, Körperfehler der übrigen Routen unverändert, Ende-Zeile je Grenze (Probe des Observability-Reviews), Beginn-Zeile der Bremse.
- **E2E**: Kontext unter dem Trenner und Lesestelle (zwei Fenstergrößen), erste neue mit Kopf, zweiter Klick, Beschriftung und zugänglicher Name, Folgen knapp über dem Ende, Markup.

## Abhängigkeiten

- Abhängig von: Nacharbeit #8 an R00011 (B0326–B0329 unter F0042 und F0044; F0044 gelb) — für F0067 und F0068 fachlich (Ziel des Sprungs, „Ende des Inhalts“ aus B0329, dieselbe Pille), für den Rest nach dem Schnitt des Directors (die Karte braucht die Nacharbeit); F0041 Senden über die API, F0043 Eingabe und eigene Nachricht, F0057 Senden im Betriebsprotokoll (gruen).
- Innerhalb: F0067 und F0068 nacheinander (beide an der Pille in `web/verlauf.js`); F0069, F0070, F0071 parallel dazu.
- Blockiert: — (I0017 Markdown muss den Test „Markup bleibt Text“ grün halten).

## Offene Fragen

- **An den architekt bzw. Director, nicht blockierend für R00020** (Vorschläge, nicht geraten; Liste in der WBS unter „Offene Fragen“, I0009 #8):
  1. **UX S5 — Platz der fehlgeschlagenen Nachricht**: Designsystem `Nachricht` und R00011 F0043 sagen „bleibt an ihrer Stelle“, R00011 sagt „vorläufige … unter allen bestätigten“; gebaut wandert sie ans Ende. Vorschlag: (b) immer unter allen bestätigten wie gebaut, Designsystem und den Satz in R00011 berichtigen. Nicht Teil dieser Anforderung.
  2. **Katalogzeilen ADR-0042** mit „Senden gebremst: zu viele Nachrichten“, dem Vorlauftext ohne Punkt und der Ende-Zeile je Grenze bestätigen — vor dem Bau von F0070.
  3. **`gebremst_herkunft` in `/api/gesundheit`** (Observability, optional): Vorschlag nicht aufnehmen; die Ende-Zeile je Grenze liefert die Messung.
- **An den Director, nicht blockierend**: die Proben des ux-reviewer zu S2, S3, S4 und O2 stehen nicht auf der Karte (Bilder `scratchpad/ux-8/bilder/`); die Kriterien zu F0067 und F0068 sind aus den Befunden abgeleitet und vor dem Bau abzugleichen. Den Wortlaut „Senden gebremst: zu viele Nachrichten“ hat der planer gewählt; der rechtschreib-reviewer kann ihn vor dem Bau ersetzen.

Entschieden vom planer (Rückfragen mit Empfehlung beantwortet):

- **Zweiter Klick führt ans Ende**, statt die Pille nach dem Sprung auf „↓ Zum Neuesten“ umzubeschriften — die Zahl gilt laut R00011, bis der Blick das Ende erreicht.
- **Die erste neue bekommt einen Kopf**, statt eine Linie „Neu“ zu zeichnen — die Ungelesen-Linie des Designsystems gehört zum Öffnen eines Channels, nicht zu live Gezähltem.
- **Die übrigen 413 bleiben**: „höchstens 4096 Bytes“ (R00010) wird nicht angeglichen; A3 ist eine Konsolidierung ohne Verhaltensänderung außerhalb der Senderoute.

## Manuelle Vorbereitungstätigkeiten

- Keine.

## Manuelle Nachbereitungstätigkeiten

- ADR-0042: Katalogzeilen der Bremse und des Vorlaufs mit dem neuen Wortlaut (architekt über den Director).

## Umfang

Gezählt aus der WBS (Bubbles). 0,4 h ist die Untergrenze je Bubble nach dem Skill `work-breakdown-structure`; Richtwert, ungemessen für diese Bubble-Typen.

- F0067 Sprung zur ersten neuen Nachricht lesbar: 3 Bubbles · 3 klar (1,2 h)
- F0068 Pille wie im Designsystem beschriftet: 1 Bubble · 1 klar (0,4 h)
- F0069 Körperfehler des Sendens in einer Schreibweise: 2 Bubbles · 2 klar (0,8 h)
- F0070 Senden im Betriebsprotokoll je Grenze: 2 Bubbles · 2 klar (0,8 h)
- F0071 Zusagen aus R00011 mit Tests belegt: 3 Bubbles · 3 klar (1,2 h)
- Zusammen: 11 Bubbles · 4,4 h klar, keine unklaren; nach der Nacharbeit #8, F0067 vor F0068

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser sind die Befunde der Auto-Reviews zu #8, die nicht Muss waren: UX S2, S3, S4, O2; Rechtschreib-Review Sollte; Code-Review A1, A2, A3; Observability S1; Security Optional 5.

Die Wirkungskette:

- Der Sprung rechnet den klebenden Trenner ein und gibt der ersten neuen ihren Kopf. Deshalb sieht die Mitleserin nach dem Klick, wer schreibt und worauf er antwortet, und muss nicht zurückscrollen.
- Der zweite Klick führt ans Ende. Deshalb bleibt die Pille ein Weg nach unten und wird nie zu einem toten Schalter.
- Die Pille spricht ihren Namen ohne Tastenbezeichnung. Deshalb hört eine Bildschirmleser-Nutzerin „3 neue Nachrichten“ und kennt die Taste über `aria-keyshortcuts`.
- Die 413 schreibt die Grenze wie die Oberfläche, und alle Routen bilden Körperfehler an einer Stelle. Deshalb lesen Agent und Mensch dieselbe Zahl, und eine fünfte Route übernimmt die Form, statt sie zu kopieren.
- Die Episode zählt je Grenze. Deshalb erkennt die Betreiberin, ob ein Agent an seinem eigenen Eimer oder am Eimer seines Rechners scheitert — die Messung, nach der das Konto je Herkunft ausgelegt wird.
- Drei Zusagen bekommen Tests mit Mutanten. Deshalb fällt ein Rückschritt im Kanalvergleich, im Folgen oder bei `textContent` im Gate auf statt beim Nutzer.

Der Hebel liegt an wenigen Stellen (`zumNeuesten`, Gliederung, Pille, eine Antwort für Körperfehler, `gebremst_melden`), nicht an neuen Abläufen.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus R00011, R00016 und dem Designsystem.
- Die Regel des Auslaufs unter dem Verlauf steht in keiner ADR (Code-Review A4, architekt).
- Die Abnahme-Proben zu UX S2, S3, S4, O2 sind auf der Karte nur skizziert; die Kriterien oben sind daraus abgeleitet.

## Notizen

- **Änderung von Zusagen aus R00011 und R00016** (beide bleiben an diesen Stellen unverändert, R00020 nennt die Änderung; R00011 verweist seit dem Review vom 2026-10-28 hierher):
  - R00011 F0041 „Prüfreihenfolge“ (413 mit „höchstens 131072 Bytes“), Kriterium „Körpergrenze als Spanne“ und US-4: der Text heißt „Der Körper ist zu groß — höchstens 131.072 Byte; längere Texte teilen.“; `tests/senden.rs:159` zieht mit.
  - R00011 F0044 Kriterium „Nicht am Ende“ (Z5): der obere Rand der ersten neuen liegt nach dem Klick eine Zeilenhöhe (± 1 px) unter dem klebenden Tagestrenner statt „höchstens eine Zeilenhöhe + 1 px unter dem oberen Rand des Verlaufs“.
  - R00011 F0042 „Vorlauf sichtbar“ und R00016 „Vorlauf der Nachrichtenzeit“: der Text endet auf „aufholt“ ohne Punkt; `src/uhr.rs:196` und `src/http/senden.rs:402` ziehen mit.
  - R00016 „Bremse“: der Text der Episode heißt „Senden gebremst: zu viele Nachrichten“ statt „Zu viele Nachrichten — gebremst“; die Ende-Zeile trägt die Klammer je Grenze hinter `<d> s`.
- UX-Review #8 S6 (keine Bildschirmleser-Ansage) gehört zu F0049 (R00012, #22), O1 (Ausweg) zu F0046; nicht hier.
- Rechtschreib-Review #8 Optional (Testname „Höhe und Schmal“, Zahlenschreibweise 16 384/16.384 in Kommentaren) ist nicht aufgenommen.

### Bewusst außerhalb

- UX S5 (Platz der fehlgeschlagenen Nachricht) → offene Frage, erst nach Entscheidung planbar.
- UX O3 (Tab aus dem Verlauf springt zum ersten Faltknopf, R00007) → eigener Befund an I0008, wenn der Director ihn aufnimmt.
- Code-Review A4 (Auslauf ohne ADR, `verlaufAnlegen` rund 480 Zeilen) → architekt; A5 (Commits nicht themenrein) → Prozess; A6 (`bestaetigt` wächst unbegrenzt), A7 (`sleep` im Schranken-Test), A8 (Leer-Regel `trim` gegen den Server, offene Frage), A9 (Ersatzschlüssel-Format, F0046).
- Observability O1–O4 → architekt (Katalogzeilen ADR-0042), danach planbar.
- Security Optional: Platte füllt sich wegen der Einrückung doppelt so schnell wie ADR-0036 rechnet (architekt, Plattenquote Betreiber); Schreibkonto für `PUT …/frist` (offene Folgeentscheidung ADR-0036, Betreiber); doppelter JSON-Schlüssel nimmt den letzten; RSS-Wachstum nach vielen maximalen Körpern.
- Angleichen der 413-Texte der übrigen Routen an „4.096 Byte“ (R00010) → eigene Entscheidung, wenn der rechtschreib-reviewer es verlangt.
