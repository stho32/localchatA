---
id: A00010
name: Anwesenheit
code: anwesenheit
ebene: Komponente
eltern: A00002
repo: localchatA
status: Aktiv
herkunft: dialog
quellen:
  - src/anwesenheit/
verwandt: [A00011]
domaene: []
blackbox: ein Zustandsautomat im Speicher; Regeln in ADR-0012, ADR-0013 und ADR-0016, Verfeinerung mit I0004–I0006
---

# Anwesenheit

## Verantwortung

Führt je Sitzung einen eindeutigen Namen samt Status online, offline oder weg.

## Schnittstellen

- `beitreten(name: Name, herkunft: IpAddr, art) -> Result<Sitzung{token, kennung}, NameVergeben{belegt_von, seit, vorschlag} | ZuVieleSitzungen>` — `Name` ist schon nach [ADR-0013](../ADRs/0013-namensregel-als-erlaubnisliste.md) geprüft; Herkunft kanonisch aus der Annahmeschleife ([ADR-0012](../ADRs/0012-identitaet-und-sitzung.md)).
- `wiederkehren(token, herkunft) -> Option<Sitzung>` — gültiges Token derselben Herkunft; zählt als Lebenszeichen. Grundlage von `beitreten` mit Header und vom Extraktor `Teilnehmer` an der [HTTP-Schnittstelle](A00004-http-schnittstelle.md).
- `verbunden(token) -> (Halter, Vec<Wechsel>)` (ab I0005) — solange ein Halter lebt (Ereignisstrom oder Warte-Aufruf, [ADR-0007](../ADRs/0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md)), ist die Sitzung online; mehrere Halter je Sitzung sind erlaubt; der `Drop` des letzten setzt das Halter-Ende und ist ein Lebenszeichen ([ADR-0016](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)).
- `pflegen(jetzt) -> Vec<Wechsel>` (ab I0005, vom Takt des Starts alle 250 ms gerufen) — Nachlauf vorbei ohne Halter → `offline`; Gnadenfrist ohne Lebenszeichen vorbei → `weg`, Name frei ([ADR-0016](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md); ob `weg` schon mit I0005 gemeldet wird, entscheidet der Zuschnitt). Beim Zugriff wird „weg“ weiterhin auch ohne Takt berechnet.
- `teilnehmer() -> Stand{stand, liste: Vec<{kennung, name, herkunft, art, status, seit}>}` (ab I0005) — alle lebenden Sitzungen, nach Kennung sortiert; `status` ist `online` | `offline`.
- Ausgehend: keine Aufrufe — jeder Statuswechsel kommt als `Wechsel{stand, teilnehmer, status, seit}` aus `beitreten`, `verbunden` und `pflegen` zurück; veröffentlicht wird er von der Integration (Handler, Takt) über den [Live-Kanal](A00011-live-kanal.md). `stand` zählt unter der Sperre hoch ([ADR-0016](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)).
- `verlassen(token, herkunft) -> Vec<Wechsel>` (ab I0006) — gültiges Token derselben Herkunft: Sitzung entfernt, Wechsel `weg` unter der Sperre veröffentlicht, Name frei; sonst nichts (idempotent, [ADR-0018](../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md)). Jede Teilnehmerzeile trägt ab I0006 `weg_um` (bei `offline` letztes Lebenszeichen + Gnadenfrist, sonst keins), jeder Stand und Wechsel `jetzt`; ein Lebenszeichen ohne Halter, das `weg_um` um ≥ 1 s verschiebt, ist ein Wechsel ([ADR-0019](../ADRs/0019-restzeit-weg-um-in-serverzeit-und-statusformen.md)).

## Abhaengigkeiten

Uhr-Provider (Aspekt [Zeit](Aspekte/X00007-zeit.md)); Provider `Zufall` für Tokens (Betriebssystem-Zufall über `getrandom`, im Test feste Folge). Den [Live-Kanal](A00011-live-kanal.md) ruft sie nicht selbst (ab 2026-10-28, ADR-0016).

## Entscheidungen

