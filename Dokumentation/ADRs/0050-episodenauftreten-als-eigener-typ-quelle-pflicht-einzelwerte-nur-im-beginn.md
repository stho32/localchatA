# ADR-0050: Episodenauftreten als eigener Typ — die Quelle ist Pflichtargument und bildet den Schlüssel der Episode, jeder weitere Wert ist Einzelwert und steht nur in der Beginn-Zeile; ein ausführbarer Katalog prüft alle Bauweisen

**Status**: Akzeptiert (nichts gebaut; getragen durch acht Funde derselben Fehlerklasse und die Wiedervorlage-Bedingung in ADR-0042, Ergänzung „Karte #66“, Option D)
**Datum**: 2026-10-30
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #70)
**Kontext-Anforderung**: — (Karte #70 „Härtung Protokoll: Typ erzwingt Quelle und Einzelwert (Option D) und Katalog trennt Beginn/Ende“; Herkunft code-reviewer #66 Anmerkung 3, observability-reviewer #66)
**Erprobt durch**: — (Einordnung aller Bauweisen gelesen in `src/` auf `main` 6d2aa3f; nicht gebaut)

## Kontext

- **Fehlerklasse**: eine Ende- oder Zwischenzeile spricht mit einem Wert des **ersten** Auftretens für die ganze Episode. Behoben für `senden`, `körper`, `schreibkonto`, `zeit` (R00022, [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Ergänzung „Karte #31“ Punkt 4), dann für die Zwischenzeile aller Episoden, `quote`, „Reservierung verfallen“, „Großer Anspruch“ (Ergänzung „Karte #66“), und im Bau von #66 fünf weitere (Platzgrenzen, Grenzschutz, Metadaten, Anhang-Abweisungen, Namensauflösung; Bericht entw3 #66). Acht Episoden mit demselben Fehler, jede einzeln gefunden.
- **Ursache im Typ** (`src/protokoll/zeile.rs:227–276`): `Kontext` ist eine Liste mit zwei Markierungslisten; `mit` legt einen Wert in Beginn **und** Ende, `mit_nur_beginn` nur in den Beginn, `mit_fortlaufend` den letzten Wert ins Ende. Der Standard (`mit`) ist die Einordnung, die man nur bei der Quelle will — wer nicht nachdenkt, schreibt einen Einzelwert in die Ende-Zeile. `fuer_ende` filtert die Markierten heraus; eine Mutante „Filter weg“ wirkt auf alle Episoden zugleich.
- **Zweite Angabe der Quelle**: `Auftreten::quelle: String` (`src/protokoll/episoden.rs:20–32`) bildet den Schlüssel der Episode, der Kontext trägt dieselbe Quelle noch einmal als Werte. Beide sind von Hand gepflegt; nichts prüft, dass sie zusammenpassen (z. B. Quelle `"{datei} {grund}"`, Kontext `channel`, `datei`, `grund`).
- **`Kontext` dient auch Nicht-Episoden-Zeilen** (`Protokoll::eintragen`, jede DEBUG-, INFORMATION- und ERROR-Zeile); dort bedeuten die Markierungen nichts.
- **Bestand**: 20 Stellen in 14 Dateien bauen ein `Auftreten` (Liste in [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung „Karte #70“); die Einzeltests aus R00038 prüfen je Stelle „zwei Auftreten, Ende ohne Einzelwert“ (`src/http/senden.rs:669`, `src/start/mod.rs:977`, `src/http/anhaenge/anmelden.rs:467`, `src/http/koerper.rs:459`).
- Qualitätsziel (Vision, Leitplanke Logging; `AI-Team/umgebung.md` „Logging-Konzept“): der Betreiber liest morgens das Protokoll und zieht daraus Schlüsse — eine Ende-Zeile mit falschem Wert ist ein falscher Befund. Szenario mit Maß: eine neue Episode wird gebaut, der Entwickler ordnet einen Wert nicht ein → die Ende-Zeile enthält **weniger** (der Wert fehlt), nie einen falschen; und ein Test wird rot, bevor gemergt ist.

## Optionen

### Option D1: Eigener Typ `Auftreten` mit Pflicht-`Quelle`, getrennten Feldern und Schlüssel aus der Quelle
`Auftreten::neu(stufe, bereich, text, quelle: Quelle)`; danach nur `.einzeln(k, v)`, `.fortlaufend(k, v)`, `.art(..)`, `.verworfen(..)`, `.bytes(..)`, `.anzeige(..)`. Felder privat im Modul `protokoll`. Der Schlüssel der Episode wird aus den Werten der `Quelle` gerechnet; `quelle: String` entfällt. `Kontext` verliert die Markierungen.
- Vorteile: Vergessen ergibt **weniger** — die Quelle ist nach dem Bau geschlossen, jeder weitere Wert ist Einzelwert; die Ende-Zeile wird aus Quelle und Fortlaufendem gebaut, Einzelwerte kommen in ihr gar nicht vor (kein Filter, den man abschalten kann); Schlüssel und Ende-Kontext sind dieselben Daten, also prüfbar über „eine Episode oder zwei“; `Kontext` ist wieder nur eine Liste.
- Nachteile: die Reihenfolge der Beginn-Zeile wird „Quelle, Einzelwerte, Fortlaufendes“ — fünf gebaute Zeilen ändern ihre Reihenfolge (`quote`, beide Schreibfehler, „Reservierung verfallen“, zwei Anhang-Abweisungen), Tests mit wörtlichen Zeilen ziehen mit; der Schlüssel kann sich ändern, wo die Quelle heute mehr oder weniger trägt als der Schlüssel-String (siehe Pre-Mortem 1).

### Option D2: Getrennte Felder, `quelle: String` bleibt als Schlüssel daneben
Wie D1, aber der Schlüssel bleibt ein eigener String.
- Vorteile: die Identität jeder Episode bleibt genau wie gebaut; keine Frage, ob eine Einstellung (`hoechstens`) in den Schlüssel gehört.
- Nachteile: zwei Angaben derselben Quelle, von Hand synchron gehalten; vergisst jemand einen Quellwert im Kontext, fehlt im Ende, **wer** — und kein Test merkt es, weil die Episode trotzdem richtig zählt.

### Option D3: Eigener Episodenkontext als geordnete Liste mit Pflicht-Rolle je Eintrag
`Episodenkontext(Vec<(Rolle, Schluessel, String)>)` mit `Rolle = Quelle | Einzeln | Fortlaufend`, kein Eintrag ohne Rolle; `Kontext` bleibt für Nicht-Episoden.
- Vorteile: die Reihenfolge der Beginn-Zeilen bleibt wie gebaut, keine wörtliche Zeile ändert sich; kleinster Umbau der Aufrufer (eine Rolle je Aufruf).
- Nachteile: die Ende-Zeile filtert weiter nach Rolle — derselbe Filter, den eine Mutante abschaltet; Schlüssel und Quellwerte bleiben getrennt wie heute; „vergessen“ gibt es nicht, aber „falsche Rolle“ ist ein Wort, so leicht wie heute `mit` statt `mit_nur_beginn`.

### Option D4: Kein Umbau — ausführbarer Katalog und Review-Punkt
Der Typ bleibt; eine Testtabelle über alle Bauweisen prüft „zwei Auftreten → Ende ohne Einzelwert“, ein Vollständigkeitstest zählt die Stellen.
- Vorteile: kleinster Eingriff; deckt die Fehlerklasse an allen heutigen Stellen.
- Nachteile: der Standard bleibt falsch herum (`mit` = Ende); die Erwartung der Tabelle schreibt der, der auch den Code schreibt — wer `mit` für einen Einzelwert hält, schreibt die Testzeile genauso. Nach acht Funden ist „Test und Review“ der Weg, der achtmal nicht gereicht hat.

## Entscheidung

**Gewählt: D1, mit dem ausführbaren Katalog aus D4 als Prüfung der Einordnung.**

Es überwiegt, dass der Fehler nicht mehr *geschrieben* werden kann, sondern höchstens eine fehlende Angabe entsteht, und dass die Einordnung über ein beobachtbares Merkmal prüfbar wird (gleiche Quelle → eine Episode; andere Quelle → zwei). In Kauf genommen: fünf Beginn-Zeilen ändern ihre Reihenfolge, und an zwei Stellen wird der Schlüssel aus mehr Werten gebildet als heute (Einstellungen des Laufs, Fehlertext) — ohne Wirkung, solange diese Werte im Lauf fest sind.

Regeln:

1. **Typ** (`src/protokoll/episoden.rs`): `Auftreten` hat private Felder `stufe`, `bereich`, `text`, `quelle: Quelle`, `einzeln: Kontext`, `fortlaufend: Kontext`, `art`, `verworfen`, `bytes`, `anzeige`. Gebaut wird nur über `Auftreten::neu(stufe, bereich, text, quelle)` und die Erweiterungen `.einzeln(k, v)`, `.fortlaufend(k, v)`, `.art(a)`, `.verworfen(v)`, `.bytes(n)`, `.anzeige(t)` (je `self -> Self`). Eine Struktur-Schreibweise `Auftreten { … }` außerhalb von `protokoll` kompiliert nicht.
2. **Quelle**: `Quelle::neu().mit(k, v)…` bzw. `Quelle::instanz()` (leer, eine Episode je Bereich und Text). Sie trägt die Schlüssel, aus denen die Episode gebildet ist, und was aus ihnen folgt (`channel` aus `datei`, `herkunft` aus `kennung`) oder im Lauf fest ist (`hoechstens` einer Einstellung). Der Schlüssel der Episode ist Bereich + Text + die Werte der Quelle in ihrer Reihenfolge (eindeutig getrennt, z. B. mit `\u{1f}`); der bisherige String `quelle` entfällt.
3. **Zeilen**: Beginn = Quelle, dann Einzelwerte, dann Fortlaufendes (Werte des ersten Auftretens). Zwischen- und Ende-Zeile = Quelle, dann Fortlaufendes mit dem Wert des **letzten** Auftretens, dann `bytes` (größte), wie bisher über **eine** Funktion (`kontext_der_zusammenfassung`). Einzelwerte werden für diese Zeilen nicht gelesen. Die Sammelepisode bleibt ohne Kontext.
4. **`Kontext`** (`src/protokoll/zeile.rs`) wird wieder eine Liste ohne Markierungen: `mit_nur_beginn`, `mit_fortlaufend`, `hat_fortlaufende`, `fuer_ende` und die beiden Markierungslisten entfallen. `Kontext` bleibt der Typ jeder Zeile (`Eintrag`).
5. **Bauweisen**: jede Stelle, die ein `Auftreten` baut, ist eine reine Funktion `<name>_auftreten(..) -> Auftreten` (die heute eingebetteten in `koerper.rs`, `ereignisse.rs`, `hochladen.rs` (zu langsam), `abriss.rs`, `start/mod.rs` (verdrängt), `vorlauf.rs`, `annahmefehler.rs` werden herausgezogen), sichtbar für Tests der Kiste (`pub(crate)`). Die Einordnung je Bauweise steht in [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung „Karte #70“ (Spalten Quelle, Beginn, Ende).
6. **Ausführbarer Katalog** (ersetzt die Einzeltests „zwei Auftreten“ aus R00038; die Integrationstests gegen das echte Programm bleiben): ein Testmodul der Kiste (`#[cfg(test)]`, z. B. `src/episodenkatalog.rs`) mit **einer Zeile je Zeile der Tabelle** in ADR-0042 „Karte #70“: Bauweise, zwei Aufrufe A und B mit **gleichen Quellargumenten** und **paarweise verschiedenen** übrigen Argumenten, ein dritter Aufruf C mit anderer Quelle, und die erwarteten Schlüssel von Beginn und Ende. Eigenschaften, je Zeile geprüft über `Episoden` mit gestellter Uhr und Speicher-Senke:
   - (E1) A und B ergeben **genau eine** Beginn-Zeile; C eine zweite.
   - (E2) Zwischenzeile (Uhr + 10 min 1 s) und Ende-Zeile (Uhr + Ruhe) enthalten **keinen** Einzelwert von A oder B (weder `schluessel=` noch den Wert) und **jeden** Quellwert.
   - (E3) die Schlüssel der Beginn-Zeile = Quelle ∪ Beginn-Spalte, die der Ende-Zeile = Quelle ∪ Ende-Spalte (Zählungsklammer und `bytes` ausgenommen, wo die Spalte sie nennt).
   - (E4) Fortlaufendes: Ende trägt den Wert von B.
   - **Vollständigkeit**: ein Test liest die Dateien unter `src/` (Standardbibliothek, `CARGO_MANIFEST_DIR`) und zählt `Auftreten::neu(` außerhalb von `#[cfg(test)]`-Modulen; die Zahl ist gleich der Zahl **verschiedener Bauweise-Funktionen** im Testkatalog (eine Funktion darf mehrere Zeilen haben, z. B. `grenzauftreten` drei, und enthält genau einen Aufruf `Auftreten::neu(`). Eine neue Bauweise ohne Katalogzeile ist rot.
7. **Durchsetzung**: (a) Typ — Regel 1 (kompiliert nicht anders); (b) Katalog — Regel 6; (c) Review-Punkt `observability-reviewer`: neue Episode ⇒ neue Zeile in ADR-0042 „Karte #70“-Tabelle **und** im Testkatalog; (d) Review-Punkt `code-reviewer`: `Quelle::neu()` mit einem Wert, der je Auftreten wechseln kann, ist ein Befund.

## Konsequenzen

**Positiv:**
- Ein vergessener Einzelwert fehlt in der Ende-Zeile, statt dort falsch zu stehen; ein Einzelwert, der versehentlich in die Quelle gerät, zerlegt die Episode — beides fällt im Katalog auf.
- Die Mutante „Kontext des ersten Auftretens“ ist im Typ nicht mehr ein abschaltbarer Filter, sondern müsste ausdrücklich `einzeln` in die Zusammenfassung schreiben.
- `Kontext` ist für alle Nicht-Episoden-Zeilen wieder ohne Bedeutungen, die dort nicht gelten.
- Der Katalog in ADR-0042 und der Testkatalog sind dieselbe Tabelle — wer eine Episode hinzufügt, schreibt beide oder ist rot.

**Negativ / Inkauf zu nehmen:**
- Fünf gebaute Beginn-Zeilen ändern die Reihenfolge ihrer Schlüssel (Quelle zuerst); wörtliche Erwartungen in Tests ziehen mit (u. a. `tests/plattenquote.rs:446`). Kein bekannter Leser hängt an der Reihenfolge ([ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) legt keine fest).
- Der Schlüssel wird aus den Quellwerten gerechnet: `quote` und Anhang „voll“/„Hochladungen“ tragen `hoechstens` im Schlüssel, „Verbindung nicht angenommen“ den Fehlertext statt der Nummer, „Großer Anspruch“ zusätzlich `herkunft` — gleichwertig, solange diese Werte je Quelle fest sind (Einstellung, Text zur Nummer, Herkunft aus der Kennung).
- Der Vollständigkeitstest liest Quelltext — er ist an die Schreibweise `Auftreten::neu(` gebunden; ein Makro oder ein Umweg darum herum entgeht ihm (Review-Punkt 7 (c)).
- Umbau aller 20 Stellen in einem Zug (Regel 1 kompiliert erst, wenn alle umgestellt sind; die Bausteine unten staffeln das).

**Pre-Mortem (ein Monat nach dem Bau gescheitert, weil …):**
1. … Episoden zerfallen oder verschmelzen anders als vorher, weil eine Quelle mehr oder weniger trägt als der alte Schlüssel-String. Früh-Indikator: Integrationstests, die „genau eine Beginn-Zeile“ zählen, werden rot; im Betrieb mehr Beginn-Zeilen je Flut. Gegenmaßnahme: Eigenschaft E1 je Katalogzeile; die Gegenüberstellung alter Schlüssel ↔ neue Quelle steht in der Tabelle der ADR-0042 (Spalte Quelle) und wurde für alle 20 Stellen gelesen.
2. … der Mini-Entwickler stellt mechanisch um und übernimmt eine heute falsche Einordnung. Früh-Indikator: Katalogzeile im Test weicht von der Tabelle in ADR-0042 ab. Gegenmaßnahme: die Erwartung kommt aus der ADR-Tabelle, nicht aus dem Code (Kriterium unten); Abweichung = Rückfrage an den architekt, nicht stilles Angleichen.
3. … eine Einstellung, die bisher fest war, wird im Lauf änderbar (z. B. Quote zur Laufzeit) und zerlegt dann eine Episode bei jeder Änderung. Früh-Indikator: Anforderung „Quote im Lauf ändern“. Gegenmaßnahme: Regel 2 nennt die Bedingung „im Lauf fest“; wer sie bricht, ordnet `hoechstens` als Fortlaufendes ein.

**Folgeentscheidungen / Aufgaben** (Bausteine für den `planer`, eine Karte für den Mini-Entwickler, je Baustein ein Commit, Gate grün je Commit):
- [ ] **B1 Typ neben dem alten** (`src/protokoll/episoden.rs`): `Quelle`, `Auftreten::neu` und die Erweiterungen aus Regel 1–2, zunächst abgebildet auf die heutigen Felder (Quelle → `mit` + Schlüssel-String aus den Quellwerten, `.einzeln` → `mit_nur_beginn`, `.fortlaufend` → `mit_fortlaufend`). Test zuerst (Unit, Speicher-Senke, gestellte Uhr): Mechanik-Eigenschaft über ein Test-Auftreten mit Quelle `herkunft=10.0.0.9`, Einzelwerten `grund=a`/`grund=b`, Fortlaufend `dauer_ms=1`/`dauer_ms=2` → eine Beginn-Zeile `| herkunft=10.0.0.9 grund=a dauer_ms=1`, Zwischenzeile und Ende `| herkunft=10.0.0.9 dauer_ms=2` ohne `grund=`; dritte Quelle `10.0.0.8` → zweite Beginn-Zeile.
- [ ] **B2 Bauweisen in `protokoll/` und `start/` umstellen**: `grenze.rs`, `vorlauf.rs`, `start/abriss.rs`, `start/annahmefehler.rs`, `start/mod.rs` (verdrängt, verfallen) — eingebettete herausziehen (Regel 5), auf `Auftreten::neu` umstellen, wörtliche Erwartungen nach Regel 3 nachziehen.
- [ ] **B3 Bauweisen in `http/` umstellen**: `senden.rs` (Schreibfehler, Quote), `bearbeiten.rs` (fremd, Schreibfehler, Quote mit `.einzeln(Route, "bearbeiten")`), `koerper.rs`, `ereignisse.rs`, `bremse.rs`, `auskunft.rs`, `grenzschutz.rs`, `anhaenge/anmelden.rs` (Anspruch, voll, Hochladungen, Platte), `anhaenge/hochladen.rs` (zu langsam, Metadaten).
- [ ] **B4 Alten Weg schließen**: Felder privat, `quelle: String` entfernt, Episodenschlüssel aus der Quelle, Zusammenfassung nur aus Quelle + Fortlaufendem + `bytes`; in `Kontext` entfallen `mit_nur_beginn`, `mit_fortlaufend`, `hat_fortlaufende`, `fuer_ende` und die Markierungslisten. Kompiliert = alle Stellen umgestellt.
- [ ] **B5 Ausführbarer Katalog** (Regel 6) mit einer Zeile je Tabellenzeile aus ADR-0042 „Karte #70“ und dem Vollständigkeitstest; danach die Einzeltests „zwei Auftreten“ aus R00038 entfernen, die er abdeckt (`senden.rs:669`, `start/mod.rs:977`, `anhaenge/anmelden.rs:467`, `koerper.rs:459`), die Mechaniktests in `episoden.rs` und alle Integrationstests unter `tests/` bleiben.
- **Abnahme-Probe** (für den `planer` als Kriterium; heute: der Katalogtest existiert nicht, die Typprobe ist rot — `Auftreten { … }` kompiliert außerhalb von `protokoll`):
  - Katalog grün mit 23 Zeilen (Tabelle ADR-0042 „Karte #70“; Zeilen mit mehreren Texten — Grenzschutz, Schreibkonto — je Text geprüft) und Vollständigkeit: Zahl der Stellen `Auftreten::neu(` = Zahl der verschiedenen Bauweise-Funktionen im Katalog.
  - Typprobe: ein Test-Crate-Schnipsel oder `trybuild` ist **nicht** verlangt; Beleg ist `grep -rn 'Auftreten {' src | grep -v -- '-> Auftreten {'` (Rückgabetypen sind keine Literale) außerhalb von `src/protokoll/` = 0 Treffer und `grep -rn 'mit_nur_beginn\|fuer_ende\|mit_fortlaufend' src` = 0.
  - Mutanten, alle rot: (M1) `kennung` der Schreibfehler-Episode `senden` in die Quelle → E1 (zwei Beginn-Zeilen); (M2) `kennung` der `quote`-Episode als Einzelwert → E2/E3 (Ende ohne `kennung`); (M3) Zusammenfassung nimmt `einzeln` dazu → E2 in jeder Zeile; (M4) Zwischenzeile nimmt den Beginn-Kontext → E2; (M5) Fortlaufendes mit dem ersten statt dem letzten Wert → E4 (`zeit`); (M6) eine Katalogzeile gelöscht → Vollständigkeit; (M7) Schlüssel ohne Quellwerte (nur Bereich + Text) → E1 (C landet in derselben Episode).
  - Integration unverändert grün, darunter `tests/plattenquote.rs`, `tests/senden_protokoll.rs`, `tests/bearbeiten_protokoll.rs`, `tests/grenzen_je_herkunft.rs`, `tests/sendefrist.rs` (wörtliche Beginn-Zeilen nach Regel 3 angepasst, sonst nichts).

## Quellen / Bezuege

- [A00015 Betriebsprotokoll](../Architektur/A00015-betriebsprotokoll.md); Aspekt [X00001 Logging und Diagnose](../Architektur/Aspekte/X00001-logging-und-diagnose.md)
- [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) (Format, Schlüsselliste), [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) (Regel 5, Ergänzungen „Karte #31“ Punkt 4, „Karte #66“ Option D, „Karte #70“), [ADR-0048](0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) Ergänzung „Karte #70“
- Code gelesen (main 6d2aa3f): `src/protokoll/zeile.rs:222–284`, `src/protokoll/episoden.rs:1–260`, `src/protokoll/mod.rs:273–300` und alle 20 Bauweisen (Liste in ADR-0042 „Karte #70“)
- Kanban-Karte #66: Kommentare code-reviewer #66 (Option D fällig), observability-reviewer #66 (Katalogzeilen), entw3 #66 (fünf weitere Funde)
