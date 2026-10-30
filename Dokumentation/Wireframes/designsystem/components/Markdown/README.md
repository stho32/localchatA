# Markdown

Die Darstellung von Markdown in Nachrichten — im Stil der Anwendung, nicht wie eine eingebettete Webseite.

- Überschriften bewusst klein: `#` in `lc-md-h1` (16 px), `##`/`###` in `lc-md-h2` (14 px). Tiefer als `###` wird als fetter Absatz gezeigt.
- Codeblock: Rahmen `kd-line`, Grund `lc-code-bg`, Kopfzeile 24 px in `kd-surface-2` mit Sprache (und Dateiname, wenn als ```` ```ts upload.ts ```` angegeben) und dem Knopf „Kopieren“ (zugänglicher Name „Code kopieren“; er hat keine Taste — die Taste `C` kopiert die Nachricht, PO 153). Text in `lc-code`, waagrecht scrollbar, nie umgebrochen. Syntax nur mit den fünf Farben `lc-syn-keyword`, `lc-syn-string`, `lc-syn-number`, `lc-syn-function`, `lc-syn-comment` (kursiv) — reicht für Lesbarkeit, bleibt ruhig.
- Inline-Code: `lc-code` auf `lc-code-bg`, ohne Rahmen.
- Tabellen: Haarlinien `kd-line`, Kopf auf `kd-surface-2`, Zeilen 24 px, 12 px Schrift; breite Tabellen scrollen in sich.
- Links in `kd-accent`, unterstrichen; externe Links öffnen in neuem Tab. Bilder im Markdown werden wie Anhänge behandelt (Platz vorab reserviert).
- Kein HTML im Markdown — wird als Text gezeigt.
