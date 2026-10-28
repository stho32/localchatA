# Logbuch — Director

Eine Zeile je Auftrag an eine Rolle und je Retrospektive.

| Datum | Karte | Phase | Rolle | Ergebnis | Dauer | Notiz |
|---|---|---|---|---|---|---|
| 2026-10-27 | — | Einrichtung | director | Auftrag eingerichtet: 7 Rollen, 11 Spalten | — | /factory start |
| 2026-10-27 | #1 I0001 | Architektur | architekt | bestanden | 0:14 | Modell + ADR-0001…0009 (9733c55); Frage an Betreiber zu ADR-0005 offen |
| 2026-10-27 | #1 I0001 | Bereit | planer | bestanden | 0:04 | R00001, 13 Bubbles (73b60e8, dccaa5f) |
| 2026-10-27 | #1 I0001 | Entwicklung | entwickler | bestanden | 0:18 | R00001 gebaut, Gate grün nachgeprüft (51 + 1 E2E) |
| 2026-10-27 | #1 I0001 | Code-Review | code-reviewer | befund | 0:05 | B1 Signal-Fenster, B2 feste Ports in Tests (nachgeprobt); Nacharbeit 1 |
| 2026-10-27 | #1 I0001 | Entwicklung | entwickler | bestanden | 0:06 | Nacharbeit 1: B1, B2, A2 (ccdd4b5, 45b0cd4, e468ffd), Gate mit belegtem 7430 grün |
| 2026-10-27 | #1 I0001 | Code-Review | code-reviewer | bestanden | 0:05 | Wiederholung: B1, B2 behoben; Anmerkung Verdrahtung Bindeplan |
| 2026-10-27 | #1 I0001 | UX-Review | — | entfällt | — | leere Seite ohne Bedienung |
| 2026-10-27 | #1 I0001 | Entwicklertest | entwicklertester | bestanden | 0:20 | 9/10 bestanden, US-8 Und-Zeile nicht beobachtbar → Restrisiko, Betreiber sieht es in der Abnahme |
| 2026-10-27 | #1 I0001 | Security-Review | security-reviewer | befund | 0:05 | Slowloris ohne Zeit-/Verbindungsgrenze (nachgeprobt); Nacharbeit 1 Security |
| 2026-10-27 | #1 I0001 | Entwicklung | entwickler | bestanden | 0:11 | Nacharbeit Security: Verbindungsgrenzen (6812820), 69 Tests grün nachgeprüft |
| 2026-10-27 | #1 I0001 | Code-Review | code-reviewer | bestanden | 0:08 | 6812820 ohne Befund |
| 2026-10-27 | #1 I0001 | Security-Review | security-reviewer | bestanden | 0:12 | Wiederholung: Slowloris behoben, Rest Optional |
| 2026-10-27 | #1 I0001 | Entwicklertest | entwicklertester | bestanden | 0:05 | enge Wiederholung nach 6812820 |
| 2026-10-27 | #1 I0001 | Abnahme | director | Zug | — | kein Remote, nichts zu pushen; wartet auf Betreiber |
| 2026-10-27 | #1 I0001 | Retrospektive | director | — | — | Lehren: Projektschicht entwicklertester (Browser), gelerntes (Annahmeschleife, LAN-Fläche); Vorschläge Scratchpad, Playwright-Ordner |
| 2026-10-27 | #2 I0002 | Architektur | architekt | bestanden | 0:12 | ADR-0011 Rückbau/Wächter; Nachzug ADR-0010 Annahmeschleife (b536c1c, 688f270, 135de1b) |
| 2026-10-27 | #2 I0002 | Bereit | planer | bestanden | 0:07 | R00002, 14 Bubbles (3f98537, 919aed2) |
| 2026-10-27 | #2 I0002 | Entwicklung | entwickler | bestanden | 0:27 | R00002 gebaut, 156 Tests + e2e nachgeprüft |
| 2026-10-27 | #2 I0002 | Security-Review | security-reviewer | bestanden | 0:05 | parallel; Optional-Punkte an #8, #3, #18 |
| 2026-10-27 | #2 I0002 | Code-Review | code-reviewer | befund | 0:14 | B1 UTF-8 löscht ganzen Tag (nachgeprobt); Nacharbeit 1 |
| 2026-10-27 | #2 I0002 | Entwicklung | entwickler | bestanden | 0:11 | Nacharbeit 1: B1, A2 (3e07878, b74ebdd), 169 Tests nachgeprüft |
| 2026-10-27 | #2 I0002 | Code-Review | code-reviewer | bestanden | 0:03 | Wiederholung B1 behoben |
| 2026-10-27 | #2 I0002 | Entwicklertest | entwicklertester | bestanden | 0:15 | US-1–US-8, explorativ ohne Befund |
| 2026-10-27 | #2 I0002 | Security-Review | security-reviewer | bestanden | 0:04 | Rückschritt-Prüfung ohne Befund |
| 2026-10-27 | #2 I0002 | Retrospektive | director | — | — | Mutationsprobe mit touch → Vorschlag; Hinweise an #7, #8, #10, #18; drei Prüfungen parallel bei festem Code bewährt |
| 2026-10-27 | #3 I0004 | Architektur | architekt | bestanden | 0:17 | ADR-0012/0013/0014 (c5b3fca) |
| 2026-10-27 | #3 I0004 | Bereit | planer | bestanden | 0:12 | R00003, 28 Bubbles in 5 Slices (d6e35eb, 3e1168c) |
| 2026-10-28 | #3 I0004 | Entwicklung | entwickler | bestanden | 0:58 | R00003 gebaut, 283 + 25 + 15 Tests nachgeprüft |
| 2026-10-28 | #3 I0004 | Code-Review | code-reviewer | befund | 0:06 | B1 Eigen-409 bei gleichzeitigen Tabs |
| 2026-10-28 | #3 I0004 | UX-Review | ux-reviewer | befund | 0:07 | Fokus auf Beitreten unsichtbar; 7 Sollte |
| 2026-10-28 | #3 I0004 | Security-Review | security-reviewer | bestanden | 0:10 | parallel; Optional an #18 |
| 2026-10-28 | #3 I0004 | Entwicklung | entwickler | bestanden | 0:28 | Nacharbeit 1: B1 Tabs, UX-Fokus, Sollte, deutsche Fehler; 284/38/41 nachgeprüft |
| 2026-10-28 | #3 I0004 | Code-Review | code-reviewer | bestanden | 0:20 | Wiederholung B1 behoben |
| 2026-10-28 | #3 I0004 | UX-Review | ux-reviewer | bestanden | 0:03 | Wiederholung; 2 Sollte offen |
| 2026-10-28 | #3 I0004 | Entwicklertest | entwicklertester | bestanden | 0:07 | US-1–US-21; LAN-Konsolenfehler COOP als Befund gewertet → Nacharbeit 2 |
| 2026-10-28 | #3 I0004 | Entwicklung | entwickler | bestanden | 0:12 | Nacharbeit 2: COOP, 422-Echo, UX, Token; 288/39/55 nachgeprüft |
| 2026-10-28 | #3 I0004 | Reviews | code-, ux-, security-reviewer | bestanden | 0:15 | Nacharbeit 2 geprüft |
| 2026-10-28 | #3 I0004 | Abnahme | director | Zug | — | kein Remote; wartet auf Betreiber |
| 2026-10-28 | #3 I0004 | Retrospektive | director | — | — | zwei Nacharbeiten durch parallele Reviews früh sichtbar; UX-Review fand Muss, den Tests nicht sahen; Lehre: erste Oberfläche braucht UX-Maßstab im Entwicklerauftrag (Fokus auf Akzentflächen, 320 px, Fehlergestalt) |
| 2026-10-28 | #4 I0005 | Architektur | architekt | bestanden | 0:20 | ADR-0015/0016, Nachzug ADR-0013/0014 (986fdd0, 92dcd8a) |
| 2026-10-28 | #4 I0005 | Bereit | planer | bestanden | 0:10 | R00004, 24 Bubbles in 5 Slices (a6bb8d0) |
| 2026-10-28 | #4 I0005 | Entwicklung | entwickler | bestanden | 1:15 | R00004 B0075–B0098 (4b6b4fc…920ab49); Nacharbeit Gate-Stabilität 413-Abbruch (240dc98) |
| 2026-10-28 | #4 I0005 | Reviews | code-, ux-, security-reviewer | befund | 0:20 | Code: B1 stand nicht monoton, B2 Reihenfolge ohne Test; UX bestanden mit S1–S6 und Wiederverbindungsfehler; Security bestanden → Nacharbeit 1 |
| 2026-10-28 | #4 I0005 | Entwicklung (Nacharbeit 1) | entwickler | bestanden | 1:05 | B1/B2 Veröffentlichung unter Sperre, Wiederverbindung, S1–S6, A1/A3/A7/A8 (b3645cf…a58f711) |
| 2026-10-28 | #4 I0005 | Reviews + Entwicklertest | code-, ux-, security-reviewer, entwicklertester | befund | 0:30 | Reviews bestanden (Sollte N1–N3, Sperr-Tests); Entwicklertest US-4 Befund, Konsolen-401 nach Neustart → Nacharbeit 2 |
| 2026-10-28 | #4 I0005 | Entwicklung (Nacharbeit 2) | entwickler | bestanden | 0:45 | US-4 Pflege beim Abruf, Wiederbeitritt ohne Konsolenfehler, N1–N3, Untergrenze, Sperr-Tests (bab9647…3b5375b) |
| 2026-10-28 | #4 I0005 | Reviews + Entwicklertest (Nacharbeit 2) | alle vier | befund | 0:25 | UX, Security, Entwicklertest bestanden; Code-Review B1 Uhr vor Sperre, B2 Rückfall ohne Locks → Nacharbeit code-review-2 (Signal: Nacharbeitszeit ≥ Entwicklung, Timebox 1 h) |
| 2026-10-28 | — | Auftrag | director | — | — | Betreiber genehmigt realist-consultant mit `realist: anpassen` (Einengungen über den planer, gesammelt für die Abnahme) |
| 2026-10-28 | #4 I0005 | Beratung | testability-consultant, realist-consultant | befund | 0:40 | Ursache Pollraster und widersprüchliches Konsolenkriterium; Tests ereignisgenau (entwickler), Kriterien eingegrenzt (planer, realist: anpassen), Regeln für planer/entwickler |
| 2026-10-28 | #4 I0005 | Entwicklung (Nacharbeit 3) | entwickler, planer | bestanden | 1:30 | B1 Uhr unter Sperre, B2/A1/A2; Tests ereignisgenau, Kriterien eingegrenzt (f6e446f…cf3cf6e, abb7d8d…cdf516c) |
| 2026-10-28 | #4 I0005 | Code-Review + Entwicklertest (Nacharbeit 3) | code-reviewer, entwicklertester | bestanden | 0:20 | B1/B2/A1/A2 geschlossen, Zeittests ereignisgenau, Rückfall ohne „vergeben“ |
| 2026-10-28 | #4 I0005 | Abnahme | director | Zug | — | kein Remote; wartet auf Betreiber; Anpassungen (realist: anpassen) im Kommentar gesammelt |
| 2026-10-28 | #4 I0005 | Retrospektive | director | — | — | etwa 6 h, drei Nacharbeiten. Ursachen: (1) Director packte Sollte in jede Nacharbeit → jede Runde breiter, neue Rückschritte; (2) Zeitkriterien gegen Wanduhr und Pollraster gemessen; (3) Konsolen-Allquantor widersprach vorgeschriebenen 401 → Zielkonflikt, kein Fix konnte ihn brechen. Lehren: Nacharbeit nur Muss; ab zweiter Nacharbeit derselben Klasse testability-/realist-consultant vor dem nächsten Fix; Regeln für Zeit- und Konsolenkriterien in planer.md/entwickler.md |
| 2026-10-28 | #5 I0006 | Architektur | architekt | bestanden | 0:55 | ADR-0017 stiller Abriss (Vorgeschlagen, Messwert), 0018 verlassen, 0019 weg_um; Nachzüge (3915255, 9bc118c) |
| 2026-10-28 | #5 I0006 | Bereit | planer | bestanden | 0:20 | R00005, 17 Bubbles in F0016–F0019 (59ead76, 6a213d6) |
| 2026-10-28 | #5 I0006 | Entscheidung | Betreiber | — | — | „Verlassen“ im Browser erst auf Nachfrage der Abnahme; stiller Abriss ≈ 14 s, höchstens 22 s angenommen |
| 2026-10-28 | #19 I0019 | Backlog → Architektur | director | — | — | neue Interaction „Claude Code einbinden“ (82284c2), vom Betreiber freigegeben |
| 2026-10-28 | #5 I0006 | Entwicklung | entwickler | blockiert | 0:45 | 17 Bubbles gebaut; Statusspalte vs. R00004-Test → Betreiber: Liste 264 px |
| 2026-10-28 | — | Demo | director | — | — | Demo für den Betreiber: cargo install, Port 7431, Ordner localchatA-demo, Aktualisierung nach jedem Zug in die Abnahme |
| 2026-10-28 | #5 I0006 | Entwicklung | entwickler | bestanden | 0:55 | R00005 B0099–B0115, Liste 264 px (e9a4de3…2f4318f) |
| 2026-10-28 | #5 I0006 | Reviews | code-, ux-, security-reviewer | befund | 0:35 | UX bestanden; Code B1 Pflege im Start ungetestet; Security verlassen hebt Platzgrenze auf → Nacharbeit 1 (nur Muss), Sollte nach #18 |
