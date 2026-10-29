---
id: R00039
status: Neu
datum: 2026-10-30
---

# R00039: Eigene Nachricht über die API bearbeiten

## Beschreibung

Ein Teilnehmer — Mensch oder Agent — ändert den Text einer eigenen Nachricht über die API, solange die Nachricht besteht. Danach liefern Verlauf, Senden-Antwort und Ereignisstrom nur noch den neuen Text mit `zustand: "bearbeitet"`; frühere Fassungen liefert keine Antwort. Absender ist die Kennung `name@herkunft` — auch eine spätere Sitzung derselben Kennung darf, jede andere Kennung wird abgewiesen, und nichts ändert sich. Jede Änderung steht lesbar als eigener Satz im Tageslog ihres Tages; wer den Strom offen hat, erfährt sie in weniger als 1 s.

Zahlt ein auf: [Vision](R00000-vision.md) — „Eigene Nachrichten bearbeiten und löschen“ (der Agent über die API, Absender ist die Kennung, nur der neue Text, „bearbeitet“) und „Live überall“.

Herkunft: Karte #48 — Betreiber 2026-10-29 (Richtungsfragen 1–3, 5, 6; `AI-Team/po-entscheidungen.md` Zeilen 70–77), entschieden vom architekt in [ADR-0048](../Dokumentation/ADRs/0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) (Regeln 1–8, 10, 11, 13).

Slices (WBS `Dokumentation/Planung/localchata.md`, unter I0032 Eigene Nachricht bearbeiten): **F0132** Bearbeiten über die API, **F0133** Änderungen live im Strom. Das Löschen steht in [R00040](R00040-eigene-nachricht-ueber-die-api-loeschen-anhaenge-mit.md), der Browser in [R00041](R00041-eigene-nachrichten-im-browser-bearbeiten-und-loeschen.md).

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in ADR-0048: Nachricht, Nachrichtenzeit (ihre ID), Kennung (`name@herkunft`), Absender, Änderungssatz, Fassung, Tageslog, Plattenquote (`belegt`, `hoechstens`), Schreibkonto, Wiederholschlüssel, Ereignisstrom.

## Geschäftlicher Nutzen

Ein Agent, der eine falsche Zahl gepostet hat, oder ein Mensch mit einem Tippfehler korrigiert heute nur durch eine zweite Nachricht — der Verlauf trägt dann zwei Aussagen, und wer nur die erste liest, arbeitet mit der falschen. Mit dem Bearbeiten steht an der Stelle der Nachricht die richtige Aussage, gekennzeichnet als „bearbeitet“, und alle offenen Sichten sehen sie sofort. Weil die API die Grundlage für den Browser (R00041) und für das Löschen (R00040) ist, kommt sie zuerst.

## Funktionale Anforderungen

### F0132 Bearbeiten über die API (ADR-0048 Regeln 1–8, 10, 13)