- [ADR-0008: Identität — Herkunft aus der Peer-Adresse, Nachweis über Sitzungs-Token](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md) — Abgelöst durch ADR-0012, 2026-10-27
- [ADR-0012: Identität und Sitzung — Herkunft aus der Gegenstelle, Sitzungs-Token im Speicher, eine Sitzung je Browser](../ADRs/0012-identitaet-und-sitzung.md) — Akzeptiert, 2026-10-27
- [ADR-0013: Namensregel als Erlaubnisliste — `[a-z0-9-]`, 1 bis 32 Zeichen, abweisen statt umformen](../ADRs/0013-namensregel-als-erlaubnisliste.md) — Akzeptiert, 2026-10-27
- [ADR-0016: Anwesenheitsstatus aus Haltern mit Nachlauf, Teilnehmerliste als Stand plus Wechsel](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md) — Akzeptiert, 2026-10-28
- [ADR-0017: Stiller Abriss — der Kernel erkennt tote Verbindungen (Abrissgrenze und Keepalive je Verbindung), Herzschlag 5 s](../ADRs/0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md) — Vorgeschlagen, 2026-10-28
- [ADR-0018: `POST /api/verlassen` — sofort `weg`, Name frei; die Oberfläche ruft es nie beim Schließen](../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md) — Akzeptiert, 2026-10-28
- [ADR-0019: Restzeit `weg_um` in Serverzeit mit Uhrenabgleich über `jetzt`; gestrichelt heißt „gleich weg“](../ADRs/0019-restzeit-weg-um-in-serverzeit-und-statusformen.md) — Akzeptiert, 2026-10-28
- [ADR-0051: Aktivität „tippt“ / „denkt nach“ — flüchtig im Speicher mit Ablauf, eigener Signalbus, eine Route je Gespräch, eigene Bremse](../ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) — Akzeptiert, 2026-10-30

## Offene Fragen

- Gnadenfrist für Agenten: Standard 60 s für alle, ein Agent verlangt beim Beitreten bis zu 3600 s (`gnadenfrist_s`)? Empfehlung: ja, mit I0014. Fachlich, für den `planer`.

## Notizen / Quellen

