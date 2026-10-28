---
id: A00015
name: Betriebsprotokoll
code: protokoll
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/
verwandt: [A00003, A00004, A00008]
domaene: []
---

# Betriebsprotokoll

## Verantwortung

Schreibt jeden Betriebseintrag als eine escapte Zeile mit Zeit, Stufe, Bereich und Kontext in die Protokolldatei des lokalen Tages unter `<ordner>/logs/`.

## Schnittstellen

- `Protokoll::oeffnen(ordner, uhr, stufe, budgets) -> Protokoll` — legt `logs/` an, scheitert nie (Rückfall auf stderr, [ADR-0040](../ADRs/0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) Regel 8); aufgerufen vom [Start](A00003-start.md) nach der Sperre.
- `eintragen(stufe, bereich, text, kontext)` — Kontext nur über die Erlaubnisliste der Schlüssel (ADR-0040 Regel 3); Stufe unter der eingestellten → nichts; ab Warning zusätzlich stderr.
- `episode(art, quelle, …)` — erste Zeile, Zählen, Zwischenzeile je 10 min, Ende nach 60 s Ruhe ([ADR-0042](../ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regel 5).
- `exception(bereich, text, kontext)` — verletzte Annahme ohne Panik; der Panik-Haken nutzt denselben Weg mit `try_lock`.
- `bereinigen(heute: NaiveDate) -> Bereinigung` und `episoden_schliessen(jetzt)` — vom Tagestakt aufgerufen ([ADR-0041](../ADRs/0041-betriebsprotokoll-als-eigenes-modul-rotation-beim-schreiben-bereinigung-14-tage.md) Regel 6).
- `kennzahlen() -> Protokollkennzahlen` — für `/api/gesundheit` ([ADR-0043](../ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md) Regel 5).
- Rein: `zeile(&Eintrag, Zeit) -> String` (Formatierer mit Escape), `entfaellt(datei_tag, heute) -> bool` (14 Kalendertage, heute zählt mit).

## Abhaengigkeiten

Uhr-Provider ([Zeit](Aspekte/X00007-zeit.md)); Dateisystem unter `<ordner>/logs/` (eigenes Verzeichnis, nicht Teil der [Logablage](A00008-logablage.md)). Aufrufer: [Start](A00003-start.md), [HTTP-Schnittstelle](A00004-http-schnittstelle.md), [Gespräche](A00006-gespraeche.md), [Anwesenheit](A00010-anwesenheit.md), [Schreibkonto](A00014-schreibkonto.md) — jeweils über ein `Arc<Protokoll>` im Zustand bzw. als Provider, nie über eine globale Variable (außer dem Panik-Haken, der eine Kopie des `Arc` hält).

## Entscheidungen

- [ADR-0040: Betriebsprotokoll — eine Datei je lokalem Tag unter `logs/`, eine Zeile je Eintrag mit Zeit, Stufe, Bereich, Text und Kontext; stderr spiegelt ab Warning](../ADRs/0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) — Akzeptiert, 2026-10-28
- [ADR-0041: Betriebsprotokoll als eigenes Modul ohne neue Crate — Rotation beim Schreiben nach Eintragsdatum, Bereinigung nach 14 Kalendertagen beim Start und beim Tageswechsel](../ADRs/0041-betriebsprotokoll-als-eigenes-modul-rotation-beim-schreiben-bereinigung-14-tage.md) — Akzeptiert, 2026-10-28
- [ADR-0042: Stufen und Ereigniskatalog des Betriebsprotokolls — Error gegen Exception, 4xx nur als Debug oder Episode, Tagesbudget je Stufenklasse](../ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) — Akzeptiert, 2026-10-28
- [ADR-0043: Protokollstufe über `--protokoll-stufe` und `LOCALCHATA_PROTOKOLL_STUFE`, Kennzahlen im Health-Check](../ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md) — Akzeptiert, 2026-10-28

## Offene Fragen

- Budgets je Tag (64/32/32 MiB) sind gerechnet, nicht gemessen — Messwert unter Last nach Profil steht als Folgeaufgabe in ADR-0042.

## Notizen / Quellen

- **Datenhoheit**: einziger Schreiber unter `<ordner>/logs/`; einziger Halter der Episoden-Tabelle (höchstens 1.024 offene Episoden) und der Zähler seit dem Start, nur im Speicher — ein Neustart setzt Zähler und Episoden zurück, die Dateien bleiben. Gelöscht wird nur nach Erlaubnisliste (`JJJJ-MM-TT.log`, reguläre Datei).
- **Begriff**: „Log“ bleibt im Projekt das Tageslog eines Gesprächs ([A00007](A00007-logformat.md)); dieser Baustein heißt Betriebsprotokoll, im Code `protokoll`, Felder im Health-Check `protokoll_*`.
- **Erster Test** (Unit, rein): Formatierer — Eintrag mit `\n`, ESC, U+2028 im Kontext → genau eine Zeile, escaped wie ADR-0040 „Folgeentscheidungen“. Ersetzt: nichts. Danach Datei mit gestellter Uhr über Mitternacht und Zeitumstellung (ersetzt: Uhr), Bereinigung mit Wegwerf-Ordner (ersetzt: nichts).
- **Betrieb**: Start — geöffnet nach der Sperre, vor dem Laden, Bereinigung sofort; Gesundheit — Felder `protokoll_*`, `warnings`, `errors`, `exceptions`, `letzter_error`, `letzte_exception`, `aufgaben_ausgefallen`; Konfiguration — `--protokoll-stufe`, `LOCALCHATA_PROTOKOLL_STUFE` (falscher Wert → Exit 2), Konstanten Aufbewahrung 14 Tage und Budgets im Code; Fehlerpfad — Rückfall auf stderr mit Warning je Episode, `protokoll_ok = false`.
- **Durchsetzung** (Makro über [X00001](Aspekte/X00001-logging-und-diagnose.md)): `eprintln!`/`println!` außerhalb von `src/protokoll/` und der Startzeile in `src/start/` ist ein Befund — Review-Punkt des `observability-reviewer` und später `clippy::disallowed_macros`; Test „Token nie im Protokoll“ (`grep`) im Integrationstest der HTTP-Schnittstelle.
- Entworfen am 2026-10-28 im Vorlauf „Betriebsprotokoll“ (Auftrag des Directors, Vorgabe des Betreibers in `AI-Team/umgebung.md`) als eigener Baustein: es ist eine eigene Ablage mit eigener Frist und eigenem Schreiber, quer zu allen Fachbausteinen.
