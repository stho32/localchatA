---
id: R00040
status: Neu
datum: 2026-10-30
---

# R00040: Eigene Nachricht über die API löschen, ihre Anhänge mit

## Beschreibung

Ein Teilnehmer — Mensch oder Agent — löscht eine eigene Nachricht über die API in einem Schritt, solange sie besteht. Danach liefern Verlauf und Ereignisstrom sie als `zustand: "geloescht"` ohne Text und ohne Anhänge; ihr Text bleibt im Tageslog, bis sein Tag entfällt, und gibt keinen Platz der Plattenquote frei. Ihre Anhänge werden mitgelöscht wie im Laufwerk — der Abruf antwortet 410, ihr Platz in der Anhang-Quote wird frei —, und das gilt auch nach einem Absturz mitten im Löschen. An der vollen Plattenquote bleibt Löschen über eine Reserve möglich. Nur der Absender (die Kennung `name@herkunft`, auch in einer späteren Sitzung) darf löschen.

Zahlt ein auf: [Vision](R00000-vision.md) — „Eigene Nachrichten bearbeiten und löschen“ (Text nur aus der Sicht, Anhänge mitgelöscht, Agenten wie Menschen) und die Leitplanken „Plattenquote“ und „Anhang-Quote“.

Herkunft: Karte #48 — Betreiber 2026-10-29 (Richtungsfragen 1, 2, 4–6; `AI-Team/po-entscheidungen.md` Zeilen 72–77), product-owner Zeilen 95 (F15/O19: über die API ein Schritt, keine Server-Bestätigung) und 96 (F16); entschieden vom architekt in [ADR-0048](../Dokumentation/ADRs/0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) (Regeln 4, 5, 9, 10, 11, 12 Option 1, 13; QE2, QE3).

Slices (WBS `Dokumentation/Planung/localchata.md`, unter I0033 Eigene Nachricht löschen): **F0136** Löschen über die API, **F0137** Anhänge einer gelöschten Nachricht mitlöschen, **F0140** Verlauf bleibt schnell mit vielen Änderungen. Grundlage ist [R00039](R00039-eigene-nachricht-ueber-die-api-bearbeiten.md) (Änderungssatz, Fassung je Nachricht, Antwortform, Ereignis `aenderung`); der Browser steht in [R00041](R00041-eigene-nachrichten-im-browser-bearbeiten-und-loeschen.md).

### Domänenbezug

Kein Glossar im Projekt. Begriffe wie in ADR-0048 und R00029: Nachricht, Nachrichtenzeit, Kennung, Absender, Löschsatz, Fassung, Tageslog, Plattenquote (`belegt`, `hoechstens`, Reserve R), Anhang-Quote (G, Zählmaß `z(n)`), Anhang im Zustand `vorhanden` oder `geloescht`, Laufwerk.

## Geschäftlicher Nutzen

Wer versehentlich etwas Falsches oder Vertrauliches gepostet hat — ein Agent einen Schlüssel aus seiner Umgebung, ein Mensch eine Datei in den falschen Channel —, kann es heute nicht zurücknehmen. Mit dem Löschen verschwindet es aus jeder Sicht, die Anhänge samt ihrem Platz eingeschlossen; die Reserve sorgt dafür, dass genau das auch an einer vollen Platte geht. Dass der Text im Tageslog bleibt, ist entschieden („Logs nur fortgeschrieben“) und steht offen da.

## Funktionale Anforderungen

### F0136 Löschen über die API (ADR-0048 Regeln 5, 9 a, 10, 12, 13)

