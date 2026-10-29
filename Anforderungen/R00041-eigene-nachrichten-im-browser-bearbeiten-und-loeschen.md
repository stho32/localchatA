---
id: R00041
status: Neu
datum: 2026-10-30
---

# R00041: Eigene Nachrichten im Browser bearbeiten und löschen

## Beschreibung

Im Browser sieht jeder Teilnehmer, dass eine Nachricht bearbeitet oder gelöscht wurde: an ihrer Stelle steht sofort der neue Text mit „bearbeitet“ bzw. „gelöscht“ ohne Text und Anhänge — live, nach dem Neuladen und nach einem Verbindungsabbruch. An den eigenen Nachrichten — auch aus einer früheren Sitzung derselben Kennung — bietet der Verlauf „Bearbeiten“ und „Löschen“ an, mit Maus, Tastatur und Touch: Bearbeiten an Ort und Stelle, Löschen nach genau einer Rückfrage, die bei Anhängen deren Zahl und Größe nennt.

Zahlt ein auf: [Vision](R00000-vision.md) — „Eigene Nachrichten bearbeiten und löschen“ (der Mensch im Browser, „bearbeitet“, „gelöscht“), „Live überall“ und die Leitplanke Zielgeräte (Desktop, Tablet, Telefon; jede Aktion auch ohne Tastenkürzel).

Herkunft: Karte #48 — Betreiber 2026-10-29 (Menü 1 „im Verlauf mit „bearbeitet“ bzw. „gelöscht“ gekennzeichnet“, `AI-Team/po-entscheidungen.md` Zeile 70), product-owner Zeile 95 (F15/O19: Rückfrage vor jedem Löschen, bei Anhängen Zahl und Größe wie im Laufwerk) und Zeile 57 (UX-Vorprüfung Pflicht); [ADR-0048](../Dokumentation/ADRs/0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) Regeln 4, 11, 12.

Slices (WBS `Dokumentation/Planung/localchata.md`): **F0134** Bearbeitete Nachrichten im Browser sehen und **F0135** Eigene Nachricht im Browser bearbeiten (unter I0032), **F0138** Gelöschte Nachrichten im Browser sehen und **F0139** Eigene Nachricht im Browser löschen (unter I0033). Server: [R00039](R00039-eigene-nachricht-ueber-die-api-bearbeiten.md), [R00040](R00040-eigene-nachricht-ueber-die-api-loeschen-anhaenge-mit.md).

### Domänenbezug

Kein Glossar im Projekt. Begriffe wie in R00011, R00027, R00030 und ADR-0048: Verlauf, Fenster, Nachholen, eigene Nachricht (Kennung gleich der eigenen), Fassung, Marke „bearbeitet“/„gelöscht“, Aktion, Rückfrage, Ansage-Region, Hinweiszeile, Touch-Gerät.

## Geschäftlicher Nutzen

Die Menschen im Team arbeiten im Browser; ohne Oberfläche könnten nur Agenten korrigieren und löschen, und niemand sähe im Verlauf, dass sich eine Nachricht geändert hat. Die Marken halten den Verlauf nachvollziehbar (Betreiber: „Kennzeichnung hält den Verlauf nachvollziehbar“), die Rückfrage fängt das versehentliche Löschen samt Dateien ab.

## Funktionale Anforderungen

### F0134 Bearbeitete Nachrichten im Browser sehen