- **Route** `POST /api/channels/{name}/nachrichten/bearbeiten`, Körper `{"zeit": "<Nachrichtenzeit>", "text": "<neuer Text>"}`, Kopf `Idempotency-Key` optional; für jeden mit Sitzung, Mensch und Agent gleich.
- **Erfolg 200** `{channel, nachricht}` mit der Nachricht in ihrer neuen Fassung: `zeit`, `von`, `art`, `anhaenge` wie bisher, `text` neu, `zustand: "bearbeitet"`, `geaendert` = Zeit des Änderungssatzes.
- **Antwortform überall**: jede Nachricht in Verlauf, Senden-Antwort, Ereignis und Warten trägt zusätzlich `zustand` (`null` | `"bearbeitet"` | `"geloescht"`) und `geaendert` (Zeit oder `null`); nur Felder kommen hinzu (ADR-0048 Regel 4).
- **Nur der Absender**: verglichen wird die Kennung der Sitzung mit `von` der Nachricht; jede Sitzung derselben Kennung darf, auch eine spätere. Andere Kennung → 403 `{fehler, regel: "fremd"}`.
- **Nachricht nicht da** (nie da, entfallen, anderer Channel) → 404 `{fehler, regel: "nachricht"}` — gleich, ob es die Zeit anderswo gibt.
- **Gelöschte Nachricht** (Löschsatz im Log, R00040) → 409 `{fehler, regel: "geloescht"}`.
- **Prüfreihenfolge** (abgewiesen, nie umgeformt): 403 Origin/Host · 401 · Vorab 1 Einheit → 429 · 413 über 131.072 Bytes (vor dem Parse) · 415 · 400 · 422 `feld` (doppelter Schlüssel, weiteres Feld) · 404 Channel · 422 `zeit` (fehlt, keine Zeichenkette, nicht RFC 3339) · 422 `text`/`laenge`/`zeichen` (Regeln wie Senden, ADR-0034 Regel 3) · 422 `schluessel` · Restbuchung ⌈Bytes des Texts / 4.096⌉ − 1 → 429 · dann unter der Sperre 404 `nachricht` · 403 `fremd` · 409 `geloescht` · 422 `leer` · unverändert · 507 `quote` · 500.
- **Text**: höchstens 16.384 Bytes, keine Steuerzeichen außer `\n` und `\t`, `\r\n` → `\n`; nur Leerraum ist erlaubt, wenn die Nachricht mindestens einen Anhang im Zustand `vorhanden` hat (dann gilt der Text `""`), sonst 422 `leer`. Bearbeiten ändert nie die Anhänge.
- **Unverändert**: gleicher Text wie die aktuelle Fassung → 200 mit der aktuellen Fassung, ohne Satz, ohne Ereignis.
- **Wiederholschlüssel**: derselbe Speicher wie beim Senden; die Prüfsumme umfasst die Art der Operation, Channel, Zielzeit und Text. Bekannt mit gleicher Prüfsumme → 200 mit der aktuellen Fassung ohne Satz; anders oder vom Senden → 422 `schluessel`.
- **Tageslog**: die Änderung ist ein Satz mit eigener Kopfzeile (eigene Nachrichtenzeit, Kennung) und genau einer Metazeile `+ bearbeitet: <zeit der Nachricht>`, darunter der ganze neue Text eingerückt — angehängt an die Tagesdatei des Tages **der Änderung**; die frühere Fassung bleibt in ihrer Datei stehen, bis ihr Tag entfällt, und erscheint in keiner Antwort.
- **Laden**: nach einem Neustart gilt jede mit 200 beantwortete Änderung genauso, auch über Tagesgrenzen; ein defekter Änderungssatz (zwei Bezugs-Metazeilen, Bezugszeit unlesbar, Bezugszeit ≥ eigene Zeit, Kennung ≠ Absender, `bearbeitet` nach `geloescht`, `+ anhang:`) wird mit `WARNING laden` (Datei, Zeile) übergangen; eine Änderung, deren Nachricht entfallen ist (Waise), wird still übergangen und in `DEBUG laden Änderungen ohne Nachricht | anzahl=<n>` gezählt.
- **Plattenquote**: Änderungssätze zählen an `belegt`; an der vollen Quote → 507 `{regel: "quote", …}` wie beim Senden.
- **Betriebsprotokoll** (Bereich `aendern`, nie Text): `DEBUG` je Bearbeitung (`channel`, `kennung`, `anzahl`, `dauer_ms`, ggf. `grund=wiederholung|unveraendert`); `WARNING aendern Ändern fremder Nachricht abgewiesen` als Episode je Kennung; `ERROR aendern Änderung nicht geschrieben` als Episode je `datei` + `grund`; die `quote`-Episode trägt `route=bearbeiten`.

### F0133 Änderungen live im Strom (ADR-0048 Regel 11)

- Jede Bearbeitung mit neuem Satz erzeugt im Ereignisstrom genau ein Ereignis des Themas `aenderung`: `{"art": "aenderung", "channel": "<name>", "nachricht": {zeit, von, art, text, zustand, geaendert, anhaenge}}` — die Nachricht in der Form der Antwort.
- Wiederholung, unveränderter Text und jede Abweisung erzeugen kein Ereignis. Kein Stand, kein Zähler; die Ordnung trägt `geaendert`.

## Nicht-funktionale Anforderungen

