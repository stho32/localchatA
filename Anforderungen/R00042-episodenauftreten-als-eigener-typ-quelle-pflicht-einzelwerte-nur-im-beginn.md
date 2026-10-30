---
id: R00042
status: Neu
datum: 2026-10-30
---

# R00042: Episodenauftreten als eigener Typ — Quelle Pflicht, Einzelwerte nur im Beginn, ein ausführbarer Katalog prüft jede Bauweise

## Beschreibung

Härtung des Betriebsprotokolls (Karte #70, Option D der Ergänzung „Karte #66“): Die Ende- und Zwischenzeile einer Episode nennen nur, was für die ganze Episode gilt — das sagen R00022 und R00038 zu. Nach acht Episoden, bei denen das jeweils einzeln vergessen und einzeln gefunden wurde, erzwingt der Code die Einordnung selbst:

- **Ein eigener Typ für das Auftreten**: Die Quelle (die Schlüssel, aus denen die Episode gebildet ist) ist Pflichtargument und bildet den Schlüssel der Episode; jeder weitere Wert ist ein Einzelwert und steht nur in der Beginn-Zeile, außer er ist ausdrücklich als fortlaufend eingeordnet (Wert des letzten Auftretens). Wer einen Wert nicht einordnet, erzeugt eine Ende-Zeile mit **weniger**, nie mit einem falschen Wert.
- **Ein ausführbarer Katalog** mit einer Testzeile je Zeile der Tabelle in [ADR-0042](../Dokumentation/ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung „Karte #70“ (Quelle, Beginn, Ende je Episode; 23 Zeilen) prüft jede Bauweise, und ein Vollständigkeitstest macht eine neue Episode ohne Katalogzeile rot.
- Sichtbar ändert sich für die Betreiberin nichts außer der **Reihenfolge der Schlüssel in fünf Beginn-Zeilen** (Quelle zuerst).

Zahlt ein auf: [Vision](R00000-vision.md) — „eine gemeinsame LAN-Instanz“ (die Betreiberin liest morgens das Protokoll und zieht Schlüsse daraus; eine Ende-Zeile mit dem Wert des ersten Auftretens ist ein falscher Befund).

Herkunft: Karte #70 — code-reviewer #66 Anmerkung 3, entw3 #66, observability-reviewer #66. Entschieden vom architekt in [ADR-0050](../Dokumentation/ADRs/0050-episodenauftreten-als-eigener-typ-quelle-pflicht-einzelwerte-nur-im-beginn.md) (Bausteine B1–B5, Variante D1, verworfen D2–D4), [ADR-0042](../Dokumentation/ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) (Ergänzungen „Karte #70“) und [ADR-0048](../Dokumentation/ADRs/0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) (Ergänzung „Karte #70“).

Slice (WBS `Dokumentation/Planung/localchata.md`): F0143 Episodenauftreten als eigener Typ (unter I0021 Betriebsprotokoll), Bubbles B0655–B0659.

Weiche (planer): nichts fehlt und nichts ist falsch gebaut — das Verhalten steht seit R00038 (F0129) fest und ist gebaut; hier wird es im Typ erzwungen und im Test belegt. Eigene Anforderung statt Nachtrag zu R00038: R00038 ist als Satz dreier Härtungen (Karte #66) geschnitten und wird nicht verändert; dies ist ein eigener Auftrag mit eigenen Kriterien (Typ, Katalog, Mutanten) und eigener Abnahme-Probe. R00038 und R00022 bleiben als Zusage bestehen.

### Domänenbezug

Kein Glossar im Projekt. Begriffe wie in R00014, R00022, R00038: Betriebsprotokoll, Bereich, Kontext, Episode (Beginn-Zeile, Zwischenzeile, Ende-Zeile), Quelle, Auftreten, Ruhe; neu: **Einzelwert** (gilt nur für ein Auftreten), **fortlaufender Wert** (Wert des letzten Auftretens).

## Geschäftlicher Nutzen

Acht Mal derselbe Fehler, acht Mal erst im Review gefunden: jeder kostete einen Durchlauf und hätte bis dahin der Betreiberin einen falschen Absender, Channel oder Wert gezeigt. Mit dem Typ kann der Fehler nicht mehr geschrieben werden; mit dem Katalog fällt jede Abweichung der Einordnung von der Tabelle vor dem Merge auf. Jede künftige Episode kostet eine Tabellenzeile statt eines weiteren Fundes.

## Funktionale Anforderungen

### F0143 Episodenauftreten als eigener Typ (ADR-0050)

- **Typ** (`src/protokoll/episoden.rs`): `Auftreten` wird nur über `Auftreten::neu(stufe, bereich, text, quelle)` und die Erweiterungen `.einzeln(k, v)`, `.fortlaufend(k, v)`, `.art(a)`, `.verworfen(v)`, `.bytes(n)`, `.anzeige(t)` gebaut; die Felder sind außerhalb von `protokoll` nicht sichtbar. `Quelle::neu().mit(k, v)` bzw. `Quelle::instanz()` trägt die Schlüssel der Episode; der Schlüssel der Episode ist Bereich + Text + Quellwerte, der bisherige String `quelle` entfällt.
- **Zeilen**: Beginn = Quelle, Einzelwerte, Fortlaufendes (Werte des ersten Auftretens). Zwischen- und Ende-Zeile = Quelle, Fortlaufendes (Wert des letzten Auftretens), größte Bytezahl — aus **einer** Funktion; Einzelwerte werden dafür nicht gelesen. Die Sammelepisode bleibt ohne Kontext.
- **`Kontext`** verliert die Markierungen (`mit_nur_beginn`, `mit_fortlaufend`, `hat_fortlaufende`, `fuer_ende`).
- **Bauweisen**: jede der 20 Stellen ist eine reine Funktion `<name>_auftreten(..) -> Auftreten`, für Tests der Kiste sichtbar; die Einordnung je Bauweise ist die Tabelle in ADR-0042 „Karte #70“ (Quelle, Beginn, Ende), nicht der heutige Code.
- **Katalog**: `src/episodenkatalog.rs` (`#[cfg(test)]`) mit einer Zeile je Tabellenzeile (Eigenschaften E1–E4 und Vollständigkeit, Akzeptanzkriterien unten); er ersetzt die Einzeltests „zwei Auftreten“ aus R00038.
- **Reihenfolge der Bausteine** (je Baustein ein Commit, Gate grün je Commit): B0655 Typ neben dem alten · B0656 `protokoll/` und `start/` · B0657 `http/` · B0658 alten Weg schließen · B0659 Katalog.

## Nicht-funktionale Anforderungen

- **Sicherheit**: nie Nachrichtentext, Token oder Körper im Protokoll (R00014); der Umbau ändert keinen Schlüssel außer der Reihenfolge der fünf Beginn-Zeilen (`quote`, beide Schreibfehler, „Reservierung verfallen“, zwei Anhang-Abweisungen).
- **Gleichwertigkeit der Episoden**: Der Schlüssel wird aus den Quellwerten gerechnet; an zwei Stellen trägt er mehr Werte als heute (Einstellungen des Laufs, Fehlertext statt Nummer) — gleichwertig, solange diese im Lauf fest sind. Wird eine Einstellung im Lauf änderbar, ist sie als Fortlaufendes einzuordnen (ADR-0050 Pre-Mortem 3).
- **Zeitkriterien** nach `AI-Team/planer.md`: keine neuen; die Zwischenzeile (10 Minuten 1 Sekunde) und die Ruhe werden nur mit gestellter Uhr geprobt, nie in Echtzeit. Der Katalogtest läuft ohne Warten.
- **Konsole**: berührt keine Oberfläche.

## Akzeptanzkriterien

Gemeinsam: Speicher-Senke und gestellte Uhr wie in R00014, R00022, R00038; „Zeile“ meint die Zeile ab der Stufe, ohne Zeitstempel. Die Kriterien 1–5 sind die Abnahme-Probe aus ADR-0050 (der Katalog und die Vollständigkeit stehen dort in einem Punkt, hier getrennt).

### F0143 Episodenauftreten als eigener Typ

- [ ] **1 Katalog**: Der Katalogtest ist grün mit **23 Zeilen** (Tabelle ADR-0042 „Karte #70“; Zeilen mit mehreren Texten — Grenzschutz, Schreibkonto — je Text geprüft). Je Zeile: zwei Aufrufe A und B mit gleichen Quellargumenten und paarweise verschiedenen übrigen Argumenten ergeben genau eine Beginn-Zeile, ein dritter Aufruf C mit anderer Quelle eine zweite (E1); Zwischenzeile (Uhr + 10 min 1 s) und Ende-Zeile (Uhr + Ruhe) enthalten keinen Einzelwert von A oder B und jeden Quellwert (E2); die Schlüssel der Beginn-Zeile sind Quelle ∪ Beginn-Spalte, die der Ende-Zeile Quelle ∪ Ende-Spalte, Zählungsklammer und `bytes` ausgenommen, wo die Spalte sie nennt (E3); Fortlaufendes trägt im Ende den Wert von B (E4). Rechenbeispiel (B0655): Quelle `herkunft=10.0.0.9`, Einzelwerte `grund=a`/`grund=b`, Fortlaufend `dauer_ms=1`/`dauer_ms=2` → Beginn `| herkunft=10.0.0.9 grund=a dauer_ms=1`, Zwischen- und Ende-Zeile `| herkunft=10.0.0.9 dauer_ms=2`; Quelle `10.0.0.8` → zweite Beginn-Zeile. Heute rot: der Katalogtest existiert nicht.
- [ ] **2 Vollständigkeit**: Die Zahl der Stellen `Auftreten::neu(` außerhalb von `#[cfg(test)]`-Modulen (Test liest die Dateien unter `src/` mit der Standardbibliothek, `CARGO_MANIFEST_DIR`) ist gleich der Zahl der **verschiedenen Bauweise-Funktionen** im Testkatalog; eine Funktion darf mehrere Katalogzeilen haben (z. B. `grenzauftreten` drei) und enthält genau einen Aufruf `Auftreten::neu(`. Eine neue Bauweise ohne Katalogzeile ist rot.
- [ ] **3 Typprobe**: `grep -rn 'Auftreten {' src | grep -v -- '-> Auftreten {'` (die Funktionsköpfe `-> Auftreten {` ausgenommen) außerhalb von `src/protokoll/` = **0** Treffer und `grep -rn 'mit_nur_beginn\|fuer_ende\|mit_fortlaufend' src` = **0** Treffer. Heute rot: `Auftreten { … }` kompiliert außerhalb von `protokoll`. Ein Test-Crate oder `trybuild` wird nicht verlangt.
- [ ] **4 Mutanten, alle rot** (einzeln eingebaut, Test rot, wieder entfernt): M1 `kennung` der Schreibfehler-Episode `senden` in die Quelle → E1 (zwei Beginn-Zeilen); M2 `kennung` der `quote`-Episode als Einzelwert → E2/E3 (Ende ohne `kennung`); M3 Zusammenfassung nimmt `einzeln` dazu → E2 in jeder Zeile; M4 Zwischenzeile nimmt den Beginn-Kontext → E2; M5 Fortlaufendes mit dem ersten statt dem letzten Wert → E4 (`zeit`); M6 eine Katalogzeile gelöscht → Vollständigkeit; M7 Schlüssel ohne Quellwerte (nur Bereich + Text) → E1 (C landet in derselben Episode).
- [ ] **5 Integration unverändert**: grün bleiben `tests/plattenquote.rs`, `tests/senden_protokoll.rs`, `tests/bearbeiten_protokoll.rs`, `tests/grenzen_je_herkunft.rs`, `tests/sendefrist.rs` — mit den wörtlichen Beginn-Zeilen nach der neuen Reihenfolge (Quelle, Einzelwerte, Fortlaufendes) angepasst, sonst nichts. Die Einzeltests „zwei Auftreten“ aus R00038 (`src/http/senden.rs:669`, `src/start/mod.rs:977`, `src/http/anhaenge/anmelden.rs:467`, `src/http/koerper.rs:459`) sind entfernt, die Mechaniktests in `episoden.rs` und alle Integrationstests unter `tests/` bleiben.
- [ ] **Unverändert grün**: die Kriterien aus R00014, R00018, R00022, R00038 (Beginn-Zeile, Zählungen, Aufteilung je Art, Zwischenzeile gleich Ende-Kontext).

## Betroffene Verzeichnisstruktur

- `src/protokoll/episoden.rs`, `src/protokoll/zeile.rs` (Typ, `Kontext`).
- `src/protokoll/{grenze,vorlauf}.rs`, `src/start/{abriss,annahmefehler,mod}.rs` (B0656).
- `src/http/{senden,bearbeiten,koerper,ereignisse,bremse,auskunft,grenzschutz}.rs`, `src/http/anhaenge/{anmelden,hochladen}.rs` (B0657).
- `src/episodenkatalog.rs` (neu, Test) und die genannten Integrationstests unter `tests/`.
- `Dokumentation/Architektur/A00015-betriebsprotokoll.md`: nur, falls dort `mit_nur_beginn` o. ä. beschrieben ist (Architekt, kein Auftrag an den Entwickler).

## Technische Überlegungen

### Ablauf

1. B0655 Typ neben dem alten (Test zuerst), 2. B0656 und 3. B0657 Aufrufer umstellen, 4. B0658 alten Weg schließen (ab hier kompiliert nur noch der neue), 5. B0659 Katalog; danach die Einzeltests streichen. Reihenfolge fest, ein Commit je Baustein, Gate grün je Commit.
- Weicht der Code für eine Bauweise von der Tabelle ab: Rückfrage an den architekt, nicht stilles Angleichen.

### Grobentwurf

Entscheidung und Alternativen (D2 Schlüssel bleibt daneben, D3 Rolle je Eintrag, D4 nur Katalog) in ADR-0050; hier nicht wiederholt.

## Aufwand (Zählung)

Gezählt aus der WBS: 1 Slice (F0143), 5 Bubbles (B0655–B0659); Spannen laut WBS: B0655 2 h, B0656 2-4 h, B0657 2-4 h, B0658 2 h, B0659 2-4 h → Summe 10 bis 16 h (Richtwert, ungemessen für diese Bubble-Typen). Mini-Entwickler-tauglich, mechanisch: die Einordnung gibt die Tabelle vor. Dateien-Überschneidung mit dem laufenden Teil D von #9 (`src/gespraeche/anhangpruefung/tilger/{isobmff,bild}.rs`): keine; `src/http/anhaenge/hochladen.rs` berührt B0657 nur in `metadaten_auftreten` und beim Aufruf „zu langsam“.

## Warum löst diese Anforderung das Problem? (Pflicht)

- Die Quelle ist nach dem Bau geschlossen und jeder weitere Wert ist Einzelwert: wer vergisst, erzeugt eine kürzere Ende-Zeile statt einer falschen — das Muster der acht Funde ist nicht mehr schreibbar.
- Die Ende-Zeile liest Einzelwerte gar nicht; es gibt keinen Filter mehr, den eine Mutante abschalten könnte (M3).
- Der Schlüssel der Episode und der Quellkontext sind dieselben Daten; ob zwei Auftreten eine Episode oder zwei sind, ist über E1 prüfbar.
- Tabelle und Testkatalog sind dieselbe Liste; Vollständigkeitstest und Review-Punkte (observability-reviewer: neue Episode ⇒ Tabellenzeile und Katalogzeile; code-reviewer: wechselnder Wert in `Quelle::neu()` ist ein Befund) schließen die Lücke, die der Typ allein lässt (eine Quelle, die zu viel trägt).

## Offene Fragen

- **Für den Director/Betreiber**: ADR-0042 Z. 354 nennt `art` bei „Hochladen abgewiesen: Metadaten nicht tilgbar“, gebaut ist sie nicht (`src/http/anhaenge/hochladen.rs:609–620`). Gebaut wird sie hier nicht; kommt sie, ist sie `.einzeln`. Ob sie gebaut oder aus der Katalogzeile genommen wird, ist nicht entschieden.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus R00014, R00022, R00038.

## Notizen

- Zusagen aus R00014, R00018, R00022 und R00038 bleiben; Tests, die in einer Beginn-Zeile die alte Reihenfolge der Schlüssel erwarten, ziehen mit.
- Verworfen (Optionen des architekt, ADR-0050): Schlüssel als eigener String daneben (D2); Rolle je Eintrag im Kontext (D3); nur Katalog ohne Umbau (D4).
- Nachtrag 2026-10-30 (planer, Karte #70, entwicklertester Anmerkung 1): Kriterium 3 Typprobe — `grep -rn 'Auftreten {' src` traf jeden Funktionskopf `-> Auftreten {` und war wörtlich nie erfüllbar; jetzt mit `| grep -v -- '-> Auftreten {'` (wie ADR-0050).