- **Route** `POST /api/channels/{name}/nachrichten/loeschen`, Körper `{"zeit": "<Nachrichtenzeit>"}`; für jeden mit Sitzung, Mensch und Agent gleich; **ein Schritt**, kein Feld `bestaetigt` (PO Zeile 95) — ein solches Feld ist ein weiteres Feld (422 `feld`).
- **Erfolg 200** `{channel, nachricht, ergebnis: "geloescht", anhaenge: {geloescht, bytes, fehler}}`; `nachricht` mit `text: ""`, `anhaenge: []`, `zustand: "geloescht"`, `geaendert` = Zeit des Löschsatzes; `zeit`, `von`, `art` bleiben.
- **Schon gelöscht** → 200 `ergebnis: "schon_geloescht"`, nichts geschrieben, kein Ereignis. **Bearbeiten danach** → 409 `geloescht` (R00039) mit `fehler` „Diese Nachricht ist gelöscht.“ (ADR-0048 Regel 13). Gelöscht ist endgültig.
- **Abweisungen** wie Bearbeiten: andere Kennung → 403 `fremd`; Nachricht nicht in diesem Channel → 404 `nachricht`; Prüfreihenfolge 403 Origin/Host · 401 · Vorab 1 Einheit → 429 · 413 über 1.024 Bytes · 415 · 400 · 422 `feld` · 404 Channel · 422 `zeit` · unter der Sperre 404 · 403 · Quote · 500. **Kosten 1 Einheit**, kein Wiederholschlüssel (Löschen ist von sich aus wiederholbar).
- **Tageslog**: ein Löschsatz — eigene Kopfzeile, eine Metazeile `+ geloescht: <zeit der Nachricht>`, kein Text — angehängt an die Tagesdatei des Tages der Änderung. Der Text der Nachricht bleibt in seiner Datei, bis sein Tag entfällt; `belegt` steigt um den Löschsatz und sinkt nicht.
- **Plattenquote mit Reserve**: Löschen antwortet erst 507 (`regel: "quote"`, Text „Nicht gelöscht: …“ nach ADR-0044 Regel 4), wenn `belegt + Bytes(Löschsatz) > hoechstens + R` mit `R = max(1.048.576, ⌈hoechstens / 100⌉)`; die Warnschwelle 90 % zählt die Reserve nicht mit; bei `--plattenquote aus` nie 507. Rechenbeispiele: 4 GiB → R = ⌈42.949.672,96⌉ = 42.949.673 Bytes; 10 MiB → ⌈104.857,6⌉ = 104.858 < 1.048.576, also R = 1.048.576.
- **Ereignis** `aenderung` mit der gelöschten Nachricht (Form von R00039 F0133).
- **Laden**: ein Löschsatz macht die Nachricht nach jedem Neustart `geloescht`; ein späterer `bearbeitet`-Satz derselben Nachricht ist defekt (`WARNING laden`) und ändert nichts.
- **Betriebsprotokoll** (Bereich `aendern`, nie Text): `DEBUG aendern Nachricht gelöscht` (`channel`, `kennung`, `anhaenge`, `dauer_ms`; `grund=schon_geloescht`); 403 und 500 in denselben Episoden wie beim Bearbeiten; `quote` mit `route=loeschen`.

### F0137 Anhänge einer gelöschten Nachricht mitlöschen (ADR-0048 Regel 9; ADR-0046 Ergänzung „Karte #9 Betreiber F1′“ Regeln 5, 8)

- **Commit-Punkt ist der Löschsatz**: im selben Sperrabschnitt wird jeder Anhang der Nachricht `vorhanden → geloescht` getauscht; ab der 200 antwortet `GET /anhaenge/{id}` 410. Ein Anhang, den vorher jemand im Laufwerk gelöscht hat, wird übersprungen — je Anhang genau ein Abzug.
- **Danach** außerhalb der Sperren: Dateien auf 0 Bytes kürzen, je Erfolg `anhang_belegt −= z`; scheitert das Kürzen, bleibt der Anhang **gelöscht** (410, anders als im Laufwerk) und gezählt, `ERROR anhang Löschen gescheitert`; das Ereignis `anhaenge` geht an alle offenen Sichten; `INFORMATION anhang Anhänge gelöscht` mit `grund=nachricht`, `fremde=0` (nur wenn etwas gelöscht oder gescheitert ist); nie ID, nie Anzeigename im Protokoll.
- **Antwort** `anhaenge: {geloescht, bytes, fehler}` je Löschen.
- **Laden**: die Anhänge einer gelöschten Nachricht sind `geloescht`, gleich welche Länge ihre Datei hat; eine Datei länger als 0 wird nach dem Laden, vor der Startzeile, gekürzt und abgezogen — so heilt ein Neustart einen Absturz zwischen Löschsatz und Kürzen.
- **Laufwerk** (F16, PO Zeile 96): die Anhänge einer gelöschten Nachricht erscheinen dort nicht mehr, auch nicht mit Namen — prüfbar erst mit I0027; hier gilt: sie sind `geloescht`.