- **Zeitkriterium** nach `AI-Team/planer.md`: „Ereignis weniger als 1 s nach der 200“ — Zweck: Vision „Live überall“, Lastprofil Team-LAN „Zustellung live < 1 s“; erwartet unter 50 ms auf dem Prüfrechner (ein Ereignis unter der Sperre, wie `nachricht`); die Uhr startet beim Empfang der 200-Antwort durch den Bearbeitenden. Abwesenheit eines Ereignisses wird über 2 s beobachtet (doppelte Grenze, damit ein spätes Ereignis sicher auffiele).
- **Leistung**: Öffnen bleibt unabhängig von der Zahl der Änderungen — gemessen in R00040 (F0140, QE2).
- **Sicherheit**: Kennung nie vom Client; kein Orakel über fremde Gespräche (404 gleich „nie da“); Text nie im Protokoll; Gate `security-reviewer` mit der Angriffsmatrix ADR-0048 M1–M4, M8, M10–M14, M16.
- **Lesbar ohne localchatA** (QE5): wer `less` auf die Tagesdatei richtet, sieht, dass und wann eine Nachricht bearbeitet wurde.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start mit `TZ=Europe/Berlin localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser --plattenquote 10MiB` auf einem Wegwerf-Ordner mit den leeren Ordnern `channels/allgemein/` und `channels/technik/`; `<p>` ist der Port aus der Startzeile. `S(<name>)` ist der Sitzungs-Header aus dem Beitritt von `<name>` per `curl` (Agent, Herkunft `127.0.0.1`, Kennung `<name>@127.0.0.1`). „Bearbeiten“ heißt `POST http://127.0.0.1:<p>/api/channels/allgemein/nachrichten/bearbeiten` mit `Content-Type: application/json`, `S(beta)` und ohne `Origin`. `t1` ist die `zeit` aus der 200, mit der `beta` in `# allgemein` den Text `Hallo` gesendet hat. `alpha` liest den Strom `GET /api/ereignisse` mit `S(alpha)`. „Tageslog unverändert“ heißt: `sha256sum` aller Dateien unter `channels/` vor und nach der Anfrage gleich. Jede Kennung sendet je Szenario höchstens 30 Anfragen in einem Zug (Vorrat ihres Eimers), außer das Kriterium sagt es anders.

### F0132 Bearbeiten über die API

