# Umgebung

Wie die Dienste des Projekts starten und geprüft werden. Gilt für den Director und jede Rolle.

| Was | Befehl / Wert |
|---|---|
| Abhängigkeiten installieren | rustup stable ≥ 1.89 (Edition 2024, clippy, rustfmt); Node.js mit npm; einmalig `npm --prefix e2e ci` und `npx --prefix e2e playwright install chromium` (ADR-0001, ADR-0009) |
| Dienste starten (freier Port) | `cargo run -- <ordner> --port 0 --host 127.0.0.1 --kein-browser`; Adresse aus der Startzeile `localchatA läuft für <ordner> unter http://…/`; Standard ohne Angaben 0.0.0.0:7430 mit Ausweichen; nie 7420 oder 7421 (kanbanD) |
| Health-Check | `curl -fs <adresse>api/gesundheit` → 200 `{"status":"bereit",…}` |
| Gate (alle Tests, Analyse) | `cargo fmt --all -- --check` · `cargo clippy --all-targets --all-features -- -D warnings` · `cargo test` · `npm --prefix e2e test` (baut das Programm selbst) |
| SDKs / Pfade | `~/.cargo/bin`; kanbanD-Board `http://127.0.0.1:7421`, Ordner `/home/shoff/Projekte/localchatA-kanbanD`; Tests nur mit `--port 0`, Wegwerf-Ordner, Einstellungen über `LOCALCHATA_*` |
