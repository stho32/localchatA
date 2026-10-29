---
id: A00017
name: Anhangeingang
code: anhangeingang
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: vermutung
quellen:
  - src/gespraeche/anhangeingang.rs
verwandt: [A00008, A00014, A00018]
domaene: []
---

# Anhangeingang

## Verantwortung

Hält hochgeladene, noch nicht gesendete Dateien je Absender bis zur Bindung an eine Nachricht oder bis zu ihrem Ablauf bereit.

## Schnittstellen

- `annehmen(kennung, name, laenge) -> Result<Platz, Abweisung>` — prüft offene Hochladungen (4 je Kennung, 16 je Herkunft) und reserviert `laenge` an der Quote über Gespräche ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 1, Schritte 3 und 5).
- `Platz::schreiben(stueck)`, `Platz::abschliessen() -> Eingangseintrag { id, name, bytes, art, masse, gueltig_bis }` — schreibt nach `.eingang/<id>.teil`, dann `rename` nach `.eingang/<id>`; Abbruch (Drop ohne Abschluss) löscht die Teildatei und gibt die Reservierung frei.
- `nehmen(kennung, ids) -> Result<Vec<Eingangseintrag>, Anhangfehler>` — beim Senden unter der Schreibsperre; `unbekannt | abgelaufen | fremd | doppelt`.
- `zuruecklegen(eintraege)` — nach gescheitertem Anhängen des Satzes ([ADR-0046](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 5).
- `ablaufen(jetzt)` — im Sekundentakt der Pflege; `leeren()` beim Start.

## Abhaengigkeiten

[Logablage](A00008-logablage.md) (Port wächst um den Eingang), [Anhangprüfung](A00018-anhangpruefung.md) (Art und Maße aus den ersten Stücken, Text-Prüfung über alle), Uhr- und Zufalls-Provider (`src/uhr.rs`, `src/zufall.rs`) für Ablauf und ID.

## Entscheidungen

- [ADR-0046: Anhänge ablegen — je Datei hochgeladen in einen Eingang, beim Senden in den Tag der Nachricht gebunden, eine Metazeile mit Anhang-ID; Bytes zählen an der Quote und an einem eigenen Konto](../ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) — Vorgeschlagen, 2026-10-29

## Offene Fragen

- Grenzen je Datei und je Nachricht, Ablauf 15 min — hängen an Frage B2 an den Betreiber (Bericht Karte #9); bis zur Freigabe `herkunft: vermutung`.

## Notizen / Quellen

- Datenhoheit: `<ordner>/.eingang/` gehört allein diesem Baustein (über den Port der Logablage); `reserviert` hält Gespräche ([ADR-0044](../ADRs/0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) Regel 2 bleibt: ein Halter der Quote).
- Erster Test (Unit, Speicherablage, gestellte Uhr, fester Zufall): `annehmen` 200 Bytes → Platz; drei Stücke schreiben, abschließen → Eintrag mit `gueltig_bis` = jetzt + 15 min; `nehmen` mit fremder Kennung → `fremd`; zweimal `nehmen` → beim zweiten `unbekannt`; Uhr + 15 min + 1 s, `ablaufen` → Speicherablage ohne Datei, Reservierung frei; Platz ohne `abschliessen` fallen lassen → keine `.teil`-Datei, Reservierung frei. Ersetzte Provider: Logablage, Uhr, Zufall.
- Betrieb: `.eingang/` wird beim Start vor dem Laden geleert; Symlinks und Unterordner dort bleiben liegen und werden als `WARNING` gemeldet.