- [ ] **Logformat** (Unit, rein; wörtlich aus ADR-0048 „Erster Unit-Test (Logformat)“): `@ t2 k / + bearbeitet: t1 / text` → Änderungssatz Art Bearbeitet, Ziel t1, Text; `+ geloescht:` ohne Text → Löschsatz; zwei Bezugszeilen, unlesbare Bezugszeit, `t1 ≥ t2` → defekt; Nachricht ohne Bezugszeile unverändert erkannt. Dazu: `+ anhang:` in einem Änderungssatz → defekt. Mutante „Nachricht ohne Bezugszeile als Änderung erkannt“ wird rot.
- [ ] **Gespräche** (Unit, Speicherablage, gestellte Uhr, aufzeichnender Rückruf; ADR-0048 „Erster Unit-Test (Gespräche)“ ohne den Teil Löschen — der steht wörtlich in R00040): `alice` sendet t1 und t2 (Tag 1); Uhr auf Tag 2; `alice` bearbeitet t1 zweimal; `bob` bearbeitet t1 → `Fremd`, Ablage unverändert; Block → t1 mit zweitem Text, `zustand: bearbeitet`, `geaendert` = Zeit der zweiten Änderung, t2 mit `zustand: null`; Rückruf erhielt zwei `aenderung` unter der Sperre; `belegt` = Σ Bytes aller Sätze; Neuladen aus der Ablage → derselbe Block; Entfallen von Tag 1 → Fassung von t1 weg, Tag 2 bleibt; Neuladen ohne Tag 1 → keine `WARNING`, `DEBUG laden Änderungen ohne Nachricht | anzahl=2`; gleicher Text → kein Satz. Mutanten „Kennung nicht geprüft“ und „Änderungssatz im Block als Nachricht“ werden rot.
- [ ] **Rundweg** (Integration, echtes Programm): `beta` bearbeitet `{"zeit": t1, "text": "Hallo zusammen"}` → 200, `nachricht.text` = `Hallo zusammen`, `zustand` = `bearbeitet`, `geaendert` > t1; `GET /api/channels/allgemein/nachrichten` → genau eine Nachricht, Zeit t1, Text `Hallo zusammen`, `zustand: "bearbeitet"` (der Änderungssatz erscheint nicht als Eintrag); Health `nachrichten` bleibt 1; Neustart → derselbe Verlauf.
- [ ] **Spätere Sitzung derselben Kennung** (Integration; Richtungsfrage 6): `beta` verlässt (`POST /api/verlassen`) und tritt danach erneut als `beta` bei → Bearbeiten von t1 mit dem neuen Token → 200.
- [ ] **Änderung an einem späteren Tag, Neustart über drei Tage** (Integration, `LOCALCHATA_JETZT`; Pre-Mortem 1): `beta` sendet t1 am 2026-10-27 um 23:59:58; Neustart mit Uhr 2026-10-28 10:00, Bearbeiten → `Zweiter`; Neustart mit Uhr 2026-10-29 10:00, Bearbeiten → `Dritter` → der erste Änderungssatz steht in `2026-10-28.log`, der zweite in `2026-10-29.log`, `2026-10-27.log` ist byte-gleich wie vor dem ersten Bearbeiten; nach einem weiteren Neustart zeigt der Verlauf t1 mit `Dritter`.
- [ ] **Fremde Kennung** (Integration; Matrix M1): `gamma` bearbeitet t1 → 403 mit `regel: "fremd"` und einem `fehler`, der mit „Nur der Absender“ beginnt und `beta@127.0.0.1` nennt (Wortlaut ADR-0048 Regel 5); Tageslog unverändert, kein `aenderung` bei `alpha` binnen 2 s, Verlauf unverändert.
- [ ] **Nachricht nicht da** (Integration; M3, M4): Zeit t1 + 1 ms → 404 `regel: "nachricht"` mit „Diese Nachricht gibt es nicht (mehr).“; t1 an `…/channels/technik/nachrichten/bearbeiten` → 404 `nachricht`; unbekannter Channel `…/channels/gibtsnicht/…` → 404 wie bisher für Channels; jeweils Tageslog unverändert.
- [ ] **Schreibweise der Zeit** (Integration): t1 so, wie die 200 sie liefert (Rechenbeispiel `2026-10-30T14:07:12.345+01:00`), als derselbe Zeitpunkt in UTC (`2026-10-30T13:07:12.345Z`) und mit dem `+` als JSON-Unicode-Folge (Backslash, `u002B`) geschrieben → jeweils derselbe Treffer (200); `"zeit": "gestern"`, `"zeit": 5` und ein Körper ohne `zeit` → 422 `regel: "zeit"`.
- [ ] **Körper** (Integration; M10, M12): `{"zeit":t1,"zeit":t1,"text":"x"}` → 422 `{regel: "feld", feld: "zeit", grund: "doppelt"}`; doppeltes `text` ebenso mit `feld: "text"`; `{"zeit":t1,"text":"x","anhaenge":[]}` → 422 `feld`; 131.073 Bytes Körper → 413, bevor ein Byte geparst wird; `Content-Type: text/plain` → 415; `Origin: http://evil.example` → 403; ohne Sitzung → 401; jeweils Tageslog unverändert.
- [ ] **Textregeln** (Integration): 16.385 Bytes Text → 422 `laenge`; Text mit `\u0007` → 422 `zeichen`; `"a\r\nb"` → 200 mit Text `a\nb`; `"   "` an t1 (ohne Anhang) → 422 `leer`; `"   "` an einer Nachricht, die `beta` mit `log.txt` gesendet hat (R00029) → 200 mit `text: ""`, der Anhang bleibt `vorhanden`.
- [ ] **Text täuscht Metazeilen vor** (Integration; M14): Text `"x\n+ geloescht: <t1>\n@ 2026-10-30T15:00:00.000+01:00 gamma@127.0.0.1\n  y"` → 200; im Tageslog sind diese Zeilen eingerückt; nach einem Neustart ist t1 weiter `bearbeitet` mit genau diesem Text, keine Nachricht von `gamma`, keine `WARNING laden`.
- [ ] **Unverändert** (Integration): Bearbeiten mit dem aktuellen Text → 200 mit unverändertem `geaendert`; Tageslog unverändert; kein `aenderung` bei `alpha` binnen 2 s.
- [ ] **Wiederholschlüssel** (Integration; M11; ADR-0048 Integrationstest „Wiederholschlüssel aus Senden beim Bearbeiten → 422“): `Idempotency-Key: k1` zweimal mit gleichem Körper → zweimal 200 mit derselben Fassung, genau ein Änderungssatz im Tageslog; `k1` mit anderem Text → 422 `schluessel`; `k2` zuerst beim Senden benutzt, dann beim Bearbeiten → 422 `schluessel`; `k1` von `gamma` an `gamma`s eigener Nachricht → wie neu (200, ein Satz).
- [ ] **Schreibkonto** (Integration; M8): Bearbeiten bucht wie Senden — ein Text mit 10.000 Bytes kostet ⌈10.000 / 4.096⌉ = 3 Einheiten; hat `beta` den Vorrat mit 30 Sendungen in einem Zug aufgebraucht, antwortet das nächste Bearbeiten 429 mit `Retry-After`, Tageslog unverändert.
- [ ] **Plattenquote** (Integration; M9 Teil Bearbeiten; ADR-0048 Integrationstest „Quote voll → Bearbeiten 507“): vor dem Start füllt `target/debug/examples/quote-ordner <ordner> allgemein <bytes>` den Channel bis 300 Bytes unter `hoechstens` (10.485.760); `beta` sendet `Hallo` → 200; Bearbeiten mit 400 Bytes Text → 507 `regel: "quote"`, Tageslog unverändert; die `quote`-Episode im Protokoll trägt `route=bearbeiten`.
- [ ] **Schreibfehler** (Unit, Speicherablage mit Schreibfehler): Anhängen scheitert → `Fehler`, Ablage byte-gleich, Index und Fassung unverändert, kein Rückruf; die Route antwortet 500.
- [ ] **Von Hand geschriebener Änderungssatz** (Integration; M13): vor dem Start steht in der Tagesdatei nach `beta`s Nachricht t1 ein Satz `@ <t1 + 1 s> gamma@127.0.0.1` / `+ bearbeitet: <t1>` / `  gefälscht` → Start: `WARNING laden` mit Datei und Zeile; der Verlauf zeigt t1 mit `Hallo` und `zustand: null`.
- [ ] **Lesbar** (QE5): nach dem Rundweg zeigt `cat channels/allgemein/<heute>.log` unter einer eigenen Kopfzeile von `beta@127.0.0.1` die Zeile `+ bearbeitet: <t1>` und eingerückt `Hallo zusammen`; die Zeile mit `Hallo` steht unverändert darüber.
- [ ] **Betriebsprotokoll** (Integration, `--protokoll-stufe debug`, `LOCALCHATA_EPISODE_RUHE_MS=200`): der Rundweg schreibt eine Zeile `DEBUG aendern Nachricht bearbeitet | channel=allgemein kennung=beta@127.0.0.1 anzahl=14 dauer_ms=…`; drei 403 von `gamma` → genau eine Beginn-Zeile `WARNING aendern Ändern fremder Nachricht abgewiesen | kennung=gamma@127.0.0.1 channel=allgemein` und eine Ende-Zeile mit 3; fünf Kennungen mit je einem 403 → fünf Episoden (M16), Budget wie ADR-0042 Regel 6; `grep -r "Hallo zusammen" logs/` ist leer.
- [ ] **Bestand bleibt grün**: Nachrichten ohne Änderung tragen in Verlauf, Senden-Antwort und Ereignis `nachricht` `zustand: null` und `geaendert: null`; die Kriterien aus R00007, R00011, R00029 laufen unverändert.

