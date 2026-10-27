---
id: A00003
name: Start
code: start
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
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

## Offene Fragen

## Notizen / Quellen

- Reihenfolge: Aufruf lesen → Ordner vorbereiten → Sperre → Logs laden (Index vollständig) → binden → Startzeile. Damit ist der Port erst offen, wenn der Verlauf da ist; der Health-Check ist ab der Startzeile grün.
- Erster Test (Unit): `["/tmp/x", "--port", "0", "--kein-browser"]` → `Startauftrag { ordner: "/tmp/x", port: 0, host: "0.0.0.0", browser_oeffnen: false }`; kein Provider.
- Erster Test (Integration): Binärprogramm mit Wegwerf-Ordner und `--port 0` starten, Adresse aus der Startzeile, `GET /api/gesundheit` → 200; zweiter Start auf demselben Ordner endet mit Exit 1 und nennt die Sperre.
