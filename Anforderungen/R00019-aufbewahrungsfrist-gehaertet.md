---
id: R00019
status: Neu
datum: 2026-10-28
---

# R00019: Aufbewahrungsfrist gehärtet

## Beschreibung

Die Aufbewahrungsfrist aus R00008 und ihre Spur im Betriebsprotokoll aus R00017 schließen die Lücken, die die Auto-Reviews zu #12 gefunden haben. Danach gilt:

- **Ein nicht lesbarer Ordner hat seinen eigenen Namen**: lässt sich der Ordner eines Channels nicht auflisten, heißt der Eintrag „Tage nicht ermittelt — Ordner nicht lesbar“ statt „Tag nicht entfallen“ oder „Einstellungen ließen sich nicht schreiben“. Verlängern gelingt trotzdem; Verkürzen antwortet 500 mit einem deutschen Satz und Ausweg und ändert nichts.
- **Die Spur nennt, wer es war**: auf Debug steht jede abgewiesene Anfrage an die Frist-Route im Bereich `frist` mit `channel`, `kennung` und Regelschlüssel; scheitert nach einem bestätigten Verkürzen das Löschen eines Tages, nennt die Zeile `grund=frist_gesetzt` und die Kennung dessen, der verkürzt hat. Im Bereich `frist` ist `grund` immer ein festes Wort (der Anlass), ein Plattenfehler steht immer in `ursache`.
- **Der Channel-Name bleibt lesbar**: `# name ▾` in der Kopfzeile wird erst gekürzt, wenn Beschreibung und Instanzname keinen Platz mehr abgeben.
- **Der Dialog bleibt ehrlich**: eine fremde Friständerung holt den zweiten Schritt zurück in den ersten, beide Schalter des zweiten Schritts tragen den Bestätigungssatz für Bildschirmleser, und „ 30“ gilt wie jede Eingabe, die nicht nur aus Ziffern besteht, als ungültig.
- **Meldungen deutsch mit Ausweg**: ein Serverfehler beim Setzen erscheint im Dialog ohne englischen Systemtext und mit einem Ausweg; das Feld ist nur bei Eingabefehlern als ungültig markiert; die 409 an Agenten spricht wie die Oberfläche von „Tagen … bis einschließlich“.

Zahlt ein auf: [Vision](R00000-vision.md) — „Mensch und KI sind gleichberechtigt“ (Agent und Mensch lesen dieselben Begriffe und bekommen beide einen Ausweg) und „eine gemeinsame LAN-Instanz“ (die einzige endgültige Handlung des Programms, das Löschen, ist auch an ihren Rändern nachvollziehbar). Kein Widerspruch zur Vision.

Herkunft: Karte #12 (I0018), Auto-Reviews vom 2026-10-28 — Observability S1, O1, O2; Code-Review Anmerkung 2; UX-Review Sollte 2 und 3; Rechtschreib-Review Sollte. Schnitt des Directors: die Muss-Befunde und das wacklige Gate sind Nacharbeit #12 an R00008 (B0315–B0318), der Rest ist diese Anforderung.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0018 Aufbewahrungsfrist je Channel festlegen): F0063 Ordner nicht lesbar, F0064 Frist-Spur mit Kennung, F0065 Channel-Name auf dem Schalter lesbar, F0066 Frist-Meldungen deutsch mit Ausweg, F0083 Frist-Dialog nach dem Entwicklertest #12.

