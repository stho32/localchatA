---
id: R00043
status: Neu
datum: 2026-10-30
---

# R00043: Löschen und Bearbeiten über die API gehärtet — Abbruch, Bremse, Bank, Plattenfehler, Wettläufe, Betreiberfeld

## Beschreibung

Härtung von R00039 und R00040 nach den Reviews von Karte #67 (code-reviewer, security-reviewer, observability-reviewer, `entw2`, `entwicklertester`). Das Verhalten steht; was fehlt, ist seine Belastbarkeit in sechs Lagen:

- **Abbruch des Clients** (F0144): Bricht der Client nach dem Start des Löschens ab, gehen heute Protokollzeile, Episoden und Ereignis verloren (Messung des Reviewers: 11 von 40 Löschungen ohne Spur, nicht nachgemessen). Künftig läuft alles nach dem Commit-Punkt im selben Blocking-Teil. Eine geschlossene Hochladebank oder eine Panik wird beantwortet wie überall (500 ohne Fremdtext, ein Eintrag).
- **Bremse** (F0145): 429 beim Bearbeiten und Löschen sagt „Bearbeitungen“ bzw. „Löschanfragen“ statt „Nachrichten“ und hat eine eigene Episode.
- **Hochladebank** (F0146): Löschen einer Nachricht ohne Anhang wartet nicht auf die Bank.
- **Plattenfehler** (F0147): Ein Tag mit Löschsatz oder Bearbeitung entfällt nicht vor dem Tag der Nachricht, auf die er zeigt; Gelöschtes kommt nach einem Neustart nicht zurück.
- **Betreiberfeld** (F0148): Health `anhaenge_ungekuerzt` zeigt, ob gelöschte Anhänge ungekürzt geblieben sind.
- **Prüfstärke** (F0149): Wettläufe laufen als feste Verschränkungen, die Ladezeit mit Änderungen wird relativ im Debug-Gate geprüft.

Dazu der **Text-Baustein für die Betreiber-Anleitung** (I0019), unten wörtlich.

Zahlt ein auf: [Vision](R00000-vision.md) — „Eigene Nachrichten bearbeiten und löschen“ (gelöscht heißt gelöscht, solange die Nachricht besteht; Agenten wie Menschen) und die Leitplanken „Plattenquote“, „Anhang-Quote“ und „Betriebsprotokoll“ (die Betreiberin liest morgens, was geschah).

Herkunft: Karte #73 — Reviews auf Karte #67; entschieden vom architekt in [ADR-0048](../Dokumentation/ADRs/0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) Ergänzung „Karte #73“ (K1–K9), [ADR-0042](../Dokumentation/ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), [ADR-0043](../Dokumentation/ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md) und [ADR-0046](../Dokumentation/ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) (je Ergänzung „Karte #73“); product-owner F17 (`AI-Team/po-entscheidungen.md` Z. 131–132): Löschen mit `Idempotency-Key` bleibt wie gebaut.

Slices (WBS `Dokumentation/Planung/localchata.md`): **F0144** Löschen überlebt den Abbruch des Clients (B0666, B0667), **F0145** Bearbeiten und Löschen sagen, wovon es zu viele gab (B0668; unter I0032), **F0146** Löschen ohne Anhang wartet nicht auf die Hochladebank (B0669), **F0147** Ein Plattenfehler holt Gelöschtes nicht zurück (B0670), **F0148** Health `anhaenge_ungekuerzt` (B0673), **F0149** Wettläufe und Ladezeit im Gate (B0671, B0672). Außer F0145 unter I0033.

**Eigene Anforderung statt Nachtrag zu R00040 oder R00039** (planer): beide sind nach `Neu` nie verändert (Anforderungs-Workflow) und gelten als Zusage weiter; dies ist ein eigener Auftrag mit eigenen Proben und eigener Abnahme — Muster R00038 → R00042. Nicht Teil von K1 (Wortlaut des Abzugs): gebaut, nur Dokumentation; nicht Teil von K7 (entschieden, kein Baustein); K9 a (Löschen ohne Anhang bleibt `DEBUG`) ist Entscheidung ohne Bau.