### F0133 Änderungen live im Strom

- [ ] **Ereignis** (Integration; ADR-0048 Integrationstest „Bearbeiten … mit Strom-Mitschnitt (Ereignis < 1 s)“): `alpha` liest den Strom; `beta` bearbeitet t1 → bei `alpha` genau ein Ereignis des Themas `aenderung` mit `channel: "allgemein"` und `nachricht` = `{zeit: t1, von: "beta@127.0.0.1", art: "agent", text: "Hallo zusammen", zustand: "bearbeitet", geaendert: <wie in der 200>, anhaenge: []}`, weniger als 1 s nach der 200.
- [ ] **Kein Ereignis ohne Änderung** (Integration): Wiederholung mit gleichem Schlüssel, unveränderter Text, 403, 404, 422 und 507 → binnen 2 s kein `aenderung` bei `alpha`.
- [ ] **Neuere Änderung trägt den größeren Zeitstempel** (Integration): zwei Bearbeitungen von t1 kurz nacheinander (`Eins`, `Zwei`) → zwei Ereignisse in dieser Reihenfolge, `geaendert` steigt, das zweite trägt `Zwei`.
- [ ] **Unter der Sperre** (Unit, Teil des Unit-Tests „Gespräche“): der Rückruf erhält `aenderung` nach dem Schreiben von Index und Fassung; Mutante „Veröffentlichen vor dem Anhängen“ (Rückruf bei Schreibfehler) wird rot.

