---
id: A00006
name: Gespräche
code: gespraeche
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/
verwandt: [A00011]
domaene: []
---

# Gespräche

## Verantwortung

Ordnet als einziger Schreiber des Ordners jede Änderung an einem Gespräch in derselben Reihenfolge in Log, Index und Live-Kanal ein.

## Schnittstellen

Eingehend (von [HTTP-Schnittstelle](A00004-http-schnittstelle.md), [Start](A00003-start.md)):

- `laden(ordner)` — alle Gespräche, alle Tagesdateien in der Frist; meldet defekte Zeilen, bricht nicht ab.
- `channel_anlegen(name, von)`, `channels()`, `frist_setzen(gespraech, tage)`.
- `senden(gespraech, von, text, anhaenge, antwort)` → `Nachricht { zeit, … }` — Zeit ist zugleich die ID ([ADR-0005](../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md)).
- `block(gespraech, vor: Option<Zeit>, 100)` → bis zu 100 Nachrichten älter als `vor`, neueste zuletzt.
- `suchen(begriff, sichtbar_fuer, nach: Option<Zeit>, 50)` → Treffer neueste zuerst, Fortsetzung über `nach`.
- `entfallen_lassen(heute)` — Tage jenseits der Frist aus Index und Ordner (samt Anhängen).

Ausgehend: Ereignis `Nachricht`/`Channel` an den [Live-Kanal](A00011-live-kanal.md), nachdem Log und Index geschrieben sind.

Gespräch = Channel oder Direktgespräch zweier Teilnehmer; beide haben dieselbe Ablage und denselben Index.

## Abhaengigkeiten

Kinder (alle geplant):

- [Logformat](A00007-logformat.md) (geplant)
- [Logablage](A00008-logablage.md) (geplant)
- [Verlaufsindex](A00009-verlaufsindex.md) (geplant)

Extern: Uhr-Provider (Aspekt [Zeit](Aspekte/X00007-zeit.md)).

## Entscheidungen

- [ADR-0003: Ablage-Layout im Ordner](../ADRs/0003-ablage-layout-im-ordner.md) — Akzeptiert, 2026-10-27
- [ADR-0005: Das Programm ist einziger Schreiber; Reihenfolge über Sperre je Gespräch und monotone Zeit](../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md) — Vorgeschlagen, 2026-10-27
- [ADR-0006: Verlaufsindex im Speicher über dem Rohtext der Tagesdateien](../ADRs/0006-verlaufsindex-im-speicher.md) — Akzeptiert, 2026-10-27

## Offene Fragen

- Gehört ein Direktgespräch zu den beiden Namen oder zu den beiden vollständigen Kennungen `name@herkunft`? Empfehlung: zu den Kennungen — sonst sieht ein späterer Träger desselben Namens fremde Direktnachrichten (hängt an Designsystem-Frage 8). Fachlich, für den `planer` bei I0012.
- Welche Frist gilt für Direktgespräche? Empfehlung: Standard 7 Tage, je Direktgespräch wählbar wie bei Channels. Fachlich, für den `planer` bei I0012/I0018.

## Notizen / Quellen

- **Aggregat** (`_modell.md`, „Ordnersorten“): Logablage und Verlaufsindex teilen veränderlichen Zustand (die Tagesdatei von heute) und werden unter **einer Sperre je Gespräch** gepflegt — Aufteilen würde die Reihenfolge zerreißen. Die Kinder sind seine Facetten.
- Datenhoheit: einziger schreibender Baustein für den ganzen Ordner (Logs, Anhänge, Einstellungen). Anwesenheit und Live-Kanal schreiben nichts auf Platte.
- Erster Test (Unit, ohne Platte): `senden` zweimal in derselben Millisekunde (Uhr-Attrappe fest) → zweite Nachricht trägt Zeit +1 ms, `block(…, None, 100)` liefert beide in Sendereihenfolge; ersetzt: Uhr, Logablage durch eine Speicher-Attrappe (zweite Implementierung des Ports, kein Mock-Framework).