### Domänenbezug

Kein Glossar im Projekt. Begriffe wie in R00039, R00040, R00029: Nachricht, Nachrichtenzeit, Kennung, Änderungssatz, Löschsatz, Fassung, Tageslog, Plattenquote, Reserve R, Anhang-Quote (G), Anhang im Zustand `vorhanden` oder `geloescht`, Hochladebank, Blocking-Teil, Commit-Punkt, Episode, Schreibkonto.

## Geschäftlicher Nutzen

Wer morgens nach einem Vorfall das Protokoll liest (Kennung, Zahl der gelöschten Anhänge, ein `ERROR` beim Kürzen), muss sich darauf verlassen können, dass die Zeile da ist, auch wenn der Agent die Verbindung abgerissen hat — sonst steht ein Anhang ungekürzt auf der Platte, und niemand sieht es. Eine Betreiberin, die „Zu viele Nachrichten“ liest, obwohl der Agent nur bearbeitet hat, sucht an der falschen Stelle. Ein Mensch, der einen versehentlich geposteten Schlüssel gelöscht hat, darf ihn nach einem Plattenfehler und Neustart nicht wieder lesen. Jeder dieser Fälle wurde in den Reviews von Karte #67 gefunden; sie kosteten einen Durchlauf und hätten im Betrieb eine falsche Spur gelegt.

## Funktionale Anforderungen

### F0144 Löschen überlebt den Abbruch des Clients (ADR-0048 K2, K3)

- **Regel**: nach dem Commit-Punkt gibt es bis zu Protokollzeile, Episode, Ereignis und Bau der Antwort **keinen `await`**; alles läuft im selben Blocking-Teil, der Handler wartet nur auf dessen Ende. Gilt für `POST …/nachrichten/loeschen` und `POST /api/anhaenge/loeschen`. Ein Abbruch vor dem Commit-Punkt (Warten auf Erlaubnis oder Sperren) ändert nichts; danach läuft alles zu Ende, die Antwort geht ins Leere.
- **Abbruch von Bank und Blocking-Teil**: `Err` aus dem Erwerb der Hochladebank (geschlossen) und `Err` aus dem Blocking-Teil (Panik, Abbruch) beim Löschen einer Nachricht, beim Löschen im Laufwerk, beim Anmelden und beim Verwerfen von Anhängen werden beantwortet wie bei Senden: 500 ohne Fremdtext, genau ein Eintrag (`ERROR … grund=panik`, `EXCEPTION … grund=abgebrochen` bzw. `grund=werkbank_geschlossen`, Fremdtext nur in `ursache`), Satz „Nachricht nicht gelöscht“ bzw. „Anhänge nicht gelöscht“.
- Gleiche Klasse in `senden.rs` und `bearbeiten.rs`: nicht hier, Karte #74.

### F0145 Bearbeiten und Löschen sagen, wovon es zu viele gab (ADR-0048 K4)

- **Ein Konto**, ein Eimer je Kennung und Herkunft (ADR-0036 unverändert); der Gegenstand der 429 benennt, was abgewiesen wurde. **Bearbeiten** (Vorab- und Rest-Buchung): „Zu viele Bearbeitungen von …“, Episode `Bearbeiten gebremst: zu viele Bearbeitungen` (WARNING, Bereich `schreibkonto`, je Kennung). **Löschen einer Nachricht**: „Zu viele Löschanfragen von …“, Episode `Löschen gebremst: zu viele Löschanfragen`, geteilt mit dem Löschen im Laufwerk. **Senden** unverändert. Eine Kennung kann Senden, Bearbeiten und Löschen nebeneinander als offene Episode haben, jede mit eigener Ende-Zeile und Zählung je Grenze.

