# ADR-0038: Eingabe — mehrzeiliges Textfeld unter dem Verlauf, `Enter` sendet, Entwurf je Channel im Tab, Fehler an der Nachricht, gebündelte Ansage in eigener Statusregion

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #8 I0009)
**Kontext-Anforderung**: — (WBS I0009; prägt I0010 Anhänge, I0013 `@`-Vorschlag, I0017 Markdown-Leiste und Codeblock-Regel)
**Erprobt durch**: — (überträgt gebaute Muster: Statuszeile `role="status"`, Fehler in `kd-danger` mit ⚠ und `aria-invalid`, Tastenauswertung als reine Funktion mit Node-Test — ADR-0024, ADR-0033; Tab-Reihenfolge — ADR-0027 Regel 7)

## Kontext

- Designsystem `components/Eingabe`: mehrzeilig, Rahmen `kd-line-strong`, Fokus 2 px `kd-focus`, wächst bis `lc-composer-max` (200 px), dann scrollt es in sich; Platzhalter „Nachricht an # entwicklung“; `Enter` sendet, `Shift Enter` neue Zeile, im offenen ```` ``` ````-Block fügt `Enter` eine Zeile ein und `Strg Enter` sendet; Markdown als Text, Leiste nur „Anhang“ und „Code“; `@`-Vorschlag; Antworten; Anhänge; „Ein nicht gesendeter Entwurf bleibt je Channel im Tab erhalten.“
- `1-tastatur.md`: `/` oder `Tab` vom Verlauf in die Eingabe; `Esc` im Verlauf zurück in die Eingabe; `↑` in leerer Eingabe fokussiert die eigene letzte Nachricht.
- `components/Hauptansicht`: Tab-Reihenfolge Kopfzeile → Seitenleiste → Verlauf → Eingabe → Teilnehmerliste; der leere Channel zeigt „Noch keine Nachrichten in # name“ und darunter, wie Agenten hier schreiben.
- Designsystem, Barrierefreiheit: „Angesagt wird **gebündelt**: höchstens eine Ansage alle 10 s („5 neue Nachrichten in # entwicklung, davon 1 an dich“); Nachrichten, die mich erwähnen, und DMs sofort.“ [ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md) Regel 9: der Verlauf ist `role="log"` mit `aria-live="off"`; die gebündelte Ansage kommt mit I0009 als **eigene Statusregion**.
- [ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md): Text höchstens 16.384 Bytes, Steuerzeichen abgewiesen; [ADR-0037](0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md): vorläufig, Warteschlange, Fehlerzustände.
- Projektregeln (AI-Team/entwickler.md, Gelerntes #3; ADR-0024): Fokus auf gefüllten Flächen ≥ 3:1, kein Querscrollen bei 320 px, Fehler mit ⚠ und `aria-invalid`.

## Optionen

### Feld

- **A: `<textarea>` mit eigener Höhenanpassung** — Stärke: Zeilen, Einfügen, Undo, Rechtschreibprüfung und IME liefert der Browser; Text bleibt Text (Markdown wird getippt, Designsystem). Schwäche: das Wachsen bis 200 px braucht ein paar Zeilen JavaScript (`style.height` aus `scrollHeight`).
- **B: `contenteditable`** — Stärke: später Hervorhebungen (`@name`) im Feld möglich. Schwäche: Einfügen bringt HTML mit, Zeilenumbrüche und IME sind je Browser verschieden; widerspricht „Markdown als Text, nicht WYSIWYG“.
- **C: einzeiliges `<input>`, mehrzeilig nur über einen Umschalter** — Stärke: einfachster Code. Schwäche: Agenten-Code und Listen werden in einer Zeile getippt; widerspricht dem Designsystem.

### Tasten

- **`Enter` sendet, `Shift Enter` neue Zeile, `Strg Enter` sendet** (Designsystem, Slack/Discord) — Stärke: gewohnt; schnelles Gespräch. Schwäche: wer Code tippt, muss `Shift Enter` kennen, bis die Codeblock-Regel kommt.
- **`Strg Enter` sendet, `Enter` neue Zeile** — Stärke: kein versehentliches Senden. Schwäche: gegen Designsystem und Gewohnheit.
- **Codeblock-Regel jetzt** (`Enter` im offenen ```` ``` ````-Block = Zeile) — Stärke: vollständig wie entworfen. Schwäche: gehört zu Markdown; in I0009 wird Markdown noch als Text gezeigt.

### Entwurf

- **`sessionStorage` je Channel** — Stärke: übersteht Neuladen und Channel-Wechsel im Tab, wie entworfen („im Tab“); andere Tabs sehen ihn nicht. Schwäche: Größe zählt gegen das Kontingent des Ursprungs (höchstens 16 KiB je Channel).
- **Nur im Speicher des Tabs** — Stärke: nichts gespeichert. Schwäche: Neuladen verliert den Text.
- **`localStorage`** — Stärke: übersteht Schließen. Schwäche: alle Tabs teilen den Entwurf — zwei Tabs im selben Channel überschreiben einander.

### Ansage

- **Eigene Statusregion `role="status"` mit Bündelung** (ADR-0026 Regel 9) — Stärke: der Verlauf bleibt still beim Nachladen; die Ansage folgt der Regel des Designsystems. Schwäche: eigene Zeitlogik.
- **Verlauf als `aria-live="polite"`** — Stärke: kein Code. Schwäche: jede eingefügte Nachricht und jedes Nachladen würde gelesen; bei einem Agenten im Takt ein Dauerton.

## Entscheidung

**Gewählt: `<textarea>`; `Enter` sendet, `Shift Enter` neue Zeile, `Strg Enter` sendet (Codeblock-Regel mit I0017); Entwurf in `sessionStorage` je Channel; Fehler an der Nachricht; Ansage in eigener Statusregion mit Bündelung.**

Es überwiegt, dass jede Wahl dem Designsystem folgt und der Browser das Schwierige (Textbearbeitung, IME, Fokus) selbst richtig macht. In Kauf genommen: bis I0017 muss, wer einen Codeblock von Hand tippt, `Shift Enter` nutzen; ein Entwurf überlebt das Schließen des Tabs nicht.

Regeln:

1. **Ort und Form**: unter dem Verlauf, über die volle Breite der Mitte, sobald ein Channel gewählt ist (ohne Auswahl keine Eingabe). `<textarea rows="1">` mit `aria-label` „Nachricht an # name“ und gleichlautendem Platzhalter; Rahmen `kd-line-strong`, Fokus 2 px `kd-focus`; wächst mit dem Inhalt bis `lc-composer-max` (200 px), danach scrollt es in sich. Leiste („Anhang“, „Code“) und `@`-Vorschlag kommen mit I0010, I0017, I0013 — bis dahin keine leeren Schalter. Bei 320 px kein Querscrollen.
2. **Tasten** (Auswertung als reine Funktion `taste(ereignis, text) → "senden" | "zeile" | nichts`, Node-Test wie ADR-0024 Pre-Mortem 1):
   - `Enter` ohne Umschalt-, Strg-, Alt- und Meta-Taste → senden; `Strg Enter` / `⌘ Enter` → senden; `Shift Enter` → neue Zeile (Verhalten des Browsers);
   - **nie senden während einer IME-Komposition** (`isComposing` oder `keyCode 229`) — sonst schickt ein Japanisch- oder Chinesisch-Tippender halbe Wörter;
   - Text nur aus Leerraum → `Enter` tut nichts (keine Anfrage, kein Fehler);
   - Text über 16.384 Bytes (`TextEncoder`) oder mit einem verbotenen Zeichen (dieselbe Regel wie der Server, eine Funktion in `web/`) → `Enter` sendet nicht; die Hinweiszeile unter dem Feld wird Fehler (Regel 4).
3. **Senden** übergibt an die Zustellung ([ADR-0037](0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md) Regel 1): Feld leer, Höhe zurück auf eine Zeile, Entwurf gelöscht, **Fokus bleibt im Feld**. Das Feld wird nie gesperrt — man kann weiterschreiben, während gesendet wird.
4. **Länge**: ab 15.000 Bytes zeigt eine 20-px-Zeile unter dem Feld „noch n Zeichen“ (`kd-ink-muted`, `aria-live="polite"` nur beim Überschreiten der Schwelle, nicht je Taste); über 16.384 „Zu lang: 17.020 von höchstens 16.384 Byte — teilen, oder ab I0010 als Anhang.“ in `kd-danger` mit ⚠, Feld `aria-invalid="true"`. Ein verbotenes Zeichen (meist eingefügte Terminal-Farbcodes) → „Steuerzeichen U+001B an Stelle 17 — bitte entfernen.“ Umgeformt wird nicht (Hausregel).
5. **Fehler an der Nachricht** (Designsystem `Nachricht`): eine fehlgeschlagene Nachricht zeigt unter dem Text ⚠ und den Grund in `kd-danger` (Texte aus ADR-0037 Regel 4) und zwei Schalter „Erneut senden“ und „Verwerfen“ (Tab-Reihenfolge innerhalb des Verlaufs). Nach beiden geht der Fokus zurück ins Feld. „Verwerfen“ legt den Text **nicht** zurück ins Feld; wer ihn noch braucht, kopiert ihn vorher (die Nachricht bleibt markierbar).
6. **Verbindung verloren**: das Feld bleibt bedienbar; gesendete Nachrichten erscheinen „wird gesendet …“ und warten (ADR-0037 Regel 4). Kein eigener Hinweis im Feld — die Leiste oben im Verlauf sagt es.
7. **Entwurf**: `sessionStorage` unter `localchatA.entwurf.<channel>`; geschrieben bei `input` (höchstens alle 500 ms) und beim Verlassen des Channels; beim Öffnen eines Channels wird sein Entwurf ins Feld gelegt; gelöscht beim Senden und wenn das Feld leer wird. Kein Entwurf in `localStorage` (Tabs überschrieben einander).
8. **Fokus und Tastatur**: Tab-Reihenfolge Kopfzeile → Seitenleiste → Verlauf → **Eingabe** → Teilnehmerliste (ändert ADR-0027 Regel 7 um die Eingabe). `/` außerhalb von Textfeldern und `Esc` im Verlauf setzen den Fokus ins Feld (1-tastatur.md). Beim Wechsel des Channels bleibt der Fokus, wo er ist (ADR-0027 Regel 4); liegt er im Feld, bleibt er im Feld — jetzt mit dem Entwurf des neuen Channels. `↑` in leerer Eingabe (eigene letzte Nachricht fokussieren) gehört zum Fokussieren einzelner Nachrichten und ist nicht I0009.
9. **Leerer Channel**: unter „Noch keine Nachrichten in # name“ steht eine Zeile in `kd-ink-muted`: „Agenten schreiben hier mit `POST /api/channels/name/nachrichten`.“ (Designsystem: Link auf `/api`; die Beschreibung `/api/openapi.json` kommt mit I0015 — bis dahin ohne Link).
10. **Gebündelte Ansage** (reine Funktion `ansage(zustand, ereignis, jetzt) → {zustand, text?}` mit gestellter Zeit): eine Statusregion `role="status"` `aria-live="polite"` außerhalb des Verlaufs (visuell verborgen). Angesagt werden **fremde** Nachrichten des **offenen** Channels: die erste nach mindestens 10 s Ruhe sofort („Neue Nachricht von claude@10.0.0.5 in # entwicklung“), alle weiteren gesammelt und höchstens alle **10 s** als eine Ansage („4 neue Nachrichten in # entwicklung“). Eigene Nachrichten, Nachgeladenes und Nachgeholtes (ADR-0037 Regel 7) werden nie angesagt. Ein Fehlschlag eigener Nachrichten wird **sofort** angesagt („Nachricht nicht gesendet: <Grund>“), unabhängig von der Bündelung. Erwähnungen und Direktnachrichten („sofort“) kommen mit I0013/I0012 hinzu.

## Konsequenzen

**Positiv:**
- Die Eingabe verhält sich wie in gewohnten Chats; IME-Nutzer verlieren nichts; Fehler stehen dort, wo die Nachricht steht.
- Screenreader hören höchstens eine Sammelansage je 10 s, auch wenn Agenten im Takt schreiben.
- Leiste, `@`-Vorschlag und Codeblock-Regel lassen sich später an Feld und Tastenfunktion anfügen, ohne die Form zu ändern.

**Negativ / Inkauf zu nehmen:**
- Bis I0017 sendet `Enter` auch mitten in einem von Hand getippten Codeblock.
- Der Entwurf verschwindet mit dem Tab; vorläufige und fehlgeschlagene Nachrichten mit dem Neuladen (ADR-0037).
- Wer eine Nachricht nur gelegentlich in einem stillen Channel bekommt, hört sie sofort; wer in einem lauten liest, hört Sammelzahlen — das ist die Regel des Designsystems.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … Nutzer mit IME halbe Wörter senden. Früh-Indikator: Node-Test mit `isComposing: true`. Gegenmaßnahme: Regel 2.
2. … nach dem Senden der Fokus im Nichts landet oder in den Verlauf springt. Früh-Indikator: E2E prüft `document.activeElement` nach Senden, „Erneut senden“, „Verwerfen“. Gegenmaßnahme: Regel 3, 5.
3. … ein Screenreader bei einem Agenten im Takt ununterbrochen redet. Früh-Indikator: Node-Test der Bündelung über 60 s mit einer Nachricht je Sekunde → höchstens 7 Ansagen. Gegenmaßnahme: Regel 10.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Node-Test Tasten: `Enter` → senden; `Shift Enter` → zeile; `Strg Enter`, `Meta Enter` → senden; `Alt Enter` → nichts; `Enter` mit `isComposing` oder `keyCode 229` → nichts; `Enter` bei Text `" \n "` → nichts; bei 16.385 Bytes → nichts.
- [ ] Erster Node-Test Ansage (gestellte Zeit): t = 0 fremd → sofort „Neue Nachricht von …“; t = 1, 2, 3 s fremd → nichts; t = 10 s → „3 neue Nachrichten in # …“; t = 25 s fremd (Ruhe ≥ 10 s) → sofort; eigene Nachricht → nie; Fehlschlag → sofort, auch innerhalb der 10 s. Spanne: 9.999 ms keine Sammelansage, 10.000 ms eine (Mutanten 9 s / 11 s werden rot).
- [ ] Erster E2E-Test: Feld hat `aria-label` „Nachricht an # allgemein“; zwei Zeilen per `Shift Enter`, `Enter` → Feld leer, Fokus im Feld, Nachricht mit beiden Zeilen im Verlauf; Entwurf „halb“ in `# allgemein`, Wechsel nach `# technik` und zurück → „halb“ wieder im Feld, auch nach Neuladen; 320 px ohne Querscrollen; Fokusring ≥ 3:1.

## Quellen / Bezuege

- [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md)
- ADR-0024, ADR-0026, ADR-0027, ADR-0033, ADR-0034, ADR-0037
- Designsystem `components/Eingabe/README.md`, `components/Nachricht/README.md`, `components/Hauptansicht/README.md`, `README.md` („Barrierefreiheit“), `1-tastatur.md`; `design-prompt.md` 4.2 („Eingabe unten: mehrzeilig, Markdown, Anhänge, Erwähnung“), Tastenbelegung („Senden (Enter) vs. neue Zeile (Shift+Enter)“)
- AI-Team/entwickler.md, Gelerntes #3 (UX-Maßstab); IME und `keyCode 229`: verbreitete Browser-Praxis (`KeyboardEvent.isComposing`, MDN) — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet

## Ergänzungen

- **2026-10-28 (Karte #31; Code-Review #8 A8) — Leer-Regel des Browsers gleich der des Servers:** Präzisiert Regel 2 („Text nur aus Leerraum → `Enter` tut nichts“). Anlass: der Browser prüft mit `text.trim() === ''` (`web/eingabe.js:17`), der Server mit `char::is_whitespace` ([ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) Regel 3.4, `src/gespraeche/textregel.rs:18`). Geprobt (node, rustc, 2026-10-28): U+FEFF — `trim()` leer, Rust nicht leer; U+0085 — `trim()` nicht leer, Rust leer; U+00A0, U+3000 — beide leer; U+200B, U+180E — beide nicht leer. `/^\p{White_Space}*$/u` ergab für alle sechs dasselbe wie Rust.
  - Option A: **der Browser übernimmt die Klasse des Servers** (gewählt): leer ⇔ `/^\p{White_Space}*$/u.test(text)` — dieselbe Unicode-Eigenschaft `White_Space`, die `char::is_whitespace` prüft. Stärke: der Server hat die Hoheit über das, was gespeichert wird; Browser und API (Agent) urteilen gleich; eine Zeile, kein Zeichenkatalog im Code. Schwäche: ein Text nur aus U+FEFF wird gesendet und erscheint als leere Nachricht — wie heute schon über die API.
  - Option B: der Server übernimmt die Klasse von `trim()`. Stärke: U+FEFF allein wäre leer. Schwäche: ändert die Zusage aus R00011 und ADR-0034 samt Tests für ein exotisches Zeichen; die JavaScript-Klasse ist eine Eigenheit der Sprache, keine Unicode-Eigenschaft.
  - Option C: beide werten zusätzlich unsichtbare Formatzeichen (Kategorie Cf: U+FEFF, U+200B) als leer. Stärke: keine unsichtbaren Nachrichten. Schwäche: Cf enthält U+200D (Verbinder in Emoji-Folgen) — die Regel bräuchte eine eigene Tabelle in zwei Sprachen; verlangt hat es niemand.
  - **Gewählt: A.** In Kauf genommen: unsichtbare Nachrichten aus Formatzeichen bleiben möglich. Ort: die Leer-Prüfung gehört in die Textregel des Browsers (`web/textregel.js`, B0246, F0048 in I0020) als eigene Funktion, `eingabe.js` ruft sie; bis F0048 gebaut ist, ersetzt sie `trim()` in `eingabe.js:17`. Nicht betroffen: das Namensfeld (`web/beitreten.js`) — für Namen gilt die Erlaubnisliste ([ADR-0013](0013-namensregel-als-erlaubnisliste.md)).
  - **Erster Test** (Node, rein): `""`, `" \n\t"`, `" "`, `"\u0085"`, `"　"` → leer; `"﻿"`, `"​"`, `"a"` → nicht leer. Mutante „`trim()`“ → rot (bei `\u0085` und `﻿`).
- **2026-10-29 (Karte #36, R00025 F0084, B0398, B0399; UX-Review #22 F1, F2 und Hinweis „zwei `role=status`“; WBS Offene Fragen #22 Punkt 4) — eine Verbindungswarnung, eine Statusregion:** Ändert Regel 6 („die Leiste oben im Verlauf“) im Ort und Regel 10 in der Zahl der Regionen; ersetzt den Hinweis `#lc-verbindung` mit `role="alert"` aus R00005 (`web/hauptansicht.js:49`). Heute gibt es bei einer Trennung zwei Warnungen mit verschiedenen Worten (Hinweis über der Seite, Leiste im Verlauf), die Leiste schiebt den Verlauf um eine Zeile (gemessen 37 px), und die Seite hat zwei `role="status"` (Statuszeile `web/hauptansicht.js:59`, Ansage-Region `:213`) plus ein `role="alert"`.
  - **Ort der einen Warnung**:
    - Option A: **eine Leiste, die oben über der Mitte liegt** (`position: absolute` in `.lc-center`), mit und ohne offenen Channel (**gewählt**). Stärke: dasselbe Muster wie Ladezeile und Pille („liegen über dem Verlauf, nicht in ihm — Ein- und Ausblenden verschiebt nichts“, `web/lc-seite.css:80`); der Ort des Designsystems (`VerlaufsMarken` „Leiste oben im Verlauf“); die Mitte gibt es auch ohne Channel, also auch die Warnung (die Zusage aus R00005 hält). Schwäche: solange sie steht, verdeckt sie die oberste Zeile des Verlaufs bzw. den Anfang des Hinweises der leeren Mitte.
    - Option B: der Hinweis über der Seite bleibt, mit **festem, immer reserviertem Platz**. Stärke: verdeckt nichts, steht auch über der Seitenleiste. Schwäche: kostet auch verbunden dauerhaft eine Zeile; weit weg vom Verlauf, wo „wird gesendet …“ steht; der Ort weicht vom Designsystem ab.
    - Option C: die Leiste im Verlauf wie heute, schiebend; ohne Channel keine Warnung. Stärke: kein Umbau. Schwäche: der Sprung bleibt (gegen F0084), und ohne Channel verlöre R00005 seine Warnung.
    - Option D: eine Meldung unten links wie die Statuszeile (`position: fixed`). Stärke: verdeckt den Verlauf nicht. Schwäche: liegt bei 320 px über Eingabe und Pillen; nicht der Ort des Designsystems.
    - **Gewählt: A.** In Kauf genommen: die oberste Zeile des Verlaufs ist während der Trennung verdeckt — man kann sie wegscrollen, und die Ladezeile tut dasselbe schon heute.
  - **Zahl der Statusregionen**:
    - Option E: **eine Region für alle Ansagen** — die Ansage-Region aus Regel 10 (Empfehlung des `planer`, **gewählt**). Stärke: eine Reihenfolge für alle Ansagen; kein Bildschirmleser bekommt dieselbe Nachricht aus zwei Regionen; die Bündelung (Regel 10) und „sofort“ (Fehlschlag) sind schon eine Funktion. Schwäche: eine Trennung unterbricht nicht (polite) — wer gerade tippt, hört sie nach dem Satz.
    - Option F: Ansage-Region plus `role="alert"` für die Trennung. Stärke: die Trennung wird sofort gehört. Schwäche: zwei Regionen ohne festgelegte Reihenfolge; `alert` unterbricht beim Tippen für einen Zustand, der nichts zerstört (Gesendetes wird gepuffert, ADR-0037 Regel 4).
    - **Gewählt: E.** In Kauf genommen: die Trennung wird höflich, nicht unterbrechend angesagt.
  - **Regeln**:
    1. **Ein Element**: die Leiste `.lc-leiste` ist Kind der Mitte (`.lc-center`, schon `position: relative`), einmal je Hauptansicht angelegt; ein Channelwechsel und das Neu-Öffnen ersetzen sie nicht. Sie liegt oben über der Mitte (`position: absolute; top: 0; left: 0; right: 0`), mindestens eine Zeile hoch (`--kd-row`), über der Ladezeile. Sie trägt beide Zustände: „⚠ Verbindung verloren · neuer Versuch in 5 s“ bzw. „· verbinde …“ (Countdown nach [ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md), Ergänzung Karte #36) und für 4 s „✓ Wieder verbunden · n nachgeladen“ (ADR-0037 Regel 7). Der Hinweis `#lc-verbindung` entfällt samt `role="alert"` und seiner Zeile im Raster `.lc-app`.
    2. **Kein Sprung**: weder die Leiste noch ihr Wechsel verändert die Lage von `.lc-log`; steht zugleich die Ladezeile, liegt sie direkt unter der Leiste (ebenfalls überlagernd). Ohne offenen Channel verdeckt die Leiste den Anfang des Hinweises der Mitte; er rückt nicht.
    3. **„Stand unbekannt“** in der Teilnehmerliste (R00005) bleibt — es ist ein Zustand der Liste, kein zweites Warnelement.
    4. **Eine Live-Region**: die Ansage-Region (Regel 10, `role="status"`, `aria-live="polite"`, visuell verborgen) ist die **einzige** Live-Region der Seite. Die Leiste ist keine Live-Region — ihr Countdown ändert sich je Sekunde. Die Statuszeile unten (Frist gesetzt, Channel angelegt) bleibt sichtbar, verliert aber `role="status"`; ihr Text geht zugleich **sofort** an die Ansage-Region.
    5. **Was angesagt wird, sofort und außerhalb der Bündelung** (neue Ereignisart der reinen Funktion `ansage`, wie `fehlschlag`): „Verbindung verloren“ **einmal je Trennung** (beim Wechsel nach getrennt, nicht je Versuch und nicht je Countdown-Schritt); der Text der Meldung ohne ✓ einmal je Meldung („Wieder verbunden · 3 nachgeladen“, „Wieder verbunden“, „Wieder verbunden · neueste geladen“); der Text der Statuszeile. Gesammelte fremde Nachrichten bleiben gesammelt, ihr Takt wird nicht zurückgesetzt. Solange das Nachholen scheitert (ADR-0037, Ergänzung Karte #22/#36, 7c), gibt es keine Ansage „Wieder verbunden“.
  - **Durchsetzung**: E2E — vor, während und nach einer Trennung `document.querySelectorAll('[aria-live], [role="status"], [role="alert"]').length === 1` und genau ein sichtbares Element mit „Verbindung verloren“; Oberkante von `.lc-log` gleich (± 1 px) — die Abnahme-Probe UX F1. Mutante „zweite Region“ → rot. Review-Punkt `ux-reviewer`: neue Ansagen gehen durch `ansage`, nie über eine eigene Region.
  - **Folgen** (für den `planer`): R00005 „Zuletzt online“ (E2E) prüft die Leiste statt „Verbindung unterbrochen“ — R00025 nennt es; die Statuszeile ohne `role` und mit Ansage ist ein kleiner Zug in B0399 oder eine eigene Bubble.
