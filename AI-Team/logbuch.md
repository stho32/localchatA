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
| 2026-10-28 | #5 I0006 | Entwicklung (Nacharbeit 1) | entwickler, architekt | bestanden | 0:25 | verlassen reserviert Platz bis Gnadenfrist (ADR-0018 2a), Start-Pflege getestet (34a97a5, 56bd997…0df1232) |
| 2026-10-28 | #5 I0006 | Reviews + Entwicklertest (Nacharbeit 1) | code-, security-reviewer, entwicklertester | befund | 0:30 | Security bestanden, Entwicklertest 15/17 (US-3/US-4 Messwert für die Abnahme); Code B1′ Untergrenze fehlt → Nacharbeit code-review-2 |
| 2026-10-28 | #5 I0006 | Messanleitung US-3/US-4 | entwicklertester | — | — | zweites Gerät über LAN als mia; WLAN aus/Deckel zu + Stoppuhr; anhalten, wenn mia nicht mehr online (erwartet ≈ 14 s, höchstens 22 s); nach 30 s WLAN an, ohne Neuladen wieder online; ≥ 3 Läufe, Werte in ADR-0017 |
| 2026-10-28 | #5 I0006 | Code-Review (Nacharbeit 2) | code-reviewer | bestanden | 0:10 | B1′ geschlossen (2022cc0) |
| 2026-10-28 | #5 I0006 | Abnahme | director | Zug | — | kein Remote; Demo aktualisiert; wartet auf Betreiber (Messwert US-3/US-4) |
| 2026-10-28 | #5 I0006 | Retrospektive | director | — | — | etwa 4 h, zwei kleine Nacharbeiten statt dreier großer: Regel „nur Muss“ und Zeitregeln aus #4 trugen (keine wackligen Zeittests). Neue Klasse: Tests, die nur eine Grenze prüfen (Obergrenze ohne Untergrenze) → Lehre: Wertprüfungen gegen eine Konfiguration immer als Spanne, mit Mutanten „zu groß“ und „zu klein“ |
| 2026-10-28 | #6 I0007 | Architektur | architekt | bestanden | 0:35 | ADR-0020–0024, A00012/A00013 (b4926fd) |
| 2026-10-28 | #6 I0007 | Bereit | planer | bestanden | 0:30 | R00006, 25 Bubbles in F0020–F0025, F0026 Ausbau (5c33c5b…a9df317) |
| 2026-10-28 | — | Prozess | Betreiber | — | — | Trommel-Puffer-Seil: Puffer 2 vor der Entwicklung, WIP 1 nur in Entwicklung; #7 sofort in die Architektur |
| 2026-10-28 | — | Prozess | Betreiber | — | — | Vorlauf (architekt, planer) im Worktree localchatA-vorlauf, Zweig vorlauf/<nr> je Karte, Übernahme per Rebase/Fast-Forward durch den Director |
| 2026-10-28 | #6 I0007 | Entwicklung | entwickler | bestanden | 0:59 | R00006 B0116–B0140 (c242a2a…8cc1f7d), WBS/Ist-Zeiten durch Director (b1076ca) |
| 2026-10-28 | #7 I0008 | Architektur | architekt | bestanden | 0:27 | ADR-0025–0028, Probe 100.000 Nachrichten (2e04da5, 5e2c2d1, 61011a4); Puffer: #7 Bereit in vorlauf/7, #12 Architektur in vorlauf/12 |
| 2026-10-28 | #7 I0008 | Bereit | planer | bestanden | 0:12 | R00007, 34 Bubbles, F0026 aktiviert (e1002a1, ebb13c7), aus vorlauf/7 auf main übernommen; Puffer 1 von 2 |
| 2026-10-28 | #12 I0018 | Architektur | architekt | bestanden | 0:28 | ADR-0029–0033 (2c8611e…99c6d49) aus vorlauf/12 übernommen |
| 2026-10-28 | — | Prozess | Betreiber | — | — | Spalte „Bereit“ heißt jetzt „Detailplanung“ (Board und prozess.md; Karten #7, #12 mitgezogen) |
| 2026-10-28 | #12 I0018 | Detailplanung | planer | bestanden | 0:20 | R00008, 27 Bubbles (aus vorlauf/12b übernommen); Puffer voll: #7, #12 |
| 2026-10-28 | #6 I0007 | Nacharbeit 1 | entwickler | bestanden | 0:25 | umformen höchstens 4×32 Zeichen, Akzente O(1); Fokus nach Fehlerantwort und Esc unter 720 px (d09e76d…0957bca) |
| 2026-10-28 | #7 I0008 | Entwicklung (Start) | entwickler | — | — | aus dem Puffer gezogen; Nachzug: #8 I0009 Architektur in vorlauf/8; #6 in Wiederholung (Security, UX) und Entwicklertest |
| 2026-10-28 | #6 I0007 | Wiederholung + Entwicklertest | ux-, security-reviewer, entwicklertester | bestanden | 0:40 | alle Phasen bestanden; Variante 2b und Anmerkungen → #18; Befundkarten #20 (Kopfzeile 320 px), #21 (JSON 2 MiB) |
| 2026-10-28 | #6 I0007 | Abnahme | director | Zug | — | Demo aus Commit 0957bca (Skript baut jetzt aus Git-Commits) |
| 2026-10-28 | #6 I0007 | Retrospektive | director | — | — | etwa 3 h, eine Nacharbeit (Muss aus Security und UX), parallel zum Vorlauf von #7/#12. Puffer hat getragen: der Engpass wartete nach der Nacharbeit nicht. Neu: zweiter Vorfall „Scratchpad geleert“ (Vorschlag); Befunde aus älteren Karten als eigene Befundkarten statt Nacharbeit |
| 2026-10-28 | #20, #21 | Detailplanung | planer | bestanden | 0:30 | R00009 (Kopfzeile 320 px), R00010 (Körpergrenze 4 KiB), F0039/F0040 (aus vorlauf/20 übernommen); kommen mit Vorrang nach #7 |
| 2026-10-28 | #8 I0009 | Architektur | architekt | bestanden | 0:55 | ADR-0034–0038, ADR-0010-Ergänzung, A00014 Schreibkonto; Berichtigung ADR-0028 R8 (aus vorlauf/8 übernommen) |
| 2026-10-28 | #8 I0009 | Detailplanung | planer | bestanden | 0:40 | R00011 (31 Bubbles); geteilt: I0020 mit R00012 als neue Karte, fertig geplant im Puffer |
| 2026-10-28 | #7 I0008 | Entwicklung | entwickler | bestanden | 1:26 | R00007 B0141–B0174 (388cbf6…0c1ff83); L1–L5 grün, M1 0,17 %, M2 47 ms; Puffer-Zug: #20 in Entwicklung |
| 2026-10-28 | #7 I0008 | Reviews | code-, ux-, security-reviewer | befund | 0:30 | Code B1 L3-Test-Timeout; UX S4 (Neuwahl nach Fehler) Muss; Security bestanden, A1 → Befundkarte #23; Nacharbeit nach #20 |
| 2026-10-28 | #20 Befund | Entwicklung | entwickler | bestanden | 0:20 | R00009 Kopfzeile umbrechen (9d7d4bb…f867195); jetzt Nacharbeit #7 |
| 2026-10-28 | — | Prozess | Betreiber | — | — | Spalten Code-, UX-, Security-Review → eine Spalte „Auto-Reviews“ mit Teilaufgabe je Rolle; Entwicklertest bleibt eigene Spalte; neue Rolle rechtschreib-reviewer (rsr) besetzt |
| 2026-10-28 | #7 I0008 | Nacharbeit 1 | entwickler | bestanden | 0:15 | L3 per Sprung, Neuwahl nach Fehler (d2d084c…baaf284); in Auto-Reviews |
| 2026-10-28 | #20 Befund | Auto-Reviews | code-, ux-reviewer | bestanden | 0:10 | Sollte → #18; weiter Entwicklertest |
| 2026-10-28 | #23 Befund | Architektur | architekt | bestanden | 0:16 | ADR-0039 Byte-Budget, Werkbank, Abrissgrenze als Sendefrist (4c61389) |
| 2026-10-28 | #20 Befund | Entwicklertest + Abnahme | entwicklertester, director | bestanden / Zug | 0:20 | 22 Fälle grün, Demo auf f867195 |
| 2026-10-28 | #23 Befund | Detailplanung | planer | bestanden | 0:15 | R00013, F0050 B0253–B0258; F0041 braucht F0050 (aus vorlauf/23b) |
| 2026-10-28 | — | Prozess | Betreiber | — | — | neue Rolle observability-reviewer (obs) in Auto-Reviews; Logging-Konzept ./logs täglich, 14 Tage; Interaction „Betriebsprotokoll“ mit Vorrang |
| 2026-10-28 | — | Lastprofil | Betreiber | — | — | „Team-LAN“: 20 Menschen + 50 Agenten, Spitze 30 Nachrichten/s, dauerhaft 5/s, 100.000 je Channel, 50 Channels; Zustellung < 1 s, Öffnen < 500 ms, Suche < 1 s |
| 2026-10-28 | #21 Befund | Entwicklung → Auto-Reviews | entwickler | bestanden | 0:30 | B0204, B0205 grün; Reviews code, security, rechtschreib, observability (erster Einsatz) parallel; UX entfällt |
| 2026-10-28 | #7 | Nacharbeit 2 | entwickler | läuft | — | nur Muss „1 Zeile“ aus dem Rechtschreib-Review |
| 2026-10-28 | — | Architektur Betriebsprotokoll | architekt | bestanden | 0:15 | ADR-0040–0043, A00015, eigenes Modul (tracing-appender per Probe verworfen), Abgleich Lastprofil: 3 Grenzen zu niedrig; Detailplanung läuft |
| 2026-10-28 | #7 | Nacharbeit 2 → Entwicklertest | entwickler | bestanden | 0:20 | „1 Zeile“; Auto-Reviews vollständig; Entwicklertest auf dc65faf läuft |
| 2026-10-28 | #23 Befund | Entwicklung | entwickler | läuft | — | F0050 B0253–B0258 (R00013) |
| 2026-10-28 | #21 Befund | Auto-Reviews | rechtschreib-reviewer, observability-reviewer, planer | befund → behoben / bestanden | 0:15 | R00010 Text korrigiert (172e376); Observability-Anbindung → Betriebsprotokoll |
| 2026-10-28 | #21 Befund | Auto-Reviews → Entwicklertest | code-reviewer, security-reviewer | bestanden | 0:25 | alle 5 Teilaufgaben ✓; Lesefrist für Körper → Paket „Grenzen nach Lastprofil“ |
| 2026-10-28 | #21 Befund | Entwicklertest → Abnahme | entwicklertester | bestanden | 0:15 | US-1–US-6 grün; Demo auf dc65faf |
| 2026-10-28 | — | Detailplanung Betriebsprotokoll, Grenzen | planer | bestanden | 0:20 | I0021 (R00014, 24 Bubbles), F0056 (R00015), F0057 (R00016); Karten #24, #25 (Vorrang) |
| 2026-10-28 | #7 | Entwicklertest → Abnahme | entwicklertester | bestanden | 0:45 | US-1–US-24 grün; Anmerkungen → #18 |
| 2026-10-28 | — | Architektur-Nachzüge | architekt | bestanden | 0:15 | ADR-0010/0012/0021/0039/0040/0042/0043/0030: Grenzen 160/128/128, Lesefrist 408, zu_langsam, I0018 bindet über Anbindungs-Feature an |
| 2026-10-28 | — | Tagestakt-Vertrag | architekt, planer | bestanden | 0:20 | ADR-0041 (a)–(e); R00014/R00008/R00017 nachgezogen; #24 baufertig |
| 2026-10-28 | #23 Befund | Entwicklung → Auto-Reviews | entwickler | bestanden | 0:45 | F0050 grün; K4 +795 MB → +8 MB; 5 Reviews parallel |
| 2026-10-28 | #24 | Entwicklung | entwickler | läuft | — | I0021 Betriebsprotokoll, 24 Bubbles (R00014) |
| 2026-10-28 | #24 | Entwicklung → Auto-Reviews | entwickler | bestanden | 1:25 | I0021 grün, 24 Bubbles, Gate 675/0 nachgefahren; UX entfällt (keine Oberfläche); 4 Reviews parallel |
| 2026-10-28 | #23 Befund | Auto-Reviews → Entwicklertest | entwickler, rechtschreib-reviewer | bestanden | 0:07 | Nacharbeit 1 (Gegenstelle IP+Port, Hilfetext Sendefrist); enge Wiederholung per grep-Probe |
| 2026-10-28 | #25 Befund | Detailplanung → Entwicklung | entwickler | läuft | — | F0056 Grenzen Team-LAN (R00015) |
| 2026-10-28 | #23 Befund | Entwicklertest → Abnahme | entwicklertester | bestanden | 0:13 | US-1–US-8 bestanden (+10 MB statt +760 MB); Demo auf 2e0004a |
| 2026-10-28 | #24 | Auto-Reviews | code-, observability-, rechtschreib-reviewer | befund | — | Muss: gelöschte Tagesdatei bleibt offen, 8 Einträge ohne Test, 2 Pluralfehler; security bestanden; architekt entscheidet M1 im Vorlauf |
| 2026-10-28 | #24 | Auto-Reviews | architekt, planer | bestanden | 0:18 | 13 Entscheidungen (ADR-0039–0043, 2c7c80e); Nacharbeit B0292–B0299 und R00014 nachgezogen; Härtung R00018 F0059–F0062 als Karte #26 in Detailplanung |
| 2026-10-28 | #24 | Auto-Reviews | Director | — | — | Betreiber: zweiter entwickler (entw2) für Nacharbeit im Worktree localchatA-entw2, Zweig nacharbeit/24 ab c2c50bc |
| 2026-10-28 | #24 | Auto-Reviews → Nacharbeit | entw2 | läuft | — | Betreiber: neue Spalte „Nacharbeit“ (entw2, WIP 1) zwischen Entwicklung und Auto-Reviews |
| 2026-10-28 | #25 Befund | Entwicklung → Auto-Reviews | entwickler | bestanden | 0:44 | F0056 grün, Gate 689/0 nachgefahren; UX entfällt; 4 Reviews parallel |
| 2026-10-28 | #12 | Detailplanung → Entwicklung | entwickler | läuft | — | I0018 Aufbewahrungsfrist (R00008, R00017), 29 Bubbles |
| 2026-10-28 | #25 Befund | Auto-Reviews → Entwicklertest | code-, security-, rechtschreib-, observability-reviewer | bestanden | 0:20 | kein Muss; Sollte/Optional an #26, #18 und architekt |
| 2026-10-28 | #24 | Nacharbeit → Auto-Reviews | entwickler (entw2) | bestanden | 0:40 | B0292–B0299 im Worktree, Rebase + Gate + Fast-Forward (261874d); Nacharbeitszeit 0,30 h gegen 1,25 h erste Entwicklung; enge Wiederholung 3 Reviews |
| 2026-10-28 | #12 | Entwicklung → Auto-Reviews | entwickler | bestanden | 0:47 | I0018 grün (30 Commits), Gate 778/0 und E2E 145/209 nachgefahren; 5 Reviews parallel |
| 2026-10-28 | #8 | Detailplanung → Entwicklung | entwickler | läuft | — | I0009 Nachricht senden (R00011, R00016), 33 Bubbles |
| 2026-10-28 | #24 | Auto-Reviews → Entwicklertest | code-, observability-, rechtschreib-reviewer | bestanden | 0:35 | enge Wiederholung: alle Muss geschlossen; A8 (Verdrahtung online/offline) → #26 |
| 2026-10-28 | #24 | Entwicklertest → Abnahme | entwicklertester | bestanden | 0:06 | US-1–US-17 bestanden; Demo d9c9755 |
| 2026-10-28 | #24 | Retrospektive | director | — | — | Arbeit gegen Nacharbeit: erste Entwicklung 1,25 h (24 Bubbles), Nacharbeit 0,30 h (8 Bubbles) — Ursache der Nacharbeit: Fertig-Kriterien ohne eigenen Test und ein Test, der am Folgetag statt am selben Tag prüfte (Vorschlag rollen/entwickler/ offen) |
| 2026-10-28 | #12 | Auto-Reviews | ux-, rechtschreib-reviewer | befund | — | Muss: Bestätigungssatz/Sperrhinweis für Bildschirmleser stumm; Pluralformen „entfallen sofort 1 Tag“, „1 Tagesdateien mit 1 Nachrichten“ (auch R00008); security und observability bestanden |
| 2026-10-28 | #12 | Auto-Reviews → Nacharbeit | planer, entwickler (entw2) | läuft | 0:11 | Nacharbeit B0315–B0318 und R00008 nachgezogen; Härtung R00019 als #27 (Detailplanung) und #28 (Architektur); entw2 baut auf nacharbeit/12 |
| 2026-10-28 | #12 | Nacharbeit → Auto-Reviews | entwickler (entw2) | bestanden | 0:14 | B0315–B0318, Rebase + Gate (849/0, E2E 168/227) + Fast-Forward 8c09b7c; Nacharbeit 0,25 h gegen 0,78 h erste Entwicklung; enge Wiederholung ux, rechtschreib, code |
| 2026-10-28 | #8 | Entwicklung → Auto-Reviews | entwickler | bestanden | 1:10 | I0009 grün (33 Bubbles), Gate gemeinsam mit Nacharbeit #12 nachgefahren; 5 Reviews parallel |
| 2026-10-28 | #22 | Detailplanung → Entwicklung | entwickler | läuft | — | I0020 Senden bei Störungen und mit Komfort (R00012) |
| 2026-10-28 | — | Vision | director | — | — | Betreiber: Archivieren ja; Vision fortgeschrieben (Leitplanke Team-LAN, Zielbild Archivieren, 5f046b4); I0022 als #29 in Architektur, sieben Fachfragen offen |
| 2026-10-28 | #12 | Auto-Reviews → Entwicklertest | ux-, rechtschreib-, code-reviewer | bestanden | 0:20 | enge Wiederholung: alle Muss geschlossen |
