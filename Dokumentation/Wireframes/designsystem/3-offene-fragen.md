# Offene Fragen

Nicht entschieden, nicht geraten — bitte beantworten.

1. **kd-tokens.css abgleichen.** Die kanbanD-Werte (Farben, Namen wie `kd-mensch-hell`) sind aus der Beschreibung im Auftrag abgeleitet, die Datei selbst lag nicht vor. Sobald sie da ist, werden alle `kd-`-Werte und -Namen exakt übernommen und die Kontraste neu geprüft.
2. **Icon-Set.** Hat kanbanD eines? Bis dahin Wörter und Unicode-Zeichen.
3. **Woran erkennt das System einen Agenten?** Eigene Angabe beim Beitreten über die API, oder gilt jeder API-Teilnehmer als Agent und jeder Browser-Tab als Mensch? Davon hängt die Marke „Agent“ ab.
4. **Gruppierungsfenster.** 5 Minuten für Folgenachrichten — passt das zu Agenten, die im Sekundentakt schreiben, oder lieber nach Anzahl (höchstens 10 in Folge)?
5. **Befund-Stufen.** Sollen Agenten eine Konvention wie `[HOCH]` / `[MITTEL]` / `[NIEDRIG]` nutzen, die als Marke dargestellt wird — oder bleibt Markdown rein?
6. **Antworten.** Einfacher Zitatbezug (wie entworfen) oder echte Threads? Die Vision nennt beides nicht.
7. **Ungelesen über Tabs hinweg.** Gilt „gelesen“ je Browser-Tab oder je Name? Bei Namen, die beim Schließen frei werden, gibt es keinen dauerhaften Leser.
8. **Name zurückgewinnen.** Ein zurückkehrender Tab findet seinen Namen wieder — was, wenn ihn in der Minute dazwischen jemand anderes genommen hat?
9. **Dateigröße.** Die 25 MB im Entwurf sind ein Platzhalter; welche Grenze gilt?
10. **Wer sieht welche Channels?** Alle sehen alle, oder gibt es Beitritt/Verlassen? Ohne Rechte vermutlich alle — dann braucht die Seitenleiste ein Ausblenden.
    **Antwort (2026-10-28, Director nach Empfehlung, R00006):** Alle sehen alle Channels; es gibt kein Beitreten oder Verlassen eines Channels. Ein Ausblenden je Browser ist nicht geplant — käme es, wäre es eine eigene Interaction (ADR-0021, ADR-0027 Regel 8).
