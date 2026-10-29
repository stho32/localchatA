---
id: R00012
status: Neu
datum: 2026-10-28
---

# R00012: Nachricht senden bei Störungen und mit Komfort

## Beschreibung

Mit R00011 kommt eine Nachricht an, wenn alles läuft. R00012 sorgt dafür, dass beim Senden auch bei Störungen nichts verloren geht und nichts doppelt ankommt. Bricht die Verbindung ab, wartet eine geschriebene Nachricht „wird gesendet …“ und geht nach dem Wiederverbinden genau einmal hinaus — auch wenn die erste Antwort unterwegs verloren ging; dafür sorgt ein Wiederholschlüssel, den auch Agenten nutzen können. Antwortet der Server nicht, wird die Nachricht nach 10 s als fehlgeschlagen markiert und lässt sich erneut senden oder verwerfen. Nach dem Wiederverbinden holt der Tab nach, was in der Zwischenzeit geschrieben wurde, und sagt, wie viel es war. Ein angefangener Text bleibt je Channel im Tab erhalten, die Eingabe warnt vor der Längengrenze und vor Steuerzeichen, bevor gesendet wird, und wer mit einem Screenreader liest, hört neue Nachrichten gebündelt statt als Dauerton.

Zahlt ein auf: [Vision](R00000-vision.md) — „Live überall“ (auch nach einem Abriss sieht jede Sicht alle Nachrichten), „Das Log ist das Gedächtnis“ (keine Nachricht doppelt im Log), „Mensch und KI sind gleichberechtigt“ (der Wiederholschlüssel gilt für Browser und Agent gleich), „Über Rechnergrenzen“ (WLAN-Wackler im LAN sind der Normalfall).

Entschieden vom architekt (ADR-0034 Regeln 3.7, 3.9, 6; ADR-0037 Regeln 4, 5, 7; ADR-0038 Regeln 2, 4–8, 10; Vision nicht berührt): `Idempotency-Key` je Kennung im Speicher (4 096 Einträge, 10 min); Warteschlange ruht bei getrenntem Strom; Zeitgrenze 10 s; Nachholen mit `nach=` in Blöcken; Entwurf in `sessionStorage`; Ansage in eigener Statusregion, höchstens alle 10 s.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0020 Nachricht senden bei Störungen und mit Komfort; geteilt aus I0009 am 2026-10-28 durch den Director, weil 47 Bubbles): F0046 Fehler und Wiederholung, F0047 Lücken nach Wiederverbinden, F0048 Entwurf, Längenanzeige und Textprüfung im Browser, F0049 Gebündelte Ansage.

## Geschäftlicher Nutzen

Im LAN wackelt das WLAN, Laptops schlafen ein, die Instanz wird neu gestartet. Ohne R00012 bleibt eine Nachricht, die in diesem Moment geschrieben wurde, für immer „wird gesendet …“, oder sie steht nach einem Wiederholen doppelt im Channel — bei Agenten, die auf Nachrichten reagieren, heißt doppelt: doppelt gearbeitet. Wer nach einem Abriss wieder online ist, sieht ohne R00012 die Nachrichten aus der Zwischenzeit nicht, bis er neu lädt. R00012 macht das Gespräch verlässlich: was geschrieben ist, kommt genau einmal an, was verpasst wurde, wird nachgeholt. Dazu drei Dinge für den Alltag: ein halb geschriebener Text geht beim Wechsel des Channels nicht verloren, ein zu langer oder mit Terminal-Farbcodes eingefügter Text wird vor dem Senden erklärt statt abgewiesen, und Screenreader-Nutzer können einem Channel folgen, in dem Agenten im Sekundentakt schreiben.

## Funktionale Anforderungen

### Fehler und Wiederholung (F0046; ADR-0034 Regeln 3.7, 3.8, 3.9, 6; ADR-0037 Regel 4; ADR-0038 Regeln 5, 6)

- **Wiederholschlüssel**: `POST /api/channels/{name}/nachrichten` nimmt einen optionalen Kopf `Idempotency-Key`. Erlaubt sind 1–64 Zeichen aus `[A-Za-z0-9_-]`, sonst 422 `{fehler, regel:"schluessel"}` (geprüft nach der Textregel, vor der Restbuchung).
- Gemerkt wird unter der Schreibsperre `(Kennung, Schlüssel) → (Channel, Zeit, Prüfsumme des Texts wie gesendet)`, höchstens die 4 096 jüngsten Einträge und höchstens 10 Minuten (600 000 ms) alt, nur im Speicher. Derselbe Schlüssel derselben Kennung mit gleichem Text und Channel → **200** `{channel, nachricht}` mit der ursprünglichen Nachricht, ohne Satz und ohne Ereignis; mit anderem Text oder anderem Channel → 422 `regel:"schluessel"`, Text „Dieser Schlüssel gehört zu einer anderen Nachricht.“. Ein Schlüssel einer anderen Kennung ist ein anderer Schlüssel. Ein Schlüssel erscheint nie in Log, Antwort oder Ereignis.
- Eine Wiederholung bucht am Schreibkonto wie ein erster Versuch; bekommt sie 429, ist nichts geschehen.
- **Im Tab** (die Zustellung aus R00011 wird erweitert):
  - **Strom getrennt** (Leiste „Verbindung verloren“): gesendete Nachrichten bleiben „wird gesendet …“; die Warteschlange ruht — kein `POST`, solange getrennt; ein Sendeauftrag, dessen Antwort ausbleibt, während der Strom getrennt ist, bleibt „wird gesendet …“. Nach dem Wiederverbinden (und nach dem Nachholen, F0047) läuft die Warteschlange mit denselben Schlüsseln weiter.
  - **Zeitgrenze**: ohne Antwort **10 s** nach Beginn des Abrufs (`AbortController`) wird die Nachricht fehlgeschlagen mit „Keine Antwort vom Server.“ (bei verbundenem Strom).
  - **401**: der vorhandene stille Wiederbeitritt (ADR-0012) und **genau ein** neuer Versuch mit demselben Schlüssel; scheitert er, fehlgeschlagen mit dem `fehler` der Antwort.
  - **Schalter** an jeder fehlgeschlagenen Nachricht: „Erneut senden“ stellt sie mit demselben Schlüssel ans Ende der Warteschlange („wird gesendet …“); „Verwerfen“ entfernt sie, ohne den Text ins Feld zurückzulegen. Nach beiden liegt der Fokus im Feld. Die Schalter liegen in der Tab-Reihenfolge des Verlaufs.

