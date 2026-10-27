# ADR-0008: Identität — Herkunft aus der Peer-Adresse, Nachweis über Sitzungs-Token

**Status**: Vorgeschlagen
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #1 I0001) — wartet auf fachliche Klärung durch den `planer` (siehe Aufgaben)
**Kontext-Anforderung**: — (WBS I0004, I0005, I0006, I0014)
**Erprobt durch**: —

## Kontext

Keine Anmeldung, keine Rechte (Vision). Wer schreiben will, gibt sich einen Namen; die Herkunft „(IP-Adresse oder Rechnername)“ ergänzt ihn zu `name@herkunft`; der Name vor dem `@` ist einmalig, egal von wo. Bricht die Verbindung ab, ist der Teilnehmer sofort offline und nach etwa einer Minute weg, der Name wieder frei. Ein Browser-Tab und ein Agent müssen bei jedem Aufruf nachweisen, wer sie sind. Das Designsystem fragt offen: woran erkennt das System einen Agenten (Frage 3), gilt „gelesen“ je Tab oder je Name (7), was, wenn ein zurückkehrender Tab seinen Namen besetzt findet (8).

## Optionen

### Option A: Herkunft = Peer-Adresse des Sockets; Beitritt liefert ein zufälliges Sitzungs-Token; jeder Aufruf trägt es
- Herkunft: `ConnectInfo<SocketAddr>`, IPv4-gemappte IPv6-Adressen als IPv4; vom Client nicht wählbar.
- `POST /api/beitreten {name, art}` → `{token, kennung}` oder 409 mit `{seit, vorschlag}` („claude-2“). Token: 128 Bit Zufall, nur im Speicher des Servers.
- Nachweis: Header `X-LocalchatA-Sitzung` (bei `EventSource` als Query-Parameter); der Server prüft zusätzlich, dass die Peer-Adresse zur Sitzung passt.
- Browser hält das Token in `sessionStorage` (je Tab, übersteht Neuladen); Agent hält es selbst (Datei oder Variable).
- Vorteile: Herkunft ist nicht fälschbar im Sinne der Vision (sie zeigt, woher die Verbindung kommt); Token macht jeden Aufruf eindeutig zuordenbar; kein Zustand auf Platte.
- Nachteile: hinter NAT oder Proxy teilen sich viele eine Herkunft; `127.0.0.1` statt `localhost`.

### Option B: wie A, Herkunft aber per Reverse-DNS als Rechnername, Rückfall auf die Adresse
- Vorteile: lesbarer (`claude@build-03`), nah an den Beispielen der Vision.
- Nachteile: zusätzliche Abhängigkeit (Standardbibliothek kann keine Rückwärtsauflösung), Wartezeit beim Beitreten, im LAN oft ohne Ergebnis oder uneinheitlich; Herkunft kann sich zwischen zwei Beitritten ändern.

### Option C: Kein Token — Identität = Name im Header plus Peer-Adresse bei jedem Aufruf
- Vorteile: am einfachsten für Agenten (`-H 'X-LocalchatA-Wer: claude'`, wie kanbanD `X-KanbanD-Wer`).
- Nachteile: zwei Tabs desselben Rechners können sich gegenseitig als derselbe Teilnehmer ausgeben; „Name wieder frei“ und „zweiter claude wird abgewiesen“ sind nicht sauber zu unterscheiden, weil keine Sitzung existiert.

## Entscheidung

**Gewählt (vorgeschlagen): Option A**, Option B als spätere Erweiterung (Schalter `--rechnernamen`), wenn der Betreiber lesbare Namen will.

Es überwiegt, dass „Name einmalig“ und „nach einer Minute frei“ eine Sitzung als Träger brauchen; das Token ist der einfachste Träger ohne Anmeldung. Empfehlungen zu den offenen Fragen des Designsystems (fachliche Entscheidung beim `planer`):

- **Frage 3 Agent erkennen:** Feld `art` beim Beitreten (`mensch` | `agent`); die Oberfläche sendet `mensch`, über die API ist `agent` der Standard. Kein Raten aus User-Agent.
- **Frage 7 gelesen:** Lesestand nur im Browser (`localStorage` je Kennung `name@herkunft`); der Server führt keinen Lesestand.
- **Frage 8 Name zurückgewinnen:** gültiges Token und Name frei → derselbe Name; Name inzwischen vergeben → 409 mit Vorschlag, der Tab fragt nach einem neuen Namen.
- **Gnadenfrist** je Sitzung, Standard 60 s (Testknopf `LOCALCHATA_GNADENFRIST_MS`); Agenten dürfen beim Beitreten bis 3600 s verlangen, damit sie über lange Arbeit hinweg ihren Namen behalten.

In Kauf genommen: Herkunft als Adresse statt Rechnername; hinter einem Proxy wäre sie wertlos (Einsatzraum ist das LAN ohne Proxy).

## Konsequenzen

**Positiv:**
- Jede Anfrage trifft an der Systemgrenze ([A00004](../Architektur/A00004-http-schnittstelle.md)) zuerst die Prüfung des Tokens; Fachbausteine bekommen eine geprüfte Kennung.

**Negativ / Inkauf zu nehmen:**
- Nach einem Neustart des Programms sind alle Tokens ungültig; Clients treten neu bei (Oberfläche automatisch mit dem gemerkten Namen).
- Wer das Token eines anderen kennt und von derselben Adresse kommt, kann für ihn schreiben — im LAN ohne Rechte hingenommen (Vision: keine Benutzerverwaltung).

**Folgeentscheidungen / Aufgaben:**
- [ ] `planer`: Fragen 3, 7, 8 und die Gnadenfrist für Agenten bei I0004/I0006/I0014 entscheiden; danach Status `Akzeptiert` oder Ablösung.
- [ ] Erlaubte Zeichen im Namen (Empfehlung: `[a-z0-9-_.]`, 1–32 Zeichen, kein `@`) — `planer` bei I0004.

## Quellen / Bezuege

- [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md); ADR-0003 (Ordnernamen der Direktgespräche), ADR-0007
- Vision, Zielbild „Mensch und KI sind gleichberechtigt“, „Wer da ist, ist sichtbar“; Designsystem `3-offene-fragen.md` Fragen 3, 7, 8; kanbanD `X-KanbanD-Wer` (Q04)

## Ergänzungen

- **2026-10-27 (Karte #2, Nachzug aus #1):** `ConnectInfo<SocketAddr>` steht in Handlern **nicht** zur Verfügung: axum setzt es nur in `axum::serve`, das Programm liefert aber über eine eigene Annahmeschleife aus ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md)). Vor I0004 muss die Schleife die bei `accept` bekannte Gegenstelle je Request als Extension `ConnectInfo<SocketAddr>` einsetzen (`src/start/ausliefern.rs`, `bedienen`), damit Handler wie geplant extrahieren. Die Normalisierung „IPv4-gemappte IPv6 als IPv4“ geschieht einmal dort, sodass Platzzählung je Gegenstelle und Herkunft dieselbe Adresse sehen. Erster Test über das echte Programm: Beitreten von `127.0.0.2` ergibt Herkunft `127.0.0.2`. Der In-Process-Testserver (`axum::serve`) taugt für Herkunftstests nicht, solange er nicht über `ausliefern` läuft. Der Status bleibt `Vorgeschlagen` (fachliche Klärung durch den `planer` offen).
