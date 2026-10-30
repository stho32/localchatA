# ADR-0052: Nachricht kopieren — Quelltext aus dem Fenster, synchron in der Geste über ein verstecktes Textfeld und `execCommand('copy')`, die Clipboard-API nur als zweiter Weg; Taste `C` als reine Funktion

**Status**: Akzeptiert (Chromium geprobt; WebKit und Firefox ungeprüft — Prüfpunkt im Entwicklertest am Telefon, siehe Konsequenzen)
**Datum**: 2026-10-30
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #78)
**Kontext-Anforderung**: [R00046](../../Anforderungen/R00046-nachricht-kopieren-im-browser.md) (WBS I0042)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (2026-10-30, Playwright 1.57.0/Chromium headless aus `e2e/node_modules` des Hauptbaums, nur gelesen, statische Seite über `python3 -m http.server` auf freiem Port, aufgerufen über `http://127.0.0.1:<p>/` **und** `http://192.168.178.65:<p>/`; Probe danach gelöscht). Ergebnisse unten unter „Probe“.

## Kontext

- **Auftrag** (PO-Entscheidungen Zeilen 142, 144, 145, 151; Vision-Zielbild „Nachrichten kopieren und mit Zitat beantworten“): Aktion „Kopieren“ an jeder bestätigten, nicht gelöschten Nachricht mit Text, auch fremd und von Agenten; kopiert wird der **Quelltext** (Markdown wie geschrieben), ohne Absender, Zeit, Marken, Anhänge; Rückmeldung „Kopiert.“ am Knopf und einmal in der Ansage-Region, bei Fehlschlag „Kopieren nicht möglich“; Taste `C` auf der fokussierten Nachricht nur als Abkürzung (nie im Eingabefeld, nie bei markiertem Text, `Strg+C` unberührt); **kein Server-Teil**. Karte #78: „im LAN läuft die Seite über http, dort ist `navigator.clipboard` nicht verfügbar (kein sicherer Kontext) — Rückfall nötig; architekt/entwickler belegen beides im Browser-Test“.
- **Offene Fragen des planer** (R00046 „Offene Fragen“ 1–5): Archiv, Direktgespräche, Ort der Rückmeldung (ux), Emoji-Kurznamen, ob neben `execCommand` ein dritter Weg nötig ist.
- **Bestand**:
  - Der Verlauf zeichnet den Text heute mit `textContent` (`web/nachricht.js:14`); ein Markdown-Renderer ist offen ([A00005](../Architektur/A00005-weboberflaeche.md) „Offene Fragen“). Sobald er kommt, ist das DOM **nicht** mehr der Quelltext.
  - Die Fassung einer Nachricht kommt aus Block und Thema `aenderung` (ADR-0048 Regeln 4, 11); der Server liefert in `text` den Text so, wie er im Log steht (ADR-0034 Regel 4, `\r\n` → `\n`).
  - Aktionsleiste, Schalter „⋯“ und Fokus der Nachrichten entstehen mit #68/#69 (F0135 B0609, B0628; F0141), je Aktion gebunden (PO 145, WBS-Nachtrag #68).
  - Ansage-Region `web/ansage.js` (gebündelte Ansage, ADR-0038); Tastenauswertung als reine Funktion mit Node-Test ist Hausmuster (ADR-0024, ADR-0038 Regel 2).
  - Gate: `npm --prefix e2e test` mit Playwright **nur Chromium** (`AI-Team/umgebung.md`).
- **Vision-Leitplanke „Zielgeräte“**: Maus, Tastatur, Touch; jede Aktion ohne Tastenkürzel erreichbar. Betrieb im LAN über `http://<LAN-IP>` ist der Normalfall (ADR-0045), `localhost` der Sonderfall des Betreibers am eigenen Rechner.

### Qualitätsziele mit Szenario

