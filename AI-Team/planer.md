# planer — Projektschicht

Ergänzt die Rolle `planer` der Factory für dieses Projekt. Gilt für Projektgebundenes (Umgebung, Stack, Pfade, Befehle, Stil); Grenzen und Kernregeln der Rolle überschreibt sie nie (`gemeinsam/regeln.md`, „Vorrang“).

## Projektregeln

- Stack laut Vision: Rust, ein Programm für Weboberfläche und WebAPI; noch kein Code — Technik entscheidet der `architekt` (ADRs unter `Dokumentation/ADRs/`).
- Datenhaltung: täglich rotierende Logdateien je Channel, Frist je Channel (Standard 7 Tage); kein Git, keine Datenbank.
- Leistungsziel: 100.000 Nachrichten — Channel öffnet sofort, Verlauf scrollt flüssig, Suche ohne spürbare Wartezeit.
- Oberfläche deutsch; Designsystem `Dokumentation/Wireframes/designsystem/` (README, Tokens, Bausteine, offene Fragen).
- WBS `Dokumentation/Planung/localchata.md`, Phasen-Datei daneben; Interactions sind aus der Vision, noch ohne Anforderung.
- Offene Fragen des Designsystems (`3-offene-fragen.md`) beim Schreiben der passenden Anforderung klären lassen, nicht raten.

## Spezialisierungen

Aus der Factory (`rollen/planer/spezialisierungen/`), gewählt bei der Bestückung; fremde mit `planer/<name>`.


## Quellen

IDs aus `AI-Team/quellen.md`, die diese Rolle besonders braucht, mit Anlass.

- Q01 — Zielbild
- Q02 — WBS
- Q03 — offene Fragen des Designsystems
- Q05 — Briefing

## Gelerntes
- 2026-10-28 (#4, realist-consultant): Zeit- und Konsolenkriterien. (1) Jede Zeitgrenze nennt ihren Zweck aus Vision oder Story und den erwarteten Wert; sie liegt mindestens 30 % über dem gemessenen oder gerechneten Wert (Nachlauf + Takt + Browser), sonst ist sie zu knapp. (2) Regel und Zustellung getrennt: die Regel exakt über `seit` bzw. die gestellte Uhr, die Zustellung als obere Grenze; Untergrenzen nur über `seit`, nie über die Ankunftszeit. (3) Jedes Zeitkriterium nennt den Startpunkt der Uhr („nachdem das Schließen abgeschlossen ist“, „ab der 201“). (4) Fristen vor dem Schreiben gegeneinander rechnen: kein Prüfender läuft im Szenario selbst in die Gnadenfrist, kein Szenario verlangt einen Zustand, den eine offene Seite verhindert. (5) „Keine Konsolenfehler“ heißt: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer den Browser-Meldungen „Failed to load resource“ zu Antworten, die die Anforderung selbst vorschreibt oder die eine beendete Instanz verursacht — diese Antworten zählt das Kriterium auf; kein Allquantor („kein einziger“, „in allen Szenarien“) über Neustart, Abbruch oder Rückfall. (6) Ausnahmewege (Rückfall, Neustart, fremder Browser) bekommen Kriterien nach ihrem Nutzen (Liste stimmt, still wieder beigetreten), nicht die Strenge des Hauptwegs; Kombinationen mehrerer Ausnahmewege nur mit Begründung aus Vision oder Story. (7) Leistungsgrenzen stehen einmal in der Anforderung; der Test übernimmt sie, nie schärfer.
