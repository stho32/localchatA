# LangeNachricht

Agenten-Nachrichten mit Code, Tabellen und langen Befundlisten werden ab einer festen Höhe eingeklappt, damit der Verlauf überschaubar und jede Zeile vorhersehbar hoch bleibt.

- Höher als `lc-collapse` (240 px, ≈ 12 Zeilen) → der Inhalt wird auf 240 px abgeschnitten (harter Schnitt, kein Verlauf), darunter die Leiste „Ganze Nachricht anzeigen“ mit einer Zusammenfassung aus dem Markdown: Zeilenzahl, Zahl der Listeneinträge bzw. Tabellenzeilen, Zahl der Codeblöcke.
- Aufklappen erweitert an Ort und Stelle; die Leiste heißt dann „Einklappen“ und bleibt am Ende. Die Ansicht wird so verankert, dass der Kopf der Nachricht an seiner Bildschirmposition bleibt.
- Eigene Nachrichten werden nie eingeklappt; Menschen-Nachrichten erst ab der doppelten Höhe.
- Aufgeklappt-Zustand gilt je Nachricht und Tab, er wird nicht gespeichert.
- Befund-Stufen als umrandete Mono-Marke mit Wort: `HOCH` in `kd-danger`, `MITTEL` in `kd-agent`, `NIEDRIG` in `kd-ink-muted` (Darstellung der Markdown-Konvention `[HOCH]`, falls Agenten sie nutzen — offene Frage).

Der Verbraucher liefert: das gerenderte Markdown und dessen gemessene Höhe; die Zusammenfassung kommt aus dem Markdown-Baum.