### F0140 Verlauf bleibt schnell mit vielen Änderungen (ADR-0048 QE2)

- In einem Channel mit 100.000 Nachrichten, davon 10.000 je dreimal bearbeitet und 5.000 gelöscht, liefert der Verlauf die neuesten 100 weiterhin in weniger als 50 ms Server-Zeit (Q1), und die Instanz startet in weniger als 2 s (Q6) — im Release-Build.

## Nicht-funktionale Anforderungen

- **Zeitkriterien** nach `AI-Team/planer.md`: (1) „Ereignis weniger als 1 s nach der 200“ wie R00039 (Zweck Vision „Live überall“, Lastprofil; erwartet unter 50 ms; Uhr ab Empfang der 200). (2) „410 sofort“: der erste Abruf nach Empfang der 200 antwortet 410 — Regel, keine Wartezeit (der Tausch geschieht vor der Antwort). (3) Abwesenheit eines Ereignisses über 2 s beobachtet. (4) QE2 übernimmt Q1 (< 50 ms) und Q6 (< 2 s) aus ADR-0006/ADR-0025 unverändert; erwartet wird ein Wert nahe dem desselben Generators ohne Änderungen, weil die Fassung je Nachricht O(log m) kostet; der Test übernimmt die Grenzen, nie schärfer.
- **Sicherheit**: Gate `security-reviewer` mit ADR-0048 M1, M5–M7, M9, M15, M16; der gelöschte Inhalt (Text, Name und Größe der Anhänge) erscheint in keiner Antwort, keinem Ereignis und keinem Abruf mehr (M15) — nur die Tagesdatei auf der Platte hält den Text (Richtungsfrage 1).
- **Neustart** (QE3): was mit 200 beantwortet wurde, gilt nach einem Neustart genau so, Anhänge eingeschlossen.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start mit `TZ=Europe/Berlin localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser --plattenquote 10MiB --anhangquote 10MiB` auf einem Wegwerf-Ordner mit den leeren Ordnern `channels/allgemein/` und `channels/technik/`; `<p>`, `S(<name>)`, Kennung `<name>@127.0.0.1`, `t1` (`beta` sendet `Hallo` in `# allgemein`), `alpha` am Strom und „Tageslog unverändert“ wie in R00039. „Löschen“ heißt `POST http://127.0.0.1:<p>/api/channels/allgemein/nachrichten/loeschen` mit `Content-Type: application/json`, `S(beta)`, ohne `Origin`. „Nachricht mit zwei Anhängen“ heißt: `beta` hat `foto.png` (307.200 Bytes) und `bericht.pdf` (2.457.600 Bytes) angemeldet, hochgeladen und mit dem Text `Bericht` gesendet (R00029); Zeit `t3`; Zählmaß z(307.200) = 307.200, z(2.457.600) = 2.457.600, `anhang_belegt` = 2.764.800.

### F0136 Löschen über die API

