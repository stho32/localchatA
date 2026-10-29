# Prozess

Je Spalte des Boards: Art (`Team` bearbeitet der Director mit der Rolle, `Manuell` gehört dem Menschen), Rolle (nur bei Team), Phase der Phasen-Datei (falls die Spalte eine darstellt) und was in der Spalte getan wird. `python3 werkzeuge/board_aus_prozess.py <projekt>/AI-Team` schreibt die Spalten mit dieser Beschreibung als Notiz aufs Board.

| Spalte | Art | Rolle | Phase | Was hier getan wird |
|---|---|---|---|---|
| Backlog | Manuell | — | — | Sammlung; der Betreiber gibt frei, indem er Karten nach „Architektur“ legt |
| Architektur | Team | architekt | — | prüfen, ob die Karte eine Technikfrage aufwirft; wenn ja entscheiden und als ADR festhalten, sonst sofort weiter; die erste Karte bringt die Technikwahl |
| Detailplanung | Team | planer, ux-reviewer (Vorprüfung bei Oberfläche) | — | Interaction bis Bubble verfeinern, Anforderung schreiben; bei Oberfläche Vorprüfung der Anforderung durch den ux-reviewer, planer arbeitet die Kriterien ein (Betreiber 2026-10-29) |
| Entwicklung | Team | entwickler | Entwicklung | bauen, Test zuerst, Gate |
| Nacharbeit | Team | entwickler | — | Muss-Befunde aus Auto-Reviews und Entwicklertest beheben: der zweite entwickler (`entw2`) im Worktree `localchatA-entw2` auf Zweig `nacharbeit/<nr>`, WIP 1; danach führt der Director per Rebase, Gate und Fast-Forward zusammen und zieht die Karte zurück in die Auto-Reviews (nur Rollen mit Befund wiederholen) |
| Auto-Reviews | Team | code-reviewer, ux-reviewer, security-reviewer, rechtschreib-reviewer, observability-reviewer | — | alle Reviews parallel am selben Stand; je Rolle eine Teilaufgabe, abgehakt, wenn sie bestanden hat (UX und Rechtschreibung entfallen bei Karten ohne sichtbaren Text); Zug erst, wenn alle abgehakt sind |
| Entwicklertest | Team | entwicklertester | Entwicklertest | User Story im echten Programm, am reviewten Stand |
| Abnahme | Manuell | — | Abnahme | der Betreiber nimmt gegen die Akzeptanzkriterien ab |
| Erledigt | Manuell | — | — | abgenommen |
| Ohne Mensch nicht lösbar | Manuell | — | — | Schleifen und Entscheidungen, die ein Mensch treffen muss |

Schleifen an den Menschen: Ohne Mensch nicht lösbar

Fluss (Betreiber 2026-10-28, Trommel-Puffer-Seil): Der entwickler ist der Engpass. In „Entwicklung“ liegt höchstens eine Karte. Architektur und Detailplanung laufen voraus und halten einen Puffer von 2 Karten, die „Detailplanung“ bestanden haben. Nachgezogen wird, sobald der entwickler eine Karte aus dem Puffer nimmt. In den Puffer kommen nur Karten, deren `Braucht` mindestens in Entwicklung ist. WBS-Status der Bubbles, Ist-Zeiten und Logbuch schreiben Director bzw. planer, nicht der entwickler.

Vorlauf im eigenen Worktree (Betreiber 2026-10-28): architekt und planer arbeiten für Karten im Puffer in eigenen Worktrees (`/home/shoff/Projekte/localchatA-vorlauf` für die erste, `/home/shoff/Projekte/localchatA-vorlauf-<nr>` für weitere Karten im Puffer), je Karte auf einem kurzen Zweig `vorlauf/<nr>` ab dem aktuellen main, mit eigenem `CARGO_TARGET_DIR` für Proben. Nach jeder bestandenen Phase setzt der Director den Zweig per Rebase auf main, übernimmt ihn per Fast-Forward und löscht ihn. Abweichung von „alles direkt auf main“: kurzlebig und nur für Dokumente (ADRs, Architektur, WBS, Anforderungen). Der entwickler arbeitet weiter im Projektordner auf main.

Zweiter entwickler für Nacharbeit (Betreiber 2026-10-28): Nacharbeit an einer Karte in den Auto-Reviews baut ein zweiter entwickler (`entw2`) parallel zum ersten, im Worktree `/home/shoff/Projekte/localchatA-entw2` auf dem Zweig `nacharbeit/<nr>` ab dem aktuellen main, mit eigenem `CARGO_TARGET_DIR` (`~/.cache/lca-target-entw2`). Er committet je Bubble auf diesen Zweig. Zusammenführen macht der Director: Zweig per Rebase auf das aktuelle main setzen, im Worktree das volle Gate (fmt, clippy, cargo test, E2E) auf dem rebasten Stand fahren, dann Fast-Forward auf main und Zweig löschen. Konflikte löst der Director nicht selbst, sondern gibt sie an entw2 zurück. Die Reviews wiederholen nach dem Merge auf main. Neue Interactions baut weiterhin nur der erste entwickler auf main (WIP 1 in „Entwicklung“).
