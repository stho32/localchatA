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
- [ADR-0034: Nachricht senden — `POST /api/channels/{name}/nachrichten` mit `{text}` für Mensch und Agent, höchstens 16 KiB, Steuerzeichen abweisen, Wiederholschlüssel](../ADRs/0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) — Akzeptiert, 2026-10-28
- [ADR-0048: Eigene Nachrichten bearbeiten und löschen — Änderungssatz im Tageslog der Änderung, Fassung je Nachricht im Index, zwei Routen, Thema `aenderung`, der Löschsatz ist der Commit-Punkt für die Anhänge](../ADRs/0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md)
- [ADR-0053: Antworten mit Zitat — Bezug als Metazeile `+ antwort_auf: <zeit> <kennung>`, aufgelöst beim Lesen, Feld `antwort_auf`, `zitat` im Thema `aenderung`, Sprung über den Verlauf, Wecken über die Kennung](../ADRs/0053-antworten-mit-zitat-bezug-als-metazeile-zeit-und-kennung-aufgeloest-beim-lesen-feld-antwort-auf-zitat-im-thema-aenderung.md) — Akzeptiert, 2026-10-30

## Offene Fragen

## Notizen / Quellen

- Erster Test (Unit): Nachricht mit Text `"# Titel\n\n@ kein Kopf\n  eingerückt\n```\ncode\n```"` → `satz` → `saetze` → `text` ergibt denselben Text; die Zeile `@ kein Kopf` im Text erzeugt keinen zweiten Satz. Varianten: `\r\n` wird `\n`, abschließende Leerzeilen entfallen, Tab bleibt erhalten.
- **Ergänzung 2026-10-27 (Karte #2, I0002):** Die Kopfzeile trennt Felder durch ein Leerzeichen — der Absender kommt deshalb als Typ `Kennung`, der beim Bilden garantiert: nicht leer, kein Leer- oder Steuerzeichen, genau ein `@` zwischen Name und Herkunft (IPv6-Herkunft mit `:` ist erlaubt). Die genaue Namensregel entscheidet der `planer` bei I0004 ([ADR-0008](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md)); `satz` muss sich auf den Typ verlassen können und prüft nicht erneut. Eine Kopfzeile, deren Zeit nicht lesbar ist (z. B. der verschmolzene Rest eines Teilschreibens, [ADR-0011](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md)), ist ein `Defekt` mit Zeilennummer, kein Satz.
- **Ergänzung 2026-10-27 (Karte #3, I0004):** Die Namensregel ist entschieden ([ADR-0013](../ADRs/0013-namensregel-als-erlaubnisliste.md)): Name `[a-z0-9-]`, 1–32 Zeichen, nicht mit `-` beginnend oder endend; Herkunft `[0-9a-f.:]`, 1–45 Zeichen. `Kennung::bilden` prüft beide Regeln — eine Kopfzeile mit Formatzeichen (U+200B, U+202E, U+FEFF), Großbuchstaben, Umlauten, `/`, `\` oder `+` in der Kennung ist ein `Defekt`. Der heutige Test mit `ä@fe80::1` wird dabei umgestellt.
- **Ergänzung 2026-10-28 (Karte #8, I0009):** (1) Erste Metazeile: `+ art: mensch | agent` direkt nach der Kopfzeile; `metazeilen(satz)` liest sie, `text` übergeht sie wie bisher; unbekannte Schlüssel bleiben erhalten (ADR-0004). (2) **Textregel** als reine Funktion neben dem Format, weil sie die Lesbarkeit des Logs mit `cat`/`less` schützt: leer (nur Leerraum), höchstens 16.384 Bytes UTF-8, verbotene Steuerzeichen (C0 außer Tab und Zeilenumbruch, `\r` nur vor `\n`, DEL, C1) mit erster Stelle ([ADR-0034](../ADRs/0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) Regel 3). Dieselbe Regel steht im Browser in `web/` (eine Quelle je Seite, beide gegen dieselbe Testtabelle).
- **Ergänzung 2026-10-29 (Karte #48, I0032/I0033), Soll:** Änderungssatz = Satz mit genau einer Bezugs-Metazeile `+ bearbeitet: <zeit>` (mit neuem Text) oder `+ geloescht: <zeit>` (ohne Text); Familie `reaktion…` für I0026 reserviert; Defekte und Waisen nach ADR-0048 Regel 1.
- **Ergänzung 2026-10-30 (Karte #79, I0043), Soll:** Metazeile `+ antwort_auf: <zeit> <kennung>` (höchstens eine, nach `+ art:`, vor `+ anhang:`); der Satz bleibt eine Nachricht (`satzart` unverändert); Defekte (zwei Zeilen, Zeit unlesbar, Kennung fehlt, Zeit nicht vor der eigenen) → `WARNING laden`, Nachricht bleibt ohne Bezug. Nie zitierter Text im Log. Regeln in ADR-0053 Regel 1.
