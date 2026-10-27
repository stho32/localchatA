---
id: R00001
status: Neu
datum: 2026-10-27
---

# R00001: Instanz starten

## Beschreibung

Der Betreiber startet mit `localchatA <ordner>` eine Instanz, die Weboberfläche und API für genau diesen Ordner ausliefert und im LAN erreichbar ist. Die Startzeile nennt die tatsächliche Adresse, ein Health-Check bestätigt die Bereitschaft, eine zweite Instanz auf demselben Ordner wird abgewiesen, und der Browser zeigt die (noch leere) Oberfläche. Diese Anforderung ist das Gerüst (Walking Skeleton), auf dem alle weiteren Interactions aufsetzen; Logablage, Verlauf und Live-Kanal gehören nicht dazu.

Zahlt ein auf: [Vision](R00000-vision.md) — Leitplanken „ein einziges Programm, das Weboberfläche und WebAPI zugleich ausliefert“ und „eine gemeinsame Instanz im LAN“; Zielbild „Über Rechnergrenzen“.

Slices (WBS `Dokumentation/Planung/localchata.md`): F0001 Starten mit Adresse, F0002 Ordner gegen zweite Instanz sperren, F0003 Oberfläche ausliefern — alle unter I0001 Instanz starten.

## Geschäftlicher Nutzen

Ohne laufende Instanz kann weder ein Mensch noch ein Agent teilnehmen. Mit diesem Gerüst ist der ganze Weg vom Aufruf über die HTTP-Grenze bis in den Browser bewiesen und durch alle drei Testebenen abgesichert; jede folgende Interaction baut darauf, statt Start, Port und Testaufbau jeweils neu zu erfinden. Die Ordner-Sperre schützt von Anfang an davor, dass zwei Instanzen denselben Verlauf verschränken (Risiko 2 in A00001).

## Funktionale Anforderungen

- `localchatA [<ordner>] [--port <n>] [--host <adresse>] [--kein-browser] [--hilfe]` startet das Programm für den Ordner; ohne `<ordner>` gilt das aktuelle Arbeitsverzeichnis.
- Ein fehlender Ordner wird angelegt.
- Standard ist Host `0.0.0.0` (alle Schnittstellen, LAN) und Port 7430; ist 7430 belegt, nimmt das Programm den nächsten freien Port aufwärts (höchstens 50 Versuche); `--port 0` lässt das System den Port wählen; ein ausdrücklich genannter belegter Port führt nicht zum Ausweichen, sondern zum Abbruch.
- Nach dem Binden schreibt das Programm genau eine Startzeile auf stdout: `localchatA läuft für <ordner> unter http://<host>:<port>/` mit absolutem Ordnerpfad und tatsächlichem Port.
- `GET /api/gesundheit` antwortet 200 mit JSON `{"status":"bereit","ordner":…,"nachrichten":…,"log_bytes":…,"ladezeit_ms":…,"defekte_zeilen":…}`.
- Das Programm sperrt den Ordner über die Datei `<ordner>/.localchatA.sperre`; ein zweiter Start auf demselben Ordner endet mit Exit 1.
- `GET /` liefert die eingebettete Oberfläche; Dateien unter `/web/` werden aus dem Programm ausgeliefert, unbekannte Pfade ergeben 404.
- Ohne `--kein-browser` öffnet das Programm nach der Startzeile den Standardbrowser mit der Oberfläche.
- Exit-Codes: 0 geordnet beendet oder `--hilfe`; 1 Ordner, Port oder Sperre scheitert; 2 falscher Aufruf. SIGTERM und Strg+C beenden geordnet.

## Nicht-funktionale Anforderungen

- Betrieb: nach dem Start lädt die Oberfläche nichts aus dem Internet nach (alle Dateien eingebettet, ADR-0002).
- Diagnose: Fehler stehen einzeilig auf stderr, die Startzeile ist die einzige Zeile auf stdout (Aspekt X00001), damit Tests und Director die Adresse maschinell lesen.
- Testbarkeit: das Programm startet in Tests mit `--port 0`, Wegwerf-Ordner und `--kein-browser`, nie auf einem festen Port; 7420 und 7421 werden nie benutzt.

## Akzeptanzkriterien

### F0001 Starten mit Adresse

