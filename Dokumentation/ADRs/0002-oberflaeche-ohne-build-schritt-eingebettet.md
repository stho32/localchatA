# ADR-0002: Oberfläche ohne Build-Schritt in das Programm eingebettet

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #1 I0001)
**Kontext-Anforderung**: — (WBS I0001)
**Erprobt durch**: bestehende Systemantwort kanbanD (Q04: `src/web/statisch.rs`, `web/`, `e2e/einheit/`)

## Kontext

Ein Programm soll die Oberfläche mit ausliefern (Leitplanke); nach dem Start darf nichts nachgeladen oder separat installiert werden, auch nicht im LAN ohne Internet. Das Designsystem (`Dokumentation/Wireframes/designsystem/`) liefert Tokens als CSS und Bausteine als HTML/CSS; es verlangt „billig je Nachricht“ und Blöcke zu 100. kanbanD bettet Vanilla-JavaScript, CSS und Schriften per `include_bytes!` ein, ohne Bundler; reine Funktionen der Oberfläche testet es in Node.

## Optionen

### Option A: Vanilla-JavaScript-Dateien ohne Build, per `include_bytes!` eingebettet (wie kanbanD)
- Vorteile: kein Node im Bau des Programms; eine Datei je Verantwortung; Unit-Tests der reinen Funktionen in Node; gleiche Machart wie kanbanD.
- Nachteile: keine Typprüfung; die Liste der Dateien steht im Rust-Code und muss gepflegt werden.

### Option B: Framework (Svelte/Preact) mit Bundler, Ergebnis per `rust-embed` eingebettet
- Vorteile: Komponentenmodell, Typen (TypeScript), Virtualisierung als Bibliothek.
- Nachteile: Node-Toolchain wird Bauvoraussetzung; zwei Builds; Abweichung vom Geschwister.

### Option C: Server rendert HTML (Vorlagen), wenig JavaScript (htmx-Stil)
- Vorteile: kaum Client-Code.
- Nachteile: Live-Verlauf, verankertes Nachladen nach oben und Tastaturbedienung brauchen ohnehin viel Client-Logik; jede Nachricht als HTML vom Server kostet mehr als JSON.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass das Programm ohne zweite Toolchain baut und sich wie kanbanD verhält. In Kauf genommen: keine Typprüfung im Client — ausgeglichen durch Node-Unit-Tests der reinen Funktionen und Playwright-E2E.

## Konsequenzen

**Positiv:**
- `cargo build` genügt für ein lauffähiges Programm; die Oberfläche ist immer passend zum Server.
- Tokens des Designsystems werden als `web/lc-tokens.css` übernommen.

**Negativ / Inkauf zu nehmen:**
- Markdown-Rendern im Client muss selbst gebaut und gegen Skript-Einschleusung gehärtet werden (Prüfpunkt `security-reviewer`).

**Folgeentscheidungen / Aufgaben:**
- [ ] Unbekannte Pfade unter `/web/` → 404; `Cache-Control: no-cache` wie kanbanD.

## Quellen / Bezuege

- [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md)
- kanbanD `src/web/statisch.rs`, `e2e/einheit/` (Q04, nur gelesen); Designsystem README „Leistung als Gestaltung“
