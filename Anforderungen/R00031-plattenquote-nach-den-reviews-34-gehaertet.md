---
id: R00031
status: Neu
datum: 2026-10-29
---

# R00031: Plattenquote nach den Reviews #34 gehärtet

## Beschreibung

Die Plattenquote aus R00023 schließt die Lücken, die die Auto-Reviews zu #34 gefunden haben und die nicht Muss waren. Danach gilt:

- **Der Ausweg an der Quote stimmt**: die 507 verspricht Platz „beim nächsten Tageswechsel“ nur, wenn dann ein Tag entfällt. Sie nennt, wo die Aufbewahrungsfrist eingestellt wird, sagt, dass Verkürzen ältere Gespräche löscht, und spricht den Betreiber ohne Kommandozeilen-Option an.
- **Ein Fehlschlag wird einmal angesagt**: der Bildschirmleser sagt „nicht gesendet“ einmal, nicht „Nachricht nicht gesendet: Nicht gesendet: …“.
- **Ein falscher Quote-Wert ist benannt**: die Meldung beim Start nennt die Variable bzw. Option, den falschen Wert und die erlaubte Spanne.
- **Das Protokoll zeigt das Ende**: fällt `belegt` unter die Quote, steht „Senden wieder möglich“ im Protokoll, und die Zeile nach dem Entfallen nennt den Stand danach, nicht einen Zwischenstand.
- **Der Quote-Code ist aufgeräumt**: Rechtschreibung in Kommentaren und Testtiteln, keine Nebenwirkung im Prüfen, kein stilles Sättigen.

Zahlt ein auf: [Vision](R00000-vision.md) — Leitplanke „Plattenquote“ (an der Grenze wird Senden abgewiesen, Platz nur über die Aufbewahrungsfrist): wer abgewiesen wird, erfährt den Ausweg, der wirklich hilft, und der Betreiber sieht, wann Senden wieder geht. Kein Widerspruch zur Vision.

Herkunft: Karte #34 (R00023, I0023), Auto-Reviews vom 2026-10-29 — UX-Review Sollte S1–S3 und Optional; Observability-Review Sollte S1, S2 und Optional; Code-Review Sollte A1 und Optional A3, A4, A6, A7; Rechtschreib-Review Sollte und Optional; Security-Review Anmerkung A2. Das Muss des Rechtschreib-Reviews ist Nacharbeit in R00023 (B0486). Was nicht hier steht und warum: Abschnitt „Notizen“.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0023 Plattenquote, Dialog D0001 Start und Ablage): F0110 Ausweg an der Quote nennt, was hilft, F0111 Fehlschlag einmal angesagt, F0112 Falscher Quote-Wert benannt, F0113 Ende der Abweisung im Protokoll, F0114 Quote-Code aufgeräumt.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00023: Quote, belegt, Tagesdatei, Frist, Entfallen, Verkürzen, Tageswechsel, Episode, Start-Eintrag; dazu „Ansage“ (ADR-0038 Regel 10).

## Geschäftlicher Nutzen

Der 507-Text verspricht heute Platz „beim nächsten Tageswechsel“, auch wenn morgen nichts entfällt — wer wartet, steht am nächsten Morgen wieder vor der Quote. Er nennt „eine Frist verkürzen“, ohne zu sagen, wo das geht und dass dabei Gespräche gelöscht werden, und er richtet sich mit `--plattenquote` an einen Leser, der keine Kommandozeile sieht. Ein Teilnehmer mit Bildschirmleser hört „nicht gesendet“ zweimal. Der Betreiber, der den Dienst über eine Umgebungsvariable konfiguriert, liest eine Meldung über die Option, die er gar nicht gesetzt hat. Und nach einem Start über der Quote fehlt im Protokoll die Zeile, ab wann wieder gesendet werden kann. Mit R00031 führt jeder Text zum richtigen nächsten Schritt.

## Funktionale Anforderungen

### Ausweg an der Quote nennt, was hilft (F0110; UX-Review #34 S1, Optional; Security-Review A2)