- Eine Nachricht mit `zustand: "bearbeitet"` zeigt ihren Text und die Marke „bearbeitet“ (Wortlaut `ux-reviewer`); Datum und Uhrzeit der **letzten** Änderung (nicht der ersten) stehen im Tooltip der Marke, die Marke selbst trägt keine Zeit (PO Zeile 123). Auf Touch-Geräten ohne Hover ist die Zeit durch Antippen der Marke erreichbar (Mechanik und Wortlaut `ux-reviewer`). Die frühere Fassung zeigt der Tab nirgends.
- **Live**: trifft ein Ereignis `aenderung` eine Nachricht im Fenster, ersetzt der Tab ihren Text an Ort und Stelle, wenn `geaendert` größer ist als das bekannte (`null` ist kleiner als jede Zeit); eine ältere Änderung überschreibt nie eine neuere; Nachrichten außerhalb des Fensters bringt das nächste Laden; Ereignisse, die vor dem Block ankommen, gehen nach dem Block durch dieselbe Regel. Die Lesestelle bleibt.
- **Nachholen**: nach dem Wiederverbinden und nach `NeuerStand` lädt der Tab die Blöcke seines Fensters neu und übernimmt Fassungen mit größerem `geaendert` (Ausnahme von ADR-0028 Regel 5 nur für dieses Feld).

### F0138 Gelöschte Nachrichten im Browser sehen

- Eine Nachricht mit `zustand: "geloescht"` zeigt Absender und Zeit wie bisher und statt Text und Anhängen die Marke „gelöscht“ (Wortlaut `ux-reviewer`), ohne Aktionen. Live und beim Nachholen nach derselben Regel wie F0134; gelöscht wird nie zurückgenommen. Ein Player der Nachricht hält an und verschwindet.

### F0135 Eigene Nachricht im Browser bearbeiten

