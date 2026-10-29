# ADR-0048: Eigene Nachrichten bearbeiten und löschen — Änderungssatz im Tageslog der Änderung mit Bezug auf die Nachrichtenzeit, Fassung je Nachricht im Index, zwei Routen, Thema `aenderung` im Strom, der Löschsatz ist der Commit-Punkt für die Anhänge

**Status**: Akzeptiert (Regeln 1–11); **Vorgeschlagen** für Regel 12 (Rückfrage beim Löschen) — wartet auf die Betreiber-Frage F15 unten
**Datum**: 2026-10-29
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #48 I0032/I0033); Richtung vom Betreiber (sechs Richtungsfragen, Karte #48, 2026-10-29)
**Kontext-Anforderung**: — (WBS I0032, I0033; prägt I0026, I0011, I0012, I0013, I0016, I0027)
**Erprobt durch**: — (überträgt Gebautes und Entschiedenes: Satz anhängen mit Rückbau ADR-0011, Zeit unter der Schreibsperre ADR-0035 `Gespraeche::zeit_vergeben`, Zähler `belegt` ADR-0044, Wiederholschlüssel `src/gespraeche/wiederholung.rs`, Feldleser ADR-0046 Ergänzung „Security-Gate“ L9, Löschen in drei Schritten ADR-0046 Ergänzung „F1′“ Regel 5; begründet am Code: die Spanne eines Satzes endet heute vor dem nächsten Satzanfang, `src/gespraeche/verlaufsindex.rs` `Tag::spanne`)

## Kontext

