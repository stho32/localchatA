# Besetzung

Rollen dieses Projekts aus der Kollektion der Factory (`/factory rollen`). Das Kürzel ist die Person der Rolle auf dem kanbanD-Board (1–8 Kleinbuchstaben oder Ziffern).

| Rolle | Kürzel | Einsatz |
|---|---|---|
| product-owner | po | Rückfragen der Rollen im Sinne der Vision entscheiden (Referenzprodukte erlaubt), Protokoll `AI-Team/po-entscheidungen.md`; dem Betreiber nur Richtungsweisendes; Rechenschaft und Kurskorrektur auf Ansage (Betreiber 2026-10-29) |
| planer | plan | Spalte Bereit: Interaction bis Bubble verfeinern, Anforderung über `/anforderung aus-slice`; Befunde einplanen |
| architekt | arch | Technikwahl vor dem ersten Bau (Rust-Stack, Ablage der rotierenden Logs, Live-Kanal, Suche bei 100.000 Nachrichten) als Architektur-Modell und ADRs; Umbauten |
| entwickler | entw | Interactions bauen, Test zuerst, Gate; Befundkarten |
| entwickler (zweiter) | entw2 | Nacharbeit aus den Auto-Reviews (Vorrang) und unabhängige Karten aus dem Puffer im Worktree `localchatA-entw2`, Zweig `nacharbeit/<nr>` bzw. `karte/<nr>` (Betreiber 2026-10-29) |
| entwickler (klein) | entw3 | nur kleine Härtungs- und Vorrang-Karten (≤ ≈ 5 Bausteine, keine Datei einer laufenden Karte), WIP 1, Worktree `localchatA-entw3`, Zweig `karte/<nr>`, kleineres Modell, Builds mit `nice`, je Commit betroffene Testdateien, volle Suite am Kartenende, wartet bei Last > 6 (Betreiber 2026-10-29) |
| code-reviewer | crev | Korrektheit, Tests, Wartbarkeit je Arbeitspaket |
| ux-reviewer | ux | Oberfläche gegen Designsystem, Tastaturbedienung, Barrierefreiheit, im Browser |
| entwicklertester | etst | User Story wörtlich im echten Programm, Browser und API |
| security-reviewer | srev | LAN ohne Anmeldung: Uploads, Pfade im Log-Ordner, Markdown-Darstellung (XSS), Namensübernahme |
| rechtschreib-reviewer | rsr | Rechtschreibung, Grammatik und Begriffstreue der Texte in Oberfläche, Fehlermeldungen, Hilfe und Anforderungen |
| observability-reviewer | obs | Betriebsprotokoll in ./logs (täglich, 14 Tage), Stufen, Fehlerpfade mit Spur, keine Tokens im Log, Gesundheit |

Beratung über `/factory fragen`: `datenschutz-spezialist` (Chatinhalte mit IP-Adressen). Vorgeschlagen für die Factory: Rolle `performance-reviewer` für das Leistungsziel.