| # | Ziel | Szenario mit Maß |
|---|---|---|
| QK1 | Genau | Nachricht aus vier Zeilen mit `**fett**`, Leerzeile, Codezaun `sh` → Zwischenablage zeichengleich (`===`, gleiche Länge), ohne Kennung, Uhrzeit, „bearbeitet“, ohne angehängten Zeilenumbruch — unter `http://127.0.0.1` **und** `http://<LAN-IP>` |
| QK2 | Ruhig | nach dem Kopieren Fokus unverändert (Nachricht bzw. Knopf), Verlauf nicht gescrollt (0 px), keine Netzanfrage, `sha256sum` über `channels/` gleich |
| QK3 | Sicher | fremder Text gelangt nur als `text/plain` in die Zwischenablage, nie als HTML in die Seite; die Zwischenablage wird nie gelesen |
| QK4 | Schnell | Rückmeldung < 200 ms nach dem Auslösen (erwartet < 5 ms, alles lokal) |

## Optionen

### Frage A — Woher kommt der Text?

| | Quelle | Stärke | Schwäche |
|---|---|---|---|
| **A1 (gewählt)** | **der `text` der Nachricht, wie ihn das Fenster hält** (Block bzw. `aenderung`, also die aktuelle Fassung) | ist der Quelltext (PO 144) — unabhängig davon, wie gezeichnet wird; bearbeitete Nachricht liefert die neue Fassung ohne eigenen Code | das Fenster muss den Text je Nachricht behalten (Eigenschaft am Element oder Abbildung des Fensters; ≤ 500 Nachrichten, ADR-0026 Regel 1) |
| A2 | `textContent` des gezeichneten Körpers | nichts zu halten | heute zufällig gleich, mit dem Markdown-Renderer falsch (gerenderte Sicht statt Quelltext, gegen PO 144); eingeklappte lange Nachrichten (ADR-0026 Regel 4) sind im DOM vollständig, aber der Renderer kann Zeichen umformen — verworfen |
| A3 | beim Klick `GET …/nachrichten` neu holen | immer frisch | Netzanfrage gegen QK2 und R00046 („nichts geht zum Server“); offline nicht möglich — verworfen |

### Frage B — Wie kommt der Text in die Zwischenablage?

| | Weg | Stärke | Schwäche |
|---|---|---|---|
| B1 | erst `navigator.clipboard.writeText` (wo vorhanden), bei Fehlen oder Ablehnung Rückfall | moderne Schnittstelle zuerst | im LAN über `http` gibt es sie nicht (Probe: `typeof navigator.clipboard === "undefined"`), der Rückfall ist also der **Normalfall** und müsste trotzdem als Zweitweg gebaut und getestet werden; die Ablehnung kommt asynchron — ob der Rückfall danach noch als Nutzergeste gilt, hängt am Browser (Chromium: Aktivierung ≈ 5 s; WebKit unbelegt); Probe: unter `localhost` lehnt Chromium ohne Berechtigung mit `NotAllowedError` ab |
| **B2 (gewählt)** | **synchron in der Geste: verstecktes, schreibgeschütztes Textfeld mit dem Text, markieren, `document.execCommand('copy')`, Feld entfernen, Fokus und vorige Markierung zurück; nur wenn das `false` liefert oder wirft und `window.isSecureContext` gilt: `navigator.clipboard.writeText` als zweiter Weg** | **ein** Weg für `localhost` und LAN, synchron in der Geste (keine Frage nach Aktivierung), in jedem Test derselbe; das Textfeld überträgt Zeilenumbrüche unverändert (Probe: zeichengleich); der zweite Weg fängt einen künftigen Browser ohne `execCommand` in sicherem Kontext | `execCommand` gilt als veraltet (alle Zielbrowser führen es; Entfernen ist angekündigt-los, aber möglich); das Feld nimmt kurz den Fokus — muss zurückgegeben werden (Probe: zurück auf der Nachricht) |
| B3 | `execCommand('copy')` ohne Feld, der Text über das Ereignis `copy` (`clipboardData.setData('text/plain', …)`) | rührt weder Fokus noch Markierung an (Probe in Chromium: wirkt auch ohne Markierung) | ob WebKit das Ereignis ohne Markierung auslöst, ist nicht belegt (kein WebKit im Gate), und das Telefon ist Zielgerät — verworfen; bleibt Option, falls B2 am Fokus scheitert |
| B4 | den gezeichneten Körper markieren lassen und das Kopieren des Browsers auslösen | kein verstecktes Feld | kopiert die **Sicht**, nicht den Quelltext (A2) — verworfen |

