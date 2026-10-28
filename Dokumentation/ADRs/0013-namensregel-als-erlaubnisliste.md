# ADR-0013: Namensregel als Erlaubnisliste — `[a-z0-9-]`, 1 bis 32 Zeichen, abweisen statt umformen

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #3 I0004)
**Kontext-Anforderung**: — (WBS I0004; prägt I0012, I0013, I0014)
**Erprobt durch**: — (Regel ohne Abhängigkeit; Beleg sind die bestehende Slug-Regel aus ADR-0003 und die Befunde des Security-Reviews #2)

## Kontext

- Vision: jeder Name vor dem `@` ist nur einmal vergeben, egal von wo. Beispiele der Vision: `shoff`, `claude`, `claude-entwickler`; Designsystem (`components/Kennung`): Kürzel aus den Bindestrich-Teilen (`claude-entwickler` → `ce`), Kennung in Mono, volle Kennung nie gekürzt, solange Platz ist.
- Security-Review #2 (Karte #2) zu `src/gespraeche/kennung.rs`: die heutige Prüfung (`Kennung::bilden`: nicht leer, keine Leer- und Steuerzeichen, genau ein `@`) lässt Formatzeichen der Unicode-Klasse Cf durch (Nullbreiten-Leerzeichen U+200B, Richtungsumkehr U+202E, BOM U+FEFF) und `/`, `\`, `+`. Damit sind `chef` und `chef\u{200B}` zwei „verschiedene“ Namen, die gleich aussehen; ein Name mit U+202E dreht die Anzeige der Herkunft um.
- Der Name landet in Dateinamen: Direktgespräche liegen unter `direkt/<kennung-a>+<kennung-b>/` ([ADR-0003](0003-ablage-layout-im-ordner.md)) — `+` trennt, `/` und `\` wären Pfadtrenner, ein führender `.` versteckt den Ordner.
- Der Name wird in Nachrichten erwähnt (`@name`, WBS I0013) und steht in Markdown (Vision): `_` ist dort Hervorhebung, ein `.` am Ende ist vom Satzende nicht zu unterscheiden.
- Channel-Slugs folgen schon `[a-z0-9-]` (ADR-0003).

## Optionen

### Option A: ASCII klein — `[a-z0-9-]`, beginnt und endet mit Buchstabe oder Ziffer, 1–32 Zeichen; alles andere wird mit Vorschlag abgewiesen
- Vorteile: keine Verwechslung über Groß-/Kleinschreibung, Unicode oder Satzzeichen; dieselbe Regel wie Channel-Slugs; ohne Kodierung in Datei- und Ordnernamen, Erwähnungen und Markdown sicher; trivial zu prüfen und zu testen.
- Nachteile: `jörg` oder `José` sind nicht möglich (Vorschlag `joerg`, `jose`); Unterstrich und Punkt, die Entwickler gewohnt sind, fehlen.

### Option B: Unicode-Buchstaben und -Ziffern plus `-`, NFC-normalisiert, Cf verboten, Eindeutigkeit über ein Verwechslungs-Skelett (UTS #39)
- Vorteile: jeder schreibt seinen Namen, wie er heißt; international.
- Nachteile: das Skelett braucht Unicode-Tabellen als Abhängigkeit und Pflege; Verwechslungen innerhalb einer Schrift (`l`/`1` in manchen Schriften) bleiben; Ordnernamen und Erwähnungen müssen kodiert werden; schwer vollständig zu testen.

### Option C: ASCII `[A-Za-z0-9._-]`, Anzeige mit Großschreibung, Eindeutigkeit über die kleingeschriebene Form
- Vorteile: `Claude` und `ClaudeEntwickler` lesbar; vertraute Zeichen aus Benutzernamen.
- Nachteile: zwei Schreibweisen eines Namens (Erwähnung `@claude` trifft `Claude`?); `claude_x` und `claude-x` und `claude.x` sind verschiedene Namen, die sich in schmaler Schrift kaum unterscheiden; `_` und `.` stören Markdown und Satzende.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass die Regel jede Verwechslung ausschließt, die das Security-Review fand, und sich in Logformat, Ablage, Erwähnung und Markdown ohne Kodierung verhält wie ein Channel-Slug. In Kauf genommen: Namen mit Umlauten oder Großbuchstaben gibt es nicht — wer `Jörg` eingibt, bekommt `joerg` vorgeschlagen.

Regeln:

- **Name**: `^[a-z0-9]([a-z0-9-]{0,30}[a-z0-9])?$` — 1 bis 32 Zeichen, nur Kleinbuchstaben a–z, Ziffern und `-`, nicht am Anfang oder Ende.
- **Abweisen statt umformen**: der Server formt nie still um. Eine Verletzung ergibt 422 `{fehler, regel, vorschlag?}`; `vorschlag` ist die Umformung (klein, Umlaute zu `ae/oe/ue/ss`, sonst nicht erlaubte Zeichen zu `-`, Mehrfach-`-` zusammengefasst, Ränder entfernt, auf 32 gekürzt), wenn sie nicht leer ist. Die Oberfläche darf beim Tippen anzeigen, was erlaubt ist, aber schickt, was der Mensch bestätigt.
- **Gleichheit**: zwei Namen sind gleich, wenn sie Zeichen für Zeichen gleich sind — die Regel lässt keine zweite Schreibweise zu.
- **Herkunft**: `[0-9a-f.:]`, 1 bis 45 Zeichen (längste Textform einer IPv6-Adresse mit eingebetteter IPv4) — die Textform einer kanonischen IP-Adresse ([ADR-0012](0012-identitaet-und-sitzung.md)); eine spätere Rechnernamen-Erweiterung erweitert die Herkunftsregel durch eine neue ADR.
- **Kennung** = Name `@` Herkunft. `Kennung::bilden` (Lesen der Logs, `src/gespraeche/kennung.rs`) prüft ab I0004 beide Regeln; eine Kopfzeile mit anderer Kennung ist ein `Defekt` mit Zeilennummer ([ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md), A00007) — da nur das Programm schreibt (ADR-0005), betrifft das keine echten Bestände.
- **Reservierte Namen**: fachlich beim `planer` (Empfehlung: `alle` und `hier` für spätere Sammel-Erwähnungen reservieren, sonst keine).

## Konsequenzen

**Positiv:**
- Ein Name ist in Teilnehmerliste, Log, Ordnername und Erwähnung dasselbe Wort; `grep "@claude-entwickler@"` findet genau einen Teilnehmer.
- Die Prüfung ist eine reine Funktion mit vollständiger Testmatrix (Ränder, Länge 32/33, jedes Nicht-Erlaubte als Klasse).

**Negativ / Inkauf zu nehmen:**
- Keine Umlaute, keine Großbuchstaben, kein `_` oder `.` im Namen.
- Die Regel ist schwer zu lockern, sobald Erwähnungen und Direktgespräche darauf bauen (Ordnernamen, ADR-0003) — eine Lockerung braucht eine neue ADR und gegebenenfalls Migration.

**Folgeentscheidungen / Aufgaben:**
- [ ] Typ `Name` (Regel, Vorschlag) als Operation ohne Abhängigkeiten; `Kennung::bilden` nutzt `Name` und Herkunftsregel. Testmatrix mindestens: `a`, `claude-2`, 32 Zeichen gültig; leer, 33 Zeichen, `-a`, `a-`, `Claude`, `jörg`, `a_b`, `a.b`, `a/b`, `a\b`, `a+b`, `a@b`, `chef\u{200B}`, `\u{202E}abc`, `\u{FEFF}abc`, `a b` ungültig; Vorschläge `Claude` → `claude`, `Jörg` → `joerg`, `a_b` → `a-b`, `___` → keiner.
- [ ] Der bestehende Test `ä@fe80::1` in `src/gespraeche/kennung.rs` widerspricht der Regel und wird mit dem Bau umgestellt.

## Quellen / Bezuege

- [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md), [A00007 Logformat](../Architektur/A00007-logformat.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md)
- ADR-0003 (Slug-Regel, Ordnernamen der Direktgespräche), ADR-0004, ADR-0005, ADR-0012
- Vision R00000, Zielbild „Mensch und KI sind gleichberechtigt“; Designsystem `components/Kennung/README.md`; Security-Review Karte #2 (über den Auftrag des Directors); Unicode TR #39 (Verwechslungs-Skelett, nur als Option B)

## Ergänzungen

- **2026-10-28 (Karte #4, Nachzug aus #3) — Korrektur:** Das Beispiel unter Option A, „`José` → Vorschlag `jose`“, ist falsch. Die Umformung unter „Regeln“ macht nur Umlaute zu `ae/oe/ue/ss`; `é` ist ein sonst nicht erlaubtes Zeichen und wird `-`, der Rand-Bindestrich entfällt: `José` → `jos` (so auch `umformen` in `src/name.rs`). Die Regel gilt unverändert; nur das Beispiel war falsch. Ob weitere Buchstaben mit Akzent auf ihren Grundbuchstaben abgebildet werden sollen (`é` → `e`), ist eine fachliche Frage für den `planer` (Empfehlung: ja, als kleine Ergänzung der Vorschlagsregel — der Vorschlag ist nur ein Angebot, die Regel bleibt).
