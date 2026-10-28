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