- [ ] **Gespräche** (Unit, Speicherablage, gestellte Uhr, aufzeichnender Rückruf; wörtlich aus ADR-0048 „Erster Unit-Test (Gespräche)“): `alice` sendet t1 (Tag 1) und t2 (Tag 1); Uhr auf Tag 2; `alice` bearbeitet t1 zweimal, löscht t2; `bob` bearbeitet t1 → `Fremd`, Ablage unverändert; Block → t1 mit zweitem Text, `zustand: bearbeitet`, `geaendert` = Zeit der zweiten Änderung; t2 `geloescht`, Text leer; Rückruf erhielt drei `aenderung` unter der Sperre; `belegt` = Σ Bytes; Neuladen aus der Ablage → derselbe Block; Entfallen von Tag 1 → Fassungen weg, Tag 2 bleibt, Neuladen ohne Tag 1 → zwei Waisen, keine `WARNING`; Bearbeiten t2 → `Geloescht`; gleicher Text → kein Satz. Mutanten „Kennung nicht geprüft“, „Bearbeitet nach Geloescht übernommen“, „Änderungssatz im Block als Nachricht“ rot. — Lesart des planer: „zwei Waisen“ sind die zwei Nachrichten t1 und t2; `DEBUG laden Änderungen ohne Nachricht` zählt die drei Änderungssätze (`anzahl=3`); „Bearbeiten t2 → `Geloescht`“ gilt vor dem Entfallen.
- [ ] **Rundweg** (Integration, echtes Programm): `beta` löscht `{"zeit": t1}` → 200 mit `ergebnis: "geloescht"`, `nachricht.text: ""`, `nachricht.anhaenge: []`, `nachricht.zustand: "geloescht"`, `geaendert` > t1, `anhaenge: {geloescht: 0, bytes: 0, fehler: 0}`; `GET /api/channels/allgemein/nachrichten` → t1 mit `zustand: "geloescht"` und Text `""`; Health `nachrichten` bleibt 1; `log_bytes` ist um die Länge des Löschsatzes gestiegen; `grep -c '^  Hallo$' channels/allgemein/<heute>.log` ergibt 1 und die Datei enthält `+ geloescht: <t1>`; Neustart → derselbe Verlauf.
- [ ] **Zweimal löschen, danach bearbeiten** (Integration; M5 Teil Folge): zweites Löschen → 200 `ergebnis: "schon_geloescht"`, Tageslog unverändert, kein `aenderung` bei `alpha` binnen 2 s; Bearbeiten von t1 → 409 `regel: "geloescht"` mit `fehler` „Diese Nachricht ist gelöscht.“, Tageslog unverändert.
- [ ] **Spätere Sitzung derselben Kennung** (Integration; Richtungsfrage 6): `beta` verlässt und tritt erneut als `beta` bei → Löschen von t1 mit dem neuen Token → 200.
- [ ] **Fremd und nicht da** (Integration; M1, M3, M4): `gamma` löscht t1 → 403 `fremd`; t1 + 1 ms → 404 `nachricht`; t1 an `…/channels/technik/nachrichten/loeschen` → 404 `nachricht`; jeweils Tageslog unverändert, kein Ereignis, bei `gamma` an t3: beide Anhänge weiter abrufbar (200).
- [ ] **Live** (Integration; ADR-0048 Integrationstest „… Löschen über die API mit Strom-Mitschnitt (Ereignis < 1 s)“): `alpha` erhält genau ein `aenderung` mit `nachricht` = `{zeit: t1, von: "beta@127.0.0.1", art: "agent", text: "", zustand: "geloescht", geaendert: <wie in der 200>, anhaenge: []}`, weniger als 1 s nach der 200; Neustart → gleicher Verlauf.
- [ ] **Körper** (Integration; M10, M12): `{"zeit":t1,"zeit":t1}` → 422 `{regel: "feld", feld: "zeit", grund: "doppelt"}`; `{"zeit":t1,"bestaetigt":true}` → 422 `feld` (weiteres Feld); 1.025 Bytes Körper → 413 vor dem Parse; `text/plain` → 415; fremder `Origin` → 403; ohne Sitzung → 401; `"zeit":"gestern"` → 422 `zeit`; jeweils Tageslog unverändert.
- [ ] **Kosten** (Integration): ein Löschen kostet 1 Einheit — nach 30 Sendungen von `beta` in einem Zug antwortet Löschen 429 mit `Retry-After`, nichts geschrieben.
- [ ] **Reserve an der Plattenquote** (Unit, Speicherablage, `hoechstens` = 1.048.576, also R = 1.048.576 und Grenze 2.097.152; L = Länge des Löschsatzes): `belegt` = 2.097.152 − L → Löschen gelingt, `belegt` = 2.097.152; nächstes Löschen → 507 `quote`, Ablage unverändert; Bearbeiten bei `belegt` ≥ 1.048.576 → 507. R für 4 GiB = 42.949.673, für 10 MiB = 1.048.576 (rein). Warnschwelle bei 90 % von `hoechstens` ohne Reserve; `aus` → nie 507.
- [ ] **Plattenquote im echten Programm** (Integration, `quote-ordner`; wörtlich aus ADR-0048 Integrationstest „Quote voll → Bearbeiten 507, Löschen 200, bis R erschöpft → 507“): vor dem Start füllt `target/debug/examples/quote-ordner <ordner> allgemein <bytes>` bis 300 Bytes unter `hoechstens` (10.485.760); `beta` sendet `Hallo` (t1) und `Welt` (t2); Bearbeiten von t1 mit 400 Bytes → 507; Löschen von t1 → 200. Neustart mit `--plattenquote 5MiB` (Grenze 5.242.880 + 1.048.576 = 6.291.456 < `belegt`) → Löschen von t2 → 507 mit „Nicht gelöscht: …“, Tageslog unverändert; die `quote`-Episode trägt `route=loeschen`.
- [ ] **Wettlauf Bearbeiten und Löschen** (M5): Unit (Gespräche, zwei Threads) — 100 Nachrichten, je ein Bearbeiten und ein Löschen gleichzeitig → jede endet `geloescht`, jedes Bearbeiten nach dem Löschen ergibt `Geloescht` (409), nie ein Änderungssatz nach dem Löschsatz derselben Nachricht; Neuladen → dieselben Fassungen. Integration — fünf solche Paare aus zwei Sitzungen von `beta` → dieselben Aussagen, Neustart → gleicher Verlauf.
- [ ] **Von Hand geschrieben** (Integration): vor dem Start steht nach einem Löschsatz zu t1 ein `bearbeitet`-Satz derselben Kennung zu t1 → `WARNING laden` mit Datei und Zeile, t1 bleibt `geloescht`.
- [ ] **Betriebsprotokoll** (Integration, `--protokoll-stufe debug`): Rundweg → `DEBUG aendern Nachricht gelöscht | channel=allgemein kennung=beta@127.0.0.1 anhaenge=0 dauer_ms=…`; zweites Löschen mit `grund=schon_geloescht`; `grep -r "Hallo" logs/` ist leer.

