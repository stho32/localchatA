# Umgebung

Wie die Dienste des Projekts starten und geprüft werden. Gilt für den Director und jede Rolle.

| Was | Befehl / Wert |
|---|---|
| Abhängigkeiten installieren | offen — Rust-Toolchain über `cargo`; der `architekt` legt Crates und Edition fest, beim ersten Lauf nachtragen |
| Dienste starten (freier Port) | offen — Aufruf laut Vision `localchatA <ordner>`; Port nach Skill `freier-port`, nie 7420 oder 7421 (kanbanD) |
| Health-Check | offen — beim ersten Lauf nachtragen |
| Gate (alle Tests, Analyse) | `cargo fmt --all -- --check` · `cargo clippy --all-targets --all-features -- -D warnings` · `cargo test` · Playwright-E2E gegen das echte Programm (Befehl beim ersten Lauf nachtragen) |
| SDKs / Pfade | `~/.cargo/bin`; kanbanD-Board `http://127.0.0.1:7421`, Ordner `/home/shoff/Projekte/localchatA-kanbanD` |
