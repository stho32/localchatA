# Eingabe

Das mehrzeilige Eingabefeld unten im Verlauf, mit Markdown, Anhängen und Erwähnung per `@`.

- Rahmen `kd-line-strong`, bei Fokus 2 px `kd-focus`. Wächst mit dem Text bis `lc-composer-max` (200 px), danach scrollt es in sich. Platzhalter: „Nachricht an # entwicklung“.
- `Enter` sendet, `Shift Enter` neue Zeile. Innerhalb eines offenen ```` ``` ````-Blocks fügt `Enter` eine Zeile ein; `Strg Enter` sendet dann.
- Markdown wird als Text getippt, nicht als WYSIWYG; Leiste nur mit „Anhang“ und „Code“.
- **@-Vorschlag**: Tippt man `@`, öffnet über dem Feld eine Liste aktiver Teilnehmer (online zuerst, dann offline mit Wort), gefiltert nach dem Getippten über Name und Herkunft, höchstens 8 Zeilen. `↑ ↓` wählen, `Enter`/`Tab` setzt `@name `, `Esc` schließt. Ein Name, den es nur einmal gibt, braucht keine Herkunft — eingesetzt wird nur der Teil vor dem `@`.
- **Antworten**: Über dem Text eine 24-px-Zeile „↩ Antwort an …“ mit Anfang der Nachricht und ✕. Die gesendete Nachricht zeigt diesen Bezug als Zitatzeile; ein Klick springt zum Original.
- **Anhänge** per Schalter, Drag & Drop auf den ganzen Verlauf (Hinweisfläche „Dateien hier ablegen“) und Einfügen aus der Zwischenablage; vor dem Senden als Zeilen über dem Text (siehe Anhang).
- Ein nicht gesendeter Entwurf bleibt je Channel im Tab erhalten.