### F0137 Anhänge einer gelöschten Nachricht mitlöschen

- [ ] **Löschen mit Anhängen** (Unit; wörtlich aus ADR-0048 „Erster Unit-Test Löschen mit Anhängen“): wie ADR-0046 F1′ „Erste Tests“ mit einer Nachricht mit drei Anhängen, Kürzen-Schranke und Kürzen-Fehler für einen → 410 sofort, G sinkt nach (c) um zwei, der dritte bleibt `geloescht` und gezählt; Neuladen → alle drei `geloescht`, der dritte wird nach dem Laden gekürzt; Laufwerk-Löschen eines vierten Anhangs derselben Nachricht vorher → `schon_geloescht`, ein Abzug. Mutante „Fehler setzt zurück auf `vorhanden`“ wird rot (Pre-Mortem 3).
- [ ] **Rundweg mit Anhängen** (Integration): Nachricht mit zwei Anhängen; `alpha` am Strom; `beta` löscht t3 → 200 mit `anhaenge.geloescht` = 2, `fehler` = 0; der erste `GET /anhaenge/<id>` nach der 200 antwortet für beide 410; `anhang_belegt` in `/api/gesundheit` sinkt von 2.764.800 auf 0, sobald beide Dateien gekürzt sind (Abfrage höchstens 1 s nach der 200 — Kürzen zweier Dateien, erwartet wenige ms); `alpha` erhält `aenderung` (t3 mit `anhaenge: []`) und `anhaenge` mit beiden IDs und Satzzeit t3, beide weniger als 1 s nach der 200; im Protokoll genau eine `INFORMATION anhang Anhänge gelöscht` mit `grund=nachricht` und `fremde=0`, ohne ID und ohne `foto.png`; Neustart → weiter 410, `anhang_belegt` 0, beide Dateien haben 0 Bytes.
- [ ] **Vorher im Laufwerk gelöscht** (Integration; M7): `gamma` löscht `foto.png` mit `POST /api/anhaenge/loeschen` und `bestaetigt: true` (R00029); danach löscht `beta` t3 → `anhaenge.geloescht` = 1; `anhang_belegt` ist insgesamt um genau 2.764.800 gesunken (jeder Anhang einmal).
- [ ] **Kürzen scheitert** (Integration, nicht als root): vor dem Löschen `chmod 0444` auf die Datei von `bericht.pdf` → Löschen 200 mit `anhaenge.geloescht` = 1, `fehler` = 1; beide Abrufe 410; `anhang_belegt` = 2.457.600; `ERROR anhang Löschen gescheitert` im Protokoll; Rechte zurück (`chmod 0644`), Neustart → nach der Startzeile `anhang_belegt` 0, die Datei hat 0 Bytes.
- [ ] **Absturz zwischen Löschsatz und Kürzen** (Integration; QE3, Pre-Mortem 3): vor dem Start enthält der Ordner eine Nachricht mit einem gebundenen Anhang in voller Länge und danach von Hand einen Löschsatz derselben Kennung → nach dem Start antwortet der Abruf 410, `anhang_belegt` ist 0, die Datei hat 0 Bytes; der Verlauf zeigt die Nachricht `geloescht` ohne Anhänge.
- [ ] **Wettlauf mit dem Entfallen** (Unit; M6): Uhr 23:59:59.900, Frist 1, Nachricht mit Anhang vom Vortag; Entfallen und Löschen gleichzeitig → entweder 404 `nachricht` oder `geloescht`; je Anhang genau ein Abzug; die Invarianten aus ADR-0046 Ergänzung F1′ Regel 1 gelten danach.
- [ ] **Gelöschter Inhalt kommt nicht zurück** (Integration; M15): nach dem Löschen von t3 nennt keine Antwort — Verlauf, Senden-Antwort einer neuen Nachricht, Ereignis, `GET /anhaenge/<id>` — `Bericht`, `foto.png` oder `bericht.pdf`.

