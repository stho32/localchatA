# Tastenbelegung

Gewohnte Konventionen (Slack/Discord, Browser), auf macOS `Strg` = `⌘`. Kürzel mit Einzeltasten wirken nur, wenn der Fokus nicht in einem Textfeld steht. `?` zeigt diese Übersicht als Overlay.

| Taste | Wirkung | Wo |
| --- | --- | --- |
| `Strg K` | Suche öffnen (4.8); im Suchfeld auch Channels per `#` springen | überall |
| `Strg P` | Schnellwechsel: Channel oder Teilnehmer per Name öffnen | überall |
| `Alt ↑` / `Alt ↓` | vorheriger / nächster Channel bzw. DM in der Seitenleiste | überall |
| `Alt Shift ↑` / `Alt Shift ↓` | vorheriger / nächster Eintrag **mit Ungelesenem** | überall |
| `Strg Alt N` | Channel anlegen (4.3) | überall |
| `Enter` auf `# name` | Channel-Einstellungen (Aufbewahrungsfrist) öffnen — kein eigenes Kürzel (ADR-0033) | Kopfzeile |
| `Strg Shift M` | Übersicht „Wo wurde ich erwähnt?“ (4.6) | überall |
| `Strg .` | Teilnehmerliste ein-/ausblenden (4.4) | überall |
| `Esc` | Overlay/Dialog schließen; im Verlauf: zurück in die Eingabe; zweimal: Channel als gelesen markieren | überall |
| `/` oder `Tab` vom Verlauf | in die Eingabe springen | außerhalb von Textfeldern |
| `Enter` | senden; auf Touch-Geräten (grober Zeiger, je Tastendruck gefragt) neue Zeile — gesendet wird dort mit dem Knopf „Senden“ (ADR-0038 Ergänzung Karte #41) | Eingabe |
| `Shift Enter` | neue Zeile | Eingabe |
| `Enter`, `Shift Enter`, `Strg Enter` | wie in der Eingabe: speichern, neue Zeile, speichern auch auf Touch-Geräten | Bearbeiten-Feld |
| `Esc` | Bearbeiten abbrechen ohne Anfrage, Fokus auf die Nachricht (Channel wird nicht als gelesen markiert); ein zweites `Esc` wie im Verlauf | Bearbeiten-Feld |
| `Ende`, `Pos1`, `↑`, `↓`, `Leertaste`, `E`, `Entf` | wirken nur auf den Text, nie auf den Verlauf (R00041 UX M2) | Bearbeiten-Feld |
| `Strg Enter` | senden, auch innerhalb eines offenen Codeblocks und auf Touch-Geräten (etwa mit angesteckter Tastatur am Tablet) | Eingabe |
| `Strg Shift Enter` | senden wie `Strg Enter` — `Strg` geht `Shift` vor, auch auf Touch-Geräten | Eingabe |
| `↑` in leerer Eingabe | eigene letzte Nachricht in den Verlauf fokussieren | Eingabe |
| `@` | Erwähnungs-Vorschlag; `↑ ↓` wählen, `Enter`/`Tab` einsetzen | Eingabe |
| `Strg Shift U` | Datei anhängen | Eingabe |
| `↑` / `↓` | vorherige / nächste Nachricht fokussieren | Verlauf |
| `R` | auf fokussierte Nachricht antworten — Regeln wie `C` (Ausschlüsse, `key`) | Verlauf |
| `Leertaste` | fokussierte lange Nachricht auf-/zuklappen | Verlauf |
| `C` | fokussierte Nachricht kopieren (Quelltext; PO 153). Nur Abkürzung — ausgewertet wird `event.key` (nicht `code`), `c` und `C` bei Feststelltaste ohne Umschalt; ohne Strg, Alt, Meta, Umschalt. Wirkungslos bei offenem `dialog[open]`, bei Fokus auf einem Knopf oder einem Zitat, im Eingabe- und im Bearbeiten-Feld und bei markiertem Text. Der Codeblock hat keine Taste, nur seinen Knopf „Code kopieren“ | Verlauf |
| `E` | fokussierte **eigene** Nachricht bearbeiten (R00041, PO 127) — nur Abkürzung, die Knöpfe „Bearbeiten“ bleiben; ohne Strg, Alt, Meta; wirkungslos an fremden Nachrichten, im Eingabe- und im Bearbeiten-Feld; ist schon ein Feld offen, Ansage „Erst speichern oder abbrechen.“ | Verlauf |
| `Entf` | fokussierte **eigene** Nachricht löschen — öffnet die Rückfrage, Startfokus „Abbrechen“ (R00041, PO 126, 127); ohne Strg, Alt, Meta; wirkungslos an fremden Nachrichten und in Feldern | Verlauf |
| `Ende` | zum Neuesten springen (bzw. zur ersten neuen Nachricht) | Verlauf |
| `Pos1` | an den Anfang des Channels | Verlauf |
| `Strg G` | Sprung an ein Datum | Verlauf |
| `?` | diese Übersicht | außerhalb von Textfeldern |