### Lücken nach Wiederverbinden (F0047; ADR-0037 Regel 7, ADR-0028 Regel 6)

- Nach jedem Wiederverbinden des Stroms und nach `NeuerStand` holt der Tab für den offenen Channel `GET …/nachrichten?nach=<jüngste bestätigte Zeit>&n=200` in Blöcken, bis `neuer` falsch ist, höchstens **5** Blöcke; gilt danach noch `neuer`, öffnet er den Channel neu wie „Zum Neuesten“. Nachgeholte Nachrichten werden über die Zeit eingeordnet (R00011), jede genau einmal.
- Die Leiste zeigt danach **4 s** lang „✓ Wieder verbunden · n nachgeladen“ (n = Zahl der nachgeholten Nachrichten, mit Tausenderpunkt), „✓ Wieder verbunden“ bei n = 0, „✓ Wieder verbunden · neueste geladen“ nach einem Neu-Öffnen.
- Erst nach dem Nachholen läuft die Warteschlange weiter (F0046) — so stehen fremde Nachrichten aus der Zeit der Trennung vor den eigenen gepufferten, wie es ihre Zeiten verlangen.
- **Nichts geht zwischen Nachholen und Strom verloren** (Nacharbeit #22): Ereignisse `nachricht` des offenen Channels werden ab dem ersten Ereignis des neuen Stroms angenommen, nicht erst nach dem Nachholen. Was nachgeholt und was live empfangen wird, steht genau einmal im Verlauf, in der Folge seiner Zeiten — auch wenn eine live empfangene Nachricht jünger ist als die nachgeholten, und ebenso nach `NeuerStand`.
- **Scheitert das Nachholen** (Nacharbeit #22): jeder Abruf `nach=` bricht 10 s nach seinem Beginn ab (`AbortController`, wie die Zeitgrenze des Sendens). Scheitert ein Abruf (Netzfehler, Zeitgrenze, Status außer 200), zeigt die Leiste nie „✓ Wieder verbunden“, solange Nachrichten der Lücke fehlen: sie bleibt „⚠ Verbindung verloren“, die Warteschlange ruht, und der Tab versucht das Nachholen still erneut — 1 s, 2 s und 5 s nach dem ersten, zweiten und dritten gescheiterten Versuch, danach alle 10 s (der Rückzug des Stroms). Erst nach einem gelungenen Nachholen kommt die Meldung (n zählt alle nachgeholten Nachrichten) und läuft die Warteschlange weiter. Nach `NeuerStand` gilt dasselbe ohne Leiste und ohne Ruhen der Warteschlange. Eine erneute Trennung beendet die Versuche (das Wiederverbinden beginnt neu); ein Channelwechsel ebenso — der neue Channel wird vollständig geöffnet, die Leiste zeigt „✓ Wieder verbunden“, die Warteschlange läuft weiter.

### Entwurf, Längenanzeige und Textprüfung im Browser (F0048; ADR-0038 Regeln 2, 4, 7, 8)

- **Textprüfung vor dem Senden** mit derselben Regel wie der Server (Länge in Bytes UTF-8 über `TextEncoder`, dieselben verbotenen Zeichen, Stelle ab 0): bei Verstoß sendet `Enter` nicht.
- **Hinweiszeile** (20 px unter dem Feld): unter 15 000 Bytes leer; ab 15 000 „noch n Zeichen“ in `kd-ink-muted` mit n = 16 384 − Bytes; über 16 384 „⚠ Zu lang: 17.020 von höchstens 16.384 Byte — bitte teilen.“ in `kd-danger`; bei einem verbotenen Zeichen „⚠ Steuerzeichen U+001B an Stelle 17 — bitte entfernen.“ (Stelle ab 1) in `kd-danger`. Bei einem Fehler trägt das Feld `aria-invalid="true"`, sonst nicht. Angesagt (`aria-live="polite"`) wird nur beim Überschreiten der Schwelle 15 000 und beim Wechsel in einen Fehler, nicht je Taste. Umgeformt wird nichts.
- **Entwurf**: je Channel in `sessionStorage` unter `localchatA.entwurf.<channel>`; geschrieben bei Eingabe (höchstens alle 500 ms) und beim Verlassen des Channels; beim Öffnen eines Channels ins Feld gelegt; gelöscht beim Senden und wenn das Feld leer wird. Er übersteht Channel-Wechsel und Neuladen des Tabs, ein anderer Tab sieht ihn nicht; nie in `localStorage`.
- **Fokus**: `/` außerhalb von Textfeldern und `Esc` im Verlauf setzen den Fokus ins Feld (das `/` wird nicht eingefügt); beim Channel-Wechsel bleibt der Fokus, wo er ist, im Feld jetzt mit dem Entwurf des neuen Channels.

### Gebündelte Ansage (F0049; ADR-0038 Regel 10, ADR-0026 Regel 9)

- Eine Statusregion `role="status"` `aria-live="polite"` außerhalb des Verlaufs, visuell verborgen.
- Angesagt werden **fremde** Nachrichten des **offenen** Channels: ist seit der letzten Ansage mindestens 10 s vergangen und nichts gesammelt, sofort „Neue Nachricht von beta@127.0.0.1 in # allgemein“; sonst gesammelt und frühestens 10 s nach der letzten Ansage als eine Ansage „4 neue Nachrichten in # allgemein“ (Singular „1 neue Nachricht in # allgemein“).
- Eigene, beim Öffnen oder Scrollen nachgeladene und nach einem Abriss nachgeholte Nachrichten werden nie angesagt; Nachrichten anderer Channels auch nicht.
- Ein Fehlschlag einer eigenen Nachricht wird **sofort** angesagt, unabhängig von der Bündelung: „Nachricht nicht gesendet: <Grund>“.

## Nicht-funktionale Anforderungen

- **Zeiten** — die Grenzen stehen nur hier; Tests übernehmen sie wörtlich und sind nie schärfer. Die Regeln (10 s, 4 s, 10 s Ansage, 10 min Schlüssel) prüfen Node- und Unit-Tests exakt mit gestellter Zeit; die Tabelle nennt die Zustellung. Z6–Z8 wie in ADR-0037.

  | # | Was | Art | Startpunkt der Uhr / Messweg | erwartet | Grenze | Zweck |
  |---|---|---|---|---|---|---|
  | Z6 | Lücke nach Abriss | Gate (E2E) | erstes Ereignis des neuen Stroms im Tab B → Leiste „✓ Wieder verbunden · 3 nachgeladen“ und die 3 Nachrichten sichtbar | ≈ 50 ms + ein Abruf | < 2 s | wer wieder online ist, sieht ohne Neuladen, was er verpasst hat |
  | Z7 | Gepuffert senden | Gate (E2E) — Regel | Strom getrennt, A sendet 2, Strom wieder | beide genau einmal im Verlauf von A und B, in Sendereihenfolge | exakt | Designsystem „gepuffert“ |
  | Z8 | Antwort verloren, Wiederholung | Gate (E2E) — Regel | Antwort geht verloren; Wiederholung mit demselben Schlüssel | genau ein Satz im Log, eine Nachricht im Verlauf | exakt | Wiederholschlüssel |
  | Z11 | Zeitgrenze ohne Antwort | Gate (E2E) — Zustellung; Regel exakt im Node-Test | `Enter` → „Keine Antwort vom Server.“ sichtbar; Route antwortet nie | 10 s + ein Frame | 9 s nach `Enter` noch „wird gesendet …“, spätestens 13 s nach `Enter` fehlgeschlagen | ein toter Server lässt keine Nachricht ewig hängen |
  | Z12 | Entwurf gespeichert | Gate (E2E) | letzte Taste → Eintrag in `sessionStorage` | ≤ 500 ms | 1 s | Neuladen kurz nach dem Tippen verliert nichts |
  | Z13 | Nachhol-Abruf ohne Antwort | Gate (E2E) — Zustellung; Regel exakt im Node-Test | Beginn des ersten Abrufs `nach=`; die Route antwortet auf ihn nie | 10 s Zeitgrenze + 1 s Rückzug = 11 s bis zum zweiten Abruf | 9 s nach Beginn noch kein zweiter Abruf, spätestens 15 s nach Beginn läuft er | ein hängender Abruf hält Lücke und Warteschlange nicht für immer fest (Nacharbeit #22) |

  Reserve: Z6 > 10 × über dem erwarteten Wert; Z11 30 % über der Regel (10 s → 13 s); Z12 das Doppelte der Schreibgrenze; Z13 36 % über dem erwarteten Wert (11 s → 15 s).
- **Sicherheit**: der Wiederholschlüssel gilt je Kennung — ein fremder Absender kann ihn weder treffen noch auslesen; Speicher begrenzt (4 096 Einträge, 10 min); Format abgewiesen, nie umgeformt; eine Wiederholung bucht am Schreibkonto.
- **Benutzerfreundlichkeit**: Fehler an der Nachricht mit ⚠ und Wort; Fokus nach „Erneut senden“, „Verwerfen“ und `/`/`Esc` im Feld; höchstens eine Ansage je 10 s außer bei Fehlschlägen; bei 320 px kein Querscrollen, auch mit Hinweiszeile und Schaltern.
- **Testbarkeit**: Zustellung, Nachholen, Hinweis, Fokus-Tasten und Ansage als reine Funktionen mit Node-Tests und gestellter Zeit; Abrisse im E2E über `context.setOffline` und `page.route`.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien wie in R00011: Start mit `TZ=Europe/Berlin localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner mit `channels/allgemein/` und `channels/technik/`; `S(<name>)`, `POST`, `alpha` (Strom-Leser), Browser A (`shoff`) und B (`anna`) wie dort. „Strom getrennt“ heißt im E2E: `context.setOffline(true)` für den Browser-Kontext, bis die Leiste „Verbindung verloren“ steht; „Strom wieder“: `context.setOffline(false)`. Anfragen aus dem Testprozess (API, `route.fetch()`) sind davon nicht betroffen.

### F0046 Fehler und Wiederholung

- [ ] Schlüssel merken (Unit, rein, gestellte Uhr): `(beta, k1)` mit Text `x` an `allgemein` → neu; dasselbe noch einmal → wiederholt mit derselben Zeit; `k1` mit Text `y` → anders; `k1` an `technik` → anders; `k1` von `gamma` → neu; vom Merken an nach 600 000 ms → wiederholt, nach 600 001 ms → neu; nach 4 097 weiteren Schlüsseln ist der älteste vergessen (neu), der 4 096-jüngste noch bekannt. Die Mutanten „10 min − 1 s“, „10 min + 1 s“, „4 095 Einträge“ und „4 097 Einträge“ werden je rot.
- [ ] Format (Integration): `Idempotency-Key` `a`, `A-z_0-9` und 64 × `a` → 201; leer, 65 × `a`, `a b`, `ä`, `a.b` → je 422 `regel:"schluessel"`; nichts geschrieben.
- [ ] Wiederholung (Integration): `beta` sendet `Hallo` mit `k1` → 201 mit Zeit `t`; noch einmal → **200** mit derselben Nachricht (Zeit `t`); `Hallo!` mit `k1` → 422 `regel:"schluessel"` mit „Dieser Schlüssel gehört zu einer anderen Nachricht.“; danach `drei` ohne Schlüssel → das nächste Ereignis bei `alpha` nach `Hallo` ist `drei` (die 200 und die 422 erzeugten keines); die Tagesdatei enthält `Hallo` genau einmal; kein `k1` in Datei, Antworten oder Ereignissen.
- [ ] Gleichzeitig (Integration): 20 gleichzeitige `POST` von `beta` mit `k2` und `Hallo` → genau eine 201, 19 × 200, alle mit derselben Zeit; ein Satz im Log; `alpha` liest genau ein Ereignis dazu.
- [ ] Wiederholung bucht (Integration): `beta` sendet 30 kleine Nachrichten; die Wiederholung der 30. mit ihrem Schlüssel unmittelbar danach → 429 (keine Ausnahme vom Konto); nach `Retry-After` dieselbe Wiederholung → 200 mit der ursprünglichen Zeit.
- [ ] Zustellung erweitert (Node-Test, rein, gestellte Zeit): (a) Strom getrennt, senden `x` → vorläufig, kein Auftrag; Strom wieder → Auftrag `x` mit seinem Schlüssel. (b) Auftrag läuft, Strom trennt, Netzfehler → bleibt vorläufig; Strom wieder → derselbe Auftrag mit demselben Schlüssel. (c) Strom verbunden, keine Antwort: bei 9 999 ms vorläufig, bei 10 000 ms fehlgeschlagen „Keine Antwort vom Server.“, der nächste Auftrag läuft (Mutanten 9 s und 11 s werden rot). (d) 401 → Wiederbeitritt, ein Versuch mit demselben Schlüssel; zweite 401 → fehlgeschlagen mit dem `fehler` der Antwort, kein dritter Versuch. (e) „Erneut senden“ → ans Ende der Warteschlange, derselbe Schlüssel; „Verwerfen“ → entfernt. (f) eigenes Ereignis kommt vor dem Netzfehler → bestätigt, der Netzfehler danach ändert nichts.
- [ ] Gepuffert (E2E, **Z7**): A und B haben `# allgemein` gewählt; Strom von A getrennt; A sendet `p1` und `p2` → beide „wird gesendet …“, laut Netzprotokoll kein `POST`; Strom wieder → `p1` und `p2` stehen in dieser Folge genau einmal bestätigt im Verlauf von A und von B; der Verlauf der API enthält beide genau einmal.
- [ ] Antwort verloren (E2E, **Z8**): A sendet `einmal`; die Route des `POST` trennt zuerst den Strom (`setOffline(true)`), leitet die Anfrage dann mit `route.fetch()` an den Server und bricht die Antwort an die Seite ab (`route.abort()`) → `einmal` bleibt „wird gesendet …“; Strom wieder → `einmal` steht genau einmal bestätigt im Verlauf von A; die Tagesdatei enthält genau einen Satz `einmal`; die Wiederholung trug denselben `Idempotency-Key` und bekam 200.
- [ ] Erneut senden (E2E): die Route bricht den ersten `POST` ab, ohne ihn weiterzuleiten (Strom verbunden) → `zwei` zeigt ⚠ „Keine Antwort vom Server.“ und die Schalter „Erneut senden“ und „Verwerfen“; Klick auf „Erneut senden“ → `zwei` steht genau einmal bestätigt im Verlauf, ein Satz im Log; der Fokus liegt im Feld.
- [ ] Verwerfen (E2E): wie oben mit `drei`, Klick auf „Verwerfen“ → `drei` ist nicht mehr im Verlauf, das Feld ist leer, der Fokus liegt im Feld; kein Satz `drei` im Log.
- [ ] Zeitgrenze (E2E, **Z11**): die Route hält den `POST` ohne Antwort fest; A sendet `still` → 9 s nach `Enter` steht noch „wird gesendet …“, spätestens 13 s nach `Enter` ⚠ „Keine Antwort vom Server.“
- [ ] 401 (E2E): die Route antwortet auf den ersten `POST` 401 und lässt danach durch → A tritt still wieder bei (`POST /api/beitreten` mit dem gemerkten Namen), der zweite `POST` trägt denselben Schlüssel, `vier` steht genau einmal bestätigt im Verlauf; keine Fehlermeldung.
- [ ] Schmal (E2E): bei 320 px mit einer fehlgeschlagenen Nachricht samt Schaltern `scrollWidth <= innerWidth`.
- [ ] Konsole bei A und B: keine unbehandelte Ausnahme, kein CSP-Verstoß; Konsolenfehler höchstens „Failed to load resource“ zur gestellten 401 in „401“, zu den abgebrochenen Anfragen in „Antwort verloren“, „Erneut senden“, „Verwerfen“ und „Zeitgrenze“ und zu Anfragen, die bei getrenntem Netz scheitern („Gepuffert“, „Antwort verloren“).

### F0047 Lücken nach Wiederverbinden

- [ ] Nachholen (Node-Test, falscher Port): jüngste bestätigte Zeit 10; der Port liefert auf `nach=10` 200 Nachrichten mit `neuer: true`, auf `nach=<letzte davon>` 5 mit `neuer: false` → 205 nachgeholt, zwei Abrufe, beide mit `n=200`; liefert der Port immer `neuer: true` → genau 5 Abrufe, dann „neu öffnen“. Die Mutanten „4 Blöcke“ und „6 Blöcke“ werden je rot.
- [ ] Meldung (Node-Test, gestellte Zeit): 3 → „✓ Wieder verbunden · 3 nachgeladen“; 1 205 → „✓ Wieder verbunden · 1.205 nachgeladen“; 0 → „✓ Wieder verbunden“; neu geöffnet → „✓ Wieder verbunden · neueste geladen“; bei 3 999 ms sichtbar, bei 4 000 ms weg.
- [ ] Lücke (E2E, **Z6**): B hat `# allgemein` gewählt und steht am Ende; Strom von B getrennt; `beta` sendet 3 Nachrichten über die API; Strom wieder → innerhalb **Z6** zeigt B „✓ Wieder verbunden · 3 nachgeladen“ und die 3 Nachrichten genau einmal in der Folge ihrer Zeiten.
- [ ] Zu viel verpasst (E2E): wie oben, aber während der Trennung senden 44 Agenten auf den Herkünften `127.0.0.2` bis `127.0.0.12` je 25 Nachrichten (1 100 > 5 × 200; je Kennung 25, je Herkunft 100 — kein 429) → nach dem Wiederverbinden hat B neu geöffnet, die neueste der 1 100 ist sichtbar, die Leiste zeigt „✓ Wieder verbunden · neueste geladen“.
- [ ] Fremde vor eigenen (E2E; Begründung: ADR-0037 Pre-Mortem 2, Designsystem „gepuffert“): Strom von A getrennt; A sendet `eigen` (gepuffert); `beta` sendet `f1` und `f2` über die API; Strom wieder → der Verlauf von A zeigt `f1`, `f2`, `eigen` in dieser Folge, je einmal; laut Netzprotokoll beginnt der `POST` von `eigen` erst nach dem Ende des Abrufs `nach=`.
- [ ] `NeuerStand` (Node-Test): das Signal `NeuerStand` bei verbundenem Strom löst dasselbe Nachholen aus wie ein Wiederverbinden.
- [ ] **Zusammenführen** (Node-Test, rein; Nacharbeit #22): bekannte Zeiten 1–3, live angenommen 6, nachgeholter Block 4, 5 → Folge 1, 2, 3, 4, 5, 6, je einmal; enthält der Block die 6 schon → 6 einmal; leerer Block → unverändert. Die Mutante „nur Zeiten größer als die jüngste“ (das heutige `einordnen`) wird rot.
- [ ] **Während des Abrufs** (E2E, Nacharbeit #22; Zweck: „jede genau einmal“ auch im Wettlauf zwischen Nachholen und Strom; Abnahme-Probe aus dem Code-Review #22, Muss B1, wörtlich übernommen): „(1) B getrennt, beta sendet l1; Abruf nach= per route.fetch, danach beta „waehrend“, Antwort 0/300/1500 ms zurückgehalten → l1 und waehrend je 1× in Zeitfolge; (2) nachricht-Ereignisse verschluckt, beta s1, s2, dann NeuerStand, während des gehaltenen Abrufs beta „live“ → s1, s2, live je 1×; (3) ohne Halten unverändert. Mutante „Block nach Live-Ereignis verworfen“ rot.“ Vor der Nacharbeit rot (waehrend = 0 bei 300 und 1 500 ms; s1 = 0, s2 = 0).
- [ ] **Neuversuch** (Node-Test, rein, gestellte Zeit; Nacharbeit #22): Nachholen scheitert bei 0 ms und danach immer → Versuche bei 1 000, 3 000, 8 000, 18 000 und 28 000 ms; keine Meldung, die Warteschlange ruht; gelingt der Versuch bei 8 000 ms mit 3 Nachrichten → „✓ Wieder verbunden · 3 nachgeladen“, die Warteschlange läuft weiter. Ein Abruf läuft bei 9 999 ms noch und ist bei 10 000 ms abgebrochen. Die Mutanten „Fehler als 0 nachgeholt“, „kein Neuversuch“, „Zeitgrenze 9 s“ und „Zeitgrenze 11 s“ werden je rot.
- [ ] **Gescheitertes Nachholen** (E2E, Nacharbeit #22; Abnahme-Probe aus dem Code-Review #22, Muss B2, wörtlich übernommen): „B getrennt, beta g1–g3, erster nach=-Abruf abort bzw. 500, danach Durchlass → nie „✓ Wieder verbunden“, solange g1–g3 fehlen; spätestens nach dem nächsten erfolgreichen Abruf je 1×; Variante zweimal scheitern; ohne Störung US-6 grün.“ Vor der Nacharbeit rot („✓ Wieder verbunden“, die Lücke blieb still).
- [ ] **Nachhol-Abruf hängt** (E2E, **Z13**; Nacharbeit #22, Code-Review A3): wie „Gescheitertes Nachholen“, aber die Route antwortet auf den ersten Abruf `nach=` nie → 9 s nach seinem Beginn kein zweiter Abruf und kein „✓ Wieder verbunden“; spätestens 15 s nach seinem Beginn läuft der zweite, danach „✓ Wieder verbunden · 3 nachgeladen“ und g1–g3 je einmal.
- [ ] Konsole bei A und B: keine unbehandelte Ausnahme, kein CSP-Verstoß; Konsolenfehler höchstens „Failed to load resource“ zu Anfragen, die bei getrenntem Netz scheitern, und zu den gestellten Abbrüchen und 500 in „Gescheitertes Nachholen“ und „Nachhol-Abruf hängt“.

### F0048 Entwurf, Längenanzeige und Textprüfung im Browser

- [ ] Textregel im Browser (Node-Test, rein): dieselben Fälle wie die Textregel in R00011 (F0041) ergeben dieselben Ergebnisse — 16 384 × `a` ok, 16 385 → Länge 16 385; 5 461 × `€` + `a` ok, 5 462 × `€` → Länge 16 386; `"ab\x1b"` → Stelle 2, `U+001B`; `"a\rb"` → Stelle 1; `"a\r\nb"`, `"👩‍💻"` ok. Die Mutanten „Grenze 16 383/16 385“ und „Zeichen statt Bytes“ werden je rot.
- [ ] Hinweis (Node-Test, rein): 14 999 Bytes → leer; 15 000 → „noch 1.384 Zeichen“; 16 384 → „noch 0 Zeichen“; 16 385 → „⚠ Zu lang: 16.385 von höchstens 16.384 Byte — bitte teilen.“; `"rot \x1b[31m"` → „⚠ Steuerzeichen U+001B an Stelle 5 — bitte entfernen.“ Die Mutanten „Schwelle 14 999“ und „Schwelle 15 001“ werden je rot.
- [ ] Zu lang (E2E): A fügt 16 385 × `a` ins Feld ein → Hinweiszeile „⚠ Zu lang: 16.385 von höchstens 16.384 Byte — bitte teilen.“ in der Farbe von `kd-danger`, das Feld trägt `aria-invalid="true"`; `Enter` → kein `POST`, der Text bleibt; A löscht ein Zeichen → „noch 0 Zeichen“, `aria-invalid` ist weg, `Enter` sendet (201).
- [ ] Steuerzeichen (E2E): A fügt `rot ` + `\u001b` + `[31m` ein → „⚠ Steuerzeichen U+001B an Stelle 5 — bitte entfernen.“, `Enter` sendet nicht.
- [ ] Ansagen der Hinweiszeile (E2E): A tippt von 14 990 auf 16 390 Bytes, je 10 Zeichen auf einmal → der angesagte Inhalt (`aria-live`) ändert sich genau zweimal: beim Überschreiten von 15 000 und beim Wechsel in „Zu lang“.
- [ ] Entwurf (E2E, **Z12**): A tippt `halb` in `# allgemein` → spätestens **Z12** nach der letzten Taste steht `halb` in `sessionStorage` unter `localchatA.entwurf.allgemein`; A wechselt nach `# technik` → das Feld ist leer; zurück → `halb` steht im Feld; A lädt neu → `halb` steht im Feld; ein neu geöffneter Tab (nicht dupliziert) in `# allgemein` → das Feld ist leer; A sendet `halb` → nach erneutem Neuladen ist das Feld leer; A tippt `x` und löscht es → kein Eintrag für `allgemein`; in `localStorage` steht nie ein Entwurf.
- [ ] **Entwurf beim schnellen Wechsel** (E2E, Nacharbeit #22; Zweck: der Entwurf geht auch vor Ablauf der 500 ms nicht verloren; Abnahme-Probe aus dem Code-Review #22, Muss B3 b, wörtlich übernommen): „A tippt x, wechselt < 200 ms nach # technik und zurück → Feld zeigt x; Variante Wechsel + Neuladen.“ Die Mutante „beim Wechsel nicht geschrieben“ wird rot.
- [ ] Fokus (Node-Test und E2E): Fokus auf der Seitenleiste, `/` → Fokus im Feld, kein `/` im Feld; im Feld `/` → wird getippt; Fokus im Verlauf, `Esc` → Fokus im Feld; Fokus im Feld mit Entwurf `a` in `# allgemein`, `Alt ↓` nach `# technik` mit Entwurf `b` → Fokus im Feld, Inhalt `b`.
- [ ] Schmal (E2E): bei 320 px mit Fehler in der Hinweiszeile `scrollWidth <= innerWidth`.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler.

### F0049 Gebündelte Ansage

- [ ] Bündeln (Node-Test, rein, gestellte Zeit; Channel `# allgemein`): t = 0 fremd von `claude@10.0.0.5` → „Neue Nachricht von claude@10.0.0.5 in # allgemein“; t = 1, 2, 3 s fremd → nichts; t = 9,999 s → nichts; t = 10 s → „3 neue Nachrichten in # allgemein“; t = 25 s fremd → sofort „Neue Nachricht von …“; t = 26 s fremd, t = 35 s → „1 neue Nachricht in # allgemein“; eigene, nachgeladene und nachgeholte Nachrichten → nie; Fehlschlag bei t = 36 s → sofort „Nachricht nicht gesendet: <Grund>“. Eine fremde Nachricht je Sekunde über 60 s → höchstens 7 Ansagen. Die Mutanten „9 s“ und „11 s“ werden je rot.
- [ ] Region (E2E): die Seite hat genau eine Region `role="status"` mit `aria-live="polite"` außerhalb des Verlaufs, visuell verborgen (Größe 1 × 1 px oder kleiner, nicht `display:none`); der Verlauf bleibt `aria-live="off"`.
- [ ] Ansage (E2E): B hat `# allgemein` gewählt; `beta` sendet `Hallo` → die Region enthält „Neue Nachricht von beta@127.0.0.1 in # allgemein“; `beta` sendet an `technik` → die Region ändert sich nicht; B sendet selbst → die Region ändert sich nicht.
- [ ] **Takt der Sammelansage** (E2E, Nacharbeit #22; die Regel prüft „Bündeln“ exakt, hier die Zustellung; Uhr ab dem Erscheinen der ersten Ansage; Abnahme-Probe aus dem Code-Review #22, Muss B3 a, wörtlich übernommen): „Ruhe > 10 s, beta h1, dann h2–h4 in 3 s → zuerst „Neue Nachricht von beta@127.0.0.1 in # allgemein“, 10 s ± 1 s danach genau einmal „3 neue Nachrichten in # allgemein“.“ Die Mutante „Takt nicht gestellt“ wird rot.
- [ ] Fehlschlag (E2E): die Route antwortet auf den `POST` von A 500 → die Region von A enthält „Nachricht nicht gesendet: Nicht gespeichert — der Server konnte nicht schreiben.“, auch wenn A in den 10 s davor schon eine Ansage bekam.
- [ ] Konsole bei A und B: keine unbehandelte Ausnahme, kein CSP-Verstoß; Konsolenfehler höchstens „Failed to load resource“ zur gestellten 500 in „Fehlschlag“.

## Betroffene Verzeichnisstruktur

- Server: `src/gespraeche/` (neu `wiederholung.rs`, rein; `senden` mit Schlüssel unter der Schreibsperre), `src/http/nachrichten.rs` (Kopf `Idempotency-Key`, 200, 422 `schluessel`).
- Oberfläche: `web/zustellung.js` (erweitert), `web/api.js` (Zeitgrenze), neu `web/nachholen.js`, `web/textregel.js`, `web/hinweis.js`, `web/entwurf.js`, `web/ansage.js`; geändert `web/eingabe.js`, `web/verlauf.js`, `web/verbindung.js`, `web/hauptansicht.js`, `web/lc-seite.css`.
- Tests: Unit im Modul; Integration `tests/senden_wiederholung.rs`; Oberfläche rein `e2e/einheit/`; Abläufe `e2e/tests/r00012-*.spec.js`.

## Technische Überlegungen

### Ablauf

1. **Senden mit Schlüssel**: wie R00011 bis zur Textregel → Schlüsselformat (422) → Restbuchung (429) → unter der Schreibsperre: Schlüssel bekannt? gleich → 200 | anders → 422 | neu → Zeit, anhängen, Index, veröffentlichen, merken → 201.
2. **Tab**: Verbindungszustand getrennt → Warteschlange ruht → verbunden → Nachholen (F0047) → Warteschlange weiter; Abruf mit 10 s → Antwort | Zeitüberschreitung | Netzfehler | 401 (Wiederbeitritt, ein Versuch) → Zustand → Zeichnen und Ansage.
3. **Eingabe**: Taste → Textregel → Hinweis; `Enter` nur ohne Fehler → Zustellung; Eingabe → Entwurf (höchstens alle 500 ms).

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0046: B0237 Wiederholschlüssel merken (rein) · B0238 Senden mit Schlüssel · B0239 Wiederholung im Tab (rein, unklar) · B0240 Zeitgrenze im Abruf · B0241 Schalter an der Nachricht · B0242 Fehlerwege verdrahten (unklar)
- F0047: B0243 Nachholen in Blöcken · B0244 Wieder-verbunden-Meldung · B0245 Nachholen verdrahten (unklar)
- F0048: B0246 Textregel im Browser (rein) · B0247 Hinweis formen (rein) · B0248 Hinweiszeile und Senden sperren · B0249 Entwurf je Channel · B0250 Fokus in die Eingabe
- F0049: B0251 Ansage bündeln (rein) · B0252 Ansage verdrahten
- Nacharbeit #22 (2026-10-29): F0047 B0390 Lücke und Live-Strom zusammenführen (rein) · B0391 Zusammenführen verdrahten (unklar) · B0392 Zeitgrenze im Nachhol-Abruf · B0393 Gescheitertes Nachholen hält die Lücke offen (unklar); F0048 B0394 Entwurf beim schnellen Channelwechsel belegt (nur Test); F0049 B0395 Takt der Sammelansage belegt (nur Test)

**Elemente** (nur öffentliche Signaturen): `Wiederholung` (rein) — `fn pruefen(&mut self, kennung, schluessel, channel, pruefsumme, jetzt) -> Neu | Wiederholt { zeit } | Anders`, Konstanten `HOECHSTENS_EINTRAEGE = 4_096`, `HOECHSTENS_ALTER = 10 min`; Oberfläche rein: `nachholen(jungste, port) → {nachrichten, zahl} | "neu"`, `meldung(zahl | "neu")`, `textPruefen(text)`, `hinweis(ergebnis, bytes)`, `fokusTaste(ereignis, ort)`, `ansage(zustand, ereignis, jetzt) → {zustand, text?}`.

Wireframe: Artboard `D0003.dc.html` fehlt (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle: Designsystem `components/Nachricht/README.md` (fehlgeschlagen, Schalter), `components/VerlaufsMarken/README.md` (Leiste, gepuffert), `components/Eingabe/README.md` (Entwurf), `README.md` „Barrierefreiheit“ — Vorgabe, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `Gespraeche::senden`: Schlüssel unter der Schreibsperre; `http::nachrichten`: Kopf, 200, 422 `schluessel`.
- `web/zustellung.js`, `web/eingabe.js`, `web/verlauf.js`, `web/verbindung.js` (Wiederverbinden meldet an Nachholen und Warteschlange).

## Tests

Nach Skill `test-pyramide` und ADR-0009; Tests sind nie schärfer als diese Anforderung und übernehmen Z6, Z11, Z12 wörtlich. **Unit:** Schlüssel (Spannen, Mutanten). **Integration:** Format, Wiederholung, 20 gleichzeitig, Wiederholung bucht. **Oberfläche rein:** Zustellung erweitert, Nachholen, Meldung, Textregel, Hinweis, Fokus, Ansage. **E2E:** Z7, Z8, Erneut senden, Verwerfen, Z11, 401, Z6, zu viel verpasst, fremde vor eigenen, Entwurf, Hinweis, Ansage — Abrisse über `setOffline` und `page.route`, Zeiten ereignisgenau, nie im Standardraster von `expect`. Seit der Nacharbeit #22 auch der Wettlauf zwischen Nachholen und Strom (zurückgehaltener Abruf `nach=`), gescheitertes und hängendes Nachholen (Z13), der Entwurf beim schnellen Wechsel und der Takt der Sammelansage; Zusammenführen und Neuversuch als reine Funktionen.

## Abhängigkeiten

- Abhängig von: I0009 Nachricht senden (R00011, rot) — F0046 braucht F0043 (Zustellung, Eingabe) und F0045 (Schreibkonto); F0047 braucht F0044 (Einordnen, Fenster); F0048 braucht F0043; F0049 braucht F0044, F0046, F0047.
- Innerhalb (WBS `Braucht`): F0046, F0047, F0048 Welle 1 (parallel, sobald I0009 steht); F0049 Welle 2.
- Blockiert: I0017 Markdown (Codeblock-Regel an der Tastenauswertung), I0019 Claude Code einbinden (`lc senden` nutzt den Wiederholschlüssel).

## Offene Fragen

- Keine an den Betreiber. Die Frage nach der Buchung der Frist am Schreibkonto steht in R00011 und betrifft R00012 nicht.

Entschieden vom planer für R00012 (Rückfragen mit Empfehlung beantwortet):

- **„noch n Zeichen“ zählt Bytes** (n = 16 384 − Bytes), wie ADR-0038 Regel 4; bei Text aus Mehrbyte-Zeichen wie `€` ist die Zahl kleiner als die Zahl der tippbaren Zeichen — sie unterschätzt, sie verspricht nie zu viel.
- **„✓ Wieder verbunden“ ohne Zahl bei n = 0** — „0 nachgeladen“ wäre Rauschen.
- **401 zweimal → Grund aus der zweiten Antwort**; ADR-0037 nennt für diesen Fall keinen Text.
- **Text für einen fremden Schlüssel**: „Dieser Schlüssel gehört zu einer anderen Nachricht.“ (ADR-0034 nennt keinen).
- **„Fremde vor eigenen“ als Kombination zweier Ausnahmewege**: begründet aus ADR-0037 Pre-Mortem 2 und dem Designsystem („gepuffert“); sonst keine Kombinationen.
- **Gescheitertes Nachholen: stiller Neuversuch** (Nacharbeit #22, Code-Review B2 und A3): keine Meldung, Leiste bleibt „Verbindung verloren“, Warteschlange ruht, Neuversuch mit dem Rückzug des Stroms, Zeitgrenze 10 s je Abruf. Verworfen: eine Fehlerleiste mit Schalter (verlangt einen Klick, ließe eigene vor fremde ältere Nachrichten) und der Rückfall auf Neu-Öffnen (scheitert unter derselben Störung, die Lesestelle ginge verloren). Folge: scheitert jeder Abruf, bleiben eigene Nachrichten „wird gesendet …“ — sie gehen nicht verloren. Bestätigung des architekt als Ergänzung zu ADR-0037 Regel 7 (WBS, Offene Fragen #22 Punkt 1).
- **Ansage-Regel „sofort“**: sofort heißt, seit der letzten Ansage sind mindestens 10 s vergangen und nichts ist gesammelt — so sind „erste nach Ruhe“ und „höchstens alle 10 s“ eine Regel.

## Manuelle Vorbereitungstätigkeiten

- Keine; Abrisse und gestellte Antworten entstehen im E2E über Playwright.

## Manuelle Nachbereitungstätigkeiten

- Die Entscheidungen oben in ADR-0037 und ADR-0038 unter „Ergänzungen“ nachtragen — architekt über den Director.

## Umfang

Aus der WBS gezählt (`/planung zaehlen` je Slice); Standard-Bubbles mit der Untergrenze 0,4h, unklare als Richtwert 2–4h (ungemessen):

- F0046 Fehler und Wiederholung = 6 Bubbles: 4 Standard (0,4h), 2 unklar (2–4h). Rest: 1,6h klar + 4–8h unklar.
- F0047 Lücken nach Wiederverbinden = 3 Bubbles: 2 Standard (0,4h), 1 unklar (2–4h). Rest: 0,8h klar + 2–4h unklar.
- F0048 Entwurf, Längenanzeige und Textprüfung im Browser = 5 Bubbles: 5 Standard (0,4h). Rest: 2,0h klar.
- F0049 Gebündelte Ansage = 2 Bubbles: 2 Standard (0,4h). Rest: 0,8h klar.
- I0020 gesamt = 16 Bubbles: 13 Standard, 3 unklar. Rest: 5,2h klar + 6–12h unklar (Richtwerte ungemessen).
- Nacharbeit #22 (2026-10-29) = 6 Bubbles: 4 Standard (0,4h) — B0390, B0392, B0394, B0395; 2 unklar (2–4h) — B0391, B0393. Rest: 1,6h klar + 4–8h unklar (Richtwerte, ungemessen für den Typ Nacharbeit).
- Nicht enthalten: Codeblock-Regel (I0017), Anhänge (I0010), Ansage von Erwähnungen und Direktnachrichten (I0013, I0012).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Im LAN bricht eine Verbindung genau dann ab, wenn jemand gerade sendet — WLAN-Wechsel, Deckel zu, Neustart der Instanz. R00011 kann dann nur „fehlgeschlagen“ zeigen oder für immer warten, und wer die Nachricht von Hand neu sendet, riskiert ein Doppel, wenn die erste doch ankam. Wenn jeder Sendeauftrag einen Schlüssel trägt, den der Server je Kennung wiedererkennt, dann ist jede Wiederholung ungefährlich, und der Tab darf nach dem Wiederverbinden einfach weitersenden — der Mensch muss nichts entscheiden. Wenn der Tab nach dem Wiederverbinden zuerst über `nach=` nachholt und erst dann seine Warteschlange sendet, dann steht alles in der Folge seiner Zeiten, ohne Neuladen. Wenn die Eingabe Länge und Steuerzeichen mit derselben Regel wie der Server vorab prüft, dann erfährt der Mensch den Grund, solange der Text noch im Feld steht. Und wenn Ansagen gebündelt werden, dann bleibt ein Channel, in dem Agenten im Takt schreiben, mit dem Screenreader benutzbar. Der Hebel ist der Schlüssel: er macht aus „vielleicht doppelt“ ein „genau einmal“, und alle Wiederholwege bauen darauf.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus ADR-0034, ADR-0037, ADR-0038.
- Kein Artboard für D0003.

## Notizen

### Verworfene Alternativen

- **Eine Anforderung je Slice**: verworfen — die vier Slices bilden das Arbeitspaket I0020 und durchlaufen gemeinsam Entwicklertest und Abnahme.
- **Fehler nur ohne Wiederholschlüssel wiederholen** (Inhaltsgleichheit im Zeitfenster): in ADR-0034 verworfen — ein Agent, der zweimal „ok“ sagt, verlöre die zweite Nachricht.
- **„noch n Zeichen“ in Zeichen statt Bytes**: verworfen — die Grenze ist in Bytes; eine Zahl in Zeichen könnte mehr versprechen, als passt.

### Review 2026-10-29 (Nacharbeit #22, planer im Auftrag des Directors)

- F0047: zwei Zusagen ausgeschrieben — „Nichts geht zwischen Nachholen und Strom verloren“ (Code-Review B1: Ereignisse während des Abrufs wurden verworfen, nach `NeuerStand` fiel der ältere Block weg) und „Scheitert das Nachholen“ (Code-Review B2: ein gescheiterter Abruf meldete „✓ Wieder verbunden“, die Lücke blieb still; A3: der Abruf hatte keine Zeitgrenze). Neu Z13 und die Kriterien „Zusammenführen“, „Während des Abrufs“, „Neuversuch“, „Gescheitertes Nachholen“, „Nachhol-Abruf hängt“, je mit der Abnahme-Probe wörtlich, wo es eine gibt.
- F0048, F0049: Kriterien „Entwurf beim schnellen Wechsel“ und „Takt der Sammelansage“ (Code-Review B3: die Zusagen standen da, kein E2E belegte sie); neu US-15.
- Nicht hier, sondern in R00025 (Härtung aus denselben Reviews): eine Warnung mit Countdown statt Leiste und R00005-Hinweis, Ansage von Trennen und Wiederverbinden (F0084); „Verwerfen“ entfernt erst nach 5 s mit „Rückgängig“ (F0085, ändert F0046 „Verwerfen“); Schalter „Steuerzeichen entfernen“ und kein Hinweis bei Text nur aus Leerraum (F0086; „Umgeformt wird nichts“ gilt weiter ohne Klick); Höchstzahl je Kennung im Wiederholspeicher, Verdrängung im Protokoll (F0087, ändert „höchstens die 4 096 jüngsten Einträge“); ein Weg zum Wiederbeitritt (F0088). Diese Stellen gelten hier, bis R00025 gebaut ist.

### Bewusst außerhalb

Codeblock-Regel für `Enter` (I0017); `↑` in leerer Eingabe; Ansage von Erwähnungen und Direktnachrichten (I0013, I0012); Entwurf über das Schließen des Tabs hinaus (`localStorage`, ADR-0038 verworfen); Wiederholschlüssel über einen Neustart hinaus (ADR-0034 nimmt das hin).