### F0140 Verlauf bleibt schnell mit vielen Änderungen

- [ ] **Messung QE2** (Release, wie `tests/leistung.rs`; wörtlich aus ADR-0048 „Leistung QE2 als Messwert mit I0032 (Release, Generator wie `tests/leistung.rs` plus 30.000 Änderungssätze)“): 100.000 Nachrichten, 10.000 davon je dreimal bearbeitet und 5.000 gelöscht → neueste 100 in weniger als 50 ms Server-Zeit, Start in weniger als 2 s; der Bericht des Entwicklers nennt beide Messwerte und denselben Lauf ohne Änderungssätze zum Vergleich.

## Betroffene Verzeichnisstruktur

- `src/gespraeche/mod.rs` (`loeschen` neben `bearbeiten`), `src/gespraeche/quote.rs` (Reserve), `src/gespraeche/loeschen.rs` (Schritte b, c mit der Ausnahme „bleibt gelöscht“), `src/gespraeche/anhangverzeichnis.rs` (`tauschen`), `src/gespraeche/anhaenge.rs` (Laden), `src/start/mod.rs` (Kürzen nach dem Laden, vor der Startzeile).
- `src/http/nachricht_loeschen.rs` neu (oder neben `src/http/bearbeiten.rs`), Route in `src/http/mod.rs`.
- `src/protokoll/` (Katalogzeilen `aendern`, `anhang` mit `grund=nachricht`).
- Tests: `tests/nachricht_loeschen.rs`, `tests/nachricht_loeschen_anhaenge.rs`, `tests/leistung.rs`; Unit in den Modulen.
- Architektur nach dem Bau (architekt): A00006, A00016, A00004.

## Technische Überlegungen

### Ablauf (Vorplanung — welche Bubbles es am Ende werden, entscheidet der Entwickler)

1. **Reserve** (B0614) → **Löschen unter den Sperren** (B0613) → **Route** (B0615) → **Protokoll** (B0616).
2. **Anhänge**: markieren im Sperrabschnitt (B0617) → kürzen und abziehen außerhalb (B0618) → Laden leitet gelöscht ab und kürzt vor der Startzeile (B0619).
3. **Messung** (B0625) nach 1 und R00039.

Reihenfolge: R00039 F0132 und F0133 vorher (gleiche Dateien). F0137 erst nach dem Merge von #9 Teil B (`src/start/mod.rs`) und #66 (`src/gespraeche/anhangpfade.rs` `kuerzen`, R00038 F0131).

### Grobentwurf

- `Gespraeche::loeschen(channel, kennung, ziel) -> Result<Loeschergebnis, Abweisung>` (Integration); `Loeschergebnis { nachricht, ergebnis: Geloescht | SchonGeloescht, gewonnen: Anhangliste }`.
- `Plattenquote::reserve(hoechstens) -> u64` (rein).
- Schritte (b) und (c) aus `src/gespraeche/loeschen.rs` (B0507) mit einem Schalter oder einer eigenen Variante für „bei Fehler gelöscht lassen“.

## Aufwand (Zählung)