Nachgezogen am 2026-10-29 (planer, `/anforderung review`, Karte #27): die Nachträge des Directors aus dem Entwicklertest #12 — F0083 neu; die User Stories von R00008 an R00017 angeglichen und „Singular“ → „Einzahl“ in R00008 (ohne Bau).

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00008 und R00017: Frist, Entfallen, Pflege, Anlass (`start`, `tageswechsel`, `frist_gesetzt`), Vorschau, Betriebsprotokoll, Eintrag, Bereich; aus R00018: **Regelschlüssel** (festes ASCII-Wort in `snake_case`, das sagt, welche Regel eine Anfrage abwies) und **`ursache`** (Fremdtext einer technischen Ursache, ADR-0040 Ergänzung).

## Geschäftlicher Nutzen

Die Reviews zu #12 zeigen, wo die Frist heute an ihren Rändern irreführt. Hat ein Rücksicherungs-Skript dem Ordner eines Channels das Leserecht genommen, meldet das Protokoll „Tag nicht entfallen“ mit dem Ordner als Datei, und beim Setzen „Einstellungen ließen sich nicht schreiben“ — die Betreiberin sucht an der falschen Stelle. Ein Agent, der die Frist nur verlängern will, bekommt dabei eine 500, obwohl nichts gelöscht werden müsste. Auf Debug stehen abgewiesene Frist-Anfragen unter `channels` ohne Channel und Kennung; nach einem bestätigten Verkürzen fehlt einer gescheiterten Löschung die Kennung dessen, der verkürzt hat. Im Browser verschwindet bei 720 px der Name des Channels hinter „# entwicklu…“, während die Beschreibung daneben Platz hat, und ein Serverfehler zeigt „Permission denied (os error 13)“ und markiert die richtige Eingabe als falsch.

Mit R00019 findet die Betreiberin die Ursache mit `grep 'Tage nicht ermittelt' logs/*.log`, ordnet jede Abweisung und jeden Löschfehler einer Kennung zu, und die Entwicklerin im Browser liest Channel und Fehler in ihrer Sprache.

## Funktionale Anforderungen

### Ordner nicht lesbar (F0063; Observability-Review #12 S1, Code-Review #12 Anmerkung 2)

- **Pflege**: scheitert in einer Pflege (Start, Tageswechsel, nach dem Setzen) das Auflisten der Tage eines Channels, entsteht genau ein Eintrag `ERROR frist Tage nicht ermittelt — Ordner nicht lesbar; erneuter Versuch beim nächsten Anlass | channel=<name> datei=channels/<name> grund=<anlass> ursache="<Text des Fehlers>"`, beim Anlass `frist_gesetzt` zusätzlich `kennung=`. Er steht in der Datei und, als ERROR, auf stderr. Häufigkeit wie bei „Tag nicht entfallen“ (R00008, R00017): einmal je Channel und Anlass, nie je Takt. Die übrigen Channels werden weiter gepflegt.
- **Verlängern** (neue Frist größer als die geltende) braucht keine Vorschau und listet den Ordner vorher nicht auf: es antwortet 200 wie in R00008, auch wenn der Ordner nicht lesbar ist. Die Pflege danach (Anlass `frist_gesetzt`) meldet den nicht lesbaren Ordner wie oben.
- **Verkürzen ohne Bestätigung**, Auflisten scheitert → 500 `{"fehler":"Frist nicht gesetzt — die Tage von # <name> ließen sich nicht ermitteln (Ordner nicht lesbar). Leserecht von channels/<name> prüfen und erneut versuchen."}`; Frist, Einstellungsdatei und `stand` bleiben, kein Ereignis; ein Eintrag `ERROR frist Frist nicht gesetzt — Tage nicht ermittelt | channel=<name> kennung=<kennung> datei=channels/<name> ursache="…"`.
- **Verkürzen mit Bestätigung**, Auflisten scheitert → gesetzt wie in R00008 (die Bestätigung braucht keine Vorschau), die 200 nennt `entfallen` 0/0, die Pflege danach meldet den Ordner wie oben; die Tage entfallen beim nächsten Anlass.

### Frist-Spur mit Kennung (F0064; Observability-Review #12 O1, O2)

- **Debug**: jede Antwort 4xx der Route `PUT /api/channels/{name}/frist` ergibt die Debug-Zeile aus R00014/R00018 im Bereich `frist` (statt `channels`) mit `channel=<name>`, `kennung=` (sobald die Sitzung bekannt ist; ohne Sitzung `herkunft=`) und `grund=<Regelschlüssel>` nach R00018 (z. B. `frist`, `bestaetigen`).
- **Bereich nach Route** (ADR-0042, Ergänzung R00019 Punkt 2): die Debug-Schicht wählt den Bereich aus **Methode und Routenmuster**, nicht aus dem Pfadpräfix — derselbe Bereich, unter dem die Information- und Error-Zeilen der Route stehen. `PUT /api/channels/{name}/frist` → `frist`; `POST /api/channels/{name}/nachrichten` → `senden` (heute `verlauf`); `GET /api/channels/{name}/nachrichten` bleibt `verlauf`; übrige Routen unter `/api/channels` und Pfade ohne Muster (Router-404) bleiben `channels`. `channel=` nimmt die Schicht aus dem Pfadparameter `{name}` der passenden Route (escaped und gekürzt wie jeder Wert) — so trägt auch die 401 aus dem Extraktor ohne Sitzung `channel=`.
- **Löschfehler mit Anlass**: die Einträge „Tag nicht entfallen“ und „Tage nicht ermittelt“ tragen den Anlass in `grund` (`start` | `tageswechsel` | `frist_gesetzt`) wie die INFORMATION-Zeile derselben Pflege; beim Anlass `frist_gesetzt` steht `kennung=` dabei. Der Text des Plattenfehlers steht in `ursache`.
- **Eine Zeilenform im Bereich `frist`** (ADR-0040, Ergänzung R00019, Option B): auch „Frist nicht gesetzt — Einstellungen ließen sich nicht schreiben“ trägt den Plattenfehler in `ursache` und hat kein `grund` (kein Anlass): `… | channel=<name> kennung=<kennung> datei=channels/<name>/einstellungen.toml ursache="…"`. Kein Eintrag des Bereichs `frist` trägt Fremdtext in `grund`.

### Channel-Name auf dem Schalter lesbar (F0065; UX-Review #12 Sollte 2)

- In der Zone „Ort“ der Kopfzeile geben Beschreibung und Instanzname zuerst Platz ab, der Schalter `# name ▾` erst danach; der Name endet nur dann auf …, wenn er allein nicht mehr passt. Das Designsystem (`components/Kopfzeile`: „damit die Ellipse nur die Beschreibung kürzt“) bleibt die Gestaltungsvorgabe.
- 320 px ohne Querscrollen (R00008) gilt weiter; der zugängliche Name des Schalters nennt immer den ganzen Namen.

### Frist-Meldungen deutsch mit Ausweg (F0066; UX-Review #12 Sollte 3, Rechtschreib-Review #12 Sollte)

- **500 Schreibfehler** (R00008 F0038): `fehler` ist „Frist nicht gesetzt — channels/<name>/einstellungen.toml ließ sich nicht schreiben. Schreibrecht des Ordners prüfen und erneut versuchen.“ — ohne Fremdtext; der Fremdtext steht weiter im Eintrag `ERROR frist Frist nicht gesetzt …` (R00017).
- **409 `bestaetigen`**: `fehler` spricht wie die Oberfläche von Tagen und „bis einschließlich“, gebeugt wie nach der Nacharbeit #12: „Mit dieser Frist entfällt sofort 1 Tag bis einschließlich 2026-10-21 mit 1 Nachricht — mit "bestaetigt":true erneut senden.“ bzw. „Mit dieser Frist entfallen sofort 2 Tage bis einschließlich 2026-10-21 mit 5 Nachrichten — …“. Das Datum bleibt `JJJJ-MM-TT` (für Agenten).
- **Dialog**: `aria-invalid="true"` am Feld nur bei Eingabefehlern (Prüfung vor dem Senden, 422 `frist`); bei 5xx, 404, 409 `einstellungen-defekt` und Netzfehler steht die Fehlerzeile mit ⚠, das Feld bleibt ohne `aria-invalid`. Bei 5xx zeigt die Fehlerzeile den deutschen `fehler` der Antwort; fehlt er, „Nicht gespeichert (Status <n>) — später erneut versuchen.“

### Frist-Dialog nach dem Entwicklertest #12 (F0083; Entwicklertest #12, UX-Wiederholung #12, Rechtschreib-Review #12)

- **Fremde Änderung im zweiten Schritt**: kommt ein Wechsel mit neuer Frist, während der Dialog den zweiten Schritt zeigt, kehrt er in den ersten Schritt zurück: das Feld ist wieder bearbeitbar und zeigt die Eingabe, der Bestätigungssatz und „Frist verkürzen“ sind weg, der Hinweis ergänzt „Inzwischen geändert: <n> Tage.“ wie im ersten Schritt (R00008), der Fokus liegt auf dem Feld. Erst ein neues Speichern fragt den Server, was jetzt entfiele. Heute bleibt der alte Satz stehen, und „Frist verkürzen“ würde eine inzwischen kürzere Frist still verlängern.
- **„Frist verkürzen“ beschrieben**: der Bestätigungssatz ist die zugängliche Beschreibung von „Zurück“ **und** von „Frist verkürzen“ (R00008 verlangte ihn an „Zurück“ oder am Dialog). Der Kopfkommentar von `web/channel-einstellungen.js` (Zeilen 4–5) sagt das richtig.
- **Nur Ziffern** (R00008 „Prüfung vor dem Senden“, so zugesagt, nicht so gebaut): eine Eingabe mit Leerzeichen wie „ 30“ ergibt die Fehlerzeile „Frist: ganze Zahl von 1 bis 3650 Tagen.“ und `aria-invalid="true"`, kein Abruf. Ob dafür der Feldtyp aus ADR-0033 Regel 3 (`type="number"`) wechseln muss, zeigt eine Probe im Chromium; ein Wechsel ist eine Ergänzung von ADR-0033 durch den architekt.

## Nicht-funktionale Anforderungen

- **Sicherheit** (CWE-117, X00002): `channel` in Debug-Zeilen ist ein Wert aus dem Pfad und wird wie jeder Wert escaped und gekürzt (R00014); Regelschlüssel und `grund` sind feste Wörter aus dem Code, Fremdtext steht nur in `ursache`. Jede Löschung und jede gescheiterte Löschung nach einem Setzen ist einer Kennung zuzuordnen.
- **Benutzerfreundlichkeit**: der Name des Channels ist die wichtigste Angabe der Kopfzeile; Fehler im Dialog sagen, was zu tun ist; Barrierefreiheit: `aria-invalid` nur, wo die Eingabe falsch ist.
- **Zeitkriterien** nach `AI-Team/planer.md`: neue Zeitgrenzen gibt es nicht; Z3 und Z4 aus R00008 gelten wörtlich.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start und Gegeben wie in R00008 — `TZ=Europe/Berlin LOCALCHATA_JETZT=2026-10-28T12:00:00.000+01:00 localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner, Schreibweise `28.(1)`, `S(beta)` und `PUT` wie dort. `heute.log` ist `logs/2026-10-28.log`; „die Zeile `X`“ meint die Zeile ab der Stufe, ohne Zeitstempel; `ursache="…"` heißt: ein nicht leerer Text in Anführungszeichen. Kriterien mit `chmod` laufen nicht als root.

### F0063 Ordner nicht lesbar

- [ ] **Pflege** (Unit, Speicherablage „Auflisten scheitert“): `pflegen(2026-10-29, Tageswechsel)` für `allgemein` ergibt genau den Eintrag `ERROR frist Tage nicht ermittelt — Ordner nicht lesbar; erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein grund=tageswechsel ursache="…"` und keinen Eintrag „Tag nicht entfallen“; ein zweiter Channel mit lesbarer Ablage wird gepflegt. Mutante „als Tag nicht entfallen gemeldet“ wird rot.
- [ ] **Verlängern trotz nicht lesbarem Ordner** (Integration): `allgemein` ohne Einstellungen mit `28.(1)`; nach der Startzeile `chmod a-r channels/allgemein`; `PUT {"tage":30}` → 200 mit `entfallen` 0/0; `channels/allgemein/einstellungen.toml` enthält `frist_tage = 30`, `GET /api/channels` nennt `frist_tage: 30`; `heute.log` enthält `INFORMATION frist Frist 7 → 30 Tage gesetzt | channel=allgemein kennung=beta@127.0.0.1` und genau eine Zeile, die mit `ERROR frist Tage nicht ermittelt — Ordner nicht lesbar; erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein grund=frist_gesetzt kennung=beta@127.0.0.1 ursache="` beginnt. Mutante „Vorschau auch beim Verlängern“ → 500, wird rot.
- [ ] **Verkürzen bei nicht lesbarem Ordner** (Integration, im Anschluss; der Agent `alpha` liest den Strom, Stand `s`): `PUT {"tage":7}` → 500 mit `{"fehler":"Frist nicht gesetzt — die Tage von # allgemein ließen sich nicht ermitteln (Ordner nicht lesbar). Leserecht von channels/allgemein prüfen und erneut versuchen."}`; `GET /api/channels` nennt `frist_tage: 30` und `stand: s`; die Einstellungsdatei ist byte-gleich; `heute.log` und stderr enthalten je genau eine Zeile, die mit `ERROR frist Frist nicht gesetzt — Tage nicht ermittelt | channel=allgemein kennung=beta@127.0.0.1 datei=channels/allgemein ursache="` beginnt; `heute.log` enthält kein „Einstellungen ließen sich nicht schreiben“.
- [ ] **Bestätigt verkürzen bei nicht lesbarem Ordner** (Integration, im Anschluss): `PUT {"tage":7,"bestaetigt":true}` → 200 mit `frist_tage: 7` und `entfallen` 0/0; `2026-10-28.log` liegt da; `heute.log` enthält eine weitere Zeile `ERROR frist Tage nicht ermittelt … grund=frist_gesetzt kennung=beta@127.0.0.1 ursache="…"`.
- [ ] **Ruhe am Tageswechsel** (Integration): `allgemein` mit `20.(1)`, `28.(1)`; `zweiter` mit `20.(1)`, `22.(1)`; Start mit `LOCALCHATA_JETZT=2026-10-28T23:59:58+01:00`; sofort nach der Startzeile `chmod a-r channels/allgemein` → innerhalb **Z3** (R00008) nach der Startzeile enthält `logs/2026-10-29.log` genau eine Zeile `ERROR frist Tage nicht ermittelt … channel=allgemein … grund=tageswechsel …` und für `zweiter` die INFORMATION-Zeile mit `anzahl=1 grund=tageswechsel` (der 22. entfällt); in **Z4** danach keine weitere Frist-Zeile. Mutante „Pflege in jedem Takt“ wird rot.

### F0064 Frist-Spur mit Kennung

- [ ] **Debug-Abweisungen im Bereich frist** (Integration, `LOCALCHATA_PROTOKOLL_STUFE=debug`; `allgemein` mit `frist_tage = 30` und `20.(2)`, `21.(3)`, `22.(1)`, `28.(1)`):
  - `PUT {"tage":0}` → 422; `heute.log` enthält genau eine `DEBUG frist`-Zeile mit `status=422`, `channel=allgemein`, `kennung=beta@127.0.0.1` und `grund=frist`;
  - `PUT {"tage":7}` → 409; genau eine `DEBUG frist`-Zeile mit `status=409`, `channel=allgemein`, `kennung=beta@127.0.0.1`, `grund=bestaetigen`;
  - `PUT {"tage":30}` ohne Sitzung → 401; genau eine `DEBUG frist`-Zeile mit `status=401`, `channel=allgemein`, `herkunft=127.0.0.1` und ohne `kennung=`;
  - `heute.log` enthält keine `DEBUG channels`-Zeile zur Route `PUT /api/channels/{name}/frist`. Mutante „Bereich nach dem Pfadpräfix `channels`“ wird rot. Mutante „`channel=` aus der Erweiterung `Abweisung` statt aus dem Pfadparameter“ → die 401-Zeile ohne `channel=`, wird rot.
- [ ] **Bereich nach Methode und Routenmuster** (Unit, rein; erster Test aus ADR-0042, Ergänzung R00019): `bereich("PUT", "/api/channels/{name}/frist")` → `Frist`; `bereich("POST", "/api/channels/{name}/nachrichten")` → `Senden`; `bereich("GET", "/api/channels/{name}/nachrichten")` → `Verlauf`; `bereich("POST", "/api/channels")` → `Channels`; Pfad ohne Muster `/api/channels/x/y` (Router-404) → `Channels`. Mutante „Präfix `/api/channels` vor `/frist` geprüft“ wird rot, Mutante „`/nachrichten` ohne Methode → `Verlauf`“ wird rot.
- [ ] **Schreibfehler mit `ursache`** (Unit, rein, ADR-0040 Ergänzung R00019; der Unit-Test `src/gespraeche/fristeintrag.rs:267` dreht sich): `frist_nicht_gesetzt_eintrag("allgemein", beta, "channels/allgemein/einstellungen.toml", "Permission denied (os error 13)")` → genau `ERROR frist Frist nicht gesetzt — Einstellungen ließen sich nicht schreiben | channel=allgemein kennung=beta@127.0.0.1 datei=channels/allgemein/einstellungen.toml ursache="Permission denied (os error 13)"`, ohne `grund=`. Integration (R00017 „Schreibfehler bleibt laut“ in der neuen Form, `tests/frist_schutz.rs`): die Zeile in `heute.log` und auf stderr beginnt wie dort, enthält ` ursache="` und kein ` grund=`. Mutante „Fremdtext in `grund`“ wird rot.
- [ ] **Einträge mit Anlass** (Unit, rein): „Tag nicht entfallen“ für die Anlässe Start, Tageswechsel und `FristGesetzt { von: beta@127.0.0.1 }` ergibt `… | channel=allgemein datei=channels/allgemein/2026-10-21.log grund=start ursache="…"`, `… grund=tageswechsel ursache="…"` bzw. `… grund=frist_gesetzt kennung=beta@127.0.0.1 ursache="…"`; dasselbe für „Tage nicht ermittelt“. Der Text enthält nie eine Kennung.
- [ ] **Löschfehler nach dem Setzen** (Router-Test mit Speicherablage „Entfernen scheitert“ für den 21. und Speicher-Senke; Kriterium nach Nutzen — über die API nicht herstellbar, weil Löschen und Schreiben der Einstellungen dasselbe Schreibrecht brauchen): `allgemein` mit Frist 30 und den Tagen 20., 21., 22., 28.; `PUT {"tage":7,"bestaetigt":true}` als `beta` → 200; die Senke hält genau eine Zeile `ERROR frist Tag nicht entfallen — erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein/2026-10-21.log grund=frist_gesetzt kennung=beta@127.0.0.1 ursache="…"`. Mutante „Kennung fehlt bei frist_gesetzt“ wird rot.
- [ ] **Nicht löschbar** (Integration, R00017 „Nicht löschbar bleibt laut“ mit der neuen Form): die beiden `ERROR frist Tag nicht entfallen`-Zeilen enden auf `| channel=allgemein datei=channels/allgemein/2026-10-20.log grund=start ursache="…"` bzw. `grund=tageswechsel ursache="…"`; Z3 und Z4 wie dort.

### F0065 Channel-Name auf dem Schalter lesbar

- [ ] **Breiten** (E2E; Breiten aus der Probe des ux-reviewer #12): Channel `entwicklung` mit einer Beschreibung von 200 Zeichen; A wählt `# entwicklung` → bei 320, 375, 720 und 768 px Fensterbreite ist am Namen auf dem Schalter `scrollWidth <= clientWidth` (nicht gekürzt); bei 720 und 768 px ist die Beschreibung sichtbar und gekürzt (`scrollWidth > clientWidth`); die Seite hat bei allen vier Breiten `scrollWidth <= innerWidth`. Mutante „Schalter mit `flex: 0 1 auto` wie heute“ wird rot.
- [ ] **Langer Name** (E2E, Ausnahmeweg, Kriterium nach Nutzen): Channel aus 32 × `a` bei 320 px → `scrollWidth <= innerWidth`, der zugängliche Name des Schalters nennt den ganzen Namen.
- [ ] Konsole: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler.

### F0066 Frist-Meldungen deutsch mit Ausweg

- [ ] **409 mit Tagen bis einschließlich** (Unit und Integration): Unit 1/1 → „Mit dieser Frist entfällt sofort 1 Tag bis einschließlich 2026-10-21 mit 1 Nachricht — mit "bestaetigt":true erneut senden.“, 2/5 → „Mit dieser Frist entfallen sofort 2 Tage bis einschließlich 2026-10-21 mit 5 Nachrichten — …“; Integration wie R00008 „Verkürzen braucht Bestätigung“ und „Einzahl in der Rückfrage“ mit diesen Sätzen. `entfaellt` bleibt unverändert.
- [ ] **500 ohne Fremdtext** (Integration, R00008 F0038 „Schreibfehler“): `PUT {"tage":30}` → 500 mit `{"fehler":"Frist nicht gesetzt — channels/allgemein/einstellungen.toml ließ sich nicht schreiben. Schreibrecht des Ordners prüfen und erneut versuchen."}`; die Antwort enthält weder `os error` noch `Permission`; der Eintrag `ERROR frist Frist nicht gesetzt …` in `heute.log` trägt den Fremdtext weiter. Mutante „Fremdtext in `fehler`“ wird rot.
- [ ] **Serverfehler im Dialog** (E2E): `allgemein` ohne Einstellungen, Ordner `channels/allgemein` ohne Schreibrecht; A öffnet den Dialog, gibt 30 ein, `Enter` → die Fehlerzeile zeigt ⚠ und den Satz mit „Schreibrecht des Ordners prüfen“, ohne `os error` und `Permission`; das Feld trägt kein `aria-invalid`; die Eingabe 30 steht noch, der Dialog bleibt offen. Gegenprobe: A gibt `0` ein → `aria-invalid="true"`. Mutante „`aria-invalid` bei jedem Fehler“ wird rot.
- [ ] Konsole: keine unbehandelte Ausnahme, kein CSP-Verstoß; in „Serverfehler im Dialog“ höchstens „Failed to load resource“ zu dem `PUT …/frist` mit **500** (vorgeschrieben oben).
- [ ] **Testname** (Review-Punkt): `e2e/tests/r00008-frist.spec.js` nennt den Test „Esc (US-18): schließt, ohne zu speichern, der Fokus liegt auf # allgemein“.

### F0083 Frist-Dialog nach dem Entwicklertest #12

- [ ] **Zweiter Schritt folgt fremder Änderung** (E2E, zwei Kontexte): `allgemein` mit `frist_tage = 30` und `20.(2)`, `21.(3)`, `22.(1)`, `28.(1)`; A gibt 7 ein, `Enter` → zweiter Schritt. `beta` setzt per API 7 mit `"bestaetigt":true` → bei A innerhalb der Grenze **Z2** aus R00008: erster Schritt, Feld 7 und bearbeitbar, Hinweis „Inzwischen geändert: 7 Tage.“, „Frist verkürzen“ nicht sichtbar, kein Text „entfallen sofort“, der Fokus im Feld. A speichert → Statuszeile „Frist für # allgemein: 7 Tage.“. Variante: `beta` setzt 3 bestätigt → A sieht im ersten Schritt „Inzwischen geändert: 3 Tage.“, und bis A speichert, geht kein `PUT …/frist` von A aus. Mutante „Wechsel im zweiten Schritt übergangen“ wird rot.
- [ ] **„Frist verkürzen“ beschrieben** (E2E wie R00008 „Bildschirmleser“): im zweiten Schritt hat `getByRole('button', {name: 'Frist verkürzen'})` eine Beschreibung, die auf `/2 Tage .* 5 Nachrichten .* nicht rückgängig/` passt; Variante Einzahl (`21.(1)`, `22.(1)`, `28.(1)`, 30 → 7) → „1 Tag“ und „1 Nachricht“. „Zurück“ behält seine Beschreibung. Mutante „`aria-describedby` an „Frist verkürzen“ entfernt“ wird rot.
- [ ] **Leerzeichen abgewiesen** (E2E): A tippt ` 30` mit der Tastatur (nicht `fill`) und drückt `Enter` → Fehlerzeile „Frist: ganze Zahl von 1 bis 3650 Tagen.“ mit ⚠, `aria-invalid="true"`, kein Abruf an `…/frist`. Gegenprobe `30` → 200. Mutante „Wert des Browsers statt Rohtext“ wird rot.
- [ ] **Kommentar** (Review-Punkt `rechtschreib-reviewer`): der Kopfkommentar von `web/channel-einstellungen.js` beschreibt beide Schalter, der Relativsatz bezieht sich auf den Schalter.
- [ ] Konsole: keine unbehandelte Ausnahme, kein CSP-Verstoß; höchstens „Failed to load resource“ zur 409 `bestaetigen` (vorgeschrieben in R00008).

## Betroffene Verzeichnisstruktur

- `src/gespraeche/pflege.rs` (Auflisten in `pflegen` und `frist_setzen`, Vorschau nur beim Verkürzen), `src/gespraeche/fristeintrag.rs` (neuer Bericht, `grund` = Anlass, `ursache`).
- `src/http/frist.rs` (500 „Tage nicht ermittelt“, 500 ohne Fremdtext, 409-Text, Kennung an den Pflege-Einträgen nach dem Setzen), `src/http/anfrageprotokoll.rs` (Bereich `frist`, `channel`).
- `web/lc-seite.css` (Kopfzeile), `web/channel-einstellungen.js` (`fehlerZeigen`, `fehlertext`).
- Tests: Unit im Modul; Integration unter `tests/frist*.rs`; E2E `e2e/tests/r00008-frist.spec.js` bzw. eine neue `r00019-*.spec.js`.

## Technische Überlegungen

### Ablauf

1. **Pflege**: Channel → Frist → `ablage.tage()` → Fehler: `TageNichtErmittelt` → Eintrag (ERROR, `grund` = Anlass, `ursache`) \| Tage → teilen → löschen wie in R00008.
2. **Setzen**: Prüfungen wie R00008 → gleicher Wert → 200 \| neue Frist > geltende → setzen \| Verkürzen ohne Bestätigung → `tage()` → Fehler: 500 „Tage nicht ermittelt“ \| Vorschau → 409 \| sonst setzen → Pflege (`frist_gesetzt`, mit Kennung) → 200.
3. **Debug-Schicht**: Antwort 4xx → Methode und Muster der Route → Bereich (`frist` für `PUT …/frist`, `senden` für `POST …/nachrichten`, sonst wie bisher) → `channel` aus dem Pfadparameter `{name}` → Zeile mit `channel`, `kennung`/`herkunft`, `grund`.
4. **Dialog**: Antwort → Eingabefehler? → `aria-invalid` \| sonst nur Fehlerzeile; Text aus `fehler` oder eigener Satz.

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0063: B0319 Tage nicht ermittelt melden · B0320 Verlängern ohne Auflisten
- F0064: B0321 Frist-Abweisungen im Bereich frist · B0322 Löschfehler nach dem Setzen mit Kennung
- F0065: B0323 Kopfzeile kürzt zuerst Beschreibung und Instanz
- F0066: B0324 Serverantworten der Frist deutsch · B0325 Fehlerzeile unterscheidet Eingabe und Server
- F0083: B0387 Zweiter Schritt folgt fremder Änderung · B0388 „Frist verkürzen“ trägt den Satz · B0389 Leerzeichen in der Frist abweisen

Elemente: `Pflegebericht::TageNichtErmittelt { channel, ordner, ursache }`; `Fristfehler::TageNichtErmittelt { ordner, ursache }`; die Einträge aus `fristeintrag.rs` bekommen den Anlass (`Pflegeanlass`) als Eingabe; die Debug-Schicht nimmt `channel` aus dem Pfadparameter, die Erweiterung `Abweisung` aus R00018 (B0303) bleibt `{ regel, kennung }` (ADR-0042, Ergänzung R00019 Punkt 2, Option A).

### Änderungen an bestehenden Klassen

- `Gespraeche::frist_setzen`: Vorschau und Auflisten nur beim Verkürzen ohne Bestätigung.
- `fristeintrag::pflege_eintrag`: Kontext `grund=<anlass>`, `kennung=` bei `frist_gesetzt`, Fremdtext in `ursache`.
- `fristeintrag::frist_nicht_gesetzt_eintrag`: Fremdtext in `ursache`, kein `grund`.
- `anfrageprotokoll::bereich(pfad)` → `bereich(methode, muster)`: `PUT …/frist` → `Bereich::Frist`, `POST …/nachrichten` → `Bereich::Senden`, die spezifischste Regel zuerst; `channel` aus dem Pfadparameter der passenden Route.

## Tests

Nach Skill `test-pyramide`.

- **Unit (rein bzw. Speicherablage)**: Pflege mit scheiterndem Auflisten, `frist_setzen` mit scheiterndem Auflisten (Verlängern, Verkürzen, bestätigt Verkürzen — erster Test aus ADR-0031, Ergänzung R00019), Einträge je Anlass, Schreibfehler mit `ursache` (`fristeintrag.rs:267` dreht sich), `bereich(methode, muster)`, 409-Text 1/1 und 2/5.
- **Router-Test (Speicherablage, Speicher-Senke)**: Löschfehler nach dem bestätigten Verkürzen.
- **Integration (echtes Programm)**: Verlängern, Verkürzen und bestätigt Verkürzen bei nicht lesbarem Ordner, Ruhe am Tageswechsel, Debug-Abweisungen, Nicht löschbar und Schreibfehler in der neuen Form (`tests/frist_schutz.rs:118`, `:238`), 500 ohne Fremdtext.
- **E2E**: Breiten, langer Name, Serverfehler im Dialog mit Gegenprobe, zweiter Schritt bei fremder Änderung, Beschreibung von „Frist verkürzen“, Leerzeichen in der Frist.
- Mutationsproben je Kriterium; das Ergebnis steht im Bericht des Entwicklers.

## Abhängigkeiten

- Abhängig von: Nacharbeit #12 an R00008 (B0315–B0318 unter F0034, F0035, F0037; gelb); F0038 Löschschutz und Fehlerwege, F0058 Frist im Betriebsprotokoll (gruen); F0059 Jeder Fehlerpfad mit Spur und festem Grund (R00018, rot) für F0063 und F0064 — Schlüssel `ursache` und Erweiterung `Abweisung`.
- Innerhalb: F0083 braucht F0037 (R00008) und ändert dieselbe Datei wie F0066 (B0325) und F0072 (R00021, B0344): `web/channel-einstellungen.js` — nacheinander bauen. F0065 und F0066 brauchen nur die Nacharbeit #12; F0063 und F0064 zusätzlich F0059. Untereinander parallel.
- Blockiert: —

## Offene Fragen

- **Entschieden** (architekt, 2026-10-28, Karte #28; nachgezogen vom planer am selben Tag):
  1. ~~`grund` der Frist-ERROR-Zeilen trägt den Anlass, Plattenfehler in `ursache`~~ — entschieden weiter als empfohlen: im **ganzen Bereich `frist`** ist `grund` fest, jeder Fremdtext steht in `ursache`, auch in „Frist nicht gesetzt — Einstellungen ließen sich nicht schreiben“ (ADR-0040, Ergänzung R00019, Option B). Nachgezogen in F0064 („Eine Zeilenform im Bereich `frist`“, Kriterium „Schreibfehler mit `ursache`“).
  2. ~~Katalogzeilen „Tage nicht ermittelt“~~ — wie empfohlen: zwei neue ERROR-Zeilen, zwei präzisierte (ADR-0042, Ergänzung R00019 Punkt 1); Auflisten nur beim Verkürzen ohne Bestätigung, sonst 500 mit dem Satz oben (ADR-0031, Ergänzung R00019, Option A); Abbildungstabelle in ADR-0030 nachgezogen. R00019 F0063 bleibt unverändert.
  3. ~~Bereich `frist` für die Debug-Zeilen~~ — wie empfohlen, als Regel: Bereich nach Methode und Routenmuster, `channel=` aus dem Pfadparameter, auch für die 401 aus dem Extraktor (ADR-0042, Ergänzung R00019 Punkt 2, Option A). Mitgenommen: `POST /api/channels/{name}/nachrichten` → `senden`. Nachgezogen in F0064 („Bereich nach Route“, Kriterium „Bereich nach Methode und Routenmuster“).
- Vor dem Bau von B0389, an den architekt über den Director, nur wenn die Probe es verlangt: liefert `<input type="number">` im Chromium ` 30` als `30`, braucht „nur Ziffern“ einen anderen Feldtyp (`type="text"` mit `inputmode="numeric"`) — Ergänzung von ADR-0033 Regel 3. Empfehlung: Feldtyp wechseln; die Zusage „nur Ziffern“ gilt seit R00008.
- Nicht blockierend, an den Director: die Probe des ux-reviewer zu F0065 nennt Breiten, aber Instanzname und Beschreibung stehen nicht auf der Karte (Bilder im Scratchpad des Reviewers); vor dem Bau abgleichen.

## Manuelle Vorbereitungstätigkeiten

- Für die Kriterien mit `chmod`: Testlauf als normaler Benutzer, nicht als root.

## Manuelle Nachbereitungstätigkeiten

- Keine.

## Umfang

Gezählt aus der WBS (Bubbles). 0,4 h ist die Untergrenze je Bubble nach dem Skill `work-breakdown-structure`; Richtwert, ungemessen für diese Bubble-Typen.

- F0063 Ordner nicht lesbar: 2 Bubbles · 2 klar (0,8 h)
- F0064 Frist-Spur mit Kennung: 2 Bubbles · 2 klar (0,8 h); seit dem Nachzug vom 2026-10-28 trägt B0321 eine Regel (`senden`) und B0322 eine Funktion (`frist_nicht_gesetzt_eintrag`) mehr — Hypothese: beides passt in die Untergrenze, ungemessen
- F0065 Channel-Name auf dem Schalter lesbar: 1 Bubble · 1 klar (0,4 h)
- F0066 Frist-Meldungen deutsch mit Ausweg: 2 Bubbles · 2 klar (0,8 h)
- F0083 Frist-Dialog nach dem Entwicklertest #12: 3 Bubbles · 2 klar (0,8 h) · 1 unklar (B0389, Rohtext eines Zahlenfelds; 2–4 h)
- Zusammen: 10 Bubbles (vorher 7, +3 aus den Nachträgen vom 2026-10-29) · 3,6 h klar + 2–4 h unklar; Karte #27 (F0065, F0066, F0083): 6 Bubbles; eine Welle nach der Nacharbeit #12 (F0063 und F0064 zusätzlich nach F0059)

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser sind die Befunde der Auto-Reviews zu #12, die nicht Muss waren: Observability S1, O1, O2, Code-Review Anmerkung 2, UX-Review Sollte 2 und 3, Rechtschreib-Review Sollte.

Die Wirkungskette:

- Der nicht lesbare Ordner bekommt einen eigenen Eintrag. Deshalb sucht die Betreiberin am Leserecht des Ordners, nicht an einer Tagesdatei oder an der Einstellungsdatei, und ein Agent kann verlängern, ohne an einer Vorschau zu scheitern, die er nicht braucht.
- Abweisungen und Löschfehler der Frist tragen Bereich, Channel, Kennung und Anlass. Deshalb ordnet die Betreiberin jede endgültige Handlung und jeden Fehlversuch einer Person und einem Anlass zu (X00002: jeder mit Sitzung darf verkürzen).
- Die Kopfzeile kürzt zuerst, was weniger wichtig ist. Deshalb sieht die Entwicklerin auch in einem halben Laptop-Fenster, in welchem Channel sie die Frist ändert.
- Der zweite Schritt folgt fremden Änderungen und beschreibt beide Schalter. Deshalb bestätigt niemand eine Zahl, die nicht mehr stimmt, und ein Bildschirmleser hört am Schalter, der löscht, was er löscht.
- Meldungen sind deutsch, mit Ausweg und markieren nur falsche Eingaben. Deshalb versteht die Entwicklerin einen Serverfehler, ohne ihre richtige Eingabe zu ändern, und Agent und Mensch lesen dieselben Begriffe.

Der Hebel liegt an wenigen Stellen (Auflisten in Pflege und Setzen, Kontext der Frist-Einträge, Bereich der Debug-Schicht, Fehlerzeile des Dialogs, eine CSS-Regel), nicht an neuen Abläufen.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus ADR-0029 bis ADR-0033 und ADR-0040 bis ADR-0042.
- Nachträge #27: für die Punkte des Entwicklertests #12 stehen keine Abnahme-Proben auf der Karte, und der Wortlaut des Rechtschreib-Befunds zum Kommentar `web/channel-einstellungen.js:4–5` fehlt; die Kriterien unter F0083 sind aus den Beschreibungen abgeleitet.
- Die Abnahme-Proben zu UX Sollte 2 und 3, Observability S1, O1, O2 sind auf der Karte nur skizziert; die Kriterien oben sind daraus abgeleitet.

## Notizen

- **Änderung von Zusagen aus R00008 und R00017** (beide bleiben an diesen Stellen unverändert, R00019 nennt die Änderung):
  - R00008 F0034: der `fehler` der 409 `bestaetigen` heißt „… Tag(e) bis einschließlich …“ statt „… Tagesdatei(en) … bis …“ (Wortlaut aus der Nacharbeit #12).
  - R00008 F0038 „Schreibfehler“: die 500 trägt keinen Fremdtext mehr.
  - R00017 „Nicht löschbar bleibt laut“: die ERROR-Zeile endet auf `grund=start|tageswechsel ursache="…"` statt `grund="…"`; die Abbildungstabelle in R00017 gilt mit dieser Form (ADR-0030, Ergänzung R00019).
  - R00017 „Schreibfehler bleibt laut“: die Zeile `ERROR frist Frist nicht gesetzt — Einstellungen ließen sich nicht schreiben | …` enthält `ursache="` statt `grund="` und kein `grund=` (ADR-0040, Ergänzung R00019); Anfang der Zeile, stderr und „keine `INFORMATION frist`-Zeile“ bleiben. Ebenso die Zeile 6 der Abbildungstabelle in R00017.
  - R00017 und R00014 F0054: die Debug-Zeilen der Frist-Route stehen im Bereich `frist`; die Debug-Zeilen von `POST /api/channels/{name}/nachrichten` (R00014 F0054, R00016) im Bereich `senden` statt `verlauf`.
- **UX-Optional aus #12** (als Notiz, nicht im Kriterium): das schreibgeschützte Feld im zweiten Schritt sieht noch bearbeitbar aus; geschützte Leerzeichen zwischen Zahl und Einheit und im Datum; Abstand vor ▾.
- Rechtschreib-Optional aus #12 (Anführungszeichen in 422/409 einheitlich, „3650“/„3.650“, „bis einschließlich“ bei genau einem Tag, Statuszeile „— entfallen: 1 Tag …“) ist nicht aufgenommen.

### Bewusst außerhalb

- `einstellungen.toml.neu` folgt beim Schreiben einem Symlink (Security-Review #12, Optional) → Karte #18 Härtung.
- Wechsel-Flut ohne Rate (Security-Review #12, bekannt) → Schreibkonto, Karte #8.
- Massenlöschung durch jede Sitzung (ADR-0031, hingenommen); `cargo audit` im Gate (Umgebung).
- Bereinigungszeile des Betriebsprotokolls bei genau einer Datei nur mit Datum (`(2026-10-14; Aufbewahrung 14 Tage; Start)` statt `2026-10-14 bis 2026-10-14`, `src/protokoll/mod.rs:849`; ADR-0041, Nachzug Punkt 3) → Notiz an F0062 (I0021, R00018, Karte #26): gehört zum Bereich `protokoll`, nicht zur Frist, und ändert Zusagen aus R00014 (Zeilen 69 und 207, `tests/protokoll_bereinigung.rs:109`).
- Fremdtext außerhalb des Bereichs `frist` nach `ursache` (ADR-0040, Ergänzung R00019, Option C, rund 15 Aufrufstellen) → eigene Anforderung, wenn der architekt C entscheidet.
- Code-Review Optional: Doppelmeldung bei gleichzeitiger Pflege, PUT überschreibt nachträglich defekte Datei (ADR-0031 hingenommen), Kommentar 401/403 in `src/http/frist.rs:2` — Konsolidierung, kein Kriterium.
