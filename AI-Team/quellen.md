# Quellen

Was die Rollen dieses Projekts lesen dürfen, wo es liegt und wie sie es nutzen. Angelegt bei der Bestückung (Factory: `rollen/director/bestueckung.md`), fortgeschrieben nur vom Director. Eine Rolle liest die Zeilen, in denen sie oder `alle` steht. Prüfen: `python3 werkzeuge/quellen.py pruefen <projekt>/AI-Team` (aus der Factory).

## Orte

- `repo:<pfad>` — im Projekt-Repo, relativ zur Wurzel, versioniert.
- `extern:<absoluter pfad>` — außerhalb des Repos auf diesem Rechner (E-Buch, Norm, vertrauliches Dokument); die Session braucht Lesezugriff (`claude --add-dir <ordner>` bzw. `/add-dir <ordner>`).
- `url:<adresse>` — im Netz; beim Zitieren mit Abrufdatum.
- `person:<rolle beim Auftraggeber>` — Auskunft nur über den Betreiber; Namen und Kontaktdaten stehen nicht hier.

## Nutzung

- `lesen` — lesen und in eigenen Worten verwenden, mit Fundstelle.
- `zitieren` — zusätzlich kurze wörtliche Zitate mit Fundstelle.
- `nur-verweis` — nennen, nicht öffnen (Zugang fehlt noch, Lizenz unklar).

Externe Quellen werden nie ins Repo kopiert, auch nicht in Auszügen über ein kurzes Zitat hinaus. Was eine Rolle daraus gewinnt, steht in eigenen Worten im Projekt (z. B. Domänenwissen) mit Quellen-ID und Fundstelle.

## Verzeichnis

| ID | Was | Ort | Nutzung | Ins Repo | Für Rollen |
|---|---|---|---|---|---|
| Q01 | Vision localchatA | repo:Anforderungen/R00000-vision.md | zitieren | ja | alle |
| Q02 | WBS localchatA mit Phasen-Datei | repo:Dokumentation/Planung/localchata.md | zitieren | ja | entwickler, code-reviewer, architekt, planer, entwicklertester |
| Q03 | Designsystem (Grundsätze, Tokens, Bausteine, Tastatur, offene Fragen) | repo:Dokumentation/Wireframes/designsystem/ | zitieren | ja | entwickler, planer, ux-reviewer |
| Q04 | kanbanD, Geschwisterprogramm — Vorbild für Aufbau, Live-Kanal, eingebettete Oberfläche | extern:/home/shoff/Projekte/kanbanD | lesen | nein — eigenes Repository, nur Vorbild | entwickler, code-reviewer, architekt |
| Q05 | Design-Auftrag (Briefing des Designsystems) | repo:design-prompt.md | lesen | ja | planer, ux-reviewer |

## Offen

- 2026-10-27 · security-reviewer · Vorgaben zum Schutzbedarf über die Vision hinaus fehlen · Betreiber
- 2026-10-27 · ux-reviewer · geforderte Stufe der Barrierefreiheit nur aus dem Designsystem (WCAG AA), keine weitere Vorgabe · Betreiber
