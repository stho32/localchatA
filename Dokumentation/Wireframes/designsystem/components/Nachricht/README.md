# Nachricht

Eine Nachricht im Verlauf: Kopf mit Absender und Zeit, darunter der Markdown-Inhalt; Folgenachrichten werden gruppiert.

- Raster: linke Spalte `lc-gutter` (40 px) mit der Kürzel-Kachel, rechts der Inhalt mit Höchstbreite `lc-measure`.
- **Gruppierung**: Folgt eine Nachricht desselben Absenders innerhalb von **5 Minuten** und ohne Tagestrenner dazwischen, entfällt der Kopf. Die Uhrzeit steht dann in der linken Spalte und erscheint nur bei Hover oder Fokus. Abstand innerhalb einer Gruppe `kd-space-4`, zwischen Gruppen `kd-space-12`.
- **Zeit**: absolut, `HH:MM` in `lc-time` (Mono, gleich breite Ziffern). Datum und relative Angabe („vor 2 min“) im Tooltip. Absolute Zeiten ändern sich nicht — der Verlauf muss nicht minütlich neu gezeichnet werden, und in einem schnellen Agenten-Gespräch ist „14:03“ genauer als „vor 1 min“.
- **Erwähnt mich**: Grund `lc-mention-bg`, im Kopf das Wort „@ dich“ in `kd-accent`, im Text `@shoff` umrandet. Andere Erwähnungen nur als `kd-accent`-Text.
- **Senden**: „wird gesendet …“ — Text in `kd-ink-muted`, Zeile darunter mit ◌; gesendet — kein Zusatz (der Normalfall braucht keine Marke); fehlgeschlagen — ⚠ und Grund in `kd-danger`, Schalter „Erneut senden“ und „Verwerfen“. Eine fehlgeschlagene Nachricht bleibt unter dem Verlauf, unter allen bestätigten, bis „Erneut senden“ oder „Verwerfen“.
- Hover: ganze Zeile `kd-surface-2`, rechts oben erscheinen „Antworten“ und „Kennung kopieren“ (per Tastatur: `R` auf der fokussierten Nachricht).
- Keine Schatten, keine Animation je Zeile.

Der Verbraucher liefert: Absender (Kennung + Art), Zeitstempel, Markdown, Sendestatus, ob ich erwähnt bin, ob gruppiert.
