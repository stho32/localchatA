# Seitenleiste

Die linke Spalte mit Channels und darunter Direktnachrichten; je Eintrag eine 28-px-Zeile.

- Abschnittskopf in `lc-label` (Versalien, `kd-ink-muted`), rechts der Schalter `+` „Channel anlegen“.
- Eintrag ohne Neues: `kd-ink-muted`, normale Stärke. **Mit Ungelesenem**: `kd-ink` und `lc-ui-strong`. Aktiver Eintrag: Fläche `kd-accent`, Text `kd-on-accent`, `aria-current="page"`.
- **Zwei Zähler, klar verschieden** in Form, Text und Farbe:
  - Ungelesen: neutrale Zahl auf `lc-unread` — `48`.
  - Erwähnt: **mit @-Zeichen** auf `lc-mention` — `@2`. Steht links vom Ungelesen-Zähler, wenn beides zutrifft.
  - Direktnachrichten zählen immer als Erwähnung (an mich gerichtet).
- Direktnachrichten zeigen statt `#` den Status-Punkt des Gegenübers; offline und „gleich weg“ zusätzlich als Wort.
- Zähler über 99 werden `99+`. Jeder Zähler hat ein `aria-label` in ganzen Worten.
- Unter 720 px Fensterbreite: Leiste 48 px, nur `#`/Punkt und Zähler, Name als Tooltip.

Der Verbraucher liefert: Liste der Channels und DMs mit Ungelesen- und Erwähnungszahl, aktiven Eintrag.
