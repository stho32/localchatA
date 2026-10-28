# ADR-0021: Channel anlegen — `POST /api/channels` für jeden mit Sitzung, höchstens 32 je Herkunft und 256 gesamt

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #6 I0007)
**Kontext-Anforderung**: — (WBS I0007)
**Erprobt durch**: — (folgt der gebauten Systemantwort für Schreibrouten: `grenzschutz`, Extraktor `Teilnehmer`, Grenzen aus ADR-0012 und Ergänzung 2a zu ADR-0018)

## Kontext

- Vision: „Mensch und KI sind gleichberechtigt“; „keine Benutzerverwaltung und keine Rechte“ (ausdrücklich nicht Ziel); „Agenten können warten und reagieren … ohne einen Menschen dazwischen“.
- Grenzregel für Routen mit Wirkung (A00004, Ergänzung #3): nur unter `grenzschutz` (Origin-Prüfung, ADR-0014), Sitzung über `X-LocalchatA-Sitzung` (ADR-0012), JSON mit `Content-Type: application/json`, Körpergrenze (`src/http/koerper.rs`).
- Lehre aus #5 (Security-Review, CWE-770, Ergänzung 2a zu ADR-0018): eine Grenze, die ein billiger Gegenzug wieder aufhebt, ist keine Grenze — `verlassen` gab Plätze sofort frei, `beitreten`/`verlassen` in Schleife erzeugte 2.456 Wechsel/s und fror einen Zuschauer-Tab 28,7 s ein.
- Ein Channel ist **dauerhaft**: Ordner und Einstellungsdatei auf Platte, ein Eintrag im Speicher, eine Zeile in jedem Stand an jeden Strom, eine Zeile in jeder Seitenleiste. Ein „Channel löschen“ gibt es in der Vision und der WBS nicht.
- Herkunft = IP-Adresse der Gegenstelle (ADR-0012); auf einem Rechner teilen sich Mensch und Agenten eine Herkunft (Betreiber und Factory auf `127.0.0.1`). Über IPv6-Privacy-Adressen kann ein Gerät im LAN viele Herkünfte haben.

## Optionen

### Option A: Jeder mit Sitzung; Obergrenzen je Herkunft (dauerhaft gezählt über `angelegt_von`) und gesamt
- Vorteile: Mensch und Agent gleich; die Gesamtgrenze beschränkt alles, was dauerhaft wächst (Platte, Speicher, Standgröße, Ereignisse — ohne Löschen kann es nie mehr als 256 Anlege-Ereignisse je Ordner geben); die Grenze je Herkunft verhindert, dass ein entlaufener Agent die Gesamtgrenze allein erschöpft; ein Neustart setzt nichts zurück, weil die Zählung aus den Einstellungsdateien kommt.
- Nachteile: Mensch und Agenten auf demselben Rechner teilen sich die 32; über viele IPv6-Adressen umgeht ein Angreifer die Grenze je Herkunft (nicht die Gesamtgrenze).

### Option B: Jeder mit Sitzung; Rate je Herkunft (z. B. 10 je Minute) plus Gesamtgrenze
- Vorteile: kurzfristige Fluten werden gebremst, auch wenn später Löschen dazukommt.
- Nachteile: ohne Löschen bremst die Gesamtgrenze die Flut ohnehin (höchstens 256 Ereignisse insgesamt); eine Rate braucht einen Zähler mit Uhr und Fenster, mehr Testfälle (Fenstergrenzen, gestellte Uhr) — Mechanik ohne heutigen Gewinn; erschöpft ein langsamer Agent die Gesamtgrenze trotzdem.

### Option C: Nur Menschen (`art: mensch`) legen an; Agenten nur lesen und schreiben
- Vorteile: kein entlaufener Agent kann Channels anlegen.
- Nachteile: `art` ist Selbstauskunft und ungeprüft (ADR-0012) — die Sperre wäre eine Attrappe; widerspricht „gleichberechtigt“ und dem Anlass der Vision (zwei Factories richten sich ihren gemeinsamen Ort selbst ein).

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass die Gesamtgrenze den dauerhaften Bestand und damit auch die Ereignisflut deckelt und die Grenze je Herkunft sie gegen einen einzelnen Ausreißer verteidigt — beide ohne Uhr und ohne Gegenzug, der sie aufhebt. In Kauf genommen: Mensch und Agenten eines Rechners teilen sich 32 Channels, und wer viele IPv6-Adressen hat, kann die Grenze je Herkunft umgehen, aber nie die Gesamtgrenze.

Regeln:

1. **Route** `POST /api/channels`, Körper `{name, beschreibung?}` als JSON, unter `grenzschutz`, Extraktor `Teilnehmer` (Mensch und Agent gleich). Antworten:
   - 201 `{channel}` — `channel` wie in Liste und Ereignis ([ADR-0023](0023-channels-als-zweites-thema-im-ereignisstrom.md)): `{name, angelegt, angelegt_von, beschreibung}`;
   - 401 ohne gültige Sitzung · 403 fremder `Origin` · 413 Körper zu groß · 415 ohne JSON-`Content-Type`;
   - 409 `{fehler, channel}` — der Name existiert; `channel` ist der vorhandene, damit Oberfläche („Channel öffnen“) und Agent („gibt es schon, gut“) weitermachen;
   - 409 `{fehler, regel: "ordner-vorhanden"}` — `channels/<name>/` liegt im Ordner, ist aber nicht geladen (von Hand angelegt, während das Programm lief); Text: „`channels/<name>` besteht im Ordner schon; localchatA übernimmt ihn beim nächsten Start.“ Ausnahmeweg, nichts wird überschrieben;
   - 422 `{fehler, regel, vorschlag?}` — Namensregel oder reserviert ([ADR-0020](0020-channelname-ist-ordnername-erlaubnisliste.md)), Beschreibung verletzt ihre Regel (ADR-0022);
   - 429 `{fehler, grenze: "herkunft" | "gesamt", hoechstens}` — Grenze erreicht;
   - 500 `{fehler}` — Platte: Ordner oder Datei nicht schreibbar; der angelegte Ordner wird zurückgebaut, eine Zeile auf stderr nennt Pfad und Grund.
2. **`GET /api/channels`** (Sitzung nötig) → 200 `{stand, liste}` nach Name sortiert — für Agenten ohne Strom und für Tests.
3. **Gesamtgrenze** `GESAMT = 256` Channels im Verzeichnis, gezählt einschließlich der beim Start geladenen. Liegen beim Start mehr im Ordner, werden alle geladen und nur neue abgewiesen. Begründung des Werts: 256 Zeilen à 28 px sind 7.168 px Seitenleiste — weit jenseits dessen, was ein Mensch durchsieht; ein Stand mit 256 Einträgen ist ohne Beschreibungen etwa 45 KB groß, mit voller Beschreibung (200 Zeichen, höchstens 800 Byte UTF-8) je Channel höchstens etwa 256 KB — gerechnet, nicht gemessen; im LAN je (Wieder-)Verbindung tragbar.
4. **Grenze je Herkunft** `JE_HERKUNFT = 32` Channels, deren `angelegt_von` diese Herkunft trägt (aus der Einstellungsdatei, also über Neustarts hinweg). Channels ohne bekannten Anleger (von Hand, Einstellungsdatei fehlt oder defekt) zählen nur zur Gesamtgrenze.
5. **Prüfreihenfolge** unter der Sperre des Verzeichnisses: Namensregel → vorhanden (409) → Gesamtgrenze → Grenze je Herkunft → Ordner anlegen → Einstellungen schreiben → eintragen, `stand` + 1, veröffentlichen. Ein vorhandener Name ergibt also 409 auch bei erreichter Grenze — er belegt nichts Neues.
6. **Keine Rate.** Solange es kein Löschen gibt, kann ein Ordner höchstens 256 Anlege-Ereignisse erleben. **Kommt Löschen dazu, wird diese ADR abgelöst**: ein gelöschter Channel gibt seinen Platz erst nach einer Frist frei (Muster Ergänzung 2a zu ADR-0018), sonst öffnet `anlegen`/`löschen` dieselbe Schleife wie #5.
7. **Blockierende Arbeit** (Ordner, Datei, `rename`) läuft über `spawn_blocking` (A00004); die Sperre des Verzeichnisses ist eine `std`-Sperre und wird nur dort genommen.

## Konsequenzen

**Positiv:**
- Kein Weg, die Grenzen ohne Zugriff auf den Ordner zurückzusetzen; ein Neustart ändert nichts.
- Agenten können „anlegen, falls es fehlt“ mit einem Aufruf: 201 oder 409 mit dem vorhandenen Channel.

**Negativ / Inkauf zu nehmen:**
- Ist eine Grenze erreicht, hilft nur Aufräumen von Hand (Programm stoppen, Ordner entfernen, starten) — es gibt kein Löschen.
- Der Betreiber auf `127.0.0.1` teilt sich die 32 mit seinen lokalen Agenten.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein Agent in einer Schleife Channels mit Zeitstempel im Namen anlegt und nach 32 Läufen alle Menschen desselben Rechners blockiert. Früh-Indikator: 429 `grenze: herkunft` im Verlauf eines Agenten. Gegenmaßnahme: die 429-Antwort nennt Grenze und Zahl; der Health-Check zeigt `channels` (Aufgabe unten); die Agenten-Anleitung (I0019) sagt „ein Channel je Zusammenarbeit, nicht je Auftrag“.
2. … ein späteres „Channel löschen“ die Gesamtgrenze wieder billig macht. Früh-Indikator: neue Interaction „löschen“ in der WBS. Gegenmaßnahme: Regel 6 — der architekt löst diese ADR dann ab.
3. … zwei gleichzeitige `POST` mit demselben Namen beide 201 bekommen. Früh-Indikator: Test „20 gleichzeitige Anlagen desselben Namens“. Gegenmaßnahme: Prüfen und Eintragen unter einer Sperre (Regel 5) plus `create_dir` als zweite Sicherung.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Test (Unit, Verzeichnis mit Speicher-Ablage, gestellter Uhr): 32 Anlagen von `10.0.0.1` → 201, die 33. → 429 `herkunft`; eine von `10.0.0.2` → 201; nach erneutem Laden derselben Ablage bleibt die 33. von `10.0.0.1` bei 429 (Grenze über den Neustart). Gesamt: 8 Herkünfte × 32 = 256 → 201, die 257. → 429 `gesamt`, ein vorhandener Name → 409. Mutanten „Grenze 31/33“ und „255/257“ müssen rot werden (Wertprüfung als Spanne, Lehre #5).
- [ ] Erster Integrationstest (echtes Programm): ohne Header 401, fremder `Origin` 403, ohne JSON 415, `Allgemein` 422 mit `vorschlag: allgemein`, `con` 422 `reserviert`, `allgemein` 201 und Ordner samt `einstellungen.toml` existieren, zweimal `allgemein` → 409 mit `channel`; 20 gleichzeitige `POST` desselben Namens → genau eine 201.
- [ ] `/api/gesundheit` nennt `channels` (Anzahl) — Betrieb sieht, wie nah die Gesamtgrenze ist.

## Quellen / Bezuege

- [A00012 Channelverzeichnis](../Architektur/A00012-channelverzeichnis.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md)
- ADR-0012 (Sitzung, Grenzen 64/512), ADR-0014 (Grenzschutz), ADR-0018 Ergänzung 2a (CWE-770), ADR-0020, ADR-0022, ADR-0023
- Vision R00000 (gleichberechtigt, keine Rechte); Logbuch #5 Retrospektive; `AI-Team/gelerntes.md` (LAN-Angriffsfläche)

## Ergänzungen

- **2026-10-28 (Karte #7, Nachzug aus #6) — Beschreibung erst mit F0026:** Regel 1 nennt `{name, beschreibung?}` und `beschreibung` in jeder Antwort. Gebaut ist in I0007 (R00006) nur der Name: eine gesetzte Beschreibung (nicht `null`, nicht `""`) ergibt bis F0026 422 `{fehler, regel: "beschreibung"}`, und `beschreibung` ist in Antwort, Liste und Ereignis immer `null` (`src/http/channels.rs`). Mit F0026 gilt Regel 1 wie geschrieben, die Prüfung nach [ADR-0022](0022-channel-einstellungen-je-channel-als-toml.md) Regel 5. Der sichtbare Ort ist die Kopfzeile ([ADR-0027](0027-channel-waehlen-adresse-als-hash-zuletzt-gewaehlt-je-tab.md) Regel 5).
- **2026-10-28 (Vorlauf „Betriebsprotokoll“) — stderr wird Betriebsprotokoll:** „eine Zeile auf stderr nennt Pfad und Grund“ (500 beim Anlegen): `ERROR channels` mit `channel`, `datei`, `grund`; 429 der Channel-Grenzen neu als `WARNING`-Episode je Herkunft ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regeln 2 und 3; Format [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md), stderr spiegelt ab Warning). Die übrigen Regeln dieser ADR gelten unverändert.
- **2026-10-28 (R00015, F0056; Lastprofil „Team-LAN“) — 128 Channels je Herkunft, Gesamtgrenze 256 bleibt:** Regel 4 gilt ab R00015 mit `JE_HERKUNFT = 128` statt 32; Zählung (über `angelegt_von`, dauerhaft, über Neustarts), Prüfreihenfolge (Regel 5) und Antwort 429 `{fehler, grenze: "herkunft", hoechstens: 128}` bleiben.
  - **Herleitung** (Kommentar der Konstante, `src/gespraeche/verzeichnis.rs`): das Profil hat 50 aktive Channels; legen die Agenten eines Rechners sie an, zählen alle auf dieselbe Herkunft — 32 schnitte genau diesen Fall ab. 128 ≈ 2,5 × 50, weil die Zählung ohne Löschen nie sinkt.
  - **Gesamt 256 bleibt** und deckelt weiter den dauerhaften Bestand (Platte, Stand, Anlege-Ereignisse). Folge: **zwei** Herkünfte mit je 128 erreichen die Gesamtgrenze (vorher acht mit je 32). Der Schutz „eine Herkunft kann nicht alles belegen“ gilt weiter (128 < 256), aber mit kleinerem Abstand; in Kauf genommen, weil das Profil 50 aktive Channels hat und die Gesamtgrenze über Monate ohnehin der engere Rand ist.
  - Die Konsequenz „der Betreiber auf `127.0.0.1` teilt sich die 32 mit seinen lokalen Agenten“ und Pre-Mortem 1 („nach 32 Läufen“) lesen sich ab hier mit 128.
  - **Offen, nicht Teil dieser ADR**: ohne Löschen wächst der Bestand gegen 256; Channel löschen bzw. archivieren ist eine künftige Interaction (Vorschlag an den planer, A00001).
  - Erster Test: wie unter „Folgeentscheidungen“ mit neuen Zahlen — 128 Anlagen von `10.0.0.1` → 201, die 129. → 429 `herkunft`, auch nach erneutem Laden; 2 Herkünfte × 128 = 256 → 201, die 257. → 429 `gesamt`; Mutanten 127/129 und 255/257 (R00015 G3).
- **2026-10-28 (Security-Review #25, Optional; Karte #28) — Namensraum aus zwei Adressen dauerhaft erschöpfbar: Zahlen bleiben, Löschen/Archivieren ist eine Frage an den Betreiber:** Lage (aus `src/gespraeche/verzeichnis.rs:61–66`): 128 je Herkunft, 256 gesamt, gezählt über `angelegt_von`, ohne Löschen nie sinkend — zwei Adressen erschöpfen die Gesamtgrenze dauerhaft, auch über Neustarts. Folge: niemand legt mehr einen Channel an; Lesen, Senden und Frist der bestehenden Channels laufen weiter; Abhilfe nur von Hand (Programm stoppen, Ordner entfernen, starten — Konsequenzen oben).
  - Option A: **`JE_HERKUNFT` = 64** (Vorschlag des `security-reviewer`: trägt das Profil mit 50 aktiven Channels ebenso). Stärke: der Angreifer braucht 4 statt 2 Adressen. Schwäche: der legitime Fall, für den R00015 auf 128 ging — die Agenten eines Rechners legen alle Channels an, die Zählung sinkt nie —, stößt nach 64 statt 128 Anlagen an die Grenze, auch wenn längst wieder nur 50 aktiv sind; im LAN kosten 4 Adressen so wenig wie 2.
  - Option B: **Gesamtgrenze anheben** (z. B. 1.024). Stärke: 8 Adressen nötig; der legitime Fall bekommt Luft. Schwäche: der dauerhafte Bestand, den Regel 3 deckelt (Stand an jeden Strom, Seitenleiste, Platte), wächst auf das Vierfache — ohne Bedarf aus dem Profil.
  - Option C: **Zahlen lassen, Ursache benennen**: ohne Löschen ist jede Grenze nur eine Frage der Zeit — für den Angreifer wie für den legitimen Dauerbetrieb. Die tragfähige Antwort ist „Channel löschen bzw. archivieren“, das Regel 6 schon vorausdenkt (Platz erst nach einer Frist frei).
  - **Gewählt: C.** Begründung wie [ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), Ergänzung „Gesamtgrenze aus wenigen Adressen“: LAN-Insider ohne Anmeldung, dem das System bewusst Schlimmeres überlässt (Frist auf 1, ADR-0031); A und B verschieben nur die Zahl der Adressen bzw. der Anlagen. In Kauf genommen: bis Löschen oder Archivieren kommt, kann ein Insider mit zwei Adressen das Anlegen sperren; der Betreiber sieht es an der Episode `WARNING channels … Channel-Grenze erreicht` (ADR-0042 Regel 2) und räumt von Hand auf.
  - **Offen an den Betreiber** (Umfang, nicht Technik — die Vision kennt kein Löschen): ob „Channel archivieren“ eine künftige Interaction wird. Empfehlung des architekt: ja, als **Archivieren** (Ordner nach `archiv/<name>/` verschieben, Verlauf bleibt lesbar auf der Platte, der Name wird nach einer Frist wieder frei und zählt dann nicht mehr zu `angelegt_von`), nicht als Löschen — es löst Erschöpfung und Dauerbetrieb, ohne eine zweite endgültige Handlung neben der Frist einzuführen. Kommt es, wird diese ADR nach Regel 6 abgelöst.
