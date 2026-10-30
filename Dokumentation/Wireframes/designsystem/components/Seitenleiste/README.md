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

## Direktnachrichten, wie gebaut (R00050 F0180, UX-Vorprüfung Karte #13 V5, V7, V8, V13)

- Zwei `nav`: „Channels“, darunter „Direktnachrichten“ — Abschnittskopf `h2` in `lc-label`, **ohne** `+` (ein Gespräch beginnt in der Teilnehmerliste), immer da.
- Einträge **alphabetisch nach Kennung** (wie die Teilnehmerliste: Zahlen als Zahlen), nie nach Aktivität — beim Einfügen springt nichts, der Fokus bleibt.
- Eintrag: Statuspunkt des Gegenübers (wie Teilnehmerliste), **volle Kennung** `name@herkunft` in Mono (zwei gleichen Namens bleiben unterscheidbar), dahinter `offline`, `gleich weg` (statisch, ohne Countdown) oder `weg` (nicht mehr in der Teilnehmerliste) als Wort. Link `#/d/<kennung>`, der offene mit `aria-current="page"`; zugänglicher Name „Direktnachricht mit ben@127.0.0.3, online“.
- Leer: `Noch keine Direktnachrichten.` und `Ein Gespräch beginnt in der Teilnehmerliste.`
- Unter 720 px: Statuspunkt und **Kürzel** (zwei Zeichen, wie die Kachel), Eintrag 44 px hoch; mit grobem Zeiger immer 44 px, mit feinem 28 px.
- Unter erzwungenen Farben nimmt der Punkt `CanvasText`, damit online und offline unterscheidbar bleiben.
- **Ohne Zähler, ohne `@n`, ohne Marker „neu“** (PO O20): ob und wie Ungelesenes erscheint, entscheidet der Auftraggeber; bis dahin sagt die bestehende Live-Region eine fremde Direktnachricht an („Neue Direktnachricht von <kennung>“, V11). Die Zähler oben bleiben Entwurf.

Der Verbraucher liefert: Liste der Channels und DMs mit Ungelesen- und Erwähnungszahl, aktiven Eintrag.
