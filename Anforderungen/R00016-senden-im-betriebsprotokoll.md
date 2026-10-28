---
id: R00016
status: Neu
datum: 2026-10-28
---

# R00016: Senden im Betriebsprotokoll

## Beschreibung

Die drei Betriebsmeldungen des Sendens erscheinen als Episoden im Betriebsprotokoll (R00014), nicht mehr als eigene stderr-Zeilen:

- die Bremse des Schreibkontos (429);
- der Schreibfehler beim Anhängen (500);
- der Vorlauf der Nachrichtenzeit.

Die Zusammenfassung einer Schreibkonto-Episode nennt, wie oft gebremst wurde und wie lange; `gebremst` in `/api/gesundheit` zählt jede 429. Damit misst der Betrieb, ob das Schreibkonto je Herkunft dem Lastprofil genügt. Diese Messung ersetzt die Frage nach der Dauer einer Spitze, die der Betreiber nicht beantworten kann (Entscheidung Director 2026-10-28: Auslegung nach Messung im Betrieb).

Zahlt ein auf: [Vision](R00000-vision.md) — „eine gemeinsame LAN-Instanz“: eine Flut, die gebremst wird, ist für die Betreiberin sichtbar und messbar.

Entschieden vom architekt: ADR-0042 Regel 2 (Katalog, Zeilen „→ I0009“), Regel 5 (Episode) und Ergänzung 5 (Ende in der Stufe der Episode), ADR-0036 Ergänzung 2026-10-28 (stderr wird Betriebsprotokoll, `gebremst` bleibt), ADR-0040 Regeln 3 und 6.

Slice (WBS `Dokumentation/Planung/localchata.md`): F0057 Senden im Betriebsprotokoll unter I0009 Nachricht senden; er braucht F0045 Schreibkonto (R00011) und F0055 Grenzen, Episoden und Tagesbudget (R00014).

## Geschäftlicher Nutzen

Ob 120 Einheiten Vorrat je Herkunft eine Spitze von 30 Nachrichten/s tragen, hängt an deren Dauer, und die kennt niemand. Statt zu raten, misst die Instanz: jede Bremsung steht mit Kennung, Zahl und Dauer im Protokoll, und `gebremst` zählt. Zeigen die Episoden Bremsungen im normalen Betrieb, ist der Vorschlag aus A00001 (16/s, Vorrat 300) begründet; zeigen sie keine, bleibt das Konto, wie es ist.

## Funktionale Anforderungen

- **Schreibkonto**: jede 429 der Bremse zählt in eine Episode `WARNING schreibkonto` je Kennung. Kontext: `kennung`, `grenze` (`kennung` oder `herkunft`).
  - Die erste 429 schreibt „… — Episode beginnt“.
  - Nach der Ruhe (Standard 60 s, `LOCALCHATA_EPISODE_RUHE_MS`) schreibt die Episode „… — Episode endet: <n> in <d> s“ in der Stufe `WARNING`. `n` ist die Zahl der 429 dieser Kennung in der Episode, `d` die Dauer nach R00014.
  - `gebremst` zählt weiter jede 429, `warnings` jedes Auftreten.
- **Schreibfehler beim Senden**: 500 zählt in eine Episode `ERROR senden` je Datei und Grund. Kontext: `channel`, `kennung`, `datei`, `grund`. `errors` zählt jedes Auftreten.
- **Vorlauf der Nachrichtenzeit** über 60 s ist eine Episode `WARNING zeit` mit `dauer_ms`. Der Text bleibt sinngemäß wie in R00011: „Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt.“ `zeit_vorlauf_ms` bleibt.
- Die Meldungen erscheinen auf stderr in der Form des Betriebsprotokolls (ab `WARNING`); eigene stderr-Zeilen und eigene Episoden-Mechaniken des Sendens entfallen.
- Nie im Protokoll: Nachrichtentext, Token.

## Nicht-funktionale Anforderungen