- [ ] `localchatA /tmp/lc-r1 --port 0 --host 127.0.0.1 --kein-browser` mit nicht vorhandenem `/tmp/lc-r1`: der Ordner existiert danach, stdout enthält genau die Zeile `localchatA läuft für /tmp/lc-r1 unter http://127.0.0.1:<p>/` mit `<p>` ≠ 0.
- [ ] `GET http://127.0.0.1:<p>/api/gesundheit` → 200, JSON mit `"status":"bereit"`, `"ordner":"/tmp/lc-r1"` und für den leeren Ordner `"nachrichten":0`, `"log_bytes":0`, `"defekte_zeilen":0`.
- [ ] Ohne `--port` und mit freiem 7430: die Startzeile nennt Port 7430 und Host `0.0.0.0`.
- [ ] Ohne `--port` und mit belegten Ports 7430 und 7431: die Startzeile nennt Port 7432 (Beispiel: zwei belegt → dritter Versuch).
- [ ] Ohne `--host`: der Health-Check antwortet 200 auch über die LAN-Adresse des Rechners (z. B. `http://192.168.1.20:<p>/api/gesundheit`), nicht nur über `127.0.0.1`.
- [ ] `--port <n>` mit belegtem `<n>`: Exit 1, eine Zeile auf stderr nennt den Port; kein Ausweichen.
- [ ] Ordner nicht anlegbar (z. B. `/tmp/lc-datei/unter`, wobei `/tmp/lc-datei` eine Datei ist): Exit 1, eine Zeile auf stderr nennt den Ordner.
- [ ] `localchatA --hilfe`: Exit 0, stdout zeigt die Aufrufzeile mit allen Optionen.
- [ ] `localchatA --gibtsnicht`: Exit 2, stderr nennt die unbekannte Option und verweist auf `--hilfe`.
- [ ] SIGTERM (bzw. Strg+C) an eine laufende Instanz: Exit 0; danach ist der Port wieder frei.

### F0002 Ordner gegen zweite Instanz sperren

- [ ] Während eine Instanz auf `/tmp/lc-r1` läuft, liegt dort `.localchatA.sperre`.
- [ ] Ein zweiter Start auf `/tmp/lc-r1` (mit `--port 0`) endet mit Exit 1; die Zeile auf stderr nennt den Ordner und dass er bereits von einer Instanz gesperrt ist.
- [ ] Nach dem abgewiesenen zweiten Start antwortet die erste Instanz weiter mit 200 auf `/api/gesundheit`.
- [ ] Nach geordnetem Ende der ersten Instanz startet eine neue Instanz auf `/tmp/lc-r1` (Sperre wieder frei), auch wenn `.localchatA.sperre` noch als Datei existiert.

### F0003 Oberfläche ausliefern

- [ ] `GET /` → 200, `Content-Type: text/html`, Inhalt enthält `<html lang="de">` und `<title>localchatA</title>`.
- [ ] `GET /web/gibtsnicht.js` → 404.
- [ ] Ausgelieferte Oberflächendateien tragen `Cache-Control: no-cache`.
- [ ] Ein Browser (Playwright, Chromium) auf der Adresse der Startzeile sieht eine Seite mit dem Titel `localchatA`, und die Konsole zeigt keine Fehler.
- [ ] Start ohne `--kein-browser`: der Standardbrowser öffnet die Oberfläche; bei Host `0.0.0.0` unter `http://127.0.0.1:<p>/`, sonst unter der Adresse der Startzeile.
- [ ] Lässt sich kein Browser öffnen (z. B. ohne grafische Sitzung), steht ein Hinweis auf stderr, und die Instanz läuft weiter (Health-Check 200).
- [ ] Mit `--kein-browser` öffnet sich kein Browser.

## Betroffene Verzeichnisstruktur

- Einstieg: `src/main.rs` (dünn) und `src/start/` (Aufruf, Ordner, Sperre, Port, Startzeile, Browser) nach A00003.
- HTTP-Grenze: `src/http/` (Router, Gesundheit, eingebettete Dateien) nach A00004; Fehlerausgabe `src/fehler.rs` (Aspekt X00004).
- Oberfläche: `web/` (`index.html`, `lc-tokens.css` aus dem Designsystem) nach A00005, eingebettet per `include_bytes!`.
- Tests: Unit im jeweiligen Modul, Integration `tests/` mit Hilfe `tests/gemeinsam/mod.rs`, E2E `e2e/` (ADR-0009).

## Technische Überlegungen

### Ablauf

