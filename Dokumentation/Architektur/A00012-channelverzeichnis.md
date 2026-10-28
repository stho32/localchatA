---
id: A00012
name: Channelverzeichnis
code: verzeichnis
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/
verwandt: [A00011, A00013]
domaene: []
---

# Channelverzeichnis

## Verantwortung

Führt die Menge der Channels samt Einstellungen als nummerierten Stand, der sich nur unter seiner Sperre ändert.

## Schnittstellen

Facette des Aggregats [Gespräche](A00006-gespraeche.md); von außen nur über dessen Operationen erreichbar:

- `channel_anlegen(name, beschreibung, von: Kennung) -> Result<Channel, Anlegefehler>` — Prüfreihenfolge und Grenzen nach [ADR-0021](../ADRs/0021-channel-anlegen-jeder-mit-sitzung-grenzen-je-herkunft-und-gesamt.md) Regel 5; `Anlegefehler` = `Vorhanden(Channel)` | `OrdnerVorhanden` | `Regel(Namensfehler | Beschreibungsfehler)` | `Grenze{art, hoechstens}` | `Ablage(String)`.
- `channels() -> ChannelStand { stand, liste }` — nach Name sortiert.
- `frist_setzen(…)` (ab I0018) — Prüfreihenfolge nach [ADR-0031](../ADRs/0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md) Regel 2.
- `mit_veroeffentlichen(Rückruf)` — jeder `ChannelWechsel { stand, channel }` geht **unter der Sperre** an den Rückruf ([ADR-0023](../ADRs/0023-channels-als-zweites-thema-im-ereignisstrom.md) Regel 2); ohne Rückruf geht er an niemanden.
- `Channel { name: Channelname, angelegt: Option<Zeit>, angelegt_von: Option<Kennung>, beschreibung: Option<String> }`; ab I0018 zusätzlich `frist: Frist` (Ergänzung unten).

## Abhaengigkeiten

[Logablage](A00008-logablage.md) (`channel_ordner_anlegen`, `einstellungen_schreiben`, `ordner_entfernen`), [Einstellungsformat](A00013-einstellungsformat.md), Uhr-Provider ([Zeit](Aspekte/X00007-zeit.md)), Namensregel `src/name.rs` ([ADR-0020](../ADRs/0020-channelname-ist-ordnername-erlaubnisliste.md)).

## Entscheidungen

- [ADR-0020: Channelname ist Ordnername — Erlaubnisliste wie Teilnehmernamen, abweisen statt umformen, Ordner beim Anlegen](../ADRs/0020-channelname-ist-ordnername-erlaubnisliste.md) — Akzeptiert, 2026-10-28
- [ADR-0021: Channel anlegen — `POST /api/channels` für jeden mit Sitzung, höchstens 32 je Herkunft und 256 gesamt](../ADRs/0021-channel-anlegen-jeder-mit-sitzung-grenzen-je-herkunft-und-gesamt.md) — Akzeptiert, 2026-10-28
- [ADR-0022: Channel-Einstellungen als `einstellungen.toml` im Ordner des Channels](../ADRs/0022-channel-einstellungen-je-channel-als-toml.md) — Akzeptiert, 2026-10-28
- [ADR-0023: Channels als zweites Thema im vorhandenen Ereignisstrom](../ADRs/0023-channels-als-zweites-thema-im-ereignisstrom.md) — Akzeptiert, 2026-10-28
- [ADR-0029: Frist in ganzen Kalendertagen nach lokalem Datum, heute zählt mit, 1 bis 3650, Standard 7](../ADRs/0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md) — Akzeptiert, 2026-10-28
- [ADR-0031: Frist setzen über `PUT /api/channels/{name}/frist` — jeder mit Sitzung, Verkürzen nur mit Bestätigung, verteilt als Channel-Wechsel](../ADRs/0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md) — Akzeptiert, 2026-10-28

## Offene Fragen

