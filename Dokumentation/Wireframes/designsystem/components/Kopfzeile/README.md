# Kopfzeile

Die dunkle 40-px-Leiste aus kanbanD mit drei Zonen: Ort · Navigation · Bedienung.

- **Ort** (links): Instanzname in `lc-id` und `kd-header-muted`, dann der aktuelle Channel in `lc-title`, danach die Channel-Beschreibung abgeschnitten mit Ellipse. In einer Direktnachricht steht statt `# name` die volle Kennung des Gegenübers und „Privat · nur ihr beide“.
- **Navigation** (Mitte): das Suchfeld, höchstens 420 px breit, `kd-header-surface`. Es ist ein Schalter, der die Suche (4.8) öffnet; Kürzel `Strg K` steht rechts darin.
- **Bedienung** (rechts): die eigene Kennung `name@herkunft` (Herkunft in `kd-header-muted`), der Schalter „Teilnehmer n“ mit `aria-pressed`, der Schalter `?` für die Tastenübersicht.
- Fokus in der Kopfzeile: `lc-focus-header`, nie `kd-focus` (zu wenig Kontrast auf dem dunklen Grund im hellen Modus).
- Schmales Fenster (< 720 px): Beschreibung entfällt, Suchfeld schrumpft auf das Symbol ⌕, die eigene Kennung bleibt sichtbar.

- **Channel-Einstellungen** (Ergänzung 2026-10-28, ADR-0033): `# name` ist ein Schalter `# name ▾` (`▾` in `kd-header-muted`, Hover `kd-header-surface`, Fokus `lc-focus-header`), der den Dialog [ChannelEinstellungen](../ChannelEinstellungen/README.md) öffnet. Ab 720 px folgt `· 7 Tage` (Aufbewahrungsfrist, `lc-small`, `kd-header-muted`, Singular „1 Tag“) **vor** der Beschreibung, damit die Ellipse nur die Beschreibung kürzt; bei defekter Einstellungsdatei `· ⚠ Frist gesperrt`. Unter 720 px entfallen Frist-Text und Beschreibung gemeinsam, der Schalter bleibt.

Der Verbraucher liefert: Instanzname, aktuellen Channel oder DM-Partner (mit Frist), eigene Kennung, Zahl der Teilnehmer.