- Die 507 nennt den Tageswechsel nur, wenn beim nächsten Tageswechsel in mindestens einem Gespräch ein Tag entfällt (nach der Frist des jeweiligen Channels, wie die Pflege sie anwendet).
- Sie nennt den Ort der Fristeinstellung in Worten der Oberfläche („in den Einstellungen eines Channels“) und dass beim Verkürzen ältere Gespräche gelöscht werden.
- Der Teil für den Betreiber nennt keine Option (`--`), sondern dass der Betreiber beim nächsten Start eine höhere Quote einstellen kann.
- Unverändert: Status 507, `regel: "quote"`, `belegt`, `hoechstens`, kein `Retry-After`, der Anfang „Nicht gesendet: Die Gespräche belegen <belegt>, die Quote dieser Instanz ist <hoechstens>.“ (groß nach dem Doppelpunkt, B0486), Größen in Worten wie R00023.
- Wortlaut nach der Entscheidung des architekt („Offene Fragen“ 1); Empfehlung des planer, je mit Großschreibung nach dem Doppelpunkt geprüft:
  - ein Tag entfällt beim nächsten Tageswechsel: „Nicht gesendet: Die Gespräche belegen 1,0 MiB, die Quote dieser Instanz ist 1,0 MiB. Beim nächsten Tageswechsel entfallen alte Tage und schaffen Platz. Sofort Platz schafft eine kürzere Aufbewahrungsfrist in den Einstellungen eines Channels; dabei werden ältere Gespräche gelöscht. Eine höhere Quote kann der Betreiber beim nächsten Start einstellen.“
  - kein Tag entfällt: „Nicht gesendet: Die Gespräche belegen 1,0 MiB, die Quote dieser Instanz ist 1,0 MiB. Platz schafft eine kürzere Aufbewahrungsfrist in den Einstellungen eines Channels; dabei werden ältere Gespräche gelöscht. Eine höhere Quote kann der Betreiber beim nächsten Start einstellen.“

### Fehlschlag einmal angesagt (F0111; UX-Review #34 S2)

- Beginnt der Grund eines Fehlschlags mit „Nicht gesendet“ (507 der Quote, `web/zustellung.js` „Nicht gesendet (Status …).“), sagt die Ansage-Region „Nachricht nicht gesendet“ und den Rest des Grundes: „Nachricht nicht gesendet: Die Gespräche belegen …“ bzw. „Nachricht nicht gesendet (Status 503).“.
- Jeder andere Grund wie bisher: „Nachricht nicht gesendet: <Grund>“.
- Entschieden vom planer (Wortlaut der Ansage, kein Architekturbedarf; der `ux-reviewer` prüft): die eine Ansage-Region und ihr Zeitpunkt (ADR-0038 Regel 10) bleiben.

### Falscher Quote-Wert benannt (F0112; Observability-Review #34 S1 = UX-Review S3)

