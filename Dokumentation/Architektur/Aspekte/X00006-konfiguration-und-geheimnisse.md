---
id: X00006
name: Konfiguration und Geheimnisse
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00003, A00006]
herkunft: dialog
quellen:
  - src/start/
---

# Konfiguration und Geheimnisse

## Hausantwort

Keine Vorlage zugeordnet; Vorbild kanbanD: nur Kommandozeile, Einstellungen im Ordner.

## Systemantwort

Programmweit nur Kommandozeile (`<ordner>`, `--port`, `--host`, `--kein-browser`, ab I0004 `--erreichbar-als <name>` mehrfach für weitere Host-Namen nach [ADR-0014](../../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md), später z. B. Anhanggrenze); je Gespräch `einstellungen.toml` im Ordner (Name, Frist). Umgebungsvariablen nur als Testknöpfe mit Präfix `LOCALCHATA_` (ADR-0009), je Knopf ein Test über den echten Weg. Geheimnisse gibt es keine (Sitzungs-Tokens leben nur im Speicher, [ADR-0012](../../ADRs/0012-identitaet-und-sitzung.md); Testknopf `LOCALCHATA_GNADENFRIST_MS`). Durchsetzung: `std::env::var` nur in einer Funktion je Knopf, Review-Punkt.

## Abweichungen

## Offene Fragen

## Notizen / Quellen

- **Ergänzung 2026-10-29 (Karte #34):** Zweite Betriebseinstellung nach `--protokoll-stufe` ([ADR-0043](../../ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md)) mit Umgebungswert für den Dienstbetrieb: `--plattenquote` / `LOCALCHATA_PLATTENQUOTE` ([ADR-0044](../../ADRs/0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) Regel 3) — Umgebungswerte sind damit nicht mehr nur Testknöpfe; Muster: Option vor Umgebung vor Standard, falscher Wert Exit 2, Start-Eintrag nennt die Herkunft. Die Systemantwort oben („nur als Testknöpfe“) liest sich so.

Entworfen am 2026-10-27.
