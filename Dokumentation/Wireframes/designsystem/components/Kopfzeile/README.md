# Kopfzeile

Die dunkle 40-px-Leiste aus kanbanD mit drei Zonen: Ort · Navigation · Bedienung.

- **Ort** (links): Instanzname in `lc-id` und `kd-header-muted`, dann der aktuelle Channel in `lc-title`, danach die Channel-Beschreibung abgeschnitten mit Ellipse. In einer Direktnachricht steht statt `# name` die volle Kennung des Gegenübers und „Privat · nur ihr beide“.
- **Navigation** (Mitte): das Suchfeld, höchstens 420 px breit, `kd-header-surface`. Es ist ein Schalter, der die Suche (4.8) öffnet; Kürzel `Strg K` steht rechts darin.
- **Bedienung** (rechts): die eigene Kennung `name@herkunft` (Herkunft in `kd-header-muted`), der Schalter „Teilnehmer n“ mit `aria-pressed`, der Schalter `?` für die Tastenübersicht.
- Fokus in der Kopfzeile: `lc-focus-header`, nie `kd-focus` (zu wenig Kontrast auf dem dunklen Grund im hellen Modus).
- Schmales Fenster (< 720 px): Beschreibung entfällt, Suchfeld schrumpft auf das Symbol ⌕, die eigene Kennung bleibt sichtbar.

Der Verbraucher liefert: Instanzname, aktuellen Channel oder DM-Partner, eigene Kennung, Zahl der Teilnehmer.