Gezählt aus der WBS; Richtwerte, ungemessen für diese Bubble-Typen.

- F0136: 4 Bubbles · 2 klar zu 0,4 h (B0614, B0616; 0,8 h) · 2 unklar zu 2–4 h (B0613, B0615; 4–8 h)
- F0137: 3 Bubbles · 1 klar (B0617; 0,4 h) · 2 unklar (B0618, B0619; 4–8 h)
- F0140: 1 Bubble · unklar (B0625; 2–4 h)
- Summe 8 Bubbles, 11,2–21,2 h.

## Abhängigkeiten

- Abhängig von: R00039 F0132, F0133 (rot); R00029 F0116 Anhänge löschen über die API und F0117 Gelöschte Anhänge in offenen Sichten (grün); I0023 Plattenquote; **#9 Teil B** (`src/start/mod.rs`) und **#66 / R00038** (`anhangpfade.rs`) — F0137 erst nach deren Merge.
- Blockiert: R00041 F0138, F0139 (Browser); I0027 Laufwerk (zeigt Anhänge gelöschter Nachrichten nicht, F16); I0026 Reaktionen an einer gelöschten Nachricht (409 `geloescht`); I0011 Suche (findet gelöschte Nachrichten nicht).

## Offene Fragen

- **An den architekt** (nicht blockierend, Lesart im Kriterium): „zwei Waisen“ im Unit-Test „Gespräche“ — gezählt als Nachrichten (t1, t2), die DEBUG-Zeile zählt Änderungssätze (3)? Und: ADR-0048 Kopf nennt Regel 12 noch „Vorgeschlagen“, obwohl F15 entschieden ist (PO Zeile 95) — die Kriterien folgen Option 1.
- **Prüfpunkt für den `security-reviewer`** (M2, gewollt): eine spätere Sitzung derselben Kennung — auch jemand anderes, der ≈ 1 min nach dem Gehen mit demselben Namen von derselben Herkunft beitritt — kann die Nachrichten samt Anhängen löschen (Richtungsfrage 6). Kein Befund, wenn so gebaut; vom Director als Anmerkung zu führen.

## Missing-Docs

- Glossar fehlt; Begriffe aus ADR-0048 und R00029.
- Wie `tests/leistung.rs` Q1 und Q6 misst (Statistik, Rechner) steht nur im Test selbst.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist die Frage des Betreibers „Und Nachricht löschen?“: heute bleibt jede Nachricht stehen, bis ihr Tag entfällt — auch eine mit vertraulichem Inhalt oder einem Anhang im falschen Channel. Wenn der Absender löscht, dann schreibt die Instanz einen Löschsatz, setzt die Fassung auf gelöscht und tauscht im selben Schritt jeden Anhang auf gelöscht, sodass ab der Antwort keine Sicht, kein Ereignis und kein Abruf den Inhalt mehr liefert; das Kürzen danach gibt den Platz der Anhang-Quote frei, und ein Neustart holt nach, was ein Absturz unterbrochen hat. Die Reserve an der Plattenquote ist der Hebel dafür, dass Löschen gerade dann geht, wenn die Platte voll ist — sonst könnte niemand eine versehentlich gepostete Nachricht verbergen, bis Tage entfallen. Der Text bleibt bewusst im Log, weil der Betreiber „nur aus der Sicht“ gewählt hat; darum ist die Zusage „aus jeder Sicht“, nicht „von der Platte“.

## Notizen

- **Verworfen** (ADR-0048): Löschen prüft die Quote nie (unbegrenztes Überschreiten); Löschen wie Senden 507 (an der vollen Quote ließe sich nichts verbergen); Server-Bestätigung wie im Laufwerk (PO Zeile 95: es trifft nur Eigenes; Agenten wollen keinen Zwei-Schritt); Rückgängig für einige Sekunden (PO: passt nicht zu „kein Rückgängig“ und „Platz sofort frei“).
- **Nicht hier**: Direktgespräche (I0012), Laufwerk-Ansicht (I0027, F16), Suche (I0011), Reaktionen (I0026).
- F6 und F7 aus ADR-0046 F1′ gelten für mitgelöschte Anhänge unverändert: kein Rückgängig, wer gelöscht hat, steht nur in der `INFORMATION`-Zeile.
