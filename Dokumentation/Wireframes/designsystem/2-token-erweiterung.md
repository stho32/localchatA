# Token-Erweiterung gegenüber kd-tokens.css

Übernommen aus kanbanD (Namen mit `kd-`): Gründe, Flächen, Linien, Text, Kopfzeile, Akzent, `kd-mensch`/`kd-agent` mit hellen Varianten, die Abstandsstufung `kd-space-2` … `kd-space-24`, `kd-row`, `kd-header`, `kd-radius`.

Neu für localchatA trägt jeder Name das Präfix `lc-`. Drei `kd-`-Namen sind ebenfalls neu, weil sie in kanbanD vermutlich fehlen und für die ganze Familie taugen — sie sollten nach kanbanD zurückwandern.

| Token | Hell / Dunkel | Warum |
| --- | --- | --- |
| `kd-line-strong` | #7d8a98 / #6b7885 | Ränder von Bedienelementen brauchen 3:1; `kd-line` ist nur Gliederung. Für die Familie. |
| `kd-header-surface` | #2c3a48 / #1a2330 | Suchfeld und gedrückte Schalter in der dunklen Kopfzeile. Für die Familie. |
| `kd-danger` | #b42318 / #ff8e80 | Fehlerzustände (Senden, Name vergeben, Datei zu groß). Für die Familie. |
| `kd-focus` | #0b62c4 / #6cb4ff | Sichtbarer Fokus auf allen hellen Flächen. Für die Familie. |
| `lc-focus-header` | #6cb4ff | Fokus auf der Kopfzeile; `kd-focus` erreicht dort im hellen Modus nur 2,5:1. |
| `lc-mention`, `lc-on-mention` | #b42318 / #ff8e80 | Erwähnungs-Zähler und Ungelesen-Linie — müssen stärker sein als „ungelesen“. |
| `lc-mention-bg` | #e2ecf6 / #1d2b39 | Grund einer Nachricht, die mich erwähnt. |
| `lc-unread`, `lc-on-unread` | #4f5d6b / #9aa7b4 | Neutraler Ungelesen-Zähler, bewusst schwächer als Erwähnungen. |
| `lc-online` | #1d7a3c / #5fcf86 | Online-Punkt ●. |
| `lc-offline` | #5f6b78 / #8d99a6 | Offline ○ und „gleich weg“ (gestrichelt). |
| `lc-privat-bg` | #e9edf2 / #20262d | Kennzeichnung von Direktnachrichten: Grund der Mitte und des Hinweisstreifens, nie allein — das Wort „Direktnachricht“ trägt die Bedeutung (R00050 V1, V7). |
| `lc-code-bg` | #f2f5f8 / #11151a | Codeblöcke und Inline-Code — kanbanD hat keinen Code. |
| `lc-syn-keyword`, `-string`, `-number`, `-function`, `-comment` | siehe Tokens | Fünf ruhige Syntaxfarben, alle ≥ 4,9:1 auf `lc-code-bg`. |
| `lc-gutter` | 40px | Spalte für Kachel bzw. Uhrzeit gruppierter Nachrichten. |
| `lc-tile` | 20px | Kürzel-Kachel statt Avatar. |
| `lc-sidebar`, `lc-members` | 240px | Spaltenbreiten der Hauptansicht. |
| `lc-collapse` | 240px | Ab dieser Höhe wird eine Nachricht eingeklappt. |
| `lc-composer-max` | 200px | Höchsthöhe der Eingabe. |
| `lc-measure` | 860px | Lesebreite des Nachrichtentexts. |
| Schriftstile `lc-*` | — | Die Typografie-Stufen des Chats (Kennung, Zeit, Code, Markdown-Überschriften). |
