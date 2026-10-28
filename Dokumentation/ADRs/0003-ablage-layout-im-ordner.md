# ADR-0003: Ablage-Layout im Ordner

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #1 I0001)
**Kontext-Anforderung**: — (WBS I0002, I0007, I0010, I0012, I0018)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (10 Channels × 7 Tagesdateien, 100.000 Nachrichten; siehe ADR-0006)

## Kontext

„Ein Chat, der ein Ordner ist“: je Channel ein täglich rotierendes Log, lesbar ohne localchatA, kein Git, keine Datenbank; Frist je Channel, Standard 7 Tage, ältere Logtage entfallen (Vision, Zielbild). Dazu kommen Direktnachrichten, Anhänge und Einstellungen je Channel. Das Layout ist schwer umkehrbar, sobald Bestand auf Platte liegt.

## Optionen

### Option A: Ein Ordner je Gespräch, darin Tageslogs, Anhänge je Tag und Einstellungen
```
<ordner>/
  .localchatA.sperre
  channels/<slug>/einstellungen.toml        name, frist_tage, angelegt, angelegt_von
  channels/<slug>/2026-10-27.log
  channels/<slug>/anhaenge/2026-10-27/<zeit>-<n>-<dateiname>
  direkt/<kennung-a>+<kennung-b>/…          gleich aufgebaut; Kennungen sortiert
```
- Vorteile: ein Tag eines Gesprächs entfällt mit genau einer Datei und einem Ordner; `ls` zeigt die Channels; jedes Gespräch für sich kopierbar.
- Nachteile: viele kleine Dateien bei vielen Gesprächen; Dateinamen aus Kennungen müssen kodiert werden.

### Option B: Ein Ordner je Tag, darin je Gespräch eine Datei (`2026-10-27/<slug>.log`)
- Vorteile: Frist für alle gleich wäre ein `rm -r` je Tag; Tagesüberblick auf einen Blick.
- Nachteile: die Frist ist je Channel verschieden — dann wird doch je Datei entschieden; ein Channel ist über viele Ordner verstreut.

### Option C: Ein Log je Tag für alle Gespräche, Channel als Feld im Satz
- Vorteile: eine Datei, eine Schreibreihenfolge für alles.
- Nachteile: Frist je Channel unmöglich ohne Umschreiben; Direktnachrichten liegen zwischen Channel-Nachrichten; Lesen eines Channels heißt filtern.

## Entscheidung

**Gewählt: Option A**

Sie bildet die Frist je Channel direkt auf Dateien ab und hält jedes Gespräch lesbar beieinander. Regeln:

- **Slug** eines Channels: Name klein, Umlaute zu `ae/oe/ue/ss`, sonst `[a-z0-9-]`; der Anzeigename steht in `einstellungen.toml`. Zwei Namen mit gleichem Slug → der zweite wird abgewiesen.
- **Direktgespräch**: Ordnername aus beiden Kennungen `name@herkunft`, sortiert, mit `+` verbunden; Zeichen außerhalb `[A-Za-z0-9._@-]` (z. B. `:` bei IPv6) als `%XX`.
- **Tagesdatei** `JJJJ-MM-TT.log` nach dem **lokalen** Datum des Servers bei der Nachrichtenzeit (Aspekt Zeit).
- **Anhänge** liegen im Tag der Nachricht und entfallen mit ihm; der gespeicherte Dateiname ist bereinigt (keine Leerzeichen, keine Steuer- oder Pfadzeichen), damit er als Feld im Satz steht (ADR-0004); erst die Datei (temporär + `rename`), dann der Satz — ein Absturz hinterlässt höchstens eine verwaiste Datei, nie einen Satz ohne Datei.
- **Einstellungen** als TOML, geschrieben über temporäre Datei + `rename`.
- **Entfallen**: Tag D entfällt, sobald D älter ist als `frist_tage` Kalendertage vor heute; geprüft beim Start, beim Tageswechsel und beim Ändern der Frist.

## Konsequenzen

**Positiv:**
- `grep -r "@claude" channels/` und `less channels/allgemein/2026-10-27.log` funktionieren ohne das Programm.
- Frist-Pflege ist Löschen ganzer Dateien, kein Umschreiben.

**Negativ / Inkauf zu nehmen:**
- Die Ordnernamen der Direktgespräche hängen an der Identitätsregel (ADR-0008) — ändert sie sich, braucht es eine Migration.

**Folgeentscheidungen / Aufgaben:**
- [x] Genaue Grenze der Frist (zählt heute mit?) — entschieden in [ADR-0029](0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md) (Karte #12). Frist für Direktgespräche — offen, `planer` bei I0012.

## Quellen / Bezuege

- [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00008 Logablage](../Architektur/A00008-logablage.md)
- Vision, Zielbild „Ein Chat, der ein Ordner ist“, „Das Log ist das Gedächtnis“, „Anhänge“; WBS I0002, I0010, I0018

## Ergänzungen

- **2026-10-28 (Karte #6 I0007) — Slug und Einstellungen präzisiert:** Den abgeleiteten Slug und den getrennten Anzeigenamen gibt es nicht; der Channelname folgt der Namensregel aus ADR-0013 und **ist** der Ordnername, Abweichungen werden mit Vorschlag abgewiesen ([ADR-0020](0020-channelname-ist-ordnername-erlaubnisliste.md)). `einstellungen.toml` trägt deshalb keinen `name`; Inhalt, Schreiben und Lesen regelt [ADR-0022](0022-channel-einstellungen-je-channel-als-toml.md). Die übrigen Regeln dieser ADR gelten unverändert.
- **2026-10-28 (Karte #12 I0018) — Frist und Entfallen präzisiert:** „Tag D entfällt, sobald D älter ist als `frist_tage` Kalendertage vor heute“ heißt genau: heute zählt mit, Tag D entfällt ⇔ `D < heute − (frist_tage − 1)` nach lokalem Datum; Spanne 1–3650 ([ADR-0029](0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md)). Wann und wie entfallen wird — Start, Tageswechsel im Sekundentakt, sofort beim Verkürzen; Löschen unter der Gesprächssperre, dann Index — regelt [ADR-0030](0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md); was gelöscht werden darf, [ADR-0032](0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md). Direktgespräche entfallen bis I0012 nicht.