- Sicherheit: eine Schleife, die gebremst wird, erzeugt höchstens eine Beginn-Zeile, eine Zwischenzeile je 10 Minuten und eine Ende-Zeile (CWE-770).
- Zeitkriterien nach `AI-Team/planer.md`: die Ruhe prüft der Unit-Test aus R00014 exakt, das echte Programm nur mit `LOCALCHATA_EPISODE_RUHE_MS=1000` und oberer Grenze.

## Akzeptanzkriterien

Gemeinsam: Start wie in R00011, dazu `LOCALCHATA_EPISODE_RUHE_MS=1000`, `TZ=Europe/Berlin`. `beta` ist von `127.0.0.1` beigetreten, `channels/allgemein/` liegt vor. `heute.log` ist `logs/<lokales Datum>.log`.

### F0057 Senden im Betriebsprotokoll

- [ ] **Bremse im Betrieb messen** (Integration):
  - `beta` sendet 60 kleine Nachrichten `Schleife-<i>` so schnell wie möglich; `k` ist die Zahl der 429.
  - Danach wartet der Test, bis in `heute.log` eine Ende-Zeile steht, höchstens 3 s nach der letzten 429. Erwartet sind ≈ 1–2 s: 1 s Ruhe plus höchstens 1 s Takt. Die Grenze liegt 50 % über 2 s.
  - Erwartet: genau eine `WARNING schreibkonto … Episode beginnt | kennung=beta@127.0.0.1 grenze=kennung` und genau eine `WARNING schreibkonto … Episode endet: <k> in <d> s` mit `k` gleich der Zahl der 429 und `d` ≥ 0;
  - `/api/gesundheit` meldet `gebremst: k`;
  - stderr enthält dieselben zwei Zeilen und keine Zeile `beta@127.0.0.1 gebremst (kennung) — Flut?`;
  - `grep -c Schleife- logs/*.log` ergibt 0.
- [ ] **Episode je Kennung** (Integration): `beta` und `gamma` (beide `127.0.0.1`) laufen je in die Bremse. Es entstehen zwei getrennte Episoden mit je eigener Beginn- und Ende-Zeile, und `gebremst` ist die Summe.
- [ ] **Schreibfehler** (Integration, nicht als root): `channels/allgemein/` ohne Schreibrecht, `beta` sendet zweimal `Hallo`. Erwartet:
  - zweimal 500;
  - genau eine `ERROR senden … Episode beginnt`-Zeile mit `channel=allgemein`, `kennung=beta@127.0.0.1`, `datei=channels/allgemein/<tag>.log` und `grund=`;
  - nach der Ruhe eine `ERROR senden … Episode endet: 2 …`;
  - `/api/gesundheit` meldet `errors` ≥ 2;
  - der Text `Hallo` steht nicht im Protokoll.
- [ ] **Vorlauf** (Integration, wie R00011 „Vorlauf im echten Programm“): Start mit `LOCALCHATA_JETZT=2026-10-28T12:00:00.000+01:00`; in `channels/allgemein/2026-10-28.log` liegt ein Satz um `14:03:30.000+01:00`. `beta` sendet zweimal. Erwartet:
  - genau eine `WARNING zeit`-Zeile mit „Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus“ und `dauer_ms=`;
  - `zeit_vorlauf_ms` wie in R00011.
- [ ] **Keine eigene Mechanik mehr** (Review-Punkt): `src/schreibkonto/` und `src/gespraeche/` enthalten kein `eprintln!` und keine eigene Episoden-Tabelle; die Episode aus B0235 (30 s) ist entfernt.

## Betroffene Verzeichnisstruktur

- `src/schreibkonto/` (Bremse meldet an das Protokoll), `src/gespraeche/` (Schreibfehler, Vorlauf), `src/http/nachrichten.rs` (Kontext `channel`, `kennung`); Tests unter `tests/` (Integration über das echte Programm).

## Technische Überlegungen

### Ablauf