1. **Aufruf lesen** — `Startauftrag::aus_argumenten(args)` → Startauftrag, Hilfewunsch oder Aufruffehler (Exit 2).
2. **Ordner vorbereiten** — anlegen, absoluten Pfad bilden; Fehler → Exit 1.
3. **Sperren** — `<ordner>/.localchatA.sperre` öffnen, `File::try_lock`; belegt → Exit 1.
4. **Binden** — Host und Port; Standard 7430 mit bis zu 50 Ausweichversuchen, `0` = System, ausdrücklicher Port ohne Ausweichen.
5. **Startzeile** auf stdout, danach (ohne `--kein-browser`) Browser öffnen.
6. **Ausliefern** — `/`, `/web/*`, `/api/gesundheit` bis SIGTERM/Strg+C; dann geordnet beenden, Exit 0.

Die Reihenfolge (Laden vor Binden) folgt A00003: ist der Port offen, ist das Programm bereit; einen Zustand „lädt“ gibt es nicht. In R00001 gibt es noch nichts zu laden.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0001: B0001 Crate-Gerüst (Crate-Liste ADR-0001 → baubare Crate `localchata`, lib + bin `localchatA`) · B0002 Aufruf lesen (Argumente → `Startauftrag` | `Aufruffehler` | Hilfewunsch) · B0003 Ordner vorbereiten (`Startauftrag.ordner` → absoluter Pfad | `Startfehler`) · B0004 Listener binden (Host, Wunschport → Listener + tatsächliche Adresse | `Startfehler`) · B0005 Gesundheit beantworten (`GET /api/gesundheit` → 200 JSON) · B0006 Start verdrahten (`Startauftrag` → laufender Server + Startzeile; Signal → geordnetes Ende) · B0007 Testhilfe Programm starten (Wegwerf-Ordner → Instanz mit Adresse, Ende beim Drop)
- F0002: B0008 Ordner sperren (Ordner → gehaltene Sperre | `Sperrfehler`) · B0009 Startfehler melden (`Startfehler` → stderr-Zeile + Exit 1; `Aufruffehler` → Exit 2)
- F0003: B0010 Eingebettete Datei nachschlagen (Pfad → Datei mit Content-Type | keine) · B0011 Oberfläche ausliefern (`GET /`, `/web/<datei>` → 200 mit `no-cache` | 404) · B0012 Browser öffnen (Adresse → Browser-Aufruf | Hinweis) · B0013 E2E-Gerüst (`e2e/` → Playwright sieht die Seite)

**Wichtige Einstiegsstellen:** `main` ruft `start`; `start` baut den Router aus `http` und bindet ihn; `http` bettet `web/` ein. Vorbild für Aufbau und Testmuster ist kanbanD (Q04, nur lesen, nichts kopieren).

**Elemente** (nur öffentliche Signaturen, Rust):
- `Startauftrag` (unveränderliches DTO) — Ordner, Port (`Option<u16>`: Standard | n | 0), Host, `browser_oeffnen`.
  - `fn aus_argumenten(args) -> Result<Aufruf, Aufruffehler>` mit `Aufruf = Starten(Startauftrag) | Hilfe`
- `Startfehler` (Fehlerart) — Ordner, Port, Sperre; je Art eine Meldung mit Ort.
- `Gesundheit` (DTO, serialisiert) — Felder wie oben.
- `fn router(zustand) -> axum::Router` (Integration an der HTTP-Grenze).

Wireframe: `Dokumentation/Wireframes/` enthält noch kein Artboard `D0001.dc.html` (Hinweis `/wireframe verfeinern D0001`); für R00001 ist die Oberfläche bewusst leer, das Designsystem liefert nur die Tokens.

### Änderungen an bestehenden Klassen

Keine — es gibt noch keinen Code.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests.

**Unit:** `Startauftrag::aus_argumenten` (erster Test laut A00003: `["/tmp/x", "--port", "0", "--kein-browser"]` → `Startauftrag { ordner: "/tmp/x", port: 0, host: "0.0.0.0", browser_oeffnen: false }`; dazu `--hilfe`, unbekannte Option, fehlender Portwert); Ausweichregel des Ports (welche Kandidaten in welcher Reihenfolge); Nachschlagen eingebetteter Dateien.

**Integration** (`tests/`, echtes Binärprogramm über `tests/gemeinsam/mod.rs`, Port 0, `tempfile`): Startzeile und Gesundheit; Ordner wird angelegt; zweiter Start → Exit 1 und erste Instanz lebt; Neustart nach SIGTERM; `GET /` und 404; Exit 2 bei falschem Aufruf; belegter ausdrücklicher Port → Exit 1.

**E2E** (`e2e/`, Playwright fest gepinnt, `globalSetup` baut, Port 0, `mkdtemp`, `TZ=Europe/Berlin`, `locale: de-DE`): Seite mit Titel `localchatA`, keine Konsolenfehler.

