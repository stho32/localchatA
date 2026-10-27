---
id: A00003
name: Start
code: start
ebene: Komponente
eltern: A00002
repo: localchatA
status: Aktiv
herkunft: dialog
quellen:
  - src/main.rs
  - src/start/
verwandt: [A00006]
domaene: []
blackbox: kleine Integration ohne eigenes Risiko; folgt dem Vorbild kanbanD src/start/
---

# Start

## Verantwortung

Bringt das Programm vom Aufruf bis zum lauschenden Server in einen bereiten Zustand.

## Schnittstellen

- Eingehend: Kommandozeile (`Startauftrag::aus_argumenten`, rein und unit-testbar), Signale SIGTERM/Strg+C.
- Ausgehend: Ordner anlegen und sperren (`<ordner>/.localchatA.sperre`, `std::fs::File::try_lock`), [Gespräche](A00006-gespraeche.md) laden lassen, Frist-Takt starten (Tageswechsel aus der [Uhr](Aspekte/X00007-zeit.md)), Listener binden, Startzeile schreiben, Browser öffnen.

## Abhaengigkeiten

[Gespräche](A00006-gespraeche.md), [HTTP-Schnittstelle](A00004-http-schnittstelle.md), [Anwesenheit](A00010-anwesenheit.md), [Live-Kanal](A00011-live-kanal.md) — Start verdrahtet sie.

## Entscheidungen

- [ADR-0010: Eigene Annahmeschleife mit hyper http1 statt `axum::serve`](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md) — Akzeptiert, 2026-10-27

## Offene Fragen

- Start hält den Server auch bis zum geordneten Ende (`src/start/ausliefern.rs`, `src/start/verbindungen.rs`), der Verantwortungssatz beschreibt nur den Weg bis zur Bereitschaft. Empfehlung: Satz bei der nächsten Verfeinerung auf „vom Aufruf bis zum geordneten Ende“ erweitern — die Annahmeschleife bleibt im Start, weil sie Lebenszyklus (Signal, Frist) ist, nicht Übersetzung von Anfragen ([A00004](A00004-http-schnittstelle.md)). Gegenposition: Verbindungsgrenzen sind Systemgrenze und gehörten zu A00004.

## Notizen / Quellen

- Reihenfolge: Aufruf lesen → Ordner vorbereiten → Sperre → Logs laden (Index vollständig) → binden → Startzeile. Damit ist der Port erst offen, wenn der Verlauf da ist; der Health-Check ist ab der Startzeile grün.
- Erster Test (Unit): `["/tmp/x", "--port", "0", "--kein-browser"]` → `Startauftrag { ordner: "/tmp/x", port: 0, host: "0.0.0.0", browser_oeffnen: false }`; kein Provider.
- Erster Test (Integration): Binärprogramm mit Wegwerf-Ordner und `--port 0` starten, Adresse aus der Startzeile, `GET /api/gesundheit` → 200; zweiter Start auf demselben Ordner endet mit Exit 1 und nennt die Sperre.
- **Ist 2026-10-27 (nach I0001, Karte #2 Nachzug):** Reihenfolge im Code (`src/start/mod.rs`, `starten`): Aufruf lesen → Ordner vorbereiten → sperren → binden → **Signale einrichten** (Commit ccdd4b5, B0014: vor der Startzeile, damit ein Signal ab der Startzeile geordnet beendet) → Startzeile → Browser → ausliefern bis zum Signal über die eigene Annahmeschleife ([ADR-0010](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md), Commit 6812820). „Logs laden“ kommt mit I0002 zwischen Sperre und Binden hinzu. Ausweichregel des Standard-Ports ist rein geprüft (B0015), kein Test bindet 7430–7479. Erster Test der Grenzen: `tests/verfuegbarkeit.rs` (Angriffsmatrix Zeilen 1–6 über das echte Programm).