1. `abbuchen` → `Gebremst` → `protokoll.episode(schreibkonto, kennung, Warning, {kennung, grenze})` → 429.
2. `senden` → Schreibfehler → `protokoll.episode(senden, datei+grund, Error, {channel, kennung, datei, grund})` → 500.
3. Zeit vergeben → Vorlauf > 60 s → `protokoll.episode(zeit, instanz, Warning, {dauer_ms})`.

### Grobentwurf (Vorplanung)

- B0286 Senden-Meldungen als Episode · B0287 Bremse im Betrieb messen. Welche Bubbles es am Ende werden, entscheidet der Entwickler.

### Änderungen an bestehenden Klassen

- `Schreibkonto`: die eigene Episode (B0235) entfällt, `gebremst()` bleibt; das Protokoll kommt als Provider hinzu.

## Tests

- Unit: keine neuen. Die Episodenregeln prüft R00014, die Eimer prüft R00011.
- Integration: die vier Kriterien oben.
- E2E: keine.

## Abhängigkeiten

- Abhängig von: F0045 Schreibkonto (rot, R00011), F0041 Senden über die API (rot, R00011), F0055 Grenzen, Episoden und Tagesbudget (rot, R00014).
- Blockiert: —

## Offene Fragen

- Keine für den Bau.
- An den Betreiber, offen: die Dauer einer Spitze aus einer Herkunft. Nach der Entscheidung des Directors folgt die **Auslegung nach Messung im Betrieb**; diese Anforderung liefert die Messung. Zeigen die Episoden im normalen Betrieb Bremsungen mit `grenze=herkunft`, ist der Vorschlag aus A00001 zu entscheiden (Nachschub je Herkunft 16/s, Vorrat 300).

## Manuelle Vorbereitungstätigkeiten

- Keine.

## Manuelle Nachbereitungstätigkeiten

- Keine.

## Umfang

- F0057 Senden im Betriebsprotokoll: 2 Bubbles · 2 klar (0,8 h; Untergrenze, ungemessen für diesen Typ)

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist die offene Frage nach der Dauer einer Spitze: von ihr hängt ab, ob das Schreibkonto je Herkunft einen Rechner mit 50 Agenten im Lastprofil bremst. Niemand kennt sie, also entscheidet der Director, im Betrieb zu messen.

Die Wirkungskette:

- Die Bremse meldet sich als Episode im Betriebsprotokoll, mit Kennung, Grenze, Zahl und Dauer.
- Deshalb sieht die Betreiberin, ob, wie oft und wie lange gebremst wird und an welcher Grenze.
- Damit wird die Entscheidung über das Konto eine Messung statt eine Vermutung.

Der Hebel liegt an der Meldung, nicht am Konto: das Konto schützt weiter vor Flut, und die Episode verhindert, dass die Messung selbst das Protokoll flutet.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).

## Notizen

- **Änderung von Zusagen aus R00011** (R00011 bleibt unverändert):
  - Die stderr-Zeile `beta@127.0.0.1 gebremst (kennung) — Flut?` (US-22, F0045 „Betrieb“) wird die Protokollzeile `WARNING schreibkonto …` mit `kennung=` im Kontext, denn ADR-0040 Regel 3 verbietet Kennungen im Freitext.
  - Das Episodenende „30 s ohne 429“ (F0045 „Episode“) wird die Ruhe der Protokoll-Episode (60 s).
  - Die Vorlauf-Episode endet nach der Ruhe statt „sobald die Uhr aufholt“.
  - Die Zeile des Schreibfehlers steht im Protokollformat; „stderr nennt Datei und Grund“ bleibt wahr.
- Empfehlung an den Director: F0045 und F0057 im selben Lauf bauen. I0009 kommt nach I0021, deshalb entsteht die eigene 30-s-Episode sonst erst und wird gleich wieder entfernt.
- Herkunft: Entscheidung Director 2026-10-28 (Schreibkonto bleibt, Auslegung nach Messung im Betrieb); A00001, Abgleich Lastprofil.

### Bewusst außerhalb

- Neue Grenzen des Schreibkontos.
- Einträge der Fristpflege (I0018).
