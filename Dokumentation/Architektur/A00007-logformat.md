---
id: A00007
name: Logformat
code: logformat
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/logformat.rs
verwandt: [A00008, A00009]
domaene: []
blackbox: reine Funktionen mit kleiner Grammatik; vollständig in ADR-0004 beschrieben
---

# Logformat

## Verantwortung

Übersetzt verlustfrei zwischen einer Nachricht und ihrem zeilensicheren Textsatz im Tageslog.

## Schnittstellen

- `satz(&Nachricht) -> String` — Kopfzeile `@ <Zeit> <name@herkunft>`, Metazeilen `+ <schluessel>: <wert>`, Textzeilen mit zwei Leerzeichen eingerückt, Leerzeile als Abschluss.
- `saetze(&str) -> impl Iterator<Item = Result<SatzBereich, Defekt>>` — Bereiche (Byte-Spannen) der Sätze im Rohtext, für Index und Laden; defekte Kopfzeilen als `Defekt` mit Zeilennummer.
- `text(&str) -> String` — Textzeilen eines Satzes ohne Einrückung (für die Ausgabe eines Blocks).

## Abhaengigkeiten

keine (rein).

## Entscheidungen

- [ADR-0004: Logformat — Kopfzeile und eingerückter Text](../ADRs/0004-logformat-kopfzeile-und-eingerueckter-text.md) — Akzeptiert, 2026-10-27

## Offene Fragen

## Notizen / Quellen

- Erster Test (Unit): Nachricht mit Text `"# Titel\n\n@ kein Kopf\n  eingerückt\n```\ncode\n```"` → `satz` → `saetze` → `text` ergibt denselben Text; die Zeile `@ kein Kopf` im Text erzeugt keinen zweiten Satz. Varianten: `\r\n` wird `\n`, abschließende Leerzeilen entfallen, Tab bleibt erhalten.