## Betroffene Verzeichnisstruktur

- `src/gespraeche/logformat.rs` (Änderungssatz lesen und schreiben), `src/gespraeche/verlaufsindex.rs` (Nachrichten und Änderungssätze getrennt, Fassung je Nachricht), `src/gespraeche/mod.rs` (Laden, `bearbeiten` neben `senden`), `src/gespraeche/abfrage.rs` (Antwortform), `src/gespraeche/wiederholung.rs` (Prüfsumme mit Art).
- `src/http/bearbeiten.rs` neu (Muster `src/http/senden.rs`, Feldleser `src/http/felder.rs`), Route in `src/http/mod.rs`; `src/http/nachrichten.rs` (Antwortform).
- `src/live/bus.rs`, `src/http/ereignisse.rs` (Thema `aenderung`).
- `src/protokoll/zeile.rs` (Bereich `aendern`), Episoden wie `src/protokoll/episoden.rs`, `quote_melden` in `src/http/senden.rs`.
- Tests: `tests/bearbeiten.rs`, `tests/bearbeiten_strom.rs`, `tests/bearbeiten_protokoll.rs`; Unit in den Modulen.
- Architektur nach dem Bau (architekt): A00006, A00007, A00009, A00004, A00011.

## Technische Überlegungen

### Ablauf (Vorplanung — welche Bubbles es am Ende werden, entscheidet der Entwickler)

1. **Logformat** (B0594): Test „Logformat“ zuerst; Bezugs-Metazeile erkennen und schreiben.
2. **Index** (B0595, B0596): Nachrichten und Änderungssätze getrennt geordnet; Fassung je Nachricht beim Laden (erst alle Nachrichten, dann alle Änderungssätze in Zeitfolge über alle Tage), beim Schreiben und beim Entfallen.
3. **Antwortform** (B0597): `zustand`, `geaendert`, Text der Fassung, Byte-Budget.
4. **Bearbeiten unter den Sperren** (B0598) → **Route** (B0599) → **Wiederholschlüssel** (B0600) → **Protokoll** (B0601).
5. **Strom** (B0602, B0603): Thema `aenderung`, Veröffentlichen unter der Schreibsperre.

Reihenfolge: 1 → 2 → 3 → 4; 5 nach 4. F0132 nach dem Merge von #66 (R00038 ändert `quote_melden` in `src/http/senden.rs`).

### Grobentwurf

- `Satz` (Logformat) bekommt die Art `Aenderung { ziel: Zeit, art: Bearbeitet(Text) | Geloescht }` neben der Nachricht.
- `Fassung` (Index, unveränderlich) `{ geaendert: Zeit, zustand: Bearbeitet { ort } | Geloescht }`; je Gespräch eine geordnete Abbildung Nachrichtenzeit → Fassung.
- `Gespraeche::bearbeiten(channel, kennung, ziel, text, schluessel) -> Result<Nachricht, Abweisung>` (Integration, unter Gesprächs- und Schreibsperre).
- `Ereignis::Aenderung { channel, nachricht }`.

## Aufwand (Zählung)

Gezählt aus der WBS; Richtwerte, ungemessen für diese Bubble-Typen.