### Frage C — Taste `C`

| | Regel | Stärke | Schwäche |
|---|---|---|---|
| **C1 (gewählt)** | **reine Funktion** `kopierTaste(ereignis, lage) → boolean`: `ereignis.key` klein `c` (auch bei Feststelltaste), **ohne** Strg, Alt, Meta, Umschalt, nicht `isComposing`, nicht `repeat`; Ziel ist die fokussierte Nachricht selbst (nie Eingabefeld, Bearbeiten-Feld, `contenteditable`, Knopf); `lage.markierung` leer (`getSelection().isCollapsed`) — Node-Test | Hausmuster (ADR-0038 Regel 2), vollständig ohne Browser prüfbar; dieselbe Form trägt `R` (ADR-0053) und `E`/`Entf` (#68) | Feststelltaste zählt mit (Absicht: `C` und `c` sind dieselbe Taste für den Menschen) |
| C2 | `ereignis.code === 'KeyC'` | unabhängig vom Belegungstext | bei anderen Belegungen (z. B. Dvorak) liegt `KeyC` woanders als das gedruckte `C` — verworfen |

## Entscheidung

**Gewählt: A1, B2, C1.**

Es überwiegt, dass der Normalbetrieb im LAN ohnehin nur `execCommand` hat: ein Weg, der überall derselbe ist und in jedem Test derselbe bleibt, ist billiger und ehrlicher als ein moderner Weg, der nur am Rechner des Betreibers läuft und den eigentlichen Weg zum Rückfall macht. In Kauf genommen: eine als veraltet geltende Schnittstelle als Hauptweg; der kurze Fokuswechsel ins versteckte Feld; WebKit und Firefox sind im Gate nicht belegt.

Regeln:

1. **Quelltext** ([A00005](../Architektur/A00005-weboberflaeche.md)): kopiert wird `nachricht.text` aus dem Fenster — die aktuelle Fassung (Block oder `aenderung`), zeichengleich, nichts ergänzt, nichts abgeschnitten (auch kein Zeilenumbruch am Ende). Das Fenster behält den Text je gezeichneter Nachricht (Wahl des Entwicklers: Eigenschaft am Element oder Abbildung über `zeit`; nie ein `data-*`-Attribut mit 16 KiB, nie aus dem DOM gelesen). Eine Nachricht nur aus Anhängen (`text === ""`), eine gelöschte (`zustand: "geloescht"`), eine vorläufige („wird gesendet …“) und eine fehlgeschlagene bieten die Aktion nicht an — die Bedingung ist eine reine Funktion `kopierbar(nachricht)` neben `bearbeitbar` (B0608), je Aktion gebunden (PO 145).
2. **Weg** (Modul `web/kopieren.js`, eine Funktion `kopieren(text, dokument) → "kopiert" | "nicht_moeglich"`, synchron im Klick- bzw. Tastenhandler aufgerufen, vor jedem `await`):
   - (a) vorher merken: `document.activeElement` und die Bereiche der aktuellen Markierung;
   - (b) `textarea` anlegen: `readOnly`, `aria-hidden="true"`, `tabindex="-1"`, `position: fixed; top: 0; left: 0; opacity: 0; pointer-events: none; font-size: 16px` (16 px verhindert das Zoomen in iOS beim Markieren), `value = text`; an `document.body` hängen;
   - (c) `select()` und `setSelectionRange(0, text.length)` (iOS markiert sonst nicht);
   - (d) `ok = document.execCommand('copy')` in `try`;
   - (e) Feld entfernen; Fokus zurück mit `focus({ preventScroll: true })`, vorige Markierung zurück; **immer**, auch bei Fehler (`finally`);
   - (f) `ok` → `"kopiert"`; sonst, wenn `window.isSecureContext && navigator.clipboard?.writeText` → `writeText(text)` (Promise; erfüllt → `"kopiert"`, abgelehnt → `"nicht_moeglich"`); sonst `"nicht_moeglich"`.
   - Die Zwischenablage wird **nie gelesen**; es wird nie HTML geschrieben, nie `innerHTML` mit dem Text gesetzt.
3. **Rückmeldung**: `"kopiert"` → am Knopf (bzw. Menüeintrag) kurz „Kopiert.“, in der Ansage-Region genau einmal „Kopiert.“; erneutes Kopieren **derselben** Nachricht innerhalb von 2 s ändert die Region nicht erneut (sonst sagt ein Bildschirmleser nichts Neues); `"nicht_moeglich"` → „Kopieren nicht möglich“ am Knopf und in der Region. Dauer, Ort und Form der Anzeige am Knopf legt der `ux-reviewer` fest (R00046 Frage 3). Weder Fokus noch Lesestelle ändern sich (QK2).
4. **Erreichen**: Aktion „Kopieren“ in der Aktionsleiste (Hover, Fokus), im Menü des Schalters „⋯“ (Touch, ≥ 44 × 44 px) und im Tab-Weg der Nachricht — Gefäß aus #68 (B0609, B0628), je Aktion gebunden; Taste `C` nach Regel C1 als Abkürzung, ausgewertet im selben Tastenhandler wie `E`/`Entf` (#68), nie der einzige Weg.
5. **Kein Server**: keine Route, keine Anfrage, keine Zeile im Betriebsprotokoll, kein Feld im Health-Check.
6. **Archiv und Direktgespräche** (R00046 Fragen 1, 2): Kopieren ist Lesen — angeboten im archivierten Channel (sobald I0038) und im Direktgespräch (sobald I0012) wie im Channel. **Emoji-Kurznamen** (Frage 4): kopiert wird, was in `text` steht, also der Kurzname.

### Probe (2026-10-30, Chromium headless, gerechnet nichts, gemessen nur Wirkung)

| Aufruf | `isSecureContext` | `navigator.clipboard` | B2 Textfeld + `execCommand` | B3 Ereignis `copy` | B1 `writeText` |
|---|---|---|---|---|---|
| `http://127.0.0.1:<p>/` ohne Berechtigung | `true` | vorhanden | `true`, zeichengleich, Fokus zurück | `true`, Inhalt gesetzt | **abgelehnt** `NotAllowedError` |
| `http://127.0.0.1:<p>/` mit `clipboard-read`/`-write` | `true` | vorhanden | `true`, zeichengleich | `true` | erfüllt |
| `http://192.168.178.65:<p>/` | **`false`** | **`undefined`** | `true`, zeichengleich, Fokus zurück | `true` | entfällt |

Gelesen wurde die Zwischenablage durch `Strg+V` in ein Textfeld der Seite — das geht auch ohne sicheren Kontext. **Befund für die Tests:** die Zwischenablage gehört dem **Browser-Prozess**, nicht dem Kontext — ein Inhalt aus einem vorigen Kontext stand im nächsten noch darin.

## Konsequenzen

**Positiv:**
- Ein Weg für alle Aufrufe; der LAN-Fall ist der getestete Normalfall, nicht ein Rückfall.
- Der Quelltext ist unabhängig vom künftigen Markdown-Renderer und folgt Bearbeitungen ohne eigenen Code.
- Kein Server-Anteil: keine Last, keine Quote, kein Protokoll, keine Angriffsfläche an der Instanz.

**Negativ / Inkauf zu nehmen:**
- `execCommand('copy')` ist als veraltet markiert. Fällt es in einem Browser weg, trägt im sicheren Kontext der zweite Weg, im LAN über `http` gibt es dann **keinen** Weg mehr („Kopieren nicht möglich“) — Abhilfe wäre dann HTTPS im LAN (eigene Entscheidung, heute nicht verlangt).
- Das versteckte Feld nimmt für die Dauer des Aufrufs den Fokus; ein Bildschirmleser könnte den Wechsel bemerken (`aria-hidden`, kein Name, sofort zurück) — Prüfpunkt `ux-reviewer`.
- WebKit (iOS Safari) und Firefox sind im Gate nicht belegt (Playwright nur Chromium, `AI-Team/umgebung.md`): der Entwicklertest kopiert auf einem echten Telefon über `http://<LAN-IP>` (Lücke im Bericht).
- Tests mit Zwischenablage teilen sich den Browser-Prozess: sie laufen **seriell** (eigene Playwright-Datei mit `test.describe.configure({ mode: 'serial' })` oder eigenem Worker) und schreiben vor jeder Prüfung einen Wächtertext in die Ablage, damit ein Rest aus einem früheren Test nicht als Erfolg zählt.

**Pre-Mortem (ein Monat nach dem Bau gescheitert, weil …):**
1. … im LAN nichts kopiert wird, obwohl der Test grün war — der Test lief nur gegen `localhost` mit Berechtigung und nahm dort den zweiten Weg. Früh-Indikator: E2E „Über http im LAN“ fehlt oder nutzt `127.0.0.1`. Gegenmaßnahme: Regel 2 (ein Weg, `execCommand` zuerst), Probe P3 über die Nicht-Loopback-IP (wie der LAN-Test der Umgebung), Mutante K1.
2. … nach dem Markdown-Renderer `**fett**` als `fett` kopiert wird. Früh-Indikator: P2 nach Einbau des Renderers. Gegenmaßnahme: Regel 1 (Text aus dem Fenster, nie aus dem DOM), Mutante K2.
3. … ein Klick auf „Kopieren“ den Verlauf springen lässt oder den Fokus ins Nichts setzt. Früh-Indikator: P6. Gegenmaßnahme: Regel 2 (e) mit `preventScroll`, `position: fixed`.
4. … der Test grün ist, weil die Ablage noch den Text eines früheren Tests enthielt. Früh-Indikator: Kopieren-Tests laufen parallel oder ohne Wächtertext. Gegenmaßnahme: seriell, Wächtertext vor jeder Prüfung, Mutante K3.
5. … ein Bildschirmleser im Lesemodus auf `C` springt statt zu kopieren. Früh-Indikator: Vorprüfung `ux-reviewer`. Gegenmaßnahme: Taste ist nur Abkürzung, der Tab-Weg bleibt (Regel 4); Wortlaut und Kollisionsprüfung beim `ux-reviewer`.

**Folgeentscheidungen / Aufgaben** (Vorschläge an den planer, keine Arbeitsliste des architekt):
- [ ] I0042 nach #68/#69 (F0135, F0141): Bausteine `kopierbar` (Regel 1), `web/kopieren.js` (Regel 2), Aktion in Leiste/„⋯“/Tab-Weg (Regel 4), `kopierTaste` (C1), Rückmeldung (Regel 3), E2E über LAN-IP — Mini-Eignung im Bericht Karte #78.
- [ ] Designsystem `components/Nachricht/README.md`: Aktion „Kopieren“ und Rückmeldung nachziehen (`ux-reviewer`/`gestalter`).

### Abnahme-Proben (Test zuerst; „heute“ rot, weil nichts gebaut ist)

- **P1 `kopierbar` (Node-Test, rein)**: `{text:"a", zustand:null}` bestätigt → `true`; `zustand:"geloescht"` → `false`; `text:""` mit einem Anhang → `false`; `text:"Bild"` mit Anhang → `true`; vorläufig, fehlgeschlagen → `false`; fremde Kennung → `true` (gegen `bearbeitbar`: dort `false`).
- **P2 `kopieren` (Node-Test mit Attrappe `dokument`)**: Attrappe zeichnet auf: Feld mit `value === text`, `readOnly`, markiert 0…Länge, `execCommand('copy')` einmal, Feld danach entfernt, `focus({preventScroll:true})` auf dem vorigen Element — auch wenn `execCommand` wirft; `execCommand → false` und `isSecureContext:false` → `"nicht_moeglich"` ohne `writeText`-Aufruf; `false` und sicher → `writeText(text)` genau einmal.
- **P3 Quelltext über LAN (E2E, seriell)**: Instanz `--host 0.0.0.0`, Seite über `http://<Nicht-Loopback-IPv4>:<port>`, `window.isSecureContext === false` geprüft; `claude` sendet die vier Zeilen aus QK1; Ablage vorher Wächter `WAECHTER-78`; „Kopieren“ (Hover) → `Strg+V` in ein Prüffeld ergibt den Text `===`, Länge gleich; dasselbe unter `http://127.0.0.1` ohne Berechtigung (nimmt Weg (d)).
- **P4 Taste (Node-Test `kopierTaste`, dann E2E)**: `c` und `C` (Feststelltaste, `shiftKey:false`) auf der Nachricht → `true`; `Strg+C`, `Alt+C`, `Meta+C`, `Umschalt+C`, `repeat`, `isComposing`, Ziel Eingabefeld/Bearbeiten-Feld/Knopf, Markierung nicht leer → `false`. E2E: `C` im Eingabefeld tippt `c`, Ablage bleibt `WAECHTER-78`; markiertes Wort + `C` → Ablage unverändert, danach `Strg+C` → genau das Wort.
- **P5 Rückmeldung (E2E)**: Region wechselt genau einmal auf „Kopiert.“; zweites Kopieren derselben Nachricht nach 1 s → keine weitere Änderung der Region; erzwungener Fehlschlag (Test ersetzt `document.execCommand` durch `() => false` unter `http://<LAN-IP>`) → „Kopieren nicht möglich“ am Knopf und in der Region, Ablage `WAECHTER-78`.
- **P6 Ruhig (E2E, Netzbeobachtung)**: beim Kopieren keine Anfrage; Fokus danach auf dem auslösenden Element; Lage der Nachricht im Verlauf vorher = nachher (0 px); `sha256sum` über `channels/` gleich.

### Mutanten (jeder muss mindestens eine Probe rot machen)

| # | Mutante | rot in |
|---|---|---|
| K1 | `writeText` zuerst, `execCommand` nur bei Fehlen der Schnittstelle | P3 (`localhost` ohne Berechtigung: abgelehnt, kein Rückfall in der Geste) |
| K2 | Text aus `textContent` des Körpers statt aus dem Fenster | P3 mit einem Renderer-Stub, der `**` entfernt; P1-Variante bearbeitete Nachricht |
| K3 | Feld nicht entfernt / Fokus nicht zurück | P2, P6 |
| K4 | Taste ohne Prüfung der Markierung | P4 |
| K5 | `kopierbar` ohne Prüfung `geloescht` | P1 |
| K6 | Region bei jedem Kopieren neu beschrieben | P5 |
| K7 | Text mit angehängtem `\n` | P3 (Länge ungleich) |

## Quellen / Bezuege

- [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md)
- ADR-0024, ADR-0026, ADR-0038, ADR-0045, ADR-0048; ADR-0053 (Antworten, teilt Leiste und Tastenmuster)
- R00046 und User Stories, R00041 (Aktionsleiste je Aktion), WBS I0042, PO-Entscheidungen Zeilen 142, 144, 145, 151; Vision „Nachrichten kopieren und mit Zitat beantworten“, Leitplanke „Zielgeräte“
- Verhalten der Browser: `isSecureContext` und `navigator.clipboard` im unsicheren Kontext, `execCommand('copy')` in der Nutzergeste — belegt durch die Probe oben (Chromium), WebKit/Firefox nicht belegt

## Ergänzungen
