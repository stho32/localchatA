# Besetzung

Rollen dieses Projekts aus der Kollektion der Factory (`/factory rollen`). Das Kürzel ist die Person der Rolle auf dem kanbanD-Board (1–8 Kleinbuchstaben oder Ziffern).

| Rolle | Kürzel | Einsatz |
|---|---|---|
| planer | plan | Spalte Bereit: Interaction bis Bubble verfeinern, Anforderung über `/anforderung aus-slice`; Befunde einplanen |
| architekt | arch | Technikwahl vor dem ersten Bau (Rust-Stack, Ablage der rotierenden Logs, Live-Kanal, Suche bei 100.000 Nachrichten) als Architektur-Modell und ADRs; Umbauten |
| entwickler | entw | Interactions bauen, Test zuerst, Gate; Befundkarten |
| code-reviewer | crev | Korrektheit, Tests, Wartbarkeit je Arbeitspaket |
| ux-reviewer | ux | Oberfläche gegen Designsystem, Tastaturbedienung, Barrierefreiheit, im Browser |
| entwicklertester | etst | User Story wörtlich im echten Programm, Browser und API |
| security-reviewer | srev | LAN ohne Anmeldung: Uploads, Pfade im Log-Ordner, Markdown-Darstellung (XSS), Namensübernahme |

Beratung über `/factory fragen`: `datenschutz-spezialist` (Chatinhalte mit IP-Adressen). Vorgeschlagen für die Factory: Rolle `performance-reviewer` für das Leistungsziel.