- F0132: 8 Bubbles · 4 klar zu 0,4 h (B0594, B0597, B0600, B0601; 1,6 h) · 4 unklar zu 2–4 h (B0595, B0596, B0598, B0599; 8–16 h)
- F0133: 2 Bubbles · 2 klar zu 0,4 h (B0602, B0603; 0,8 h)
- Summe 10 Bubbles, 10,4–18,4 h.

## Abhängigkeiten

- Abhängig von: I0009 Nachricht senden (gelb; Senden, Strom, Schreibkonto, Wiederholschlüssel gebaut), I0021 Betriebsprotokoll (Episoden), I0023 Plattenquote (`belegt`), R00029 F0099 Senden mit Anhängen (grün; für „leer mit Anhang“); **#66 / R00038** (gleiche Datei `src/http/senden.rs`).
- Blockiert: R00040 (Löschen braucht Änderungssatz, Fassung und Ereignis), R00041 (Browser), I0026 Reaktionen (gleiche Form, ADR-0048 „Folgen“), I0011 Suche (findet nur die aktuelle Fassung).

## Offene Fragen

- **An den architekt** (nicht blockierend, Kriterium nach Annahme): Text der 507 beim Bearbeiten — angenommen nach ADR-0044 Regel 4 mit „Nicht bearbeitet: …“; Text der 409 `geloescht` — angenommen „Diese Nachricht ist gelöscht.“
- **Prüfpunkt für den `security-reviewer`** (Matrix M2, gewollt): wer ≈ 1 min nach dem Gehen eines Teilnehmers mit demselben Namen von derselben Herkunft beitritt, gilt als derselbe Absender und kann dessen Nachrichten ändern (Richtungsfrage 6, ADR-0048 Regel 6). Kein Befund, wenn es so gebaut ist; Restrisiko gleiche IP (NAT, gemeinsamer Rechner mehrerer Agenten), nachvollziehbar über die `DEBUG`-Zeile mit Kennung. Vom Director als Anmerkung zu führen.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus ADR-0048.
- Anleitung für Agenten (I0019): zwei Routen, `zustand`/`geaendert`, 403 `fremd`, 404 heißt auch „entfallen“ — bis I0019 nur in ADR-0048.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist die Frage des Betreibers, ob man eine Nachricht nachbearbeiten kann: heute bleibt eine falsche Aussage stehen, und die Korrektur ist eine zweite Nachricht, die jemand übersehen kann. Wenn der Absender den Text über die Route ändert, dann schreibt die Instanz einen Änderungssatz und setzt die Fassung der Nachricht im Index, sodass jede Antwort — Verlauf, Strom, nach einem Neustart — nur den neuen Text mit „bearbeitet“ liefert; dadurch liest jeder, auch ein Agent, an der Stelle der Nachricht die richtige Aussage. Die Prüfung der Kennung unter der Sperre verhindert, dass jemand anderes Worte in fremde Nachrichten legt. Der Hebel liegt bei der API und nicht beim Browser, weil Agenten nur die API haben und der Browser auf genau dieser Route und diesem Ereignis aufsetzt (R00041); und er liegt im nur angehängten Log statt im Umschreiben, weil der Betreiber „Logs nur fortgeschrieben“ entschieden hat.

## Notizen

- **Verworfen** (Optionen des architekt, ADR-0048): Änderungssatz in der Tagesdatei der Nachricht (bricht „Datei und Zeit stimmen überein“), eigene Änderungsdatei je Tag (zweite Dateiart), Umschreiben an Ort und Stelle (gegen „Logs nur fortgeschrieben“), Diff statt ganzem Text, REST-Methoden mit der Zeit im Pfad, eine Sammelroute `aendern`, `nachricht` erneut senden, `stand`-Zähler.
- **Nicht hier**: Direktgespräche (I0012 bekommt dieselbe Route), `warten` (I0016 wird durch Änderungen nicht geweckt, dort zu entscheiden), verlustfreies Nachholen für Agenten ohne Strom (Folgeoption F2 der ADR), Erwähnungen durch Bearbeiten (I0013), Markdown (I0017).
- Die frühere Fassung bleibt im Tageslog lesbar für jeden mit Dateizugriff, bis ihr Tag entfällt (ADR-0048, in Kauf genommen).