### F0146 Löschen ohne Anhang wartet nicht auf die Hochladebank (ADR-0048 K5)

- Die Erlaubnis der Hochladebank nimmt nur, wer Anhänge kürzen wird. Vor dem Warten liest der Handler lesend die Zahl der Anhänge der Nachricht in ihrer Fassung (gelöscht, entfallen, fremd oder unbekannt: 0); ohne Anhang läuft der Blocking-Teil ohne Erlaubnis. Die Anhänge einer Nachricht ändern sich nie (Bearbeiten ändert nur den Text), die Vorabprüfung verlangt daher höchstens zu viel, nie zu wenig.

### F0147 Ein Plattenfehler holt Gelöschtes nicht zurück (ADR-0048 K6)

- `channel_pflegen` entfernt einen Tag **nicht**, solange er einen Änderungssatz trägt, dessen Ziel in einem Tag liegt, der noch im Index steht. Der Tag bleibt in Datei und Index, die nächste Pflege versucht beide erneut, in aufsteigender Folge. Der Tag zählt nicht als `NichtEntfallen` (die Ursache steht im `ERROR frist Tag nicht entfallen` des Tags A); er schreibt `DEBUG frist Tag zurückgestellt` (`channel`, `datei`, `ziel`) je Pflege und Tag. Ein Tag **ohne** Änderungssätze entfällt weiter wie bisher, auch einer, der jünger ist als der gescheiterte.
- **Benannter Rest**: ist Tag A dauerhaft nicht entfernbar, bleibt auch B über die Frist hinaus liegen, die Platte wächst um die Tage dahinter bis zur Quote; sichtbar durch die beiden Zeilen, behoben durch die Ursache (Text-Baustein Punkt 5).

### F0148 Der Betreiber sieht im Health-Check, ob gelöschte Anhänge ungekürzt geblieben sind (ADR-0048 K9 b, c; ADR-0043)

- Neues flaches Feld `anhaenge_ungekuerzt` (Momentwert, Zahl): Anhänge gelöschter Nachrichten, deren Kürzen scheiterte und die noch in der Anhang-Quote zählen. 0 im Normalfall; +1 je gescheiterter Anhang (`ERROR anhang Löschen gescheitert`); −1 bei der Heilung nach dem Laden und beim Entfallen des Tags. Ein Zähler, kein Durchlauf des Verzeichnisses je Abruf.
- `anhaenge_geloescht` zählt weiter alle gekürzten Anhänge seit dem Start, auch die der Heilung; die Beschreibung im Health-Text sagt es. Die Heilung steht zusätzlich als `INFORMATION anhang Anhänge nach dem Start gekürzt | anzahl, bytes` — Stand 2026-10-30 nicht gebaut (`grep` auf `main` = 0 Treffer), wird hier gebaut.

### F0149 Wettläufe und Ladezeit mit Änderungen sind im Gate erzwungen (ADR-0048 K8)

- `Gespraeche` bekommt im Test-Build (`cfg(test)`) eine Schranke **vor den Sperren** je Vorgang (`bearbeiten`, `nachricht_loeschen`, `pflegen` je Tag) nach dem Muster von `vor_kuerzen_warten`; kein Knopf der Umgebung, nichts davon im Release.
- Neben dem absoluten Q6-Test (nur Release, bleibt) prüft ein Test im Debug-Gate die Ladezeit mit Änderungen **relativ** zu derselben Last ohne Änderungen.

## Nicht-funktionale Anforderungen

