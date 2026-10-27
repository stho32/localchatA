---
id: A00010
name: Anwesenheit
code: anwesenheit
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/anwesenheit/
verwandt: [A00011]
domaene: []
blackbox: ein Zustandsautomat im Speicher; Regeln in ADR-0008, Verfeinerung mit I0004–I0006
---

# Anwesenheit

## Verantwortung

Führt je Sitzung einen eindeutigen Namen samt Status online, offline oder weg.

## Schnittstellen

- `beitreten(name, herkunft, art) -> Result<Sitzung{token, kennung}, NameVergeben{seit, vorschlag}>`.
- `wiederkehren(token, herkunft) -> Result<Sitzung, Abgelaufen>` — Tab-Neuladen, Agent nach Neustart.
- `verbunden(token) -> Halter` — solange der Halter lebt (SSE-Strom oder Warte-Aufruf), ist der Teilnehmer online; sein `Drop` meldet den Abbruch.
- `pflegen(jetzt)` — offline seit länger als die Gnadenfrist → weg, Name frei; Takt aus dem Uhr-Provider.
- `teilnehmer() -> Vec<Teilnehmer{kennung, art, status, offline_seit}>`.
- Ausgehend: Ereignis `Anwesenheit` an den [Live-Kanal](A00011-live-kanal.md).

## Abhaengigkeiten

Uhr-Provider (Aspekt [Zeit](Aspekte/X00007-zeit.md)); [Live-Kanal](A00011-live-kanal.md).

## Entscheidungen

- [ADR-0008: Identität — Herkunft aus der Peer-Adresse, Nachweis über Sitzungs-Token](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md) — Vorgeschlagen, 2026-10-27

## Offene Fragen

- Agenten arbeiten oft länger als eine Minute zwischen zwei Warte-Aufrufen. Gilt für sie dieselbe Gnadenfrist (dann verlieren sie mitten in der Arbeit ihren Namen) oder eine längere, beim Beitreten gewählte? Empfehlung: Gnadenfrist je Sitzung, Standard 60 s, Agent darf beim Beitreten bis 3600 s verlangen. Fachlich, für den `planer` bei I0006/I0014.

## Notizen / Quellen

- Keine Persistenz: nach einem Neustart sind alle Namen frei; ein Tab kehrt mit seinem Token zurück und bekommt den Namen, wenn er frei ist.
- Erster Test (Unit, Uhr-Attrappe): `beitreten("claude", 10.0.0.5)` ok; `beitreten("claude", 127.0.0.1)` → `NameVergeben`; Halter fallen lassen → Status offline; Uhr +61 s, `pflegen` → weg, `beitreten("claude", 127.0.0.1)` ok.
