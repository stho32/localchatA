# Teilnehmerliste

Die rechte, einklappbare Spalte (`Strg .`) mit allen aktiven Teilnehmern, ihrer vollen Kennung und ihrem Status.

- Gliederung: zuerst **Menschen**, dann **Agenten** — Menschen sind wenige und am häufigsten Ziel einer Antwort. Ab 8 Agenten werden diese zusätzlich **nach Herkunft** gruppiert (Unterkopf in `lc-id`), alphabetisch innerhalb. So bleiben 20 und mehr lesbar.
- Status nie nur über Farbe:
  - online: gefüllter Kreis ● in `lc-online`, kein Wort (Normalfall).
  - offline: hohler Kreis ○ in `lc-offline` und Wort „offline“; Kennung in `kd-ink-muted`.
  - **offline, gleich weg**: gestrichelter Kreis und Restzeit „weg in 0:40“, sekundengenau nur für diese Zeile. Die Zeile bleibt an ihrem Platz, bis sie verschwindet; sie springt nicht ans Ende.
  - Verschwinden: die Zeile wird entfernt, die Zeilen darunter rücken ohne Animation nach.
- Klick oder Enter auf einen Teilnehmer öffnet ein Menü mit zwei Zeilen: „Direktnachricht“ und „Erwähnen“ (setzt `@name ` in die Eingabe).
  - **Gebaut** (Ergänzung 2026-10-30, Karte #13, R00050 F0178, UX-Vorprüfung V5, V6): jede **andere** Zeile trägt Kachel und Kennung in einem `button` mit `aria-haspopup="menu"` und `aria-expanded`; die **eigene** Zeile hat keinen Knopf und kein Menü (an sich selbst keine Direktnachricht). Die Liste ist **ein** Tab-Stopp (Roving-Tabindex): ↑/↓ wechseln die Zeile, Home/Ende springen zur ersten und letzten. Klick, Enter oder Leertaste öffnen `role="menu"` mit `menuitem` „Direktnachricht“ (die Zeile „Erwähnen“ ergänzt I0013); im Menü ↑/↓, Home/Ende; Escape schließt und gibt den Fokus an die Zeile zurück, Tab schließt und geht weiter, ein Klick daneben schließt. Die Auswahl öffnet `#/d/<kennung>`, schließt das Menü und unter 1100 px das Panel, der Fokus steht in der Eingabe. Eine Kennung frei einzutippen gibt es nicht.
  - Zeilenknopf und Menüzeile sind 28 px hoch, mit grobem Zeiger 44 px; die Zeile bleibt so hoch wie ihr Knopf.
  - Der Sekundentakt („weg in …“) erneuert nur Status und Form der Zeile — Knopf, Fokus und Menü bleiben; geht die Zeile bei offenem Menü, schließt es.
- Statuswechsel werden **nicht** vom Screenreader angesagt (zu lebhaft); die Liste ist eine Region, die man gezielt abfragt.

Der Verbraucher liefert: Teilnehmer mit Name, Herkunft, Art, Status und — bei „gleich weg“ — Zeitpunkt des Verschwindens.
