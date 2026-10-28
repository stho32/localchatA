# Umgebung

Wie die Dienste des Projekts starten und geprüft werden. Gilt für den Director und jede Rolle.

| Was | Befehl / Wert |
|---|---|
| Abhängigkeiten installieren | rustup stable ≥ 1.89 (Edition 2024, clippy, rustfmt); Node.js mit npm; einmalig `npm --prefix e2e ci` und `npx --prefix e2e playwright install chromium` (ADR-0001, ADR-0009) |
| Dienste starten (freier Port) | `cargo run -- <ordner> --port 0 --host 127.0.0.1 --kein-browser`; Adresse aus der Startzeile `localchatA läuft für <ordner> unter http://…/`; Standard ohne Angaben 0.0.0.0:7430 mit Ausweichen; nie 7420 oder 7421 (kanbanD) |
| Health-Check | `curl -fs <adresse>api/gesundheit` → 200 `{"status":"bereit",…}` |
| Gate (alle Tests, Analyse) | `cargo fmt --all -- --check` · `cargo clippy --all-targets --all-features -- -D warnings` · `cargo test` · `npm --prefix e2e test` (erst `node --test einheit/` für die Oberfläche, dann Playwright; baut das Programm selbst) |
| Demo (Betreiber) | `http://127.0.0.1:7431/`, Ordner `/home/shoff/Projekte/localchatA-demo/daten`, über `~/.cargo/bin/localchatA` (cargo install); aktualisiert nur der Director mit `/home/shoff/Projekte/localchatA-demo/demo-aktualisieren.sh` nach jedem Zug in die Abnahme und auf Wunsch — Rollen fassen Port 7431 und den Demo-Ordner nie an, auch nicht zum Testen |
| SDKs / Pfade | `~/.cargo/bin`; kanbanD-Board `http://127.0.0.1:7421`, Ordner `/home/shoff/Projekte/localchatA-kanbanD`; Tests nur mit `--port 0`, Wegwerf-Ordner, Einstellungen über `LOCALCHATA_*` (Testuhr `LOCALCHATA_JETZT=<RFC 3339>`, Zeitzone in Tests `TZ=Europe/Berlin`, Gnadenfrist `LOCALCHATA_GNADENFRIST_MS` ab I0004, Herzschlag des Ereignisstroms `LOCALCHATA_HERZSCHLAG_MS` ab I0005, Standard 5000 ab I0006, im E2E nie über 5000 wegen des Wächters von 15 s — 0 ist Fehler, Exit 2); zusätzliche Hostnamen für den Grenzschutz mit `--erreichbar-als <name>` (ADR-0014); der LAN-Test braucht eine Nicht-Loopback-IPv4 mit Default-Route |
