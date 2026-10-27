# Anhang

Dateien an einer Nachricht: Bilder als Vorschau, alles andere als 28-px-Zeile mit Name, Größe, Typ.

- **Datei-Zeile**: Typkürzel (`lc-label`-Mono auf `kd-surface-2`), Name mit Ellipse, Größe in `kd-ink-muted`, dann „Öffnen“ und „Laden“ in `kd-accent`. Höchstens 420 px breit; mehrere Dateien stapeln sich mit `kd-space-4`.
- **Bild**: Vorschau höchstens 360 × 240 px im Seitenverhältnis. Der Platz wird **aus Breite und Höhe in den Metadaten reserviert**, bevor das Bild lädt; bis dahin `kd-surface-2` mit Name und Maßen. Klick öffnet das Original in voller Größe im Dialog.
- **Hochladen läuft**: 4-px-Balken in `kd-accent` mit Prozent und „Abbrechen“. **Fehlgeschlagen**: Rahmen und Text `kd-danger`, ⚠ und Grund im Wortlaut, „Erneut“. **Zu groß**: sofort beim Auswählen geprüft, mit tatsächlicher Größe und Grenze, „Entfernen“ — die Nachricht kann ohne die Datei gesendet werden.
- Fehler werden mit `role="alert"` angekündigt.
