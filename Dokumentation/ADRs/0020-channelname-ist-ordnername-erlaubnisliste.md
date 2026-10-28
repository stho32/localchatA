# ADR-0020: Channelname ist Ordnername — Erlaubnisliste wie Teilnehmernamen, abweisen statt umformen, Ordner beim Anlegen

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #6 I0007)
**Kontext-Anforderung**: — (WBS I0007; prägt I0008, I0009, I0011, I0015, I0018)
**Erprobt durch**: — (Regel ohne Abhängigkeit; Beleg ist die gebaute Namensregel `src/name.rs` aus ADR-0013 samt Testmatrix)

Präzisiert [ADR-0003](0003-ablage-layout-im-ordner.md), Regel „Slug“: es gibt keinen abgeleiteten Slug mehr und keinen getrennten Anzeigenamen — der Name **ist** der Ordnername.

## Kontext

- Vision, Zielbild „Ein Chat, der ein Ordner ist“ und „Mehrere Channels“ (Beispiel: ein Channel zwischen Security-Factory und Entwicklungs-Factory); „lesbar ohne localchatA“.
- ADR-0003 legte fest: Slug = Name klein, Umlaute zu `ae/oe/ue/ss`, sonst `[a-z0-9-]`; Anzeigename in `einstellungen.toml`; gleicher Slug → abgewiesen. Gebaut ist davon nur das Lesen (`src/gespraeche/logablage.rs:80` nimmt **jeden** Unterordner von `channels/` als Channel, ohne Prüfung).
- ADR-0013 (Teilnehmernamen) entschied nach dem Security-Review #2: Erlaubnisliste `[a-z0-9-]`, 1–32 Zeichen, abweisen statt umformen, Vorschlag als Angebot. Gründe dort: keine Verwechslung über Groß/Klein, Unicode-Formatzeichen (U+200B, U+202E), Pfadzeichen; Name ist zugleich Datei- und Ordnername und steht in Erwähnungen und Markdown.
- Designsystem: Seitenleiste zeigt `# name` (`components/Seitenleiste/README.md`); Suche springt per `#` zu Channels (`1-tastatur.md`, `Strg K`); Bildschirm 4.3 „Channel anlegen“ kennt die Zustände „Name schon vorhanden“ und „ungültiger Name“ (`design-prompt.md`). Fachwort „Channel“, nie „Kanal“ (README).
- Der Ordner kann auf einer Freigabe oder einem Windows-Rechner liegen (Plattform des Betreibers laut `~/.claude/CLAUDE.md`): `con`, `nul`, `com1` … sind dort keine anlegbaren Ordner.

## Optionen

### Option A: Name = Ordnername nach der Regel aus ADR-0013, abweisen mit Vorschlag
- Regel `^[a-z0-9]([a-z0-9-]{0,30}[a-z0-9])?$`; was sie verletzt, ergibt 422 mit `vorschlag` aus derselben Umformung wie bei Teilnehmernamen (`Prüfung` → `pruefung`).
- Vorteile: eine Schreibweise je Channel — in Seitenleiste, Ordner, `#`-Sprung und späterer Erwähnung dasselbe Wort; Kollision ist Zeichengleichheit; die Prüfung existiert schon (`Name::bilden`) und ist vollständig getestet; nichts muss kodiert werden; keine Einstellungsdatei nötig, um den Namen zu kennen.
- Nachteile: keine Großbuchstaben, Umlaute oder Leerzeichen im sichtbaren Namen (`# security-entwicklung` statt „Security ↔ Entwicklung“).

### Option B: Freier Anzeigename (Unicode) plus abgeleiteter Slug als Ordner (Wortlaut ADR-0003)
- Vorteile: der Mensch schreibt den Namen, wie er ihn sagt; nah am Satz in ADR-0003.
- Nachteile: zwei Namen je Channel — welcher gilt beim `#`-Sprung und in der Erwähnung? Kollisionen über die Ableitung überraschen (`Prüfung` und `Pruefung` und `PRÜFUNG` → derselbe Slug, der zweite wird mit einer Meldung abgewiesen, die er nicht versteht); der Anzeigename bringt die Unicode-Befunde aus dem Security-Review #2 (Formatzeichen, Richtungsumkehr, Doppelgänger) zurück; Anzeigename und Ordner können auseinanderlaufen, wenn jemand die Einstellungsdatei von Hand ändert.

### Option C: Server formt still um (Slug aus Eingabe), kein Abweisen
- Vorteile: jede Eingabe führt zu einem Channel, keine Fehlermeldung.
- Nachteile: verletzt „abweisen statt umformen“ aus ADR-0013 — wer `Prüfung` anlegt, bekommt `pruefung` ohne gefragt zu sein; zwei Eingaben landen still im selben Channel; Agenten können nicht vorhersagen, welcher Name entsteht.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass ein Channel genau ein Wort hat, das überall gleich ist und ohne Kodierung als Ordner taugt — dieselbe Abwägung wie ADR-0013, mit gebautem und getestetem Code. In Kauf genommen: sichtbare Namen nur klein, ohne Umlaute und Leerzeichen; Wunsch nach Lesbarem deckt die optionale Beschreibung ([ADR-0022](0022-channel-einstellungen-je-channel-als-toml.md)).