Standard-Port 7430 mit Ausweichen und Browser-Öffnen werden im automatisierten Lauf über die reine Ausweichregel bzw. den Schalter geprüft; der echte Weg ist Sache von Entwicklertest und Abnahme (kein fester Port in Tests).

## Abhängigkeiten

- Abhängig von: — (erster Slice; Architektur A00002–A00005, ADR-0001, ADR-0002, ADR-0005 Abschnitt Ordner-Sperre, ADR-0009)
- Blockiert: I0002 Nachricht landet im Log, I0004 Namen wählen und beitreten (beide `Braucht` I0001); innerhalb: F0002 und F0003 brauchen F0001.

## Offene Fragen

Keine. Die Fragen des architekt zu I0001 sind mit seiner Empfehlung entschieden (Standard-Port 7430 mit Ausweichen, Startzeile mit tatsächlicher Adresse; Browser öffnet sich, abschaltbar mit `--kein-browser`; Startzeile und Exit-Codes nach A00002). Die Richtungsfrage aus ADR-0005 („an den Dateien“ schreiben) betrifft R00001 nicht: die Ordner-Sperre gilt in beiden Varianten.

## Manuelle Vorbereitungstätigkeiten

- Rust stable ≥ 1.89, Node.js mit npm; einmalig `npm --prefix e2e ci` und `npx --prefix e2e playwright install chromium` (`AI-Team/umgebung.md`).

## Manuelle Nachbereitungstätigkeiten

- `AI-Team/umgebung.md` gegen die tatsächlichen Befehle prüfen, sobald das Gerüst steht.

## Umfang

Aus der WBS gezählt (`/planung zaehlen`), Richtwerte ungemessen:

- F0001 Starten mit Adresse = 7 Bubbles: 6 Standard (~2h), 1 unklar (2–4h). Rest: 12h klar + 2–4h unklar · 0 von 7 Werten belegt, Rest Richtwerte (ungemessen).
- F0002 Ordner gegen zweite Instanz sperren = 2 Bubbles: 2 Standard (~2h). Rest: 4h klar · 0 von 2 Werten belegt, Rest Richtwerte (ungemessen).
- F0003 Oberfläche ausliefern = 4 Bubbles: 2 Standard (~2h), 2 unklar (2–4h). Rest: 4h klar + 4–8h unklar · 0 von 4 Werten belegt, Rest Richtwerte (ungemessen).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Es gibt noch keinen Code, und jede Interaction der WBS setzt eine laufende Instanz voraus. Wenn zuerst das dünnste durchgehende Stück — Aufruf, Ordner, Sperre, Port, HTTP, eingebettete Seite, drei Testebenen — gebaut wird, dann ist der ganze Weg vom Terminal bis in den Browser einmal bewiesen, und der Director wie die Tests finden die Instanz über die Startzeile und den Health-Check. Dadurch bauen I0002 (Log) und I0004 (Beitreten) nur noch ihr eigenes Verhalten an eine erprobte Grenze, statt Start und Testaufbau mitzuerfinden. Die Sperre gehört schon hierher, weil eine zweite Instanz auf demselben Ordner die Logs verschränken würde, sobald I0002 schreibt — billig jetzt, teuer nach dem ersten Datenverlust. Der Standard-Host `0.0.0.0` ist der Hebel für „über Rechnergrenzen“: ohne ihn erreichen Agenten auf anderen Rechnern die Instanz nicht.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe wie Instanz, Ordner, Startzeile sind hier aus Architektur und ADRs übernommen.
- Browser-Öffnen je Plattform (Linux `xdg-open`, macOS `open`, Windows `start`) ist im Projekt nicht dokumentiert.

## Notizen

### Verworfene Alternativen

- **Je Slice eine eigene Anforderung** (R für F0001, F0002, F0003): verworfen, weil die drei Slices ein gemeinsames Ziel (laufende, geschützte, sichtbare Instanz) haben und das Arbeitspaket I0001 als Ganzes Entwicklertest und Abnahme durchläuft.
- **Nur F0001 jetzt, Sperre und Oberfläche später**: verworfen, weil der Walking Skeleton den Weg bis in den Browser beweisen soll und die Sperre vor dem ersten Schreiben (I0002) stehen muss.

### Bewusst außerhalb

Logablage, Verlaufsindex, Kennzahlenzeile nach dem Laden, Live-Kanal, Identität, OpenAPI-Beschreibung, Gestaltung der Oberfläche über die Tokens hinaus.