- **Zeitkriterien** nach `AI-Team/planer.md`: (1) „nach 2 s“ in F0144: Uhr ab dem letzten Reset der 40 Verbindungen; Zweck: alle Blocking-Teile sind fertig; Erwartung (Hypothese, ungemessen): unter 0,5 s bei einem Kürzen je Löschung; die Probe fragt bis 2 s ab und wertet erst dann. (2) „unter 500 ms“ in F0146: Uhr ab dem Absenden des Löschens bis zur Antwort; Zweck: kein Hängen an der Bank, die Wartezeit hinter einer vollen Bank wäre Sekunden; Erwartung (Hypothese): unter 50 ms; 500 ms liegen das Zehnfache über der Erwartung, also weit über einem Zuschlag von 30 %. (3) Ladezeit in F0149: Grenze 3 × (ohne Änderungen) + 100 ms, die kleinere von zwei Messungen je Seite; Rechenbeispiel: ohne 400 ms → Grenze 1.300 ms. Zweck: ein lineares Suchen je Änderungssatz sprengt jede Grenze dieser Art; der Test übernimmt die Grenzen der ADR, nie schärfer. (4) Abwesenheit einer Zeile oder eines Ereignisses wird über 2 s beobachtet.
- **Konsole**: berührt keine Oberfläche; „keine Konsolenfehler“ entfällt.
- **Sicherheit**: das Protokoll nennt nie Nachrichtentext, Anhangname oder ID (R00014, R00040); die Fremdtexte der Abbrüche stehen nur in `ursache`. Review-Punkt für `code-reviewer` und `observability-reviewer` (ADR-0042 Ergänzung „Karte #73“ (4)): nach dem Commit-Punkt kein `await` bis Protokollzeile und Ereignis.
- **Neustart**: was mit 200 beantwortet wurde, gilt nach einem Neustart genau so, auch nach einem Plattenfehler beim Entfernen eines Tags (F0147).

## Akzeptanzkriterien

Gemeinsam, wenn nicht anders genannt: Start mit `TZ=Europe/Berlin localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser --plattenquote 10MiB --anhangquote 10MiB --protokoll-stufe debug` auf einem Wegwerf-Ordner mit den leeren Ordnern `channels/allgemein/` und `channels/technik/`; Beitritte, Sitzungen, „Nachricht mit Anhängen“ und „Tageslog unverändert“ wie in R00040 und R00029. Die Proben der ADR gelten wörtlich; **jeder Test beginnt rot** (Test zuerst).

### F0144 Löschen überlebt den Abbruch des Clients

- [ ] **Abbruch, Unit** (Handler, Speicherablage, `vor_kuerzen_warten`-Schranke, Protokoll im Speicher, Strom-Abonnent): Löschen einer Nachricht mit einem Anhang; der Blocking-Teil steht an der Schranke, der Test lässt die Zukunft des Handlers fallen, öffnet die Schranke, wartet auf das Ende → genau eine Zeile `DEBUG aendern Nachricht gelöscht`, eine `INFORMATION anhang Anhänge gelöscht … grund=nachricht`, ein Ereignis `anhaenge`; dasselbe für das Löschen im Laufwerk (Ereignis `anhaenge`); mit Kürzen-Fehler: eine `ERROR anhang`-Zeile. Mutant „Protokoll nach dem `await`“ rot.
- [ ] **Abbruch, Integration** (echtes Programm, `--protokoll-stufe debug`, ein zweiter Strom-Abonnent): 40-mal Nachricht mit Anhang anlegen, den Körper senden und die Verbindung sofort zurücksetzen (RST, Antwort nie gelesen) → nach 2 s 40 Zeilen `Nachricht gelöscht`, 40 `Anhänge gelöscht`, 40 Ereignisse `anhaenge`, Health `anhaenge_geloescht` 40 (Messwert der Reviewer vorher: 29 von 40).
- [ ] **Geschlossene Bank und Panik, Unit**: geschlossene Bank (`close()`) → 500, genau eine `EXCEPTION programmfehler … grund=werkbank_geschlossen operation=nachricht_loeschen kennung=… channel=…`; Panik im Blocking-Teil (Testknopf `LOCALCHATA_TESTPANIK=loeschen`, nur Debug-Build, wie `verlauf`/`takt`; anderer Wert Exit 2, im Release nicht gelesen) → 500, genau eine `ERROR programmfehler … grund=panik`, danach funktioniert Senden unverändert. Dasselbe mit den Operationen `anhaenge_loeschen`, `anhaenge_anmelden`, `anhaenge_verwerfen`. Heute rot (keine Zeile).

### F0145 Bearbeiten und Löschen sagen, wovon es zu viele gab

- [ ] **Drei Gegenstände, drei Episoden** (Unit, Schreibkonto mit gestellter Uhr, Protokoll im Speicher): dieselbe Kennung wird nacheinander beim Senden, Bearbeiten und Löschen gebremst → drei 429 mit „Zu viele Nachrichten/Bearbeitungen/Löschanfragen von …“, drei Episoden-Beginn-Zeilen mit je eigenem Text, keine zweite Beginn-Zeile bei weiteren 429 desselben Gegenstands; `Retry-After` unverändert. Heute rot: Bearbeiten und Löschen sagen „Zu viele Nachrichten“ und teilen die Senden-Episode. Tests, die den alten Text erwarten, werden angepasst.
- [ ] **Katalog** (nach Merge von Karte #70): `Bearbeiten gebremst: zu viele Bearbeitungen` hat eine Bauweise-Funktion und eine Zeile im Katalogtest (dann 24); der Vollständigkeitstest ist grün.

### F0146 Löschen ohne Anhang wartet nicht auf die Hochladebank

- [ ] **Bank voll** (Integration, echtes Programm): 8 Hochladungen halten die Bank (Health `hochladebank_belegt` 8) → Löschen einer Nachricht **ohne** Anhang antwortet 200 in unter 500 ms; Löschen einer Nachricht **mit** Anhang wartet und gelingt nach dem Ende einer Hochladung. Mutant „Erlaubnis immer“ rot.

### F0147 Ein Plattenfehler holt Gelöschtes nicht zurück

- [ ] **Tage A, B, C** (Unit, Speicherablage mit Fehler beim Entfernen von Tag A, Frist so, dass A und B entfallen): Tag A: Nachricht M mit einem Anhang; Tag B: Löschsatz zu M und Bearbeitung einer Nachricht N aus A; Tag C: nur eigene Nachrichten (entfällt auch). Pflege → A `NichtEntfallen`, B bleibt in Ablage und Index, C entfällt; `Verlauf` zeigt M weiter `geloescht` und N mit der bearbeiteten Fassung; Neuladen → dasselbe, der Anhang von M bleibt `geloescht`; Fehler behoben, Pflege → A und B entfallen. Mutant „B entfällt trotzdem“ rot (Neuladen zeigt M mit Text). Heute rot.
- [ ] **Protokoll**: je Pflege genau eine `DEBUG frist Tag zurückgestellt` mit `channel`, `datei`, `ziel` für B; B zählt nicht in `NichtEntfallen`.

### F0148 Der Betreiber sieht im Health-Check, ob gelöschte Anhänge ungekürzt geblieben sind

- [ ] **Zähler** (Unit): Löschen einer Nachricht mit drei Anhängen, Kürzen-Fehler für einen → `anhaenge_ungekuerzt` 1 und `anhang_belegt` bleibt um dessen Zählmaß höher; Neustart mit Heilung → 0; Entfallen des Tags → 0; Mutante „zählt nie zurück“ rot. Rechenbeispiel: drei Anhänge mit z = 100.000, 200.000 und 300.000 Bytes, der mittlere scheitert → `anhang_belegt` sinkt von 600.000 auf 200.000, `anhaenge_ungekuerzt` 1; nach der Heilung 0 und `anhang_belegt` 0.
- [ ] **Heilungszeile** (Integration): nach dem Neustart mit der Heilung steht genau eine `INFORMATION anhang Anhänge nach dem Start gekürzt` mit `anzahl=1` und `bytes=…`, ohne ID und ohne Anhangname; `anhaenge_geloescht` zählt die Heilung mit, der Health-Text sagt es.

### F0149 Wettläufe und Ladezeit mit Änderungen sind im Gate erzwungen

- [ ] **Drei Verschränkungen** (Unit, Haken vor den Sperren): (1) Bearbeiten steht an der Schranke, Löschen läuft vollständig durch, Bearbeiten wird freigegeben → 409 `Geloescht`, im Log kein `bearbeitet` hinter dem Löschsatz; (2) Löschen steht an der Schranke, Bearbeiten läuft durch, Löschen freigegeben → beide Sätze in dieser Folge, Endzustand gelöscht, Neuladen gleich; (3) Löschen steht an der Schranke, die Pflege entfernt den Tag der Nachricht, Löschen freigegeben → 404 oder gelöscht, je Anhang genau ein Abzug. Mutanten „Bearbeiten ignoriert `Geloescht`“ und „Bearbeiten prüft `Geloescht` vor der Sperre“ werden in (1) rot. Der Test mit den 100 Paaren bleibt als Stichprobe. Ist F0147 gebaut, prüft (3) zusätzlich, dass der Tag mit dem Löschsatz nicht vor dem Tag der Nachricht entfällt.
- [ ] **Ladezeit relativ** (Debug-Gate, `tests/leistung_aenderungen.rs`): 30.000 Nachrichten, jede zehnte dreimal bearbeitet, jede zwanzigste gelöscht, je Seite zweimal gestartet, der kleinere `ladezeit_ms` zählt; Grenze mit Änderungen höchstens 3 × ohne + 100 ms. Mutant „lineare Suche der Nachricht je Änderungssatz beim Laden“ rot. Der absolute Q6-Test (nur Release) bleibt.

### Text-Baustein Betreiber-Anleitung (I0019)

- [ ] **Der Text steht unten wörtlich und ist in der Notiz von I0019 verzeichnet** (grep `Karte #73` in der Notiz von I0019 = 1 Treffer). Wird die Anleitung für I0019 geschrieben, enthält sie jeden der sieben Punkte sinngemäß und mit dem genannten Wert; Prüfung dann je Punkt mit einer Stichwortsuche im Anleitungstext (`Länge 0`, `Reserve`, `Idempotency-Key`, `404`, `403`, `Plattenfehler`, `Text`). Die Anleitung wird nicht in dieser Anforderung gebaut; die Prüfung ist offen, bis I0019 eine Anforderung hat.

> **Text-Baustein.** Eine gelöschte Nachricht verschwindet aus jeder Sicht, aber nicht von der Platte.
> 1. Der **Text** der Nachricht und der **Name ihrer Anhänge** (Zeile `+ anhang:`) bleiben im Tageslog, bis dieser Tag entfällt.
> 2. Die **Datei** eines gelöschten Anhangs bleibt als Datei der Länge 0 mit ihrem (gekürzten) Dateinamen im Ordner `anhaenge/<tag>/`, bis der Tag entfällt; sie wird nicht entfernt.
> 3. Eine **bearbeitete** Nachricht trägt ihre frühere Fassung ebenso bis zum Entfallen des Tags.
> 4. Die **Reserve R** der Plattenquote (`max(1 MiB, 1 % der Quote)`, bei 4 GiB 42.949.673 Bytes, nach ADR-0048 etwa 280.000 Löschsätze) kann durch Löschen aufgebraucht werden. Dann antwortet auch das Löschen 507, und das Senden aller bleibt gesperrt, bis Tage entfallen. Abhilfe: Quote anheben (`--plattenquote`) oder Frist verkürzen.
> 5. Ein **Plattenfehler** beim Entfernen eines Tags hält die Tage mit Löschsätzen zu seinen Nachrichten zurück; die Ursache zu beheben genügt. Sichtbar durch `ERROR frist Tag nicht entfallen` und `DEBUG frist Tag zurückgestellt`.
> 6. Ein gesetzter **`Idempotency-Key` beim Löschen wirkt nicht** und ist kein Fehler: eine Wiederholung antwortet 200 `schon_geloescht` ohne die Bilanz der Anhänge der ersten Antwort. Wer die erste Antwort verloren hat, liest die Bilanz im Health-Check (`anhang_belegt`, `anhaenge_geloescht`, `anhaenge_ungekuerzt`).
> 7. **404** heißt auch „entfallen“ (der Tag der Nachricht ist abgelaufen); **403 `fremd`** nennt den Absender.

## Betroffene Verzeichnisstruktur

- Strom I: `src/http/nachricht_loeschen.rs`, `src/http/anhaenge/loeschen.rs`, `anmelden.rs`, `verwerfen.rs`, `src/http/bremse.rs`, `src/http/bearbeiten.rs`, `src/protokoll/episodentexte.rs`, `src/gespraeche/nachricht_loeschen.rs` (`anhangzahl`), `src/gespraeche/anhaenge.rs`, `loeschen.rs`, `src/http/gesundheit.rs`.
- Strom II: `src/gespraeche/pflege.rs`, `src/gespraeche/verlaufsindex.rs`.
- Strom III: `src/gespraeche/mod.rs` (Haken, `cfg(test)`), Tests in `nachricht_loeschen.rs` und `pflege.rs`, `tests/leistung_aenderungen.rs`.
- Tests der Slices: Unit in den Modulen; Integration `tests/nachricht_loeschen*.rs`, `tests/anhaenge_loeschen.rs`.
- Architektur nach dem Bau (architekt): A00004, A00006, A00016.

## Technische Überlegungen

### Ablauf (Vorplanung — welche Bubbles es am Ende werden, entscheidet der Entwickler)

1. **Strom I** (eine Hand, fester Ablauf, je Baustein ein Commit): B0666 → B0667 → B0668 → B0669 → B0673.
2. **Strom II**: B0670, unabhängig.
3. **Strom III**: B0671, B0672, unabhängig; (3) in B0671 prüft nach B0670 zusätzlich K6.

### Grobentwurf

- Handler: `spawn_blocking` trägt Wirkung, Protokoll, Episoden, Ereignisse und Antwortbau; Rückgabe ist die fertige Antwort.
- `Gespraeche::anhangzahl(gespraech, zeit) -> usize` (lesend); `Gegenstand::Bearbeitungen` neben `Loeschungen`; `Pflegebericht::Zurueckgestellt`; Zähler `ungekuerzt` im Anhangzähler.

## Aufwand (Zählung)

Gezählt aus der WBS (2026-10-30): 6 Slices, 8 Bubbles (B0666–B0673); B0667, B0668, B0672, B0673 je 2 h (8 h), B0666, B0669, B0670, B0671 je 2-4 h (8–16 h) → 16 bis 24 h, Richtwert, ungemessen für diese Bubble-Typen. Strom I 10–14 h, Strom II 2–4 h, Strom III 4–6 h. Mini-Entwickler-tauglich: alle außer B0670. **Dateien-Überschneidung** mit #70 (F0143, B0657: `bearbeiten.rs`, `bremse.rs`, `anhaenge/anmelden.rs`; B0659 Katalog): Strom I erst nach dem Merge von #70; II und III berühren nichts daraus. #74 (`senden.rs`, `bearbeiten.rs`) nach Strom I.

## Abhängigkeiten

- Abhängig von: R00039 und R00040 (F0132, F0136, F0137, F0140 grün), Karte #70 (Merge vor Strom I), R00029 (Hochladebank, Anhang-Zustände).
- Blockiert: Karte #74 (gleiche Klasse in Senden und Bearbeiten, nach Strom I); die Anleitung I0019 (Text-Baustein).

## Offene Fragen

- **An den Director/Betreiber** (nicht blockierend): PO F17 verlangt, dass „OpenAPI und Anleitung I0019“ die Wirkungslosigkeit des `Idempotency-Key` beim Löschen sagen. Eine Route `/api/openapi.json` steht in A00004, im Code ist sie nicht vorhanden (`grep openapi src` = 0 Treffer, Stand `1ed27c8`). Das Kriterium gilt hier für die Anleitung (Text-Baustein Punkt 6); der OpenAPI-Teil wird mit der OpenAPI-Beschreibung gebaut — Empfehlung: dort mit aufnehmen, kein Baustein in dieser Karte.
- **An den Director** (Pflege der Projektschicht): `AI-Team/umgebung.md` kennt `LOCALCHATA_TESTPANIK=verlauf|takt`; B0667 fügt `loeschen` hinzu.
- **Für den architekt** (nicht blockierend): ADR-0048 K7 und der Kopf (Status „K7 offen“) auf „entschieden (F17 Option 1)“ — der Director plant es für den nächsten Kontakt (#72).

## Missing-Docs

- Glossar fehlt; Begriffe aus ADR-0048, R00039, R00040.
- Die Messung „11 von 40“ steht nur im Kommentar des Reviewers auf Karte #67, nicht im Repository.

## Warum löst diese Anforderung das Problem? (Pflicht)

- Bricht ein Client mitten im Löschen ab, dann verwirft hyper die Handler-Zukunft am `await`, und Protokollzeile, Episode und Ereignis fehlen (11 von 40 beim Reviewer). Wenn die Regel „kein `await` nach dem Commit-Punkt“ gilt und der Blocking-Teil alles schreibt, dann bleibt von 40 abgebrochenen Löschungen keine ohne Spur; die Integrationsprobe zählt 40 von 40.
- Ein falscher Text der Bremse („Nachrichten“ für einen Agenten, der nur bearbeitet hat) lässt die Betreiberin an der falschen Stelle suchen; mit eigenem Gegenstand und eigener Episode steht im Protokoll, was gebremst wurde.
- Achtet das Löschen auf die Bank nur, wenn es Platte kürzt, dann hängt eine Nachricht ohne Anhang nicht hinter acht Hochladungen.
- Ein Plattenfehler an Tag A nahm bisher dem jüngeren Tag B den Beleg einer Löschung; hält die Pflege B zurück, bleibt „gelöscht“ nach dem Neustart gelöscht.
- Ein Zähler im Health-Check beantwortet „ist noch etwas offen?“ ohne Logsuche; die feste Verschränkung der Tests macht den Mutanten zuverlässig rot statt nur zufällig.

## Notizen

- Verworfen (ADR-0048): Code auf „der Löschende zieht ab“ zurückbauen (K1); abgekoppelte Aufgabe oder `Drop`-Wächter statt Blocking-Teil (K2); Bank in `Werkbank` aufgehen lassen (K3); alles unter „Nachrichten“ lassen oder vierter Gegenstand (K4); Erlaubnis erst im Blocking-Teil holen oder eigene Bank (K5); erster Fehler bricht die Pflege des Channels ab, benannte Grenze statt Bau, Löschung zusätzlich in den Tag der Nachricht (K6); mehr Wiederholungen statt Haken, Q6 absolut im Debug-Build (K8); `INFORMATION` je Löschen, eigenes Feld `anhaenge_geheilt` (K9).
- K7 / F17 (PO): Option 2 (Wiederholschlüssel mit Bilanz als Wert, 640 je Kennung, 10 min) ist verworfen; DELETE ist von sich aus wiederholbar (RFC 9110 §9.2.2).
- Zusagen aus R00014, R00039, R00040 bleiben; Tests, die den alten Text „Zu viele Nachrichten“ beim Bearbeiten oder Löschen erwarten, ziehen mit F0145 mit.
