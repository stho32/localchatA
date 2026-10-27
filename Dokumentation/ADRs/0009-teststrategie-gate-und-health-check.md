# ADR-0009: Teststrategie, Gate und Health-Check

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #1 I0001)
**Kontext-Anforderung**: — (alle Interactions)
**Erprobt durch**: bestehende Systemantwort kanbanD (Q04: `tests/programm.rs`, `tests/gemeinsam`, `e2e/playwright.config.js`, `e2e/bauen.js`, `e2e/tests/hilfen.js`)

## Kontext

Das Gate in `AI-Team/umgebung.md` ist gesetzt bis auf den E2E-Befehl: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, Playwright gegen das echte Programm. Der Auftrag verlangt, dass Log-Ablage, Index, Live-Kanal und HTTP getrennt testbar sind und das Programm in Tests auf freiem Port mit Wegwerf-Ordner startet. Die Ports 7420 und 7421 (kanbanD) sind tabu.

## Optionen

### Option A: Drei Ebenen wie kanbanD — Unit im Modul, Integration in `tests/` gegen das Binärprogramm, Playwright in `e2e/` gegen das Binärprogramm; Port 0, Wegwerf-Ordner, Adresse aus der Startzeile
- Vorteile: erprobt; nie ein fester Port; kein geteilter Zustand zwischen Tests; E2E sieht genau, was ein Nutzer sieht.
- Nachteile: Integrationstests starten Prozesse (langsamer als In-Process).

### Option B: Integration in-process über `tower::ServiceExt::oneshot` gegen den Router
- Vorteile: schnell, kein Prozess.
- Nachteile: umgeht Start, Port, Signale, `ConnectInfo` (Herkunft!) und SSE-Verbindungsende — genau die Stellen mit Risiko (Lehre „Probe gegen den echten Einstieg“).

### Option C: Nur E2E plus Unit
- Vorteile: weniger Testcode.
- Nachteile: API-Verhalten für Agenten (Warten, Fehlercodes) nur über den Browser prüfbar; langsam und spröde.

## Entscheidung

**Gewählt: Option A**, In-Process-Tests (B) zusätzlich erlaubt, wo nur Routing und Serialisierung geprüft werden.

Festlegungen:

- **Unit** (`#[cfg(test)]` im Modul): Logformat, Verlaufsindex, Anwesenheit (Uhr-Attrappe), Live-Filter, Startauftrag — ohne Platte, ohne Netz. Provider (Uhr, Logablage) mit eigener Zweitimplementierung, keine Mock-Bibliothek.
- **Integration** (`tests/*.rs`): Hilfe `tests/gemeinsam/mod.rs` startet `env!("CARGO_BIN_EXE_localchatA") <tempdir> --port 0 --host 127.0.0.1 --kein-browser`, liest die Adresse aus der Startzeile, beendet den Prozess beim Drop; `reqwest` für JSON, SSE und Warten; Logablage gegen `tempfile`.
- **E2E** (`e2e/`, `@playwright/test` fest gepinnt): `globalSetup` baut (`cargo build -q`), jeder Test startet `target/debug/localchatA` in eigenem `mkdtemp`-Ordner auf Port 0, `TZ=Europe/Berlin`, `locale: de-DE`; Unit-Tests der reinen Browser-Funktionen unter `e2e/einheit/` mit `node --test`.
- **Health-Check**: `GET /api/gesundheit` → 200 `{"status":"bereit","ordner","nachrichten","log_bytes","ladezeit_ms","defekte_zeilen"}`. Weil erst nach dem Laden gebunden wird, gibt es keinen Zustand „lädt“; antwortet der Port, ist das Programm bereit.
- **Testknöpfe** nur als Umgebungsvariablen mit Präfix `LOCALCHATA_` (`HERZSCHLAG_MS`, `GNADENFRIST_MS`, `JETZT` für eine feste Startuhr); mindestens ein Test je Knopf über den echten Weg (Variable → Prozess).
- **Gate** (vor jedem Push): `cargo fmt --all -- --check` · `cargo clippy --all-targets --all-features -- -D warnings` · `cargo test` · `npm --prefix e2e ci` (einmalig) · `npm --prefix e2e test`.

## Konsequenzen

**Positiv:**
- Jede Komponente hat einen ersten Test ohne die anderen (je Baustein unter „Notizen / Quellen“ genannt).
- Der Director startet das Programm wie die Tests: freier Port, Adresse aus der Startzeile, Health-Check.

**Negativ / Inkauf zu nehmen:**
- E2E braucht Node.js und einen Chromium-Download (`npx --prefix e2e playwright install chromium`).

**Folgeentscheidungen / Aufgaben:**
- [ ] Gerüst `tests/gemeinsam/mod.rs`, `e2e/playwright.config.js`, `e2e/bauen.js`, `e2e/tests/hilfen.js` in I0001.
- [ ] Coverage-Messung (`cargo llvm-cov`) als Suchwerkzeug, nicht als Gate-Schwelle — Entscheidung beim Director.

## Quellen / Bezuege

- [A00002 Programm localchatA](../Architektur/A00002-programm-localchata.md), [A00001 localchatA](../Architektur/A00001-localchata.md) (Makroarchitektur); Aspekt [Testbarkeit](../Architektur/Aspekte/X00009-testbarkeit.md)
- kanbanD `tests/programm.rs`, `e2e/` (Q04, nur gelesen); `AI-Team/umgebung.md`; Factory `gemeinsam/gelerntes.md` „Probe gegen den echten Einstieg“, „Konfiguration über den echten Weg prüfen“

## Nachträge

Die Entscheidung oben bleibt unverändert; hier steht, was der Bau gezeigt hat.

- **2026-10-27 (Karte #2, Nachzug aus #1):** Gerüst steht (`tests/gemeinsam/mod.rs`, `e2e/playwright.config.js`, `e2e/bauen.js`, `e2e/tests/hilfen.js`); die Startzeile wird mit Zeitgrenze abgewartet (B0016). Seit [ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md) gibt es **zwei Server-Stacks**: das Programm liefert über die eigene Annahmeschleife aus, der In-Process-Testserver (`src/http/mod.rs`, `testserver`) noch über `axum::serve`. Damit gilt die Erlaubnis für In-Process-Tests (Option B) nur für Routing und Serialisierung; was Herkunft (`ConnectInfo`), Ströme, Zeitgrenzen oder Verbindungsgrenzen berührt, wird über das echte Programm geprüft (wie `tests/verfuegbarkeit.rs`), bis der Testserver über `ausliefern` läuft (Folgeaufgabe in ADR-0010). Weil die Bibliothek nur `start::ausfuehren` zeigt, laufen Tests einzelner Bausteine gegen `tempfile` als `#[cfg(test)]` im Modul, nicht unter `tests/`. Verfügbarkeitsgrenzen werden über das echte Programm geprüft (Deskriptorgrenze per `sh -c ulimit`, zweiter Absender `127.0.0.2`); der LAN-Test braucht eine Nicht-Loopback-IPv4 mit Default-Route (`AI-Team/umgebung.md`).
