# Prozess

Je Spalte des Boards: Art (`Team` bearbeitet der Director mit der Rolle, `Manuell` gehört dem Menschen), Rolle (nur bei Team), Phase der Phasen-Datei (falls die Spalte eine darstellt) und was in der Spalte getan wird. `python3 werkzeuge/board_aus_prozess.py <projekt>/AI-Team` schreibt die Spalten mit dieser Beschreibung als Notiz aufs Board.

| Spalte | Art | Rolle | Phase | Was hier getan wird |
|---|---|---|---|---|
| Backlog | Manuell | — | — | Sammlung; der Betreiber gibt frei, indem er Karten nach „Architektur“ legt |
| Architektur | Team | architekt | — | prüfen, ob die Karte eine Technikfrage aufwirft; wenn ja entscheiden und als ADR festhalten, sonst sofort weiter; die erste Karte bringt die Technikwahl |
| Detailplanung | Team | planer | — | Interaction bis Bubble verfeinern, Anforderung schreiben |
| Entwicklung | Team | entwickler | Entwicklung | bauen, Test zuerst, Gate |
| Auto-Reviews | Team | code-reviewer, ux-reviewer, security-reviewer, rechtschreib-reviewer, observability-reviewer | — | alle Reviews parallel am selben Stand; je Rolle eine Teilaufgabe, abgehakt, wenn sie bestanden hat (UX und Rechtschreibung entfallen bei Karten ohne sichtbaren Text); Zug erst, wenn alle abgehakt sind |
| Entwicklertest | Team | entwicklertester | Entwicklertest | User Story im echten Programm, am reviewten Stand |
| Abnahme | Manuell | — | Abnahme | der Betreiber nimmt gegen die Akzeptanzkriterien ab |
| Erledigt | Manuell | — | — | abgenommen |
| Ohne Mensch nicht lösbar | Manuell | — | — | Schleifen und Entscheidungen, die ein Mensch treffen muss |

Schleifen an den Menschen: Ohne Mensch nicht lösbar

Fluss (Betreiber 2026-10-28, Trommel-Puffer-Seil): Der entwickler ist der Engpass. In „Entwicklung“ liegt höchstens eine Karte. Architektur und Detailplanung laufen voraus und halten einen Puffer von 2 Karten, die „Detailplanung“ bestanden haben. Nachgezogen wird, sobald der entwickler eine Karte aus dem Puffer nimmt. In den Puffer kommen nur Karten, deren `Braucht` mindestens in Entwicklung ist. WBS-Status der Bubbles, Ist-Zeiten und Logbuch schreiben Director bzw. planer, nicht der entwickler.

Vorlauf im eigenen Worktree (Betreiber 2026-10-28): architekt und planer arbeiten für Karten im Puffer in eigenen Worktrees (`/home/shoff/Projekte/localchatA-vorlauf` für die erste, `/home/shoff/Projekte/localchatA-vorlauf-<nr>` für weitere Karten im Puffer), je Karte auf einem kurzen Zweig `vorlauf/<nr>` ab dem aktuellen main, mit eigenem `CARGO_TARGET_DIR` für Proben. Nach jeder bestandenen Phase setzt der Director den Zweig per Rebase auf main, übernimmt ihn per Fast-Forward und löscht ihn. Abweichung von „alles direkt auf main“: kurzlebig und nur für Dokumente (ADRs, Architektur, WBS, Anforderungen). Der entwickler arbeitet weiter im Projektordner auf main.