- ~~Sehen alle Teilnehmer alle Channels?~~ **Entschieden 2026-10-28** (Director nach Empfehlung, R00006): ja, alle sehen alle; kein Ausblenden geplant (Designsystem `3-offene-fragen.md` Frage 10 beantwortet).
- ~~Legt ein frischer Ordner einen ersten Channel (`allgemein`) selbst an?~~ **Entschieden 2026-10-28** (Director nach Empfehlung, R00006): nein — Laden legt nichts an, die leere Seitenleiste zeigt den Weg (ADR-0024 Regel 2).

## Notizen / Quellen

- **Warum ein eigener Baustein im Aggregat**: bis I0006 war die Menge der Gespräche nach `laden` unveränderlich (`Gespraeche.verlaeufe: BTreeMap<Gespraech, Mutex<Verlauf>>`, `src/gespraeche/mod.rs:33`). Mit I0007 wächst sie zur Laufzeit; Menge, Einstellungen und `stand` müssen gemeinsam unter einer Sperre stehen, sonst laufen Nummer und Veröffentlichung auseinander (Befund B1 aus #4). Schnitt: `RwLock<Verzeichnis { verlaeufe: BTreeMap<Gespraech, Arc<Mutex<Verlauf>>>, channels: BTreeMap<Channelname, Channel>, stand: u64 }>`. Senden (ab I0009) nimmt die Sperre nur **lesend** und nur zum Nachschlagen des `Arc<Mutex<Verlauf>>`, danach die Sperre des Gesprächs (ADR-0005) — Sperrordnung immer Verzeichnis vor Gespräch, nie umgekehrt.
- **Datenhoheit**: einziger Schreiber der Channel-Ordner und ihrer `einstellungen.toml` ist dieser Baustein über die Logablage; die HTTP-Schnittstelle übersetzt nur ([ADR-0005](../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md), Grenzregel A00004).
- **Zählung je Herkunft** aus `angelegt_von.herkunft` aller Channels im Verzeichnis — ohne eigenen Zähler, damit Neustart und Laden sie nicht verfälschen können (ADR-0021 Regel 4). Bei ≤ 256 Channels ist das Zählen beim Anlegen ein Durchlauf über die Liste — hingenommen.
- **Erster Test** (Unit, ohne Platte): Verzeichnis über `Speicherablage`, gestellte Uhr, aufzeichnender Rückruf; `channel_anlegen("allgemein", None, shoff@127.0.0.1)` → `Channel{angelegt: Uhr, angelegt_von: shoff@127.0.0.1}`, Rückruf erhält `stand` 1 unter der Sperre, die Speicherablage enthält den Ordner und eine Einstellungsdatei, `channels()` liefert `stand` 1 mit einem Eintrag; zweites Anlegen desselben Namens → `Vorhanden` ohne Rückruf und ohne Schreibvorgang. Ersetzt: Logablage (Speicher), Uhr, Rückruf (Vektor).
- **Fehlerpfad**: die Speicherablage kann „Einstellungen schreiben scheitert“ simulieren → `Ablage`-Fehler, der Ordner ist zurückgebaut, `stand` unverändert, kein Rückruf.
- **Ergänzung 2026-10-28 (Karte #12, I0018):** `Channel` wächst um `frist: Frist` mit `Frist = Tage(1..=3650) | Gesperrt { datei }` — `Gesperrt`, wenn die Einstellungsdatei beim Start defekt oder nicht lesbar war; an der Grenze `frist_tage` als Zahl bzw. `null` ([ADR-0029](../ADRs/0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md), [ADR-0031](../ADRs/0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md)). Eine Änderung der Frist zählt `stand` hoch und geht unter der Sperre an `mit_veroeffentlichen` wie das Anlegen (ADR-0023 Regel 1). Erster Test: ADR-0031, „Folgeentscheidungen“ (Unit mit Speicherablage, gestellter Uhr, aufzeichnendem Rückruf).
