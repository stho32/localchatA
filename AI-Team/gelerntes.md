# Gelerntes — projektgebunden

Lehren, die an dieses Projekt gebunden sind und mehrere Rollen betreffen. Lehren einer einzelnen Rolle stehen in `AI-Team/<rolle>.md`; allgemeine Lehren schlägt der Director für die Factory vor.
- 2026-10-27 (#1): Die Instanz bedient Verbindungen über eine eigene Annahmeschleife (src/start/ausliefern.rs, hyper http1) statt `axum::serve`: Kopf-/Leerlaufgrenze 10 s, 64 Verbindungen je Gegenstelle, Frist 5 s beim Ende. Wer die Herkunft braucht (ADR-0008, ab I0004), setzt `ConnectInfo` dort selbst; Ströme (SSE, Long-Poll) belegen Plätze und werden beim Ende nach 5 s abgebrochen. gilt für: entwickler, code-reviewer, security-reviewer, architekt, planer
- 2026-10-27 (#1): Der Listener auf 0.0.0.0 ohne Anmeldung macht jede neue Route zur LAN-Angriffsfläche — vor Markdown, Uploads und schreibenden Endpunkten Host-/Origin-Prüfung und Sicherheitsheader einplanen (Karte #18). gilt für: planer, security-reviewer, entwickler
