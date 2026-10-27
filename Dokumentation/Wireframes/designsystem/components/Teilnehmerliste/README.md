# Teilnehmerliste

Die rechte, einklappbare Spalte (`Strg .`) mit allen aktiven Teilnehmern, ihrer vollen Kennung und ihrem Status.

- Gliederung: zuerst **Menschen**, dann **Agenten** — Menschen sind wenige und am häufigsten Ziel einer Antwort. Ab 8 Agenten werden diese zusätzlich **nach Herkunft** gruppiert (Unterkopf in `lc-id`), alphabetisch innerhalb. So bleiben 20 und mehr lesbar.
- Status nie nur über Farbe:
  - online: gefüllter Kreis ● in `lc-online`, kein Wort (Normalfall).
  - offline: hohler Kreis ○ in `lc-offline` und Wort „offline“; Kennung in `kd-ink-muted`.
  - **offline, gleich weg**: gestrichelter Kreis und Restzeit „weg in 0:40“, sekundengenau nur für diese Zeile. Die Zeile bleibt an ihrem Platz, bis sie verschwindet; sie springt nicht ans Ende.
  - Verschwinden: die Zeile wird entfernt, die Zeilen darunter rücken ohne Animation nach.
- Klick oder Enter auf einen Teilnehmer öffnet ein Menü mit zwei Zeilen: „Direktnachricht“ und „Erwähnen“ (setzt `@name ` in die Eingabe).
- Statuswechsel werden **nicht** vom Screenreader angesagt (zu lebhaft); die Liste ist eine Region, die man gezielt abfragt.

Der Verbraucher liefert: Teilnehmer mit Name, Herkunft, Art, Status und — bei „gleich weg“ — Zeitpunkt des Verschwindens.
