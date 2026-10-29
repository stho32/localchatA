# Kopfzeile

Die dunkle 40-px-Leiste aus kanbanD mit drei Zonen: Ort · Navigation · Bedienung.

- **Ort** (links): Instanzname in `lc-id` und `kd-header-muted`, dann der aktuelle Channel in `lc-title`, danach die Channel-Beschreibung abgeschnitten mit Ellipse. In einer Direktnachricht steht statt `# name` die volle Kennung des Gegenübers und „Privat · nur ihr beide“.
- **Navigation** (Mitte): das Suchfeld, höchstens 420 px breit, `kd-header-surface`. Es ist ein Schalter, der die Suche (4.8) öffnet; Kürzel `Strg K` steht rechts darin.
- **Bedienung** (rechts): die eigene Kennung `name@herkunft` (Herkunft in `kd-header-muted`), der Schalter „Teilnehmer n“ mit `aria-pressed`, der Schalter `?` für die Tastenübersicht.
- Fokus in der Kopfzeile: `lc-focus-header`, nie `kd-focus` (zu wenig Kontrast auf dem dunklen Grund im hellen Modus).
- Schmales Fenster (< 720 px): Beschreibung entfällt, Suchfeld schrumpft auf das Symbol ⌕, die eigene Kennung bleibt sichtbar.

- **Channel-Einstellungen** (Ergänzung 2026-10-28, ADR-0033): `# name` ist ein Schalter `# name ▾` (`▾` in `kd-header-muted`, Hover `kd-header-surface`, Fokus `lc-focus-header`), der den Dialog [ChannelEinstellungen](../ChannelEinstellungen/README.md) öffnet. Ab 720 px folgt `· 7 Tage` (Aufbewahrungsfrist, `lc-small`, `kd-header-muted`, Singular „1 Tag“) **vor** der Beschreibung, damit die Ellipse nur die Beschreibung kürzt; bei defekter Einstellungsdatei `· ⚠ Frist gesperrt`. Unter 720 px entfallen Frist-Text und Beschreibung gemeinsam, der Schalter bleibt.

Der Verbraucher liefert: Instanzname, aktuellen Channel oder DM-Partner (mit Frist), eigene Kennung, Zahl der Teilnehmer.

- **Schmal und Touch** (Ergänzung 2026-10-29, Karte #27, R00019 F0065; ADR-0027 und ADR-0033, je Ergänzung Karte #27): die 40-px-Leiste gilt ab 480 px mit feinem Zeiger.
  - **Unter 480 px zwei Zeilen**, fest 80 px: Zeile 1 (44 px) ist die Zone „Ort“ über die ganze Breite — nur `# name ▾`, ohne Instanzname (er steht im Tab-Titel), ohne Frist-Text und Beschreibung; der Name endet erst auf …, wenn er allein die Zeile nicht füllt. Zeile 2 (36 px) trägt Kachel, Kennung, bei Agenten die Marke und „Teilnehmer n“, linksbündig. Lese- und Tab-Reihenfolge bleiben Ort → Bedienung (nur Rasterplatz, keine Umordnung).
  - **Lange Kennung**: sie bricht in Zeile 2 um und wird nie gekürzt; braucht sie mehr als zwei Zeilen, wächst die Kopfzeile über 80 px (F2, product-owner 2026-10-29: die Kennung gewinnt). Verworfen: Kennung in einem Scrollkasten bei fester Höhe — der verdeckte Teil wäre gekürzt.
  - **Ab 480 px nach Inhalt zweizeilig** (entschieden, ADR-0027 Ergänzung Karte #27 UX, F3 = Option 1, `AI-Team/po-entscheidungen.md` Eintrag 112): passen Zone „Ort“ (Instanzname, `# name ▾`, Frist — ohne Beschreibung) und Bedienteil nicht nebeneinander, rutscht der Bedienteil linksbündig in Zeile 2; die Leiste ist dann 80 px hoch (40 + 40, Touch 44 + 36), der Instanzname bleibt. Die Beschreibung erzwingt nie den Umbruch, sie füllt nur den Rest. Verworfen: feste Grenze bei 720 px (hängt nicht am Inhalt), Messen in JS, Kennung im einzeiligen Raster umbrechen.
  - **Touch** (grober Zeiger): der Schalter `# name ▾` ist mindestens 44 × 44 px; ab 480 px ist die Leiste dann 44 px hoch, unter 480 px passt er in Zeile 1. Mit feinem Zeiger bleibt er 28 px. Verworfen: eine unsichtbare, größere Trefferfläche um den 28-px-Schalter — sie ragte über die Leiste und läge über der Hauptansicht.