- **Datenhoheit:** einziger Schreiber aller Sitzungen (Token, Name, Herkunft, Art, Gnadenfrist, letztes Lebenszeichen, Halterzahl); alle anderen lesen über die Schnittstelle. Nur im Speicher — ein Neustart beendet alle Sitzungen; Tokens kommen nie auf Platte (der Ordner ist für Dritte lesbar).
- **Lebensdauer** ([ADR-0012](../ADRs/0012-identitaet-und-sitzung.md)): lebt, solange Halter da sind oder das letzte Lebenszeichen höchstens die Gnadenfrist zurückliegt (Standard 60 s, Testknopf `LOCALCHATA_GNADENFRIST_MS`). Grenzen: 64 Sitzungen je Herkunft, 512 gesamt; mitgezählt werden auch durch `verlassen` entfernte Plätze, bis ihre Gnadenfrist ab dem `verlassen` abgelaufen ist — der Name ist sofort frei ([ADR-0018](../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md), Ergänzung Regel 2a, Security-Review #5).
- **Nebenläufigkeit:** ein `Mutex` um den ganzen Zustand; jede Operation ist kurz und ohne Ein-/Ausgabe, darum kein `spawn_blocking` ([X00008](Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)). Prüfen und Belegen eines Namens geschehen unter derselben Sperre — zwei gleichzeitige `beitreten("claude")` ergeben genau einen Erfolg.
- **Erster Test (Unit, Uhr-Attrappe, feste Zufallsfolge):** `beitreten(claude, 10.0.0.5, agent)` → Kennung `claude@10.0.0.5`, Token aus der Folge; `beitreten(claude, 127.0.0.1, mensch)` → `NameVergeben{belegt_von: claude@10.0.0.5, vorschlag: claude-2}`; Uhr +59 s → weiterhin vergeben; `wiederkehren(token, 10.0.0.5)` bei +59 s → Sitzung, Lebenszeichen erneuert; Uhr +61 s ab dort → `beitreten(claude, 127.0.0.1)` ok; `wiederkehren(alter_token, 10.0.0.5)` → keine Sitzung; `wiederkehren(token, 10.0.0.6)` → keine Sitzung (andere Herkunft). Ersetzt: Uhr, Zufall.
- **Erster Test (Integration, echtes Programm):** zwei Absender `127.0.0.1` und `127.0.0.2`; `POST /api/beitreten {"name":"claude"}` von `.2` → 201 `claude@127.0.0.2`; derselbe Name von `.1` → 409 mit `vorschlag: "claude-2"`; `GET /api/ich` mit Token von `.2` → 200, mit Token von `.1` → 401.
- Betrieb: kein eigener Start, kein Port; Gesundheit kann später `sitzungen` nennen (Früh-Indikator für Namensbesetzung). Fehler sichtbar als 409/422/429 mit deutschem `fehler`-Text, nie still.
- **Stand 2026-10-27 (Karte #3):** Entwurf nach ADR-0012/0013 angeglichen; die frühere Notiz „Tab kehrt mit seinem Token zurück“ (je Tab, ADR-0008) gilt nicht mehr — eine Sitzung je Browser (`localStorage`).
- **Ergänzung 2026-10-28 (Karte #4, I0005):** Status nach [ADR-0016](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md): `online` ⇔ Halter > 0 oder weniger als 1 s (`NACHLAUF`) seit Beitritt bzw. Ende des letzten Halters; `offline` ⇔ kein Halter, Nachlauf vorbei, Sitzung lebt; `weg` ⇔ Gnadenfrist ohne Lebenszeichen vorbei. Neu im Bestand je Sitzung: Halterzahl, Ende des letzten Halters, zuletzt gemeldeter Status; neu im Ganzen: `stand` (u64). Ein Browser hält höchstens einen Halter je Sitzung (ein Strom je Browser, [ADR-0015](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)); mehrere Halter entstehen im Rückfall je Tab und mit Warte-Aufrufen (I0016). Der Halter hält eine Referenz auf die Anwesenheit (`Arc`) und nimmt im `Drop` kurz die Sperre — ohne `await`, ohne Ein-/Ausgabe.
  - **Erster Test (Unit, gestellte Uhr, feste Zufallsfolge):** siehe ADR-0016, „Folgeentscheidungen“ — Anlauf, Nachlauf 999 ms/1 s, zweiter Halter, Wiederkehr eines Halters im Nachlauf ohne Wechsel, `stand` streng steigend. Ersetzt: Uhr, Zufall. Kein Bus nötig, weil Wechsel Rückgabewerte sind.
  - Betrieb: Gesundheit kann `sitzungen` und `halter` nennen (Früh-Indikator für Namensbesetzung und hängende Ströme, Vorschlag an den `planer`).
- **Ergänzung 2026-10-28 (Karte #5, I0006):** (1) **Veröffentlichen unter der Sperre** (Nachzug aus #4, Commits b3645cf, f6e446f): die Integration gibt eine synchrone Funktion mit (`mit_veroeffentlichen`), die Anwesenheit ruft sie für jeden Wechsel, solange sie die Sperre hält; „jetzt“ wird unter der Sperre gelesen und ist monoton (`bestand_und_jetzt`). Die frühere Zeile „Ausgehend: keine Aufrufe … veröffentlicht wird er von der Integration“ gilt so nicht mehr — ausgehend ist genau diese eine eingesetzte Funktion; den Bus kennt die Anwesenheit weiter nicht ([ADR-0016](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md), Ergänzungen). (2) **Stiller Abriss** braucht hier nichts Neues: der Halter fällt, wenn die Annahmeschleife die Verbindung als tot erkennt ([ADR-0017](../ADRs/0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md)); sein `Drop` ist das Lebenszeichen, ab dem Nachlauf und Gnadenfrist laufen. (3) **`verlassen`** und **`weg_um`** siehe Schnittstellen. Neu im Bestand je Sitzung: das zuletzt gemeldete `weg_um` (für die Drossel ≥ 1 s).
  - **Erster Test (Unit, gestellte Uhr):** siehe ADR-0018 und ADR-0019, „Folgeentscheidungen“ — `verlassen` mit eigener, fremder und unbekannter Herkunft; `weg_um` = Halter-Ende + 60 s, `weg` erst bei + 60 s + 1 ms, Drossel 0,5 s / 1,1 s. Ersetzt: Uhr, Zufall, Veröffentlichen (aufzeichnende Funktion — prüft zugleich, dass unter der Sperre veröffentlicht wird).
- **Ergänzung 2026-10-28 (R00015, F0056):** Grenzen jetzt **128 Sitzungen je Herkunft**, 512 gesamt — nicht mehr gleich der Verbindungsgrenze (160); Herleitung aus dem Lastprofil (51 Sitzungen, bis 101 bei Neustarts mit neuem Namen) und die Annahme „Agenten nutzen die Wiederkehr“ in [ADR-0012](../ADRs/0012-identitaet-und-sitzung.md), Ergänzung R00015. Die Zählregel (verlassene bis zum Ende der Gnadenfrist) bleibt. Soll ab F0056, Ist bis dahin 64 (`src/anwesenheit/bestand.rs:19`).
- **Ergänzung 2026-10-30 (Karte #77), Soll:** Die Integration, die Wechsel veröffentlicht (`live::bus::anwesenheit_veroeffentlichen`), ruft nach dem Ereignis `anwesenheit` bei `weg` (jede Art) und bei `offline` einer Sitzung der Art `mensch` `aktivitaet.beenden_alle(kennung)` ([Aktivität](A00019-aktivitaet.md)). Sperrfolge Anwesenheit → Aktivität; die Aktivität ruft die Anwesenheit nie. Die Anwesenheit selbst bleibt unverändert. Quelle: [ADR-0051](../ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) Regel 7 d, e (F5, Bestätigung beim product-owner erfragt).