- **Eigene Nachricht**: `von` gleich der eigenen Kennung (`GET /api/ich`), bestätigt (nicht „wird gesendet …“, nicht fehlgeschlagen), nicht gelöscht — auch aus einer früheren Sitzung derselben Kennung. Nur dort gibt es „Bearbeiten“ und „Löschen“.
- **Aktionen erreichen**: am Desktop bei Hover und Fokus rechts oben an der Nachricht (Designsystem `components/Nachricht`); auf Touch-Geräten über einen Schalter „Aktionen“ an jeder eigenen Nachricht; per Tastatur auf der fokussierten eigenen Nachricht `E` (Bearbeiten) und `Entf` (Löschen) — Ort, Touch-Zugang und Tasten sind Vorschläge für die UX-Vorprüfung. `↑` in leerer Eingabe fokussiert wie bisher die eigene letzte Nachricht (`1-tastatur.md`).
- **Bearbeiten an Ort und Stelle** (PO Zeile 121): mehrzeiliges Feld mit dem aktuellen Text, Fokus darin, Schreibmarke am Ende; „Speichern“ und „Abbrechen“; Tasten wie in der Eingabe (ADR-0038, Ergänzung Karte #41): `Enter` speichert, auf Touch-Geräten neue Zeile; `Shift Enter` neue Zeile; `Strg Enter` speichert überall; `Esc` bricht ab. „Speichern“ mit leerem Text geht nur bei einer Nachricht mit Anhang (wie die Bereit-Regel von R00030).
- **Speichern** ruft `POST …/nachrichten/bearbeiten` mit einem `Idempotency-Key` je Versuch; bei 200 schließt das Feld, die Nachricht zeigt die neue Fassung, der Fokus steht auf der Nachricht. Eine Abweisung zeigt den Text der Instanz am Feld, das Getippte bleibt; ist die Nachricht inzwischen gelöscht (409), schließt das Feld, die Nachricht zeigt „gelöscht“, die Hinweiszeile sagt „Die Nachricht wurde inzwischen gelöscht.“ Das Getippte geht verloren: keine Zwischenablage, kein Zurückschreiben in die Eingabe unten (PO Zeile 122).
- Gleicher Text → kein Fehler, Feld zu, keine neue Marke. Ein Channelwechsel bricht das Bearbeiten ab wie „Abbrechen“ (Vorschlag).
- Genau eine Ansage über die vorhandene Region bei Beginn, Speichern und Abweisung (Wortlaute `ux-reviewer`).

### F0139 Eigene Nachricht im Browser löschen

- **Rückfrage** vor jedem Löschen, genau einmal, als modaler Dialog: ohne Anhänge „Diese Nachricht löschen?“, mit Anhängen „Diese Nachricht mit <n> Anhängen (<größe>) löschen? Die Anhänge werden mitgelöscht.“ bzw. „… mit 1 Anhang (<größe>) …“ — gezählt werden Anhänge im Zustand `vorhanden`, Größe in Worten nach R00030 aus der Summe ihrer `bytes` (Wortlaut-Vorschlag nach ADR-0046 F1′ 5 a ohne „fremde“, PO Zeile 95; endgültig `ux-reviewer`). Knöpfe „Löschen“ und „Abbrechen“; `Esc` und „Abbrechen“ schließen ohne Anfrage.
- **Löschen** ruft `POST …/nachrichten/loeschen` (ein Schritt, kein `bestaetigt`); bei 200 steht „gelöscht“ an der Stelle, der Fokus auf der Nachricht, eine Ansage. Eine Abweisung oder keine Antwort zeigt den Text in der Hinweiszeile, die Nachricht bleibt und lässt sich erneut löschen.
- **Ausnahme zu R00030 „Kein Lösch-Knopf“**: an eigenen Nachrichten gibt es die Aktion „Löschen“ der Nachricht; an fremden Nachrichten, auch mit Anhängen, gibt es weiter kein Element, dessen Name „lösch“ enthält.

## Nicht-funktionale Anforderungen

- **Geräte** wie R00027 und R00030: Telefon 375 × 812 (Faktor 3), Tablet 768 × 1024 (Faktor 2), Tablet quer 1024 × 768 (Faktor 2), je mit `hasTouch` und `isMobile` und nur durch Tippen bedient; Desktop 1280 × 800 ohne Touch.
- **Schmal**: bei 320 px (Telefon-Kontext) kein Querscrollen mit Aktionen, Bearbeiten-Feld samt Knöpfen und Rückfrage-Dialog.
- **Zielfläche**: Schalter „Aktionen“, „Bearbeiten“, „Löschen“, „Speichern“, „Abbrechen“ auf Touch-Geräten mindestens 44 × 44 px, auf dem Desktop mindestens 24 × 24 px (wie R00030 UX M1).
- **Zeitkriterium** nach `AI-Team/planer.md`: „weniger als 1 s nach der 200“ für Marke und Text in jedem offenen Tab — Zweck Vision „Live überall“, Lastprofil Zustellung < 1 s; erwartet unter 100 ms im LAN (Ereignis plus ein Frame); Uhr ab Empfang der 200 beim Ändernden (im E2E: Antwort der Route im Netzprotokoll von A, bzw. des `curl`-Aufrufs).
- **Konsole**: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zu den Antworten, die ein Kriterium vorschreibt: 409 („Inzwischen gelöscht“), 507 („Platte voll beim Speichern“), der im Test abgebrochene `POST` („Fehlschlag beim Löschen“) und die Anfragen der Seite, solange sie offline ist („Nachholen“).
- **Kontrast**: Marken „bearbeitet“ und „gelöscht“, Feldtext und Dialogtext mindestens 4,5 : 1 gegen ihren Hintergrund.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, Beitritte per `curl`, `S(<name>)` und Browser A (`shoff`) und B (`anna`) wie in R00011, Kennung `<name>@127.0.0.1`; Geräte wie unter „Nicht-funktionale Anforderungen“, ohne Angabe Desktop; A und B haben `# allgemein` offen. „Gegeben über die API“ heißt: `beta` sendet, bearbeitet oder löscht per `curl` (R00039, R00040). „Eigene Nachricht von A“ heißt: A hat `Hallo` im Browser gesendet (bestätigt). „Mit und ohne SharedWorker“ wie in R00026. „Bearbeiten“, „Löschen“, „Speichern“, „Abbrechen“, „Aktionen“ meinen `getByRole('button', { name: … })` im Knoten der Nachricht bzw. im Dialog.

### F0134 Bearbeitete Nachrichten im Browser sehen

- [ ] **Übernahme-Regel** (Node, rein): bekannt `geaendert: null`, neu `t5` → übernehmen; bekannt `t5`, neu `t4` → verwerfen; bekannt `t5`, neu `t5` → verwerfen; bekannt `geloescht` mit `t5`, neu `bearbeitet` mit `t6` → verwerfen; Nachricht nicht im Fenster → verworfen, kein Fehler; Ereignis vor dem Block gepuffert, Block bringt `geaendert: null` → nach dem Block übernommen. Mutante „Zeit ≤ bekannt verwirft auch `aenderung`“ wird rot.
- [ ] **Darstellung** (Node, rein): Nachricht `{text: "Hallo zusammen", zustand: "bearbeitet", geaendert: "2026-10-30T14:09:00.000+01:00"}` → Text „Hallo zusammen“, Marke „bearbeitet“, Tooltip mit „30.10.2026, 14:09“; `zustand: null` → keine Marke. Dazu (E2E, Telefon und Tablet, ohne Hover): Gegeben über die API `Hallo` von `beta`, danach bearbeitet zu `Hallo zusammen` → Antippen der Marke „bearbeitet“ macht Datum und Uhrzeit der Änderung sichtbar (Format `TT.MM.JJJJ, HH:MM`, gleich der Zeit `geaendert` der Antwort); bearbeitet `beta` ein zweites Mal, zeigt die Marke danach die neue Zeit, nicht die erste.
- [ ] **Live** (E2E, mit und ohne SharedWorker): Gegeben über die API `Hallo` von `beta`; `beta` bearbeitet zu `Hallo zusammen` → bei A und B weniger als 1 s nach der 200 „Hallo zusammen“ mit „bearbeitet“; `Hallo` steht in keinem Textknoten des Verlaufs; kein Neuladen der Seite; nach dem Neuladen dieselbe Darstellung.
- [ ] **Lesestelle** (E2E): B hat 60 Nachrichten im Fenster und ist so gescrollt, dass die bearbeitete Nachricht über dem sichtbaren Bereich liegt; `beta` bearbeitet sie zu einem Text mit fünf Zeilen → die oberste sichtbare Nachricht bei B steht danach höchstens 1 px verschoben.
- [ ] **Reihenfolge** (E2E): `beta` bearbeitet zweimal kurz nacheinander (`Eins`, `Zwei`) → bei A steht am Ende `Zwei`, auch wenn der Test das erste Ereignis um 300 ms verzögert ausliefert (verzögerter Strom über `page.route`).
- [ ] **Nachholen** (E2E; Pre-Mortem 4): B geht offline (`context.setOffline(true)`), `beta` bearbeitet, B ist wieder online → nach „Wieder verbunden“ zeigt B „Hallo zusammen“ mit „bearbeitet“ ohne Neuladen. Mutante „Nachholen verwirft bekannte Nachrichten ganz“ wird rot.
- [ ] **Außerhalb des Fensters** (E2E): Channel mit 600 Nachrichten (Testdaten-Erzeuger), B am Ende; `beta` bearbeitet die erste → kein Fehler bei B; B scrollt an den Anfang → die erste zeigt den neuen Text mit „bearbeitet“.
- [ ] Konsole bei A und B nach der Regel unter „Nicht-funktionale Anforderungen“.

### F0138 Gelöschte Nachrichten im Browser sehen

- [ ] **Darstellung** (Node, rein): `{text: "", anhaenge: [], zustand: "geloescht"}` → Kopf mit Absender und Zeit, Marke „gelöscht“, kein Textabsatz, keine Anhang-Zeile, keine Aktionen — auch wenn `von` die eigene Kennung ist.
- [ ] **Live** (E2E, mit und ohne SharedWorker): Gegeben über die API `Hallo` von `beta`; `beta` löscht → bei A und B weniger als 1 s nach der 200 „gelöscht“ an der Stelle der Nachricht, `Hallo` steht in keinem Textknoten; nach dem Neuladen dieselbe Darstellung.
- [ ] **Mit Anhängen** (E2E): Gegeben über die API eine Nachricht von `beta` mit `foto.png` und `film.webm`; `beta` löscht sie → ihr Knoten enthält bei A danach kein `<img>`, `<video>`, `<audio>` und keinen Link auf `/anhaenge/`, und nach der 200 fragt A nichts mehr unter `/anhaenge/` an.
- [ ] **Nachholen** (E2E): B offline, `beta` löscht, B online → nach „Wieder verbunden“ „gelöscht“ ohne Neuladen.
- [ ] **Endgültig** (E2E): der Test liefert nach dem Löschen ein verspätetes `aenderung` mit `zustand: "bearbeitet"` und kleinerem `geaendert` aus → die Nachricht bleibt „gelöscht“.
- [ ] Konsole bei A und B nach der Regel.

### F0135 Eigene Nachricht im Browser bearbeiten

- [ ] **Eigene erkennen** (Node, rein): `von` = eigene Kennung und bestätigt → bearbeitbar; fremde Kennung, „wird gesendet …“, fehlgeschlagen, `zustand: "geloescht"` → nicht bearbeitbar.
- [ ] **Desktop mit Maus** (E2E): eigene Nachricht von A; Hover → „Bearbeiten“ sichtbar; Klick → Feld mit `Hallo`, Fokus im Feld, Schreibmarke am Ende; A tippt ` zusammen`, Klick auf „Speichern“ → genau ein `POST …/nachrichten/bearbeiten` mit Kopf `Idempotency-Key`; das Feld ist zu; bei A und B steht „Hallo zusammen“ mit „bearbeitet“, bei B weniger als 1 s nach der 200; der Fokus steht bei A auf der Nachricht.
- [ ] **Tastatur** (E2E): in leerer Eingabe `↑` → Fokus auf der eigenen letzten Nachricht; `E` → Feld; `Shift Enter` → neue Zeile im Feld; `Enter` → gespeichert. Noch einmal `E`, Text ändern, `Esc` → kein `POST`, Text unverändert, Fokus auf der Nachricht. `Strg Enter` speichert ebenso.
- [ ] **Telefon** (E2E, Telefon): Tippen auf „Aktionen“ an der eigenen Nachricht → „Bearbeiten“; Tippen → Feld; `Enter` macht eine neue Zeile, kein `POST`; Tippen auf „Speichern“ → gespeichert. Tablet (E2E, Tablet): dasselbe.
- [ ] **Nur eigene** (E2E): Nachrichten von `anna` (B) und `beta` (API) enthalten bei A weder „Bearbeiten“ noch „Aktionen“; eine Nachricht „wird gesendet …“ (Senden im Test verzögert) und eine fehlgeschlagene ebenfalls nicht.
- [ ] **Frühere Sitzung** (E2E): `shoff` sendet `Früher` per `curl` und verlässt; danach tritt Browser A als `shoff` bei → an `Früher` gibt es „Bearbeiten“, und Speichern gelingt (200).
- [ ] **Inzwischen gelöscht** (E2E): A hat das Feld an der eigenen Nachricht offen; der Test löscht dieselbe Nachricht über die API mit dem Sitzungs-Token von A (`page.evaluate` mit dem Header, den die Seite benutzt — ein zweiter Beitritt als `shoff` ginge nicht, der Name ist nur einmal vergeben, ADR-0012); A klickt „Speichern“ → 409; das Feld ist zu, die Nachricht zeigt „gelöscht“, die Hinweiszeile sagt „Die Nachricht wurde inzwischen gelöscht.“ (PO Zeile 122). Das Getippte wird nicht gerettet: der Test hat einen vom Ausgangstext abweichenden Text getippt; danach steht er in keinem Textknoten des Verlaufs, die Eingabe unten ist unverändert (leer, oder mit dem Entwurf, der vorher darin stand), und die Zwischenablage wird nicht beschrieben (`navigator.clipboard.writeText` wird nie aufgerufen).
- [ ] **Platte voll beim Speichern** (E2E; Gegeben vor dem Start mit `quote-ordner` bis 300 Bytes unter `--plattenquote 10MiB`): A sendet `Hallo`, bearbeitet zu 400 Bytes Text und speichert → 507; der Text der Instanz steht am Feld, das Getippte bleibt, das Feld bleibt offen.
- [ ] **Leer und unverändert** (E2E): Text gelöscht bis leer → „Speichern“ nicht auslösbar (`disabled`), `Enter` sendet nichts; unverändert speichern → genau ein `POST`, Feld zu, keine Marke „bearbeitet“.
- [ ] **Während des Bearbeitens** (E2E): `beta` sendet drei Nachrichten, während A tippt → sie erscheinen, das Feld behält Text und Fokus; Wechsel nach `# technik` und zurück → das Feld ist zu, die Nachricht unverändert, kein `POST`.
- [ ] **Bildschirmleser** (E2E): das Feld hat den zugänglichen Namen „Nachricht bearbeiten“, „Speichern“ und „Abbrechen“ sind Knöpfe; Speichern meldet über die eine Ansage-Region genau einmal (Wortlaut `ux-reviewer`); Fokus ist an allen Knöpfen sichtbar.
- [ ] **Zielfläche und Schmal** (E2E, Telefon, Tablet, Desktop; Telefon zusätzlich bei 320 px): Mindestmaße wie unter „Nicht-funktionale Anforderungen“; mit offenem Feld gilt `scrollWidth <= innerWidth`.
- [ ] Konsole bei A nach der Regel.

### F0139 Eigene Nachricht im Browser löschen

- [ ] **Text der Rückfrage** (Node, rein; Größe in Worten nach R00030): keine Anhänge → „Diese Nachricht löschen?“; `foto.png` 307.200 und `bericht.pdf` 2.457.600 Bytes → „Diese Nachricht mit 2 Anhängen (2,6 MiB) löschen? Die Anhänge werden mitgelöscht.“ (2.764.800 / 1.048.576 = 2,64 → „2,6 MiB“); nur `foto.png` → „Diese Nachricht mit 1 Anhang (300 KiB) löschen? Die Anhänge werden mitgelöscht.“; ein Anhang mit `zustand: "geloescht"` zählt nicht (beide gelöscht → „Diese Nachricht löschen?“).
- [ ] **Desktop mit Maus** (E2E): eigene Nachricht von A; Hover → „Löschen“; Klick → Dialog (`role="dialog"`, modal) mit „Diese Nachricht löschen?“; „Abbrechen“ → Dialog zu, kein `POST`; noch einmal „Löschen“ und im Dialog „Löschen“ → genau ein `POST …/nachrichten/loeschen` mit dem Körper `{"zeit": …}` (ohne weiteres Feld); bei A und B „gelöscht“, bei B weniger als 1 s nach der 200; Fokus bei A auf der Nachricht; eine Ansage.
- [ ] **Tastatur** (E2E): fokussierte eigene Nachricht, `Entf` → Dialog, der Fokus steht im Dialog und bleibt dort bei `Tab`; `Esc` → Dialog zu, kein `POST`, Fokus zurück auf der Nachricht; `Entf`, dann `Enter` auf „Löschen“ → gelöscht.
- [ ] **Telefon** (E2E, Telefon): Tippen auf „Aktionen“ → „Löschen“ → Dialog → Tippen auf „Löschen“ → gelöscht; der Dialog passt in 375 × 812 und bei 320 px ohne Querscrollen.
- [ ] **Mit Anhängen** (E2E): `shoff` sendet per `curl` den Text `Bericht` mit `foto.png` und `bericht.pdf` (R00029) und verlässt; Browser A tritt als `shoff` bei, wählt an `Bericht` „Löschen“ → der Dialog nennt „mit 2 Anhängen (2,6 MiB)“; nach dem Bestätigen antworten beide Abrufe 410, und `anhang_belegt` ist 0.
- [ ] **Fehlschlag beim Löschen** (E2E): der `POST` wird im Test abgebrochen (`route.abort()`) → die Hinweiszeile sagt „Keine Antwort vom Server.“ (wie R00030), die Nachricht bleibt unverändert; ein zweites Löschen ohne Abbruch gelingt.
- [ ] **Nur eigene; Ausnahme zu R00030** (E2E): an Nachrichten von `anna` und `beta` — auch mit Anhängen — gibt es bei A kein Element, dessen zugänglicher Name „lösch“ enthält; an der eigenen Nachricht genau eines („Löschen“). Das Kriterium „Kein Lösch-Knopf“ aus R00030 F0102 gilt mit dieser Ausnahme; sein Test zieht mit.
- [ ] **Bildschirmleser** (E2E): der Dialog hat als zugänglichen Namen bzw. Beschreibung den Text der Rückfrage; nach dem Löschen meldet die Ansage-Region genau einmal (Wortlaut `ux-reviewer`).
- [ ] Konsole bei A und B nach der Regel.

## Betroffene Verzeichnisstruktur

- `web/live-verlauf.js` (Übernahme-Regel, Verteilung), `web/verteiler.js`, `web/strom.js` (Thema `aenderung`), `web/nachholen.js`, `web/nachhollauf.js` (Fassungen übernehmen), `web/nachricht.js` (Marken, eigene Nachricht), `web/verlauf.js` (Aktionen, Ersetzen ohne Sprung), `web/fokus.js`, `web/ansage.js`, `web/api.js`, `web/lc-seite.css`.
- `web/bearbeiten.js` und `web/loeschen.js` neu.
- Tests: `e2e/einheit/` (Node: Übernahme, Darstellung, Rückfrage), `e2e/tests/` (Playwright: `r00041-*.spec.js`).
- Designsystem: `components/Nachricht/README.md` (Marken, Aktionen, Feld), `1-tastatur.md` (`E`, `Entf`) — nach der UX-Vorprüfung.

## Technische Überlegungen

### Ablauf (Vorplanung — welche Bubbles es am Ende werden, entscheidet der Entwickler)

1. **Sehen** (F0134): Übernahme-Regel rein (B0604) → Marke (B0605) → Verteilen (B0606) → Nachholen (B0607); dann **gelöscht** (F0138: B0620, B0621).
2. **Bedienen** (F0135): eigene Nachricht erkennen (B0608) → Aktionen erreichen (B0609) → Feld (B0610) → Speichern (B0611) → Ansage (B0612); dann **Löschen** (F0139: B0622 Text der Rückfrage, B0623 Dialog, B0624 Aufruf).

Reihenfolge: F0134 braucht R00039 F0133 (Ereignis), F0138 zusätzlich R00040 F0136; F0135 nach F0134, F0139 nach F0135 und F0138. Läuft parallel zu R00040 (keine gemeinsame Datei).

### Grobentwurf

- `fassungUebernehmen(bekannt, neu) -> Nachricht | null` (rein, `web/live-verlauf.js`).
- `bearbeitbar(nachricht, eigeneKennung) -> boolean` (rein, `web/nachricht.js`).
- `rueckfrageText(nachricht) -> string` (rein, `web/loeschen.js`); `groesseInWorten` aus R00030 (wiederverwenden oder nach denselben Regeln bauen, falls R00030 F0102 noch nicht gebaut ist).

## Aufwand (Zählung)

Gezählt aus der WBS; Richtwerte, ungemessen für diese Bubble-Typen.

- F0134: 4 Bubbles · 2 klar zu 0,4 h (B0604, B0605; 0,8 h) · 2 unklar zu 2–4 h (B0606, B0607; 4–8 h)
- F0138: 2 Bubbles · 2 klar (B0620, B0621; 0,8 h)
- F0135: 5 Bubbles · 3 klar (B0608, B0611, B0612; 1,2 h) · 2 unklar (B0609, B0610; 4–8 h)
- F0139: 3 Bubbles · 2 klar (B0622, B0624; 0,8 h) · 1 unklar (B0623; 2–4 h)
- Summe 14 Bubbles, 13,6–23,6 h.

## Abhängigkeiten

- Abhängig von: R00039 F0132, F0133; R00040 F0136 (für F0138, F0139; der Teil „Mit Anhängen“ zusätzlich F0137); R00011, R00026, R00027 (Eingabe, Strom, Touch-Regeln, gebaut); R00030 F0102 nur für „Größe in Worten“ (sonst hier gebaut).
- Blockiert: I0026 Reaktionen im Browser (dieselbe Übernahme über `geaendert`), I0013 Erwähnungen (Markierung nach Bearbeiten).

## Offene Fragen

- **Entschieden** vom product-owner (`AI-Team/po-entscheidungen.md`): Zeile 121 Bearbeiten an Ort und Stelle, Zeile 122 409 `geloescht` beim Speichern ohne Rettung des Getippten, Zeile 123 Zeit der letzten Änderung im Tooltip der Marke, auf Touch durch Antippen. Wortlaute und Mechanik der Touch-Zeit bleiben bei der UX-Vorprüfung.
- **UX-Vorprüfung** (`ux-reviewer`, Pflicht vor dem Bau von F0135 und F0139, PO Zeile 57): Ort der Aktionen am Desktop, Touch-Zugang („Aktionen“-Schalter gegen Langes Drücken), Tasten `E` und `Entf`, Startfokus im Dialog (Vorschlag „Abbrechen“), Wortlaute „bearbeitet“, „gelöscht“, Rückfrage, Hinweis, Ansagen; Channelwechsel bricht ab (Vorschlag).
- **An den Director**: das Kriterium „Kein Lösch-Knopf“ aus R00030 F0102 bekommt hier eine Ausnahme für die eigene Nachricht (R00030 wird nicht geändert, sein Test zieht beim Bau von F0139 mit).

## Missing-Docs

- Glossar fehlt. Designsystem beschreibt Aktionen an der Nachricht nur für „Antworten“ und „Kennung kopieren“ (`components/Nachricht/README.md`) — Bearbeiten, Löschen und die Marken kommen mit der UX-Vorprüfung dazu.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist die Frage des Betreibers nach „Nachricht nachbearbeiten“ und „Nachricht löschen“ — er arbeitet im Browser, nicht mit `curl`. Wenn der Verlauf jedes `aenderung` über `geaendert` übernimmt und beim Nachholen die Fenster-Blöcke neu liest, dann zeigt jeder offene Tab sofort und nach jeder Lücke die aktuelle Fassung mit „bearbeitet“ oder „gelöscht“, und niemand liest mehr die überholte Aussage. Die Aktionen an der eigenen Nachricht machen die Server-Routen für Menschen auf jedem Gerät erreichbar, und die Rückfrage mit Zahl und Größe der Anhänge fängt genau das Versehen ab, das ohne Rückgängig nicht zu heilen wäre. Der Hebel liegt im Tab und nicht im Server, weil der Server alles Nötige schon liefert (R00039, R00040); fehlt die Oberfläche, bleibt die Fähigkeit für Menschen unsichtbar.

## Notizen

- **Verworfen**: Rückgängig statt Rückfrage (PO Zeile 95, kein Rückgängig); Bearbeiten in der Eingabe unten (die Nachricht verschwindet aus dem Blick, Verwechslung mit Senden); Aktionen nur über Tastatur (Leitplanke Zielgeräte).
- **Nicht hier**: frühere Fassungen ansehen (Betreiber: keine Einsicht); Markdown im neuen Text (I0017); Bearbeiten in Direktgesprächen (I0012); Laufwerk (I0027).
- Slice-Karten: C1 Sehen (F0134, F0138) kann vor der UX-Vorprüfung gebaut werden, wenn die Wortlaute als Vorschlag gelten; C2 Bedienen (F0135, F0139) erst danach.