- Ein falscher Umgebungswert (ohne gültige Option) endet weiter mit Exit 2 ohne `logs/`; die Meldung auf stderr nennt `LOCALCHATA_PLATTENQUOTE`, den Wert in „…“ (maskiert wie bei `LOCALCHATA_PROTOKOLL_STUFE`, auch für einen Wert, der kein UTF-8 ist) und die Spanne 1MiB bis 1024GiB mit Beispiel.
- Eine falsche Option `--plattenquote <wert>` nennt die Option, den Wert und dieselbe Spanne.
- Wortlaut nach der Entscheidung des architekt („Offene Fragen“ 2); Empfehlung nach dem Muster `Stufenfehler` (`src/start/protokollstufe.rs`, F0062): `LOCALCHATA_PLATTENQUOTE ist keine Größe: „4“ — erlaubt sind 1MiB bis 1024GiB wie 4GiB oder 512MiB, oder aus` und `--plattenquote ist keine Größe: „4“ — erlaubt sind 1MiB bis 1024GiB wie 4GiB oder 512MiB, oder aus`. Nach dem Doppelpunkt folgt ein Wert, kein Satz — klein bzw. in Anführung richtig.
- Unverändert: Option ohne Wert („--plattenquote braucht einen Wert — Aufruf siehe localchatA --hilfe“), Vorrang der gültigen Option (ADR-0044 Ergänzung Karte #26).

### Ende der Abweisung im Protokoll (F0113; Observability-Review #34 S2, Optional; Code-Review A1, A5; Rechtschreib-Review Optional)

- Nach dem Entfallen (Start, Tageswechsel, Verkürzen) steht höchstens eine `quote`-Zeile, und sie nennt `belegt` nach dem letzten entfallenen Tag dieses Anlasses.
- Fällt `belegt` von über der Quote auf höchstens die Quote, bleibt aber über 90 %: einmal `INFORMATION quote Gespräche wieder unter der Quote — Senden wieder möglich | belegt=… hoechstens=… grund=<anlass>`.
- Fällt es auf 90 % oder darunter: einmal `INFORMATION quote Gespräche wieder unter 90 % der Quote | …` wie R00023 (Empfehlung: auch dann, wenn es vorher über der Quote lag, nur diese eine Zeile).
- Nach der Entscheidung des architekt („Offene Fragen“ 3): ob der Zustand „über der Quote“ nur beim Start über der Quote beginnt oder auch mit einer 507 an der Quote; eine Zeile je Anlass über alle Channels oder je Channel; `belegt` der Ende-Zeile der 507-Episode; Reihenfolge der Zeilen beim Verkürzen; `grund=start`; Wortlaut „belegen über 90 %“.

### Quote-Code aufgeräumt (F0114; Rechtschreib-Review #34 Sollte und Optional; Code-Review A3, A4, A6, A7)

- Kommentare und Testtitel zur Quote schreiben nach einem Doppelpunkt groß, wenn ein ganzer Satz folgt; „weitergeht“ zusammen; Wortstellung und Fülltext wie im Review.
- `src/http/senden.rs` ohne doppelte Doc-Zeile; `Belegung::pruefen` prüft nur, das Zählen der Abweisung geschieht sichtbar beim Aufrufer oder in einer eigenen Operation; die Start-Meldung der Quote entsteht an einer Stelle; `Belegung::weg` sättigt nicht still (`debug_assert!` auf `bytes ≤ belegt`).

## Nicht-funktionale Anforderungen

- **Benutzerfreundlichkeit**: jeder Text an der Quote führt zu einem Schritt, der wirkt; kein Versprechen, das nicht eintritt; keine Kommandozeilen-Syntax im Text für Teilnehmer.
- **Leistung**: die Frage „entfällt beim nächsten Tageswechsel ein Tag?“ kostet nur im Fall der 507 etwas und keinen Plattenzugriff; die Leistungskriterien aus R00011 und R00023 gelten unverändert.
- **Sicherheit**: der falsche Wert in der Exit-2-Meldung wird maskiert (Steuerzeichen, Nicht-UTF-8), wie bei `LOCALCHATA_PROTOKOLL_STUFE`.
- **Zeit- und Konsolenkriterien** nach `AI-Team/planer.md`: neue Zeitgrenzen gibt es nicht. Konsole wie R00023 „Im Browser“: erlaubt ist nur „Failed to load resource“ zu der einen 507, die das Kriterium herbeiführt.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, Sitzungen, `heute.log`, „der Quote-Ordner“ (Channel `allgemein`, Frist 7, Tagesdatei von gestern mit 1.048.276 Bytes, Testhilfe B0359) und „Senden“ (`beta` als Agent mit dem Text `Quote-Probe 17 Z.`, Satz 81 Bytes) wie in R00023; `TZ=Europe/Berlin`, `LOCALCHATA_JETZT`.

### F0110 Ausweg an der Quote nennt, was hilft

- [ ] **Kein Tag entfällt** (Integration; Abnahme-Probe aus dem UX-Review #34, Sollte S1, wörtlich übernommen): „nur gestern, Frist 7 → fehler ohne „beim nächsten Tageswechsel“, mit Hinweis auf Channel-Einstellungen, ohne „--““. Ausgeschrieben: Quote-Ordner, Start mit `--plattenquote 1MiB`; `beta` sendet viermal → die vierte 507, `fehler` beginnt mit „Nicht gesendet: Die Gespräche belegen 1,0 MiB, die Quote dieser Instanz ist 1,0 MiB.“, enthält „Einstellungen eines Channels“ und „gelöscht“, enthält weder „Tageswechsel“ noch „--“. Mit der Empfehlung aus „Funktionale Anforderungen“: Text wörtlich.
- [ ] **Ein Tag entfällt** (Integration): Quote-Ordner, Start mit `--plattenquote 1MiB`; `delta` setzt die Frist von `allgemein` auf 2 → 200 (heute und gestern bleiben, nichts entfällt); `beta` sendet viermal → die vierte 507, `fehler` enthält „Beim nächsten Tageswechsel“ und „Einstellungen eines Channels“, nicht „--“. Rechnung: Frist 2 hält heute und gestern; nach dem Tageswechsel ist gestern vorgestern und entfällt. Die Mutante „Tageswechsel immer genannt“ wird im ersten Kriterium rot, „Tageswechsel nie genannt“ hier.
- [ ] **Antwort sonst unverändert** (Unit der Antwort): Status 507, kein `Retry-After`, `regel: "quote"`, `belegt`, `hoechstens`; die Größen in Worten wie R00023 „Größen in Worten“.
- [ ] **Im Browser** (E2E, wie R00023 „Im Browser“): die fehlgeschlagene Nachricht zeigt den Text ohne „--“; bei 320 px Breite bricht keine Zeile mitten in einem Wort um, das einen Bindestrich trägt (UX Optional).

### F0111 Fehlschlag einmal angesagt

- [ ] **507 der Quote** (Unit, rein, `web/ansage.js`; Abnahme-Probe aus dem UX-Review #34, Sollte S2, wörtlich übernommen): „genau ein „nicht gesendet““. Ausgeschrieben: Ereignis `fehlschlag` mit dem Grund „Nicht gesendet: Die Gespräche belegen 1,0 MiB, …“ → Ansage „Nachricht nicht gesendet: Die Gespräche belegen 1,0 MiB, …“; „nicht gesendet“ kommt darin ohne Rücksicht auf Groß-/Kleinschreibung genau einmal vor.
- [ ] **Statusfehler** (Unit): Grund „Nicht gesendet (Status 503).“ → „Nachricht nicht gesendet (Status 503).“.
- [ ] **Andere Gründe** (Unit): Grund „Der Text ist zu lang.“ → „Nachricht nicht gesendet: Der Text ist zu lang.“ (unverändert).
- [ ] **Im Browser** (E2E, wie R00023 „Im Browser“): nach der 507 enthält die Ansage-Region genau einmal „nicht gesendet“.

### F0112 Falscher Quote-Wert benannt

- [ ] **Umgebungswert** (Integration, echtes Programm; Abnahme-Probe aus dem Observability-Review #34, Sollte S1, wörtlich übernommen): „LOCALCHATA_PLATTENQUOTE=4 → stderr enthält Variable und „4“; heute rot“. Ausgeschrieben: ohne Option, `LOCALCHATA_PLATTENQUOTE=4` → Exit 2, stderr enthält `LOCALCHATA_PLATTENQUOTE`, „4“ in Anführung und „1MiB bis 1024GiB“; im Ordner gibt es kein `logs/`. Ebenso `4GB`, `0MiB`, `1025GiB`.
- [ ] **Wert ohne UTF-8** (Integration): `LOCALCHATA_PLATTENQUOTE` mit dem Byte `0xFF` → Exit 2, stderr nennt die Variable, keine Panik.
- [ ] **Option** (Unit Aufruf): `--plattenquote 4` → Aufruffehler, Text enthält `--plattenquote`, „4“ in Anführung und „1MiB bis 1024GiB“; `--plattenquote` ohne Wert unverändert.
- [ ] **Vorrang bleibt** (Integration): `--plattenquote 2MiB` mit `LOCALCHATA_PLATTENQUOTE=4` startet mit 2 MiB (R00023, ADR-0044 Ergänzung Karte #26).

### F0113 Ende der Abweisung im Protokoll

- [ ] **Stand nach dem Entfallen** (Unit, Speicher-Senke; Abnahme-Probe aus dem Code-Review #34, A1, wörtlich übernommen): „Quote 1.000, 800 + 100 geladen, ein Satz → 965, Frist 1 → genau eine INFORMATION mit belegt=65, heute 165“. Ausgeschrieben: geladen vorgestern 800 und gestern 100 Bytes (900, nicht über 90 %); ein Satz zu 65 Bytes → 965, eine `WARNING … 90 %`; Verkürzen auf 1 → beide Tage entfallen → genau eine `INFORMATION quote Gespräche wieder unter 90 % der Quote | belegt=65 hoechstens=1000 grund=frist_gesetzt`. Die Mutante „Meldung je entfallenem Tag“ wird rot.
- [ ] **Stand nach dem Entfallen im echten Programm** (Integration, US-7): `allgemein` mit 850.000 Bytes vorgestern und 93.700 Bytes gestern (943.700 ≤ 943.718,4 = 90 % von 1 MiB), Start mit `--plattenquote 1MiB`; `beta` sendet einmal → 943.781, eine `WARNING … 90 %`; Frist auf 1, bestätigt → nach der 200 genau eine `quote`-Zeile `INFORMATION quote Gespräche wieder unter 90 % der Quote | belegt=81 hoechstens=1048576 grund=frist_gesetzt`.
- [ ] **Wieder unter der Quote** (Integration, echtes Programm über Mitternacht; Abnahme-Probe aus dem Observability-Review #34, S2, wörtlich übernommen): „200.000 + 1.000.000 B, Frist 3, 1 MiB über Mitternacht → INFORMATION „wieder unter der Quote — Senden wieder möglich“; heute rot“. Ausgeschrieben: ein Channel mit 200.000 Bytes am ältesten noch gehaltenen Tag und 1.000.000 Bytes an einem jüngeren, Frist 3, Start mit `--plattenquote 1MiB` kurz vor Mitternacht (`LOCALCHATA_JETZT`) → `WARNING quote Gespräche belegen mehr als die Quote — Senden wird abgewiesen | belegt=1200000 hoechstens=1048576`; nach dem Tageswechsel genau eine `INFORMATION quote Gespräche wieder unter der Quote — Senden wieder möglich | belegt=1000000 hoechstens=1048576 grund=tageswechsel` und keine 90-%-Zeile (1.000.000 × 10 = 10.000.000 > 9.437.184); danach antwortet Senden 201. Das Gegeben stellt die Testhilfe aus B0359 her, erweitert um das Datum des Tages (Entscheidung des Entwicklers).
- [ ] **Von über der Quote unter 90 %** (Unit): geladen 1.100 bei Quote 1.000 (Tage 500 + 600); der 600-Byte-Tag entfällt → genau eine `INFORMATION … wieder unter 90 % der Quote | belegt=500 …` und keine Zeile „wieder unter der Quote“ (Empfehlung, „Offene Fragen“ 3).
- [ ] **Nach der Entscheidung des architekt** („Offene Fragen“ 3): die Ende-Zeile der Episode „Senden abgewiesen“ nennt kein veraltetes `belegt`; die Reihenfolge von `frist`- und `quote`-Zeile beim Verkürzen; `grund=start` kommt vor oder steht nicht mehr im Katalog; der Wortlaut der 90-%-Zeile.

### F0114 Quote-Code aufgeräumt

- [ ] **Rechtschreibung** (Review-Punkt für den `rechtschreib-reviewer`; Rechtschreib-Review #34 Sollte und Optional): die Kommentare an `src/start/plattenquote.rs:84`, `:152`, `tests/plattenquote.rs:90`, `:226`, `:443`, `src/gespraeche/quote.rs:356`, `:409`, `src/start/mod.rs:476` und der Testtitel `e2e/tests/r00023-quote.spec.js:50` schreiben nach dem Doppelpunkt groß, wo ein ganzer Satz folgt; `git grep -n 'weiter geht' -- src tests e2e` → 0; `quote.rs:129` „noch unter `hoechstens` passen“; der Fülltext in `src/testdaten.rs:178` mit Umlauten („Füllung für die Quote“) oder begründet ASCII.
- [ ] **Doc-Kommentar** (Review-Punkt für den `code-reviewer`; A3): `src/http/senden.rs` hat über `groesse_in_worten` genau eine Zeile „Bytes in Worten …“.
- [ ] **Prüfen ohne Nebenwirkung** (Unit; A4): ein Aufruf der Prüfung, die abweist, ändert `quote_abgewiesen` nicht; die Route zählt die 507 genau einmal (Integration R00023 „507 im echten Programm“ bleibt grün: `quote_abgewiesen: 1`).
- [ ] **Start-Meldung an einer Stelle** (Review-Punkt für den `code-reviewer`; A6): `quote_beim_start_melden` wird im echten Programm genau einmal je Start gerufen; R00023 „Start“ bleibt grün.
- [ ] **Abziehen sättigt nicht still** (Unit, Debug-Build; A7): `weg(bytes)` mit `bytes > belegt` löst im Debug-Build einen `debug_assert` aus; im Release bleibt das Sättigen.

## Betroffene Verzeichnisstruktur

- HTTP: `src/http/senden.rs` (507-Text, Doc-Kommentar).
- Gespräche: `src/gespraeche/mod.rs` bzw. `src/gespraeche/frist.rs` (entfällt beim nächsten Tageswechsel ein Tag?), `src/gespraeche/pflege.rs` (eine Zeile je Anlass mit Stand danach), `src/gespraeche/quote.rs` (Zustand „über der Quote“, `pruefen`, `weg`).
- Start: `src/start/plattenquote.rs`, `src/start/aufruf.rs` (Meldungen), `src/start/mod.rs` (Start-Meldung).
- Oberfläche: `web/ansage.js`.
- Tests: `tests/plattenquote.rs`, Unit neben dem Code, `e2e/einheit/` (Ansage), `e2e/tests/r00023-quote.spec.js`; Testhilfe `src/testdaten.rs`, `examples/quote-ordner.rs`.

## Technische Überlegungen

### Ablauf

1. **Senden an der Quote**: Prüfung wie R00023 → `Quote { belegt, hoechstens }` → (neu) „entfällt beim nächsten Tageswechsel ein Tag?“ → 507-Text mit oder ohne Tageswechsel.
2. **Ansage**: Fehlschlag → Grund beginnt mit „Nicht gesendet“? → „Nachricht nicht gesendet“ + Rest, sonst wie bisher.
3. **Start**: Wert lesen → Fehler mit Quelle (Variable | Option), Wert maskiert, Spanne → Exit 2.
4. **Entfallen**: alle Tage des Anlasses abziehen → einmal Wechsel bestimmen (über der Quote → höchstens Quote; über 90 % → höchstens 90 %) → eine Zeile mit dem Stand danach.

### Grobentwurf (Vorplanung)

Die Slices sind noch nicht bis Bubble geplant (Rolling Wave, Architektur zuerst für F0110, F0112, F0113). Nach den Entscheidungen des architekt: `/planung verfeinern F0110 F0111 F0112 F0113 F0114 --bis Bubble`. F0111 und F0114 brauchen keine Architekturentscheidung und können vorher verfeinert werden.

### Änderungen an bestehenden Klassen

- `http::senden::quote_erreicht`: Text abhängig von „ein Tag entfällt beim nächsten Tageswechsel“.
- `gespraeche::quote`: Zustand „über der Quote“ und Wechsel nach unten mit zwei Stufen; `Belegung::pruefen` ohne Zählen, `Belegung::weg` mit `debug_assert!`.
- `gespraeche::pflege`: Meldung nach dem letzten Tag eines Anlasses statt je Tag.
- `start::plattenquote::Groessenfehler`: trägt Quelle und Wert.
- `web/ansage.js` `ansage`: Fall `fehlschlag`.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests.

- **Unit (rein bzw. Speicher-Senke)**: 507-Antwort, Ansage, Größenfehler, Stand nach dem Entfallen, von über der Quote unter 90 %, Prüfen ohne Nebenwirkung, Sättigen.
- **Integration (echtes Programm)**: kein Tag entfällt, ein Tag entfällt, Umgebungswert, Wert ohne UTF-8, Vorrang, wieder unter der Quote über Mitternacht.
- **E2E**: Text an der fehlgeschlagenen Nachricht, Ansage-Region.

## Abhängigkeiten

- Abhängig von: R00023 (I0023) — F0074 Quote einstellen (grün) für F0112; F0075 Senden an der Quote abgewiesen (gelb, Nacharbeit B0486) für F0110 und F0111; F0076 Quote im Betriebsprotokoll (grün) für F0113; F0075 und F0076 für F0114.
- Nicht parallel, sondern nacheinander mit: F0114 nach F0110–F0113 (dieselben Dateien); F0110 nach B0486 (derselbe Text).
- Blockiert: keine.

## Offene Fragen

An den architekt, vor dem Bau (Architektur zuerst); Empfehlungen des planer:

1. **Ausweg-Text und Tageswechsel** (F0110, ADR-0044 Regel 4): Wortlaut für beide Fälle; wie die 507 erfährt, ob beim nächsten Tageswechsel ein Tag entfällt (aus dem Verlaufsindex und der Frist je Gespräch unter der Sperre, die die 507 ohnehin hält; Channels mit defekten Einstellungen zählen nicht, ADR-0032); ob Agenten im Text zusätzlich `PUT …/frist` lesen sollen. Empfehlung: die beiden Texte aus „Funktionale Anforderungen“, die Frage nur im Fall der 507 rechnen, ohne API-Pfad im Text (Agenten erkennen den Fall an `regel: "quote"`).
2. **Meldung zum falschen Wert** (F0112, ADR-0044 Regel 3): Wortlaut für Umgebung und Option. Empfehlung: Muster `Stufenfehler` wie in „Funktionale Anforderungen“.
3. **Ende der Abweisung** (F0113, ADR-0044 Regel 7, Katalog ADR-0042): (a) beginnt „über der Quote“ nur beim Start über der Quote oder auch mit einer 507 an der Quote (dann bekäme auch der Fall „an der Quote, Tag entfällt, Rest über 90 %“ die Zeile „Senden wieder möglich“)? Empfehlung: auch mit einer 507 — sonst sieht der Betreiber nach der Episode „Senden abgewiesen“ nie, dass es vorbei ist. (b) eine Zeile je Anlass über alle Channels oder je Channel (Code-Review A5)? Empfehlung: je Anlass über alle Channels, nach dem letzten Channel. (c) Ende-Zeile der Episode „Senden abgewiesen“: `belegt` weglassen oder den Stand beim Ende nennen? (d) Reihenfolge beim Verkürzen: erst die `frist`-Zeile (Ursache), dann die `quote`-Zeile (Folge). (e) `grund=start` kommt im echten Programm nie vor — aus dem Katalog nehmen oder beim Start nach dem Entfallen melden? (f) „Gespräche belegen über 90 % der Quote“ statt „belegen 90 %“ (Rechtschreib-Review Optional) — ändert die Zeile aus R00023 (US-7, „Wechsel melden“).

## Manuelle Vorbereitungstätigkeiten

- architekt: Entscheidungen zu „Offene Fragen“ 1–3 als Ergänzung zu ADR-0044 und ADR-0042; danach `/planung verfeinern F0110 F0112 F0113 --bis Bubble` (planer). F0111 und F0114 sofort verfeinerbar.

## Manuelle Nachbereitungstätigkeiten

- `AI-Team/umgebung.md`: Wortlaut der Exit-2-Meldung zur Plattenquote, falls dort zitiert (Director).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser sind die Reviews #34: der 507-Text verspricht einen Tageswechsel, der nichts bringt, und verschweigt, dass sein zweiter Ausweg Gespräche löscht; die Ansage verdoppelt sich; die Exit-2-Meldung nennt die falsche Quelle; das Protokoll schweigt, wenn Senden wieder geht, und meldet einen Zwischenstand.

- Die 507 nennt den Tageswechsel nur, wenn er Platz schafft. Deshalb wartet niemand auf einen Morgen, der nichts ändert, und wer die Frist verkürzt, weiß, dass dabei Gespräche verschwinden.
- Die Exit-2-Meldung nennt Variable, Wert und Spanne. Deshalb findet der Betreiber den Tippfehler in der Dienst-Konfiguration statt in einer Option, die er nie gesetzt hat.
- Das Protokoll meldet „Senden wieder möglich“ mit dem Stand danach. Deshalb weiß der Betreiber nach einer Abweisung, ab wann die Instanz wieder schreibt, ohne `/api/gesundheit` zu pollen.

Der Hebel liegt an den Stellen, die R00023 gebaut hat (507-Text, Pflege, Meldung beim Start, Ansage); eine neue Oberfläche oder ein Live-Hinweis an die Tabs (ADR-0044 Regel 10) wäre mehr, als die Befunde verlangen.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/`).
- Kein Betriebshandbuch; die Monitoring-Empfehlung aus ADR-0044 Regel 9 hat keinen Ort.

## Notizen

### Zuordnung der Befunde aus den Reviews #34

- In R00023 als Nacharbeit (nicht hier): Rechtschreib-Review Muss (B0486).
- In R00023 ohne Bau angeglichen (nicht hier): Abweichungen des `entwickler` 1–4, Code-Review A2, UX-Anmerkung zu R00023 Zeile 40/93, Rechtschreib-Review Sollte „weitergeht“ im Dokument.
- An den architekt, ohne Knoten: Security-Review A1 (viele Herkünfte füllen 4 GiB in ≈ 17 min; ADR-0044 Q2 „je Herkunft“ präzisieren, Option E als Folgeentscheidung — hängt an der Betreiberfrage F1 zu I0010); Katalog ADR-0042 `plattenquote_option`; Wortlaut ADR-0044 Regel 4 „Die“.
- An den Director, ohne Knoten: Security-Review A3 (Health nennt den absoluten Pfad ohne Sitzung) — älter als #34.

### Verworfene Alternativen

- UX Optional „Erneut senden ohne sichtbare Wirkung bei voller Quote (zuletzt versucht HH:MM)“: nicht hier — eine Gestaltungsfrage an der fehlgeschlagenen Nachricht (R00012 F0046) ohne Probe; Kandidat für eine Ausbaustufe dort, wenn `ux-reviewer` oder Betreiber sie wollen.
- Den Ausweg-Text ohne Tageswechsel für alle Fälle: verworfen — wo morgen ein Tag entfällt, ist Warten der Ausweg, der niemanden etwas kostet.
- Die Ansage ohne Vorsatz „Nachricht nicht gesendet:“ für alle Gründe: verworfen — die meisten Gründe („Der Text ist zu lang.“) sagen nicht, dass die Nachricht nicht ankam.
- Härtung als eigene Interaction: verworfen — dasselbe Verhalten an der Systemgrenze, reifer, wie F0093–F0096 unter I0024.

### Änderung von Zusagen

- R00023 „Falscher Wert beim Start“: der Wortlaut der Exit-2-Meldung ändert sich mit F0112 (R00023 bleibt unverändert, R00031 nennt die Änderung).
- R00023 „Antwort“, US-3, US-4: der Ausweg-Teil des 507-Texts ändert sich mit F0110; der Anfang bleibt.
- R00023 F0076: nach „Offene Fragen“ 3 (f) gegebenenfalls „belegen über 90 %“.

### Umfang

Nicht bis Bubble geplant (`/planung zaehlen I0023`: F0110–F0114 ohne Bubbles). Hypothese zur Ordnung, nicht gezählt: F0110 etwa 3 Bubbles, F0111 etwa 1, F0112 etwa 2, F0113 etwa 3, F0114 etwa 3 — zur Untergrenze 0,4 h je Bubble rund 5 h, Faktor 0,5–2.