Regeln:

1. **Channelname** = Teilnehmer-Namensregel aus ADR-0013: 1–32 Zeichen aus `a–z`, `0–9`, `-`, `-` nicht am Rand. Typ `Channelname` über `Name` (`src/name.rs`), mit Regel 2 zusätzlich.
2. **Reserviert** (422, `regel: "reserviert"`, kein Vorschlag): `con`, `prn`, `aux`, `nul`, `com0`–`com9`, `lpt0`–`lpt9` — Gerätenamen, die Windows nicht als Ordner zulässt. Sonst keine; Sammel-Erwähnungen (`alle`, `hier`) betreffen Teilnehmer, nicht Channels.
3. **Abweisen statt umformen**: Verletzung → 422 `{fehler, regel, vorschlag?}` wie `beitreten`; `vorschlag` ist die Umformung aus ADR-0013 (inklusive Ergänzung 2026-10-28), wenn sie nicht leer und nicht reserviert ist.
4. **Kollision**: zwei Channels sind gleich, wenn ihre Namen Zeichen für Zeichen gleich sind. `Allgemein` ist kein zweiter `allgemein`, sondern ungültig (422, Vorschlag `allgemein`).
5. **Ordner** `channels/<name>/` entsteht **beim Anlegen** (nicht erst mit der ersten Nachricht) — sonst wäre ein leerer Channel nach einem Neustart verschwunden, und `ls channels/` zeigte nicht alle Channels. Angelegt mit `create_dir` (scheitert, wenn er existiert) und darin sofort die Einstellungen (ADR-0022).
6. **Laden**: ein Unterordner von `channels/`, dessen Name Regel 1 oder 2 verletzt, ist kein Channel. Beginnt er mit `.`, wird er still übergangen (Arbeitsordner); sonst meldet der Start eine Zeile auf stderr („`channels/Mein Channel`: kein gültiger Channelname, übergangen“) und zählt ihn bei den Defekten. Er wird nie umbenannt oder gelöscht.

## Konsequenzen

**Positiv:**
- `ls channels/` ist die Channel-Liste; `grep -r … channels/security-entwicklung/` funktioniert ohne Wissen über Slugs.
- Die Prüfung ist eine reine Funktion mit vorhandener Testmatrix; neu sind nur die reservierten Namen.

**Negativ / Inkauf zu nehmen:**
- `# Prüfung` gibt es nicht, nur `# pruefung`.
- Von Hand angelegte Ordner mit Großbuchstaben oder Leerzeichen erscheinen nicht; die Startzeile sagt es.
- Wird die Regel je gelockert, braucht es eine neue ADR und eine Migration der Ordnernamen.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein Agent `Security` anlegen will, 422 bekommt und in einer Schleife wiederholt. Früh-Indikator: viele 422 derselben Herkunft. Gegenmaßnahme: der `vorschlag` steht maschinenlesbar in der Antwort; die Agenten-Anleitung (I0019) nennt die Regel.
2. … ein von Hand angelegter Ordner `channels/Test/` stillschweigend fehlt und der Betreiber einen Datenverlust vermutet. Früh-Indikator: Frage „wo ist mein Channel“. Gegenmaßnahme: Regel 6, Zeile auf stderr und Zähler im Health-Check.

**Folgeentscheidungen / Aufgaben:**
- [ ] Typ `Channelname` (Operation, rein): Testmatrix aus ADR-0013 plus `con`, `nul`, `com1`, `lpt9` → 422 `reserviert`; `com10`, `console` gültig; Vorschlag `CON` → keiner (reserviert), `Prüfung` → `pruefung`.
- [ ] Laden übergeht ungültige Ordner (Integration gegen `tempfile`): `channels/allgemein`, `channels/Test`, `channels/.neu-x` → genau ein Channel, eine Defektzeile für `Test`, keine für `.neu-x`.

## Quellen / Bezuege

- [A00012 Channelverzeichnis](../Architektur/A00012-channelverzeichnis.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00008 Logablage](../Architektur/A00008-logablage.md)
- ADR-0003 (Slug, präzisiert), ADR-0013 (Namensregel), ADR-0005 (einziger Schreiber)
- Vision R00000, Zielbild „Ein Chat, der ein Ordner ist“, „Mehrere Channels“; `design-prompt.md` 4.3; Designsystem `README.md`, `1-tastatur.md`, `components/Seitenleiste/README.md`
- Microsoft, „Naming Files, Paths, and Namespaces“ (reservierte Gerätenamen), https://learn.microsoft.com/windows/win32/fileio/naming-a-file (aus dem Gedächtnis, in diesem Auftrag nicht geöffnet — Vermutung, die der Entwickler mit der Testmatrix festhält)
