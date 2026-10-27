# security-reviewer — Projektschicht

Ergänzt die Rolle `security-reviewer` der Factory für dieses Projekt. Gilt für Projektgebundenes (Umgebung, Stack, Pfade, Befehle, Stil); Grenzen und Kernregeln der Rolle überschreibt sie nie (`gemeinsam/regeln.md`, „Vorrang“).

## Projektregeln

- Stack laut Vision: Rust, ein Programm für Weboberfläche und WebAPI; noch kein Code — Technik entscheidet der `architekt` (ADRs unter `Dokumentation/ADRs/`).
- Datenhaltung: täglich rotierende Logdateien je Channel, Frist je Channel (Standard 7 Tage); kein Git, keine Datenbank.
- Leistungsziel: 100.000 Nachrichten — Channel öffnet sofort, Verlauf scrollt flüssig, Suche ohne spürbare Wartezeit.
- Oberfläche deutsch; Designsystem `Dokumentation/Wireframes/designsystem/` (README, Tokens, Bausteine, offene Fragen).
- Angriffsfläche: LAN ohne Anmeldung, Datei-Uploads, Dateinamen und Channel-Namen im Log-Ordner (Pfad-Traversal), Markdown-Darstellung (XSS), Übernahme fremder Namen, Herkunft aus Headern fälschbar.

## Spezialisierungen

Aus der Factory (`rollen/security-reviewer/spezialisierungen/`), gewählt bei der Bestückung; fremde mit `security-reviewer/<name>`.


## Quellen

IDs aus `AI-Team/quellen.md`, die diese Rolle besonders braucht, mit Anlass.

- Q01 — Rahmen: LAN, keine Anmeldung

## Gelerntes