- **Vision** (Stand c256cbb), Zielbild „Eigene Nachrichten bearbeiten und löschen“: der Absender ändert den Text nachträglich oder löscht die Nachricht, **solange sie besteht**; Mensch im Browser und Agent über die API gleich; Absender ist jede Sitzung mit derselben Kennung `name@herkunft`, auch eine spätere; im Verlauf „bearbeitet“ mit **nur dem neuen Text** („frühere Fassungen sieht niemand“) bzw. „gelöscht“; der Text einer gelöschten Nachricht **bleibt im Log, bis sein Tag entfällt**; ihre **Anhänge werden mitgelöscht** wie im Laufwerk, ihr Platz in der Anhang-Quote wird frei. Leitplanke Plattenquote: „auch eine gelöschte Nachricht gibt keinen Platz frei“.
- **WBS** I0032, I0033 (Fertig-Kriterien: alle offenen Sichten sofort, nach dem Neuladen und über die API ebenso; fremde Kennung abgewiesen, Nachricht unverändert); I0026 Reaktionen „Fragen an den architekt“ 1–3 (Tagesformat, Strom-Ereignis, Grenzen) — dieselbe Frage „eine bestehende Nachricht ändert sich im nur angehängten Log“.
- [ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md): Satz = Kopfzeile `@ <zeit> <kennung>`, Metazeilen `+ schluessel: wert` (unbekannte bleiben erhalten), eingerückter Text; lesbar mit `less`.
- [ADR-0005](0005-programm-als-einziger-schreiber-und-reihenfolge.md), [ADR-0035](0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md): die Nachrichtenzeit ist ID, global streng steigend unter **einer** Schreibsperre; die Tagesdatei ist das lokale Datum der vergebenen Zeit; „Datei und Zeit stimmen immer überein“; jede Zeit im Ordner ist eindeutig.
- [ADR-0006](0006-verlaufsindex-im-speicher.md), [ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md), [ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md): je Tag Rohtext plus Satzanfänge, Block über die Zeit als Cursor mit binärer Suche über Kopfzeiten; Q1 neueste 100 in < 50 ms, Q6 Start < 2 s; Felder kommen hinzu, keines wird umgedeutet.
- [ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md) Regel 5: der Tab verwirft eine Nachricht mit Zeit ≤ bekannt; [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzung „F1′“ Regel 3 verwarf E-b („Nachricht als geändert neu senden … bricht die Regel, dass Nachrichten unveränderlich sind“) — diese Regel hebt die Vision jetzt auf.
- [ADR-0030](0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md): Entfallen ganzer Tage unter der Gesprächssperre; [ADR-0044](0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md): `belegt = Σ Verlauf::bytes()`, „jede Route, die Gesprächsbytes schreibt“ antwortet an der Quote 507.
- [ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) (Prüfreihenfolge, Textregeln, Körpergrenze 128 KiB, Wiederholschlüssel), [ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md) (Konto, Kosten ⌈Bytes/4.096⌉), [ADR-0012](0012-identitaet-und-sitzung.md) (Kennung = Name + kanonische IP der Gegenstelle, nie vom Client).
- [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „F1′“ Regel 5 (Vergleichen-und-Tauschen `vorhanden → geloescht` unter der Schreibsperre, Kürzen außerhalb, Abzug danach; Regel 8: eine Datei der Länge 0 heißt beim Laden `geloescht`); Ergänzung „Security-Gate“ L2 (Pfade über Deskriptoren) und L9 (Feldleser, doppelte Schlüssel 422).

**Qualitätsziele dieser Entscheidung (Szenarien mit Maß):**
- **QE1 Live**: Absender ändert eine Nachricht → jede offene Sicht zeigt den neuen Zustand < 1 s nach der Antwort (Q3).
- **QE2 Öffnen bleibt schnell**: 100.000 Nachrichten, davon 10.000 bearbeitet (je dreimal) und 5.000 gelöscht → neueste 100 weiterhin < 50 ms Server-Zeit (Q1), Start < 2 s (Q6).
- **QE3 Neustart**: was mit 200 beantwortet wurde, gilt nach einem Neustart genau so (ADR-0011 sinngemäß) — auch für Anhänge einer gelöschten Nachricht.
- **QE4 Nur der Absender**: eine andere Kennung ändert nichts — weder Log, Index, Strom noch Anhänge.
- **QE5 Lesbar ohne localchatA**: wer `less` auf die Tagesdatei richtet, versteht, dass und wann eine Nachricht bearbeitet oder gelöscht wurde.

## Optionen

### Frage A — Wo steht eine Änderung?

| | Form | Stärke | Schwäche |
|---|---|---|---|
| **A1 (gewählt)** | **Änderungssatz im Tageslog der Änderung**: ein gewöhnlicher Satz mit eigener Kopfzeile (eigene Nachrichtenzeit, Kennung des Ändernden) und einer Bezugs-Metazeile `+ bearbeitet: <zeit der Nachricht>` bzw. `+ geloescht: <zeit>`, an die heutige Datei angehängt | nur angehängt, nur an die Datei, die ohnehin offen ist; „Datei und Zeit stimmen überein“ bleibt (ADR-0035 Regel 5); die Zeit ordnet auch Änderungen global — Strom, Nachholen und Wiederholschlüssel brauchen kein neues Merkmal; mit `less` lesbar; trägt Reaktionen (I0026) in derselben Form | eine Änderung kann ihre Nachricht überleben (Waise, unsichtbar, zählt an der Quote, bis ihr Tag entfällt); beim Laden muss über Tage hinweg zusammengeführt werden |
| A2 | Änderungssatz an die Tagesdatei **der Nachricht** anhängen | Änderung und Nachricht entfallen gemeinsam, kein Zusammenführen über Tage | eine Zeit von heute in einer Datei von gestern — bricht „Datei und Zeit stimmen überein“ und die Sortierung der Kopfzeiten, auf der die binäre Suche des Blocks steht (ADR-0025 Regel 4); ein zweiter Schreibort je Gespräch; ein Leser sieht am Dateiende Sätze aus der Zukunft — verworfen |
| A3 | eigene Datei `<tag>.aenderungen.log` neben dem Tageslog | Nachrichtenlog bleibt rein | zweite Dateiart mit eigener Erlaubnisliste beim Löschen (ADR-0032), eigenem Zeilenende-Wächter, eigener Quote-Zählung; ein Leser braucht zwei Dateien — verworfen |
| A4 | Satz an Ort und Stelle umschreiben (Tilgen, Muster ADR-0030) | Platte zeigt nur die neue Fassung | widerspricht der Betreiber-Entscheidung „nur aus der Sicht“ und „Logs bleiben nur fortgeschrieben“; Rewrite eines Tages je Änderung (ADR-0011 Option B) — verworfen |

### Frage B — Form im Log und frühere Fassung

| | Form | Stärke | Schwäche |
|---|---|---|---|
| **B1 (gewählt)** | Kopfzeile `@ ` wie jeder Satz, Bezugs-Metazeile, beim Bearbeiten der **ganze neue Text** eingerückt; die frühere Fassung bleibt, wo sie steht | zeilensicher nach ADR-0004 ohne neue Grammatik; der Text der aktuellen Fassung liegt als Bytes im Index, Block und Suche kopieren ihn wie einen Nachrichtentext | ein älteres Programm (Rückstufung) zeigte Änderungssätze als neue Nachrichten — sichtbar, nichts geht verloren; hingenommen |
| B2 | eigenes Zeichen in Spalte 1 (`~ `) | ältere Programme sehen keine Nachricht | ADR-0004: unbekannte Spalte 1 ist defekt — die eingerückten Zeilen danach hängte ein älterer Parser an den **vorigen** Satz an und verfälschte ihn — verworfen |
| B3 | Unterschied (Diff) statt ganzem Text | weniger Bytes bei kleinen Korrekturen | Fassung nur über die Kette aller Änderungen rekonstruierbar; fällt ein Glied durch Defekt aus, ist jede spätere falsch; mit `less` unlesbar — verworfen |

**Frühere Fassung im Log (Hinweis des planer):** sie **bleibt** im Tageslog, bis ihr Tag entfällt — wie der Text einer gelöschten Nachricht. Gründe: die Logs werden nur fortgeschrieben (Betreiber, Richtungsfrage 1); A4 ist verworfen; „frühere Fassungen sieht niemand“ heißt: keine Sicht und keine API liefert sie (Betreiber, Richtungsfrage 3: „keine Einsicht in frühere Fassungen“). Wer die Dateien des Ordners liest (Betreiber, Agent mit Dateizugriff), sieht sie — wie gelöschte Texte. In Kauf genommen.

### Frage C — Lesen: wie kommt die aktuelle Fassung in Block, Strom und Suche?

| | Verfahren | Stärke | Schwäche |
|---|---|---|---|
| **C1 (gewählt)** | **Fassung je Nachricht** im Verlaufsindex: je Gespräch eine geordnete Abbildung `Nachrichtenzeit → {geaendert, zustand: bearbeitet(Ort des Änderungssatzes) \| geloescht}`, beim Laden und beim Schreiben gepflegt; der Block schlägt je kopierter Nachricht einmal nach | O(log m) je Nachricht im Block (m = geänderte Nachrichten), unabhängig von der Zahl der Änderungen; Speicher je **geänderter Nachricht**, nicht je Änderung; Entfallen nimmt die Einträge des Tages per Bereich heraus | ein zweiter Zustand im Index, der beim Laden aus allen Tagen gebildet wird |
| C2 | beim Block die Änderungssätze späterer Tage durchsuchen | kein Zustand | Kosten wachsen mit der Zahl der Änderungen und Tage — reißt Q1 bei vielen Änderungen — verworfen |
| C3 | Fassung beim Laden in den Rohtext „einbauen“ (Index weicht von der Datei ab) | Block unverändert | Index ≠ Datei bricht ADR-0006 („die Datei im Speicher ist dieselbe wie auf Platte“) und `belegt = Σ bytes` (ADR-0044) — verworfen |

### Frage D — API

| | Route | Stärke | Schwäche |
|---|---|---|---|
| D1 | `PUT/PATCH/DELETE /api/channels/{name}/nachrichten/{zeit}` | REST-Form | die Zeit im Pfad (`+`, `:`) muss kodiert werden — der Fehler, den ADR-0025 bei `vor=` schon als 422 abfängt, träfe jeden Agenten neu; `DELETE` mit Körper (Wiederholschlüssel-Prüfsumme, später `bestaetigt`) ist unüblich — verworfen |
| **D2 (gewählt)** | **`POST …/nachrichten/bearbeiten`** `{zeit, text}` und **`POST …/nachrichten/loeschen`** `{zeit}` | Zeit im JSON ohne Kodierproblem; JSON mit `application/json` ist kein einfacher Cross-Origin-Request (ADR-0014); dieselbe Form wie `POST /api/anhaenge/loeschen` (ADR-0046 F1′ Frage I); je Route eigene Körpergrenze, eigene Kosten | `POST` mit Verb im Pfad statt Methoden |
| D3 | eine Route `POST …/nachrichten/aendern` `{zeit, art, text?}` | eine Route | zwei Prüfreihenfolgen, Kosten und Grenzen hinter einem Feld `art`; I0026 käme als dritte `art` dazu — die Route wüchse zu einem Sammelbecken — verworfen |

### Frage E — Strom

| | Ereignis | Stärke | Schwäche |
|---|---|---|---|
| **E1 (gewählt)** | **Thema `aenderung`** mit der ganzen Nachricht im neuen Zustand und `geaendert` (Zeit des Änderungssatzes); der Tab übernimmt, wenn `geaendert` größer ist als sein bekanntes | idempotent und unabhängig von der Reihenfolge (die neuere Änderung gewinnt, gelöscht ist endgültig); kein Zähler, kein Stand — `geaendert` stammt aus derselben Uhr wie die Nachrichtenzeit; I0026 schickt Zählerstände in derselben Form | ein Thema mehr; ein Ereignis trägt bis zu 16 KiB Text (wie `nachricht`) |
| E2 | `nachricht` erneut senden | kein neues Thema | Tab verwirft nach ADR-0028 Regel 5 (Zeit ≤ bekannt) — bräuchte eine Ausnahme mitten in der wichtigsten Zusammenführungsregel; Agenten hielten eine Änderung für eine neue Nachricht — verworfen |
| E3 | nur Hinweis `{channel, zeit}`, Tab holt per API | kleiner Strom | je Änderung und offenem Tab ein Abruf; Latenz gegen QE1 (wie ADR-0028 Option D) — verworfen |
| E4 | `stand`-Zähler je Channel | Form wie Teilnehmer/Channels | zweites Ordnungsmerkmal, beginnt nach Neustart bei 0 (ADR-0028 Option B) — verworfen |

### Frage F — Nachholen nach einer Lücke

| | Verfahren | Stärke | Schwäche |
|---|---|---|---|
| **F1 (gewählt)** | nach Wiederverbinden oder `NeuerStand` lädt der Tab die Blöcke seines Fensters neu (≤ 5, ADR-0026) und übernimmt `zustand`/`text` bekannter Nachrichten mit größerem `geaendert` | keine neue Route, keine neue Zusage; dieselbe Regel wie für gelöschte Anhänge (ADR-0047 F1′ Regel 3); Kosten ≤ 5 Abrufe je Lücke, Lücken sind selten | ein Agent, der nur `nach=` liest, erfährt Änderungen an älteren Nachrichten nicht |
| F2 | Route `GET …/aenderungen?nach=<zeit>` über die Liste der Änderungssätze | verlustfreies Nachholen für Agenten mit **einem** Zeiger, O(log n) über dieselbe Mechanik wie Blöcke | eine Route und eine Zusage ohne Szenario in I0032/I0033 — **Folgeoption**, fällig, sobald I0016 (`warten`) oder ein Agent sie braucht; der Index führt die Änderungssätze ohnehin geordnet (Regel 3), die Route wäre klein |

### Frage G — Quote und Löschen

| | Regel | Stärke | Schwäche |
|---|---|---|---|
| G1 | Bearbeiten und Löschen wie Senden: 507 an der Quote | eine Regel | an der vollen Quote kann niemand eine Nachricht verbergen, die versehentlich ein Geheimnis enthielt — bis Tage entfallen — verworfen für Löschen |
| G2 | Löschen prüft die Quote nie | Löschen geht immer | theoretisch unbegrenztes Überschreiten (ein Löschsatz je bestehender Nachricht, ≈ 150 B) — verworfen |
| **G3 (gewählt)** | **Bearbeiten 507 wie Senden; Löschen darf die Quote um eine Reserve R = max(1 MiB, 1 % der Quote) überschreiten** | Löschen bleibt an der Quote möglich (Standard 4 GiB → R ≈ 41 MiB ≈ 280.000 Löschsätze); die Platte bleibt begrenzt | eine zweite Grenze für eine Route; die Invariante „`belegt ≤ hoechstens`“ wird „`≤ hoechstens + R`“ |

## Entscheidung

**Gewählt: A1, B1, C1, D2, E1, F1, G3.**

Es überwiegt, dass ein Änderungssatz **dieselbe** Mechanik nutzt wie eine Nachricht — Zeit unter der Schreibsperre, ein `write_all` mit Rückbau, Index-`push`, Veröffentlichen unter der Sperre, Zähler `belegt` —, damit Ordnung, Fehlerverhalten und Neustart ohne neuen Begriff gelten, und dass Reaktionen (I0026) in genau dieser Form Platz finden. In Kauf genommen: Waisen (Änderungssätze, deren Nachricht schon entfallen ist) liegen bis zum Ende ihres Tages unsichtbar auf der Platte und zählen an der Quote; die frühere Fassung und der gelöschte Text bleiben im Log lesbar; ein zweiter Zustand im Index (Fassung je Nachricht); Agenten ohne Strom erfahren Änderungen an älteren Nachrichten erst mit F2.

Regeln:

1. **Änderungssatz** ([A00007](../Architektur/A00007-logformat.md)) — ein Satz nach ADR-0004 mit genau einer **Bezugs-Metazeile** direkt nach der Kopfzeile:
   ````text
   @ 2026-10-29T15:02:11.004+01:00 claude@10.0.0.5
   + bearbeitet: 2026-10-27T14:07:12.345+01:00
     Der Scan ist durch. **Drei Befunde:**

   @ 2026-10-29T15:03:00.120+01:00 claude@10.0.0.5
   + geloescht: 2026-10-27T14:07:12.345+01:00

   ````
   - Kopfzeile: eigene Nachrichtenzeit `t_ä` aus `zeit_vergeben` (ADR-0035, also `t_ä > t_ziel` immer und eindeutig im Ordner) und die Kennung des Ändernden (gleich der des Absenders, Regel 6).
   - Bezugs-Schlüssel heute `bearbeitet` und `geloescht`; für I0026 reserviert die Familie `reaktion…` (Form entscheidet I0026). Ein Satz mit Bezugs-Metazeile ist **keine Nachricht**: er erscheint nie als Eintrag im Verlauf, zählt nicht in `nachrichten` (Health), seine Bytes zählen in `log_bytes`/`belegt`.
   - `bearbeitet` trägt den ganzen neuen Text nach den Textregeln von ADR-0004 (leerer Text erlaubt, Regel 7); `geloescht` trägt keinen Text. Änderungssätze tragen kein `+ anhang:` und kein `+ art:` (die Art bleibt die der Nachricht).
   - **Defekt beim Laden** (`WARNING laden` mit Datei und Zeile, Satz übergangen, nie still): zwei Bezugs-Metazeilen; Bezugszeit unlesbar; `t_ziel ≥ t_ä`; Kennung ≠ Absender der Nachricht; `bearbeitet` nach `geloescht` derselben Nachricht; `+ anhang:` in einem Änderungssatz. **Waise** (Nachricht nicht im Index — entfallen oder nie da): kein Defekt, still übergangen, gezählt in `DEBUG laden Änderungen ohne Nachricht | anzahl`.
2. **Tag** — der Änderungssatz steht in der Tagesdatei des lokalen Datums **seiner eigenen Zeit** (ADR-0035 Regel 5), also immer in der heute offenen Datei. **Frist-Folgen**: `tag(t_ä) ≥ tag(t_ziel)`, also entfällt die Nachricht **nie nach** ihren Änderungen; liegt die Änderung an einem späteren Tag, wird sie mit dem Entfallen der Nachricht zur Waise und bleibt bis zum Entfallen ihres eigenen Tages auf der Platte (unsichtbar, an der Quote gezählt). Tagesgrenze: eine Änderung um 23:59:59.999 an einer Nachricht von 23:59:59.998 landet im selben Tag, eine Millisekunde später im nächsten — beides gleich gültig. Entfallen nimmt mit dem Tag auch die Fassungen aller Nachrichten dieses Tages aus dem Index (Regel 3), unter der Gesprächssperre wie ADR-0030 Regel 4.
3. **Index** ([A00009](../Architektur/A00009-verlaufsindex.md)):
   - je Tag bleiben Rohtext und Satzanfänge byte-gleich mit der Datei (ADR-0006); Nachrichten und Änderungssätze werden **getrennt geordnet** geführt (z. B. alle Satzanfänge plus die Positionen der Nachrichten, oder zwei Listen mit Spannen) — Invarianten: der Block zählt und durchsucht nur Nachrichten; die Spanne einer Nachricht endet vor dem nächsten Satz **gleich welcher Art**; die Kosten des Blocks hängen weiter nur an `n` und der Zahl der Tage (ADR-0025 Regel 4). Die Datenstruktur wählt der Entwickler.
   - **Fassung je Nachricht** je Gespräch: `Nachrichtenzeit → Fassung { geaendert: t_ä, zustand: Bearbeitet { Ort des Änderungssatzes } | Geloescht }` in einer geordneten Abbildung; gesetzt beim Laden (Änderungssätze in Zeitfolge über alle Tage, nach den Nachrichten) und beim Schreiben nach dem Anhängen; Bearbeitet ersetzt Bearbeitet, Geloescht ist endgültig; `tag_entfernen` nimmt den Bereich der Zeiten des Tages heraus.
   - Block, Senden-Antwort, Ereignis und Suche liefern die Nachricht **in ihrer Fassung**: bearbeitet → Text aus dem Änderungssatz; gelöscht → Text `""`, `anhaenge: []`. Das Byte-Budget (ADR-0039) zählt die gelieferten Bytes (bei Bearbeitet den Änderungssatz statt der Nachricht).
4. **Antwortform** (ergänzt ADR-0025 Regel 3, nur Felder hinzu): je `nachricht` zusätzlich `zustand`: `null` | `"bearbeitet"` | `"geloescht"` und `geaendert`: Zeit des letzten Änderungssatzes oder `null`. Bei `geloescht`: `text: ""`, `anhaenge: []` (Name und Größe der Anhänge sind Inhalt), `von`, `art`, `zeit` bleiben — die Sicht zeigt „gelöscht“ an der Stelle der Nachricht. Wer `zustand` nicht kennt, sieht eine leere Nachricht — hingenommen (Anleitung I0019).
5. **Routen** ([A00004](../Architektur/A00004-http-schnittstelle.md)) unter `grenzschutz`, Extraktor `Teilnehmer`, Mensch und Agent gleich (Betreiber, Richtungsfrage 5); Körper gelesen mit dem **Feldleser** (ADR-0046 Ergänzung „Security-Gate“ L9): ein doppelter Schlüssel → 422 `{regel: "feld", feld, grund: "doppelt"}` unmittelbar vor „weiteres Feld“, nie der letzte Wert.
   - **Bearbeiten** `POST /api/channels/{name}/nachrichten/bearbeiten`, Körper `{zeit, text}`, Kopf optional `Idempotency-Key`. **Körpergrenze 131.072 Bytes** (Herleitung ADR-0034 Regel 2: 98.315 Bytes für `text` in der längsten Schreibweise plus `"zeit":"…",` mit 29 Zeichen als `\u`-Folgen ≈ 190 Bytes — innerhalb der Reserve). Prüfreihenfolge (abgewiesen, nie umgeformt): 403 Origin/Host · 401 · Vorab 1 Einheit → 429 · 413 · 415 · 400 · 422 `feld` (doppelt, weiteres Feld) · 404 Channel · `zeit` fehlt, keine Zeichenkette oder nicht RFC 3339 → 422 `{regel: "zeit"}` · `text` nach ADR-0034 Regel 3 Schritte 3.3–3.6 (`text`, `laenge`, `zeichen`; `leer` erst unter der Sperre, Regel 7) · `schluessel` · Restbuchung ⌈Bytes/4.096⌉ − 1 → 429 · dann unter Gesprächs- **und** Schreibsperre (Regel 8).
   - **Löschen** `POST /api/channels/{name}/nachrichten/loeschen`, Körper `{zeit}` (ein Feld `bestaetigt` kommt nur mit Option 2 von Regel 12), **Körpergrenze 1.024 Bytes** (`{"zeit":"…"}` in der längsten Schreibweise ≈ 190 Bytes), Parse auf dem Handler. Prüfreihenfolge wie Bearbeiten ohne `text`, ohne Wiederholschlüssel (Löschen ist von sich aus idempotent, Regel 9); **Kosten 1 Einheit** (nur die Vorab-Buchung — höchstens 10 Anhänge je Nachricht, ADR-0046 Regel 5, decken sich mit ⌈n/100⌉ = 1 aus F1′ Regel 9).
   - **Antworten unter der Sperre**, in dieser Reihenfolge: Nachricht mit dieser Zeit nicht im Index dieses Gesprächs (nie da, entfallen, anderer Channel, Direktgespräch ohne Beteiligung) → **404** `{fehler, regel: "nachricht"}` („Diese Nachricht gibt es nicht (mehr).“) — die Route ist kein Orakel über fremde Gespräche oder die Frist; Absender ≠ Kennung → **403** `{fehler, regel: "fremd"}` („Nur der Absender `alice@10.0.0.7` kann diese Nachricht ändern.“) — alle sehen alle Channels, die Kennung des Absenders ist ohnehin sichtbar; Bearbeiten einer gelöschten → **409** `{fehler, regel: "geloescht"}`; Quote → **507** (Regel 10); Plattenfehler → 500 wie ADR-0034, nichts geschrieben (ADR-0011).
   - **Erfolg**: Bearbeiten **200** `{channel, nachricht}` (neue Fassung); Löschen **200** `{channel, nachricht, ergebnis: "geloescht" | "schon_geloescht", anhaenge: {geloescht, bytes, fehler}}`.
   - Direktgespräche (I0012) bekommen dieselben zwei Routen unter ihrer Ressource (ADR-0025 Regel 7).
6. **Absender = Kennung** (Betreiber, Richtungsfrage 6): verglichen wird die Kennung der Sitzung (Name + kanonische IP, ADR-0012) mit dem `von` der Kopfzeile der Nachricht, als Zeichenkette nach Kanonisierung. Jede Sitzung dieser Kennung darf, auch eine spätere — gewollt; Folgen in der Angriffsmatrix (M2). Eine neue Adresse (DHCP, IPv6-Datenschutzadresse) ist eine neue Kennung und darf nicht mehr — hingenommen.
7. **Text beim Bearbeiten**: Regeln von ADR-0034 Regel 3 (Länge 16 KiB, Zeichen, `\r\n` → `\n`). `leer` (nur Leerraum) ist erlaubt, wenn die Nachricht mindestens einen Anhang im Zustand `vorhanden` hat (B5), sonst 422 `{regel: "leer"}` — geprüft unter der Sperre. Gleicher Text wie die aktuelle Fassung → **200 ohne Satz, ohne Ereignis** („unverändert“). Bearbeiten ändert nur den Text, nie die Anhänge.
8. **Ablauf Bearbeiten** unter Gesprächs- und Schreibsperre (Sperrordnung ADR-0035 Regel 1): Wiederholschlüssel prüfen (bekannt mit gleicher Prüfsumme → 200 mit der **aktuellen** Fassung, ohne Satz) → nachschlagen (404/403/409, `leer`) → unverändert? → Quote → Zeit vergeben → Satz anhängen (Rückbau bei Fehler) → Index (Satz, Fassung) → `belegt += Bytes` → Schlüssel merken → `veroeffentlichen(aenderung)` → frei. **Wiederholschlüssel**: derselbe Speicher wie Senden (ADR-0034 Regel 6 und Ergänzungen Karte #36: 640 je Kennung, 36.000 gesamt, 10 min), Prüfsumme über **Art der Operation**, Channel, Zielzeit und Text — ein Schlüssel des Sendens, beim Bearbeiten wiederverwendet, ergibt 422 `schluessel`.
9. **Ablauf Löschen — der Löschsatz ist der Commit-Punkt** (Kopplung an ADR-0046 F1′ Regel 5, dieselben drei Schritte):
   - (a) **unter Gesprächs- und Schreibsperre**: nachschlagen (404/403); schon gelöscht → 200 `schon_geloescht`, nichts geschrieben; Quote mit Reserve (Regel 10) → Zeit vergeben → **Löschsatz anhängen** (Rückbau bei Fehler → 500, nichts geändert, auch kein Anhang) → Index (Satz, Fassung `Geloescht`) → `belegt += Bytes` → je Anhang der Nachricht im [Anhangverzeichnis](../Architektur/A00016-anhangverzeichnis.md) `vorhanden → geloescht` per Vergleichen-und-Tauschen (schon `geloescht` durch das Laufwerk: übersprungen; die Bytes zählen weiter, J1) → `veroeffentlichen(aenderung)` → frei. Ab hier antwortet `GET /anhaenge/{id}` 410.
   - (b) **außerhalb jeder Sperre**, `spawn_blocking` unter der Erlaubnis der Hochladebank: je gewonnenem Anhang kürzen auf 0 Bytes über Deskriptoren (L2), `ENOENT` ist Erfolg.
   - (c) **unter der Schreibsperre**: je Erfolg `G −= z`; je Fehler bleibt der Anhang **`geloescht`** (anders als im Laufwerk, wo er zu `vorhanden` zurückkehrt — die Nachricht ist gelöscht, ihr Anhang darf nicht wieder abrufbar werden), G unverändert, `ERROR anhang Löschen gescheitert`; danach Ereignis `anhaenge` je Gespräch (ADR-0047 F1′ Regel 3) für das Laufwerk, Protokoll, Antwort.
   - **Atomarität**: entscheidend ist allein der Löschsatz — ein `write_all` mit Rückbau. Alles danach ist aus ihm ableitbar: **beim Laden** sind die Anhänge einer gelöschten Nachricht `geloescht`, **gleich welche Länge** ihre Datei hat (ändert ADR-0046 F1′ Regel 8 für diesen Fall); eine Datei länger als 0 zählt in G und wird nach dem Laden, vor der Startzeile, außerhalb der Sperren gekürzt (Schritte b–c), Fehler → `ERROR anhang`, bleibt gezählt bis zum nächsten Start oder bis ihr Tag entfällt. Absturz zwischen (a) und (c) heilt so der nächste Start (QE3).
   - **Wettläufe** (je Anhang genau ein Abzug): Laufwerk-Löschen und Nachricht-Löschen tauschen beide per Vergleichen-und-Tauschen, einer gewinnt; Entfallen (ADR-0030) läuft unter der Gesprächssperre und ist mit (a) seriell — entweder 404 oder gelöscht, danach zieht Entfallen nur `vorhanden` ab (F1′ Regel 5); Bearbeiten und Löschen derselben Nachricht sind über die Sperren seriell — nach dem Löschen antwortet Bearbeiten 409.
10. **Quote** (ADR-0044, G3): Änderungssätze sind Gesprächsbytes und zählen an `belegt`. Bearbeiten → 507 `{regel: "quote", …}` wie Senden (Schritt 9 dort). Löschen → 507 erst, wenn `belegt + Bytes(Löschsatz) > hoechstens + R`, `R = max(1 MiB, ⌈hoechstens / 100⌉)`; Text „Nicht gelöscht: …“ nach ADR-0044 Regel 4. Die Warnschwelle 90 % zählt die Reserve nicht mit. Bei `--plattenquote aus` kein 507. Löschen gibt keinen Platz der Text-Quote frei (Vision, Leitplanke Plattenquote), wohl aber Anhang-Quote (Regel 9 c).
11. **Ereignis** ([A00011](../Architektur/A00011-live-kanal.md)): Thema `aenderung`, Bus `Ereignis::Aenderung { channel, nachricht }`, Form `{"art":"aenderung","channel":"<name>","nachricht":{zeit, von, art, text, zustand, geaendert, anhaenge}}` — die Nachricht in der Form von Regel 4. Veröffentlicht unter der Schreibsperre nach dem Index (ADR-0035). Kein Stand, kein Filter im Strom; Direktgespräche nur an Beteiligte.
    - **Tab** (reine Funktion, [A00005](../Architektur/A00005-weboberflaeche.md)): übernimmt ein `aenderung`-Ereignis für eine Nachricht im Fenster, wenn `geaendert` größer ist als das bekannte (`null` ist kleiner als jede Zeit); `geloescht` wird nie zurückgenommen; Nachrichten außerhalb des Fensters: verworfen (das nächste Laden bringt die Fassung). Gepufferte Ereignisse vor dem Block (ADR-0028 Regel 5) gehen nach dem Block durch dieselbe Regel. Ausnahme zu ADR-0028 Regel 5 („Zeit ≤ bekannt wird verworfen“) gilt nur für dieses Thema und nur über `geaendert`.
    - **Nachholen** (F1): nach Wiederverbinden und `NeuerStand` lädt der Tab die Blöcke seines Fensters neu und übernimmt Fassungen mit größerem `geaendert`.
    - **Agenten**: `curl -N` sieht `aenderung`; `warten` (I0016) wird durch Änderungen **nicht** geweckt (Empfehlung für I0016, dort zu entscheiden); verlustfreies Nachholen über F2 ist Folgeoption.
12. **Rückfrage beim Löschen** (**Vorgeschlagen**, Betreiber-Frage F15): bis zur Antwort gilt Option 1 — der Server verlangt **keine** Bestätigung, auch nicht mit Anhängen; die Oberfläche fragt vor jedem Löschen einmal nach und nennt die Zahl der Anhänge, die mitgelöscht werden; Agenten löschen in einem Schritt. Kommt Option 2, wächst der Körper um `bestaetigt?` und die Route antwortet ohne ihn bei Anhängen 409 `{regel: "bestaetigen", loeschen: {anhaenge, bytes}}` wie ADR-0046 F1′ Regel 5 a — additiv, kein Umbau.
13. **Protokoll** ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Bereich **`aendern`**; nie Text, nie Anzeigename eines Anhangs):

    | Bereich | Ereignis | Stufe | Kontext |
    |---|---|---|---|
    | `aendern` | Nachricht bearbeitet (200; `grund=wiederholung` \| `unveraendert` ohne Satz) | DEBUG | `channel`, `kennung`, `anzahl` (Bytes des neuen Texts), `dauer_ms` |
    | `aendern` | Nachricht gelöscht (200; `grund=schon_geloescht`) | DEBUG | `channel`, `kennung`, `anhaenge`, `dauer_ms` |
    | `aendern` | Ändern fremder Nachricht abgewiesen (403 `fremd`) | WARNING, **Episode je Kennung** | Beginn `kennung`, `channel`; Ende zählt |
    | `aendern` | Änderung nicht geschrieben (500) | ERROR, Episode je `datei`+`grund` | `channel`, `kennung`, `datei`, `grund` |
    | `anhang` | Anhänge gelöscht (bestehende Zeile, F1′ K-b) — nur wenn `geloescht + fehler > 0` | INFORMATION | wie bisher, dazu `grund=nachricht`, `fremde=0` |
    | `laden` | defekter Änderungssatz (Regel 1) | WARNING (bestehende Regeln: ≤ 10 je Datei, dann Sammelzeile) | `datei`, `zeile` |
    | `laden` | Änderungen ohne Nachricht | DEBUG, eine Zeile je Start | `anzahl` |
    | `quote` | 507 beim Bearbeiten/Löschen | wie ADR-0044 Regel 8 (Episode je Kennung) | dazu `route=bearbeiten\|loeschen` |

    **Health**: keine neuen Felder — `nachrichten` zählt weiter nur Nachrichten (gelöschte eingeschlossen, sie stehen im Log), `log_bytes`/`belegt` enthalten die Änderungssätze, `quote_abgewiesen` und `anhaenge_geloescht` zählen die neuen Wege mit. Das Monitoring entscheidet nichts Neues (Regel ADR-0043, Ergänzung Karte #31); wer Missbrauch sucht, liest die Episode `fremd`.

## Konsequenzen

**Positiv:**
- Eine Mechanik für drei Interactions (I0032, I0033, I0026): Satz, Zeit, Rückbau, Index, Zähler, Strom — nichts davon ist neu erfunden.
- Ordnung über alle Änderungen kostet nichts Neues: `geaendert` stammt aus derselben Uhr wie die Nachrichtenzeit; eine spätere Route F2 nutzt dieselbe Cursor-Regel wie `vor`/`nach`.
- Öffnen bleibt unabhängig von der Zahl der Änderungen (C1, QE2); der Speicher wächst mit der Zahl geänderter Nachrichten, nicht der Änderungen.
- Ein Löschen ist nach einer 200 auch nach Absturz und Neustart vollständig — Anhänge eingeschlossen (Regel 9).

**Negativ / Inkauf zu nehmen:**
- Frühere Fassungen und gelöschte Texte bleiben bis zum Ende ihres Tages lesbar für jeden mit Dateizugriff; Waisen bleiben bis zum Ende ihres eigenen Tages — bei einer Änderung kurz vor Fristende bis zu einer Frist länger als die Nachricht selbst (Frist 7 Tage: eine Änderung am 7. Tag hält ihren Text noch bis zu 6 Tage nach dem Entfallen der Nachricht).
- Eine Nachricht, die oft bearbeitet wird, schreibt jedes Mal ihren ganzen Text (bis 16 KiB) — gebremst vom Schreibkonto wie Senden, gezählt an der Quote.
- Löschen darf die Quote um R überschreiten.
- Ein älteres Programm zeigte Änderungssätze als Nachrichten (Rückstufung).
- Agenten, die nur über `nach=` lesen, sehen Änderungen an schon gelesenen Nachrichten nicht ohne F2.

**Pre-Mortem (ein Monat nach dem Bau gescheitert, weil …):**
1. … nach einem Neustart eine bearbeitete Nachricht wieder den alten Text zeigt, weil Laden die Änderungssätze vor den Nachrichten eines späteren Tages anwandte oder einen Tag ausließ. Früh-Indikator: Integrationstest Neustart über drei Tage. Gegenmaßnahme: Regel 3 (erst alle Nachrichten, dann alle Änderungssätze in Zeitfolge), Test zuerst.
2. … ein Block nach vielen Änderungen langsam wird, weil die Spannen der Nachrichten Änderungssätze mitnehmen oder der Block Änderungssätze überspringen muss. Früh-Indikator: Messung QE2. Gegenmaßnahme: Regel 3 (getrennt geordnet, Spanne endet vor jedem Satz).
3. … der Anhang einer gelöschten Nachricht nach einem gescheiterten Kürzen und einem Neustart wieder abrufbar ist. Früh-Indikator: Unit mit Kürzen-Fehler und Neuladen. Gegenmaßnahme: Regel 9 (Laden leitet `geloescht` aus dem Löschsatz ab, nicht aus der Dateilänge); Mutante „Fehler setzt zurück auf `vorhanden`“ rot.
4. … ein Tab nach einer Lücke den alten Text zeigt. Früh-Indikator: E2E Strom trennen, bearbeiten, wiederverbinden. Gegenmaßnahme: Regel 11 Nachholen.
5. … Suche (I0011) frühere Fassungen oder gelöschte Texte findet. Früh-Indikator: Kriterium in I0011. Gegenmaßnahme: Folgen für I0011 unten.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (Logformat, rein): `@ t2 k / + bearbeitet: t1 / text` → Änderungssatz Art Bearbeitet, Ziel t1, Text; `+ geloescht:` ohne Text → Löschsatz; zwei Bezugszeilen, unlesbare Bezugszeit, `t1 ≥ t2` → defekt; Nachricht ohne Bezugszeile unverändert erkannt.
- [ ] Erster Unit-Test (Gespräche, Speicherablage, gestellte Uhr, aufzeichnender Rückruf): `alice` sendet t1 (Tag 1) und t2 (Tag 1); Uhr auf Tag 2; `alice` bearbeitet t1 zweimal, löscht t2; `bob` bearbeitet t1 → `Fremd`, Ablage unverändert; Block → t1 mit zweitem Text, `zustand: bearbeitet`, `geaendert` = Zeit der zweiten Änderung; t2 `geloescht`, Text leer; Rückruf erhielt drei `aenderung` unter der Sperre; `belegt` = Σ Bytes; Neuladen aus der Ablage → derselbe Block; Entfallen von Tag 1 → Fassungen weg, Tag 2 bleibt, Neuladen ohne Tag 1 → zwei Waisen, keine `WARNING`; Bearbeiten t2 → `Geloescht`; gleicher Text → kein Satz. Mutanten „Kennung nicht geprüft“, „Bearbeitet nach Geloescht übernommen“, „Änderungssatz im Block als Nachricht“ rot.
- [ ] Erster Unit-Test Löschen mit Anhängen: wie ADR-0046 F1′ „Erste Tests“ mit einer Nachricht mit drei Anhängen, Kürzen-Schranke und Kürzen-Fehler für einen → 410 sofort, G sinkt nach (c) um zwei, der dritte bleibt `geloescht` und gezählt; Neuladen → alle drei `geloescht`, der dritte wird nach dem Laden gekürzt; Laufwerk-Löschen eines vierten Anhangs derselben Nachricht vorher → `schon_geloescht`, ein Abzug.
- [ ] Erster Integrationstest (echtes Programm): Bearbeiten und Löschen über die API mit Strom-Mitschnitt (Ereignis < 1 s), Neustart → gleicher Verlauf; fremde Kennung → 403, Log byte-gleich; `{"zeit":a,"zeit":b}` → 422 `feld`/`doppelt`; unkodierte und kodierte Zeit mit anderem Versatz desselben Zeitpunkts → derselbe Treffer; Wiederholschlüssel aus Senden beim Bearbeiten → 422; Quote voll → Bearbeiten 507, Löschen 200, bis R erschöpft → 507.
- [ ] Leistung QE2 als Messwert mit I0032 (Release, Generator wie `tests/leistung.rs` plus 30.000 Änderungssätze).

**Folgen für andere Knoten** (dort nicht geändert):
- **I0026 Reaktionen**: dieselbe Form — Änderungssatz mit Bezugs-Metazeile der Familie `reaktion…` im Tag der Reaktion; Zähler in der Fassung je Nachricht; Ereignis `aenderung` mit dem ganzen Zählerstand der Nachricht (idempotent über `geaendert`); Frist und Waisen wie Regel 2; eine gelöschte Nachricht verliert ihre Reaktionen aus der Sicht und nimmt keine neuen an (409 `geloescht`); Kosten am Konto 1 Einheit, an der Quote wie Bearbeiten (507). Was je Reaktion gespeichert und gezählt wird, entscheidet I0026 nach den Betreiber-Fragen dort.
- **I0011 Suche**: Treffer gehören immer einer Nachricht und ihrer **aktuellen** Fassung; ein Treffer im Rohtext einer bearbeiteten oder gelöschten Nachricht, in einer früheren Fassung oder in einer Waise zählt nie; die aktuellen Fassungen in späteren Tagen werden über die Fassung je Nachricht gefunden, geordnet und fortgesetzt über die **Nachrichtenzeit**, nicht die Zeit des Änderungssatzes. Die Mechanik entscheidet die ADR zu I0011 nach ihrer Probe.
- **I0013 Erwähnungen**: eine Erwähnung, die erst durch Bearbeiten entsteht, ist eine Frage für I0013 (Empfehlung: kein Wecken).
- **I0016 Warten**: Regel 11; F2 als Folgeoption.
- **I0012 Direktgespräche**: Routen und Regeln gleich (Regel 5).
- **I0027 Laufwerk** (Karte #45): Anhänge einer gelöschten Nachricht sind `geloescht`; ob das Laufwerk ihre Namen zeigt, ist Betreiber-Frage F16.
- **I0019 Anleitung**: zwei Routen, `zustand`/`geaendert`, 403 `fremd`, 404 heißt auch „entfallen“.

**Angriffsmatrix-Skizze** (für den `security-reviewer`, Gate an den Anforderungen von I0032/I0033; Akteure wie ADR-0034 Ergänzung „Karte #36 Angriffsmatrix“: A0 ohne Sitzung, A1 Kennung mit Sitzung, A2 Opfer, A3 Nachfolger mit gleicher Kennung):

| # | Variante | Akteur | Erwartet | Linie |
|---|---|---|---|---|
| M1 | fremde Nachricht bearbeiten/löschen | A1 gegen A2 | 403 `fremd`, Log byte-gleich, kein Ereignis, Anhänge `vorhanden`; WARNING-Episode je Kennung, keine Zeile je Anfrage | L1, L2 |
| M2 | Nachfolger: A2 geht, A3 tritt ≈ 1 min später mit demselben Namen von derselben Herkunft bei und bearbeitet/löscht A2s Nachrichten samt Anhängen | A3 | **erlaubt — gewollt** (Vision, Richtungsfrage 6); Restrisiko: gleiche IP (NAT, gemeinsamer Rechner mehrerer Agenten); im Protokoll als DEBUG mit Kennung nachvollziehbar | L2 |
| M3 | Direktgespräch ohne Beteiligung, Zeit geraten | A1 | 404 `nachricht`, gleich „nie da“ — kein Orakel | L1 |
| M4 | Zeit einer Nachricht aus einem anderen Channel | A1 | 404 | L1 |
| M5 | Wettlauf Bearbeiten ↔ Löschen derselben Nachricht (zwei Tabs, 100 Paare parallel) | A2 | seriell: Endzustand gelöscht, jede Bearbeitung danach 409; Fassung im Index = Log nach Neustart | L1, L2 |
| M6 | Wettlauf Löschen ↔ Entfallen (Uhr 23:59:59.9, Frist 1) | A2 | entweder 404 oder gelöscht; je Anhang genau ein Abzug; Invarianten ADR-0046 F1′ Regel 1 | L1 |
| M7 | Wettlauf Nachricht-Löschen ↔ Laufwerk-Löschen derselben Anhänge | A2, A1 | einer gewinnt je Anhang, `schon_geloescht` beim anderen, G einmal | L1 |
| M8 | Flut von Änderungen: eine Nachricht im Takt mit 16 KiB bearbeiten | A1 | Konto 1/s je Kennung, 8/s je Herkunft wie Senden; Quote zählt jede Fassung; Index wächst nicht je Änderung (eine Fassung je Nachricht); Strom ≤ 8 Ereignisse/s je Herkunft | L1, L2r |
| M9 | Platte: Quote füllen mit Änderungssätzen | A1 | Bearbeiten 507 an `hoechstens`; Löschen bis `hoechstens + R`, dann 507; nichts wird deswegen gelöscht | L2 |
| M10 | doppelte Schlüssel `{"zeit":a,"zeit":b}`, `{"text":…,"text":…}` | A1 | 422 `feld`/`doppelt`, nichts geändert | L1, L2 |
| M11 | Wiederholschlüssel aus Senden beim Bearbeiten, Schlüssel einer anderen Kennung | A1 | 422 `schluessel` bzw. wie neu (Geltung je Kennung) | L1 |
| M12 | Körper zu groß, `text/plain`, fremder Origin, ohne Sitzung | A0, A1 | 413 vor Parse, 415, 403, 401 | L2 |
| M13 | handgeschriebener Änderungssatz mit fremder Kennung im Log | Betreiber-Ordner | beim Laden defekt, WARNING, übergangen | L1 |
| M14 | Text, der Metazeilen oder eine Kopfzeile vortäuschen will (`\n+ geloescht: …`, `\n@ …`) | A1 | eingerückt nach ADR-0004, bleibt Text; Steuerzeichen 422 | L1 |
| M15 | gelöschten Inhalt zurückholen: Verlauf, Ereignis, Suche, `GET /anhaenge/{id}` | A1 | `text: ""`, `anhaenge: []`, Suche ohne Treffer, 410 — nur die Datei auf der Platte hält den Text (Betreiber, Richtungsfrage 1) | L2 |
| M16 | Protokoll-Flut über 403 aus vielen Kennungen | A1 | je Kennung eine Episode; Budget ADR-0042 Regel 6 | L2 |

## Offene Fragen an den Betreiber

**F15 — Braucht das Löschen einer eigenen Nachricht eine Rückfrage?**
1. Die Oberfläche fragt vor jedem Löschen einmal nach und nennt die Zahl der Anhänge, die mitgelöscht werden; über die API löscht ein Agent in einem Schritt **(Empfehlung: es trifft nur eigene Nachrichten; Agenten wollen keinen Zwei-Schritt-Ablauf; die Server-Bestätigung im Laufwerk gibt es, weil dort auch fremde Anhänge betroffen sind)**
2. Wie im Laufwerk: mit Anhängen verlangt auch der Server eine Bestätigung (erst 409 mit den Zahlen, dann mit `bestaetigt`) — für Browser und Agenten
3. Keine Rückfrage; stattdessen lässt sich ein Löschen wenige Sekunden lang rückgängig machen (die Oberfläche hält die Anfrage zurück)

**F16 — Zeigt das Laufwerk die Anhänge einer gelöschten Nachricht noch mit Namen?** (für Karte #45)
1. Nein — mit der Nachricht verschwindet ihr Inhalt aus der Sicht, Namen der Anhänge eingeschlossen; das Laufwerk zeigt sie nicht mehr **(Empfehlung: der Name ist Inhalt; der Verlauf zeigt sie auch nicht mehr, Regel 4)**
2. Ja, als „gelöscht“ mit Name und Größe wie andere im Laufwerk gelöschte Anhänge

F6 und F7 aus ADR-0046 F1′ (Rückgängig, wer gelöscht hat) gelten für mitgelöschte Anhänge unverändert; bis zur Antwort jeweils Option 1.

## Quellen / Bezuege

- [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00007 Logformat](../Architektur/A00007-logformat.md), [A00009 Verlaufsindex](../Architektur/A00009-verlaufsindex.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00016 Anhangverzeichnis](../Architektur/A00016-anhangverzeichnis.md); Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00008](../Architektur/Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)
- ADR-0004, 0005, 0006, 0011, 0012, 0014, 0023, 0025, 0026, 0028, 0030, 0032, 0034, 0035, 0036, 0039, 0042, 0043, 0044, 0046, 0047
- Vision R00000 (c256cbb) Zielbild „Eigene Nachrichten bearbeiten und löschen“, Leitplanken Plattenquote und Anhang-Quote; WBS I0032, I0033, I0026 (Fragen an den architekt 1–3), I0011; Karte #48 (Betreiber-Antworten 2026-10-29)
