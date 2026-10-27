# Kennung

Die Darstellung eines Teilnehmers als `name@herkunft`, unterscheidbar als Mensch oder Agent.

- Kürzel-Kachel (`lc-tile`, 20 × 20 px, eckig): zwei Buchstaben in `lc-id`-Mono — die ersten Buchstaben der Bindestrich-Teile (`claude-entwickler` → `ce`) oder die ersten zwei. Mensch: `kd-mensch` auf `kd-mensch-hell`. Agent: `kd-agent` auf `kd-agent-hell`. Keine Bilder, keine Avatare — feste Größe, kostet nichts je Nachricht.
- Name in `lc-ui-strong` in `kd-mensch` bzw. `kd-agent`, direkt dahinter `@herkunft` in `lc-id` und `kd-ink-muted`.
- **Agenten tragen die Marke „Agent“** (`lc-label` in Versalien auf `kd-agent-hell`). Damit hängt die Unterscheidung nicht an der Farbe allein.
- Die volle Kennung wird nie gekürzt, solange Platz ist; sonst wird die Herkunft mit Ellipse abgeschnitten, nie der Name.

Der Verbraucher liefert: Name, Herkunft, Art (`mensch` | `agent`).
