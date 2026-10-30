# ADR-0051: Aktivität „tippt“ / „denkt nach“ — flüchtig im Speicher mit Ablauf, eigener Signalbus neben dem Ereignisbus, eine Route je Gespräch für Mensch und Agent, eigene Bremse

**Status**: Akzeptiert
**Datum**: 2026-10-30
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #77)
**Kontext-Anforderung**: [R00045](../../Anforderungen/R00045-tippanzeige-und-denkt-nach-status.md) (WBS I0040, I0041)
**Erprobt durch**: — (gerechnet, nicht gemessen; überträgt gebaute Mechanik: Token-Eimer `src/schreibkonto/eimer.rs` (ADR-0036), Veröffentlichen unter der Sperre über eine eingesetzte Funktion `src/anwesenheit/bestand.rs` (ADR-0016), Serverzeit mit `jetzt` (ADR-0019), Episoden mit Pflicht-Quelle (ADR-0050). Die Messung ist die Lastprobe P11 in I0041.)

## Kontext

- **Auftrag** (PO-Entscheidungen Zeilen 133–141, `AI-Team/po-entscheidungen.md`; Vision-Zielbild „Wer tippt oder nachdenkt, ist sichtbar“, 1751c08): zwei Zustände mit festem Wortlaut `tippt` und `denkt nach`; **eine** Route für Mensch und Agent; der Browser meldet `tippt` selbst (höchstens alle 3 s, Gültigkeit 6 s), ein Agent setzt seinen Zustand über die API (Gültigkeit angebbar, Standard 30 s, höchstens 120 s, erneuerbar); Ende sofort bei Senden einer Nachricht dieses Absenders im Gespräch, leerem Feld, Löschen über die API, Channel-Wechsel, Verlassen, offline; nur die eigene Kennung setzt und löscht; sichtbar nur, wer das Gespräch sehen darf, nie sich selbst; flüchtig (kein Tageslog, kein Verlaufsindex, kein Nachholen, keine Protokollzeile je Signal, höchstens Episode bei Abweisung); nach Abriss nichts nachholen; nicht im Schreibkonto, nicht in der Plattenquote; eigene Bremse je Kennung und Gespräch (Zahl: architekt); weckt keinen wartenden Agenten; Agenten lesen die aktiven Zustände über dieselbe Route; keine Abschaltung.
- **Offene Fragen des planer** (R00045 „Offene Fragen“ 1–5, Kommentar auf Karte #77): größer als 120 s abweisen (422) oder begrenzen; ein Zustand je Kennung und Gespräch; Erneuerung ab Eingang; Lesen ohne eigene Kennung; Antwort nennt Ablaufzeit.
- **Bestand**:
  - Ein Bus `tokio::sync::broadcast` mit Kapazität 1.024 für alle Themen (`src/live/bus.rs`, `Ereignis::{Anwesenheit, Channel, Nachricht, Aenderung, Anhaenge}`); wer mehr als 1.024 zurückliegt, bekommt `NeuerStand` und der Tab lädt Stände und Blöcke nach (ADR-0016 Regel 5, ADR-0028 Regel 6) — teuer; dazu eine Episode `WARNING live` (ADR-0042, Ergänzung Karte #31 Punkt 6). [A00001](../Architektur/A00001-localchata.md), Ergänzung Lastprofil: „Puffer ≈ 34 s Spitze für einen langsamen Leser“ bei 30 Ereignissen/s.
  - Der Strom kennt die Kennung seiner Sitzung (`src/http/ereignisse.rs`, `Strom::kennung`); ein Strom je Browser über einen `SharedWorker` (ADR-0015), der den jüngsten Stand der Teilnehmer und Channels neuen Tabs nachreicht (`web/verteiler.js`).
  - Anwesenheit: `online` nur mit Halter (Strom oder Warte-Aufruf) oder ≤ 1 s danach; „eine Sitzung ohne je einen Halter (Agent, der nur sendet) ist nach dem Anlauf `offline`“ (ADR-0016 Regel 1); jeder Wechsel geht unter der Sperre über eine eingesetzte Funktion hinaus (A00010, Ergänzung Karte #5).
  - `warten` (I0016) ist noch nicht gebaut; es soll nur bei Nachricht, Direktnachricht, Erwähnung wecken (PO Zeile 88, 140).
  - Schreibkonto: Eimer je Kennung (30, 1/s) und je Herkunft (120, 8/s) für Senden, Bearbeiten, Frist, Anmelden, Löschen (ADR-0036); der Eimer-Typ ist ein eigenes Modul (`src/schreibkonto/eimer.rs`).
  - Körper: ein weiteres Feld im JSON ist 422 `feld` (Hausregel, `src/http/senden.rs:768–770`).
  - Der Name `zustand` ist zweimal vergeben: im Browser für den Verbindungszustand zwischen Worker und Tab (`web/live.js:32`, `web/verteiler.js` `beiZustand`), im Server für den Anwendungszustand der HTTP-Schnittstelle (`State<Arc<Zustand>>`, `src/http/mod.rs`).
- **Lastprofil Team-LAN** (`AI-Team/umgebung.md`, Vision-Leitplanke): bis 20 Menschen und 50 Agenten, Spitze 30 Nachrichten/s, dauerhaft 5/s, Zustellung live < 1 s. Die Factory betreibt viele Agenten auf **einer** Herkunft (`127.0.0.1`).

### Qualitätsziele mit Szenario

| # | Ziel | Szenario mit Maß |
|---|---|---|
| QA1 | Live | `anna` tippt in `# allgemein` → bei `ben` steht „anna tippt …“ binnen 1 s (erwartet wenige ms); `anna` sendet → bei `ben` binnen 1 s weg |
| QA2 | Nachrichten zuerst | Lastprofil plus alle Signale (20 Menschen tippen, 50 Agenten denken nach) → jede Nachricht beim Leser < 1 s nach der 201 (Maximum), **kein** `NeuerStand` und keine Episode `live` durch Signale |
| QA3 | Flüchtig | 60 s Tippen und Denken → `sha256sum` über `channels/` unverändert, im Betriebsprotokoll (Stufe Information) keine Zeile, nach Neuladen und nach einem Abriss keine Anzeige |
| QA4 | Kein Orakel, kein fremder Zustand | `cora` erfährt über Strom und Lesen nichts über ein Gespräch, das sie nicht sehen darf; niemand setzt oder löscht den Zustand einer anderen Kennung |
| QA5 | Flutfest | ein Agent ruft Setzen in einer Schleife → je Gespräch höchstens 1 Ereignis/s dauerhaft, je Herkunft höchstens 12/s; seine Nachrichten und die aller anderen gehen ungebremst |

## Optionen

### Gesamtschnitt

#### Option A: Eigener Baustein „Aktivität“, Ereignis als neues Thema auf dem **vorhandenen** Bus
- Vorteile: kleinster Eingriff im Strom (ein `match`-Arm mehr wie bei `Aenderung`); Reihenfolge Nachricht → Ende auf einem Bus garantiert; ein Abonnement je Strom.
- Nachteile: Signale teilen die 1.024 Plätze mit Nachrichten — im Lastprofil verdoppelt sich die Ereignisrate (≈ 30 → ≈ 76/s Spitze, siehe Lastnachweis), der Puffer eines langsamen Lesers schrumpft von ≈ 34 s auf ≈ 13 s; ein Überholen durch Signale löst `NeuerStand` aus und damit das teure Nachladen von Ständen und Blöcken — ein flüchtiges Signal kostet dann so viel wie eine verlorene Nachricht (gegen QA2); `warten` muss Signale ausdrücklich herausfiltern (eine Mutante „Filter weg“ weckt Agenten).

#### Option B: Eigener Baustein „Aktivität“ und **eigener Signalbus** (zweiter `broadcast`) neben dem Ereignisbus; der Strom liest beide, Nachrichten zuerst *(gewählt)*
- Vorteile: Signale können Nachrichten weder verdrängen noch ein `NeuerStand` auslösen (QA2 strukturell); ein Überholen auf dem Signalbus kostet nur „alle Anzeigen weg“, weil Signale flüchtig sind; `warten` abonniert den Signalbus nie — „weckt nicht“ gilt durch den Bau, nicht durch einen Filter; der Bus für flüchtige Signale ist zugleich der Ort für spätere flüchtige Themen (Lesebestätigung, Mauszeiger o. Ä.) ohne Eingriff in den Ereignisbus.
- Nachteile: zwei Abonnements je Strom; über die Busse hinweg keine garantierte Reihenfolge — ein Ende-Signal kann vor der Nachricht ankommen, die es auslöste (gemildert durch „Nachrichten zuerst“ im Strom und die Client-Regel „Nachricht blendet aus“, Regel 7); ein zweiter Ort, an dem die Sichtbarkeit „sichtbar für“ (I0012) angewendet werden muss.

#### Option C: Aktivität als Teil der **Anwesenheit** — die Teilnehmerzeile trägt `aktivitaet{channel, zustand, bis}`, verteilt als Anwesenheits-Wechsel (`stand + 1`)
- Vorteile: Ende bei `offline`/`weg` fällt unter derselben Sperre mit an; Stand plus Wechsel, Reihenfolge über `stand`, für neue Tabs im Worker gemerkt — alles gebaut.
- Nachteile: der Stand beim Öffnen des Stroms enthielte die Aktivität — das ist genau das Nachholen, das PO Zeile 138 ausschließt; jedes Signal erhöht `stand` und zeichnet die Teilnehmerliste neu (PO Zeile 137: nicht in der Liste, kein Flackern bei 70 Teilnehmern); ≈ 46 Signale/s nehmen die Sperre der Anwesenheit, die auch Halter-`Drop` und `pflegen` im 250-ms-Takt brauchen; eine Aktivität je Gespräch passt nicht in eine Zeile je Sitzung.

#### Option D: Kein Zustand im Server — Setzen ist ein reines Weiterleiten (Discord „Trigger Typing“), der Ablauf lebt nur im Client
- Vorteile: kein Speicher, keine Aufräumregel, kein Rennen mit `offline`; am wenigsten Code.
- Nachteile: **Lesen** (PO Zeile 140, I0041 (4)) ist unmöglich — ein Agent ohne Strom erfährt nichts; „ein Zustand je Kennung und Gespräch“ und „Ende beim Senden“ bräuchten doch Wissen im Server, ob etwas aktiv war (sonst je Nachricht ein Ende-Ereignis an alle, auch ohne Zustand).

| | QA1 Live | QA2 Nachrichten zuerst | QA3 Flüchtig | QA4 | QA5 | Lesen | Schwäche |
|---|---|---|---|---|---|---|---|
| A | ja | geteilt, `NeuerStand` möglich | ja | ja | Bremse | ja | Signale verdrängen Nachrichten |
| **B** | ja | **strukturell** | ja | ja (zweiter Filterort) | Bremse | ja | Reihenfolge über zwei Busse |
| C | ja | geteilt, Stand wächst | **nein** (Stand holt nach) | ja | Bremse | ja | Nachholen, Flackern, Sperre |
| D | ja | geteilt | ja | ja | Bremse | **nein** | Lesen fehlt, Ende je Nachricht |

### Einzelfragen

- **F1 Form der Route**: (a) `PUT | DELETE | GET /api/channels/{name}/aktivitaet` *(gewählt)* — Stärke: Gespräch im Pfad wie Verlauf, Senden, Frist; Setzen idempotent (`PUT` ersetzt), Löschen als `DELETE` wie bei Anhängen; Schwäche: drei Methoden an einer Ressource. (b) `POST /api/aktivitaet {channel, zustand}` — Stärke: ein Aufruf für Direktgespräche ohne Pfadfrage; Schwäche: Gespräch im Körper statt im Pfad weicht von allen Gesprächsrouten ab, Lesen bräuchte eine zweite Form. (c) Feld `aktivitaet` am Senden/Verlauf — Stärke: kein neuer Pfad; Schwäche: Setzen ohne Nachricht geht nicht, verletzt „eine Route“ für Setzen und Lesen.
- **F2 Gültigkeit über 120 s**: (a) **abweisen** 422 `regel: "gueltigkeit"`, ohne Wirkung *(gewählt, wie Empfehlung planer)* — Stärke: der Agent merkt seinen Fehler, Hausregel „abweisen statt umformen“ (ADR-0013); Schwäche: ein Agent, der 300 s schickt, zeigt gar nichts. (b) begrenzen auf 120 s — Stärke: der Agent zeigt etwas; Schwäche: stilles Umformen, der Agent glaubt an 300 s und erneuert zu spät.
- **F3 Ablauf**: (a) **der Client lässt selbst ablaufen**, der Server sendet beim Ablauf kein Ereignis, er filtert beim Lesen und räumt beim Zugriff auf *(gewählt)* — Stärke: PO Zeile 138 verlangt es ohnehin („verfällt von selbst“ nach Abriss), kein Takt-Ereignis, halbe Ereignisrate; Schwäche: wer den Strom mit `curl -N` liest, rechnet den Ablauf selbst (`bis`, `jetzt` stehen im Ereignis). (b) Server sendet ein Ende-Ereignis beim Ablauf (Takt) — Stärke: der Strom ist die eine Wahrheit; Schwäche: doppelt so viele Ereignisse, Takt-Abhängigkeit, und der Client braucht den lokalen Ablauf trotzdem für den Abriss.
- **F4 Standard-Gültigkeit**: (a) **je Zustand**: `tippt` 6 s, `denkt_nach` 30 s, angebbar 1–120 s für beide *(gewählt)* — Stärke: unabhängig von der selbst gewählten `art` (Gleichberechtigung, PO Zeile 135), der Browser schickt nur den Zustand; Schwäche: ein Agent, der `tippt` ohne Angabe setzt, bekommt 6 s statt 30 s. (b) je `art` der Sitzung (Mensch 6, Agent 30) — Stärke: wörtlich wie PO Zeile 136; Schwäche: dieselbe Meldung wirkt je nach selbst gewählter Art verschieden.
- **F5 „offline“ beendet**: (a) **für Sitzungen der Art `mensch` beim Wechsel nach `offline`, für alle beim Wechsel nach `weg`** *(gewählt; Bestätigung beim product-owner erfragt, Frage im Bericht)* — Stärke: ein Agent ist zwischen zwei Warte-Aufrufen immer `offline` (ADR-0016 Regel 1, ADR-0007 Option A „ohne Verbindung zwischen zwei Warte-Aufrufen“), gerade während er nachdenkt; R00045 prüft für Agenten „Strom getrennt, bis die Gnadenfrist ablief → weg“; Schwäche: die Regel liest die selbst gewählte `art`. (b) alle bei `offline` — Schwäche: „denkt nach“ eines Agenten ohne Strom erlischt nach ≤ 1,25 s, das Zielbild für Agenten liefe ins Leere. (c) nur bei `weg` — Schwäche: ein Mensch, dessen Browser abreißt, „tippt“ bis zu 6 s weiter, PO Zeile 136 „sofort … offline“ verfehlt.
- **F6 Ort der Bremse**: (a) **im Baustein Aktivität, mit dem vorhandenen Eimer-Typ** *(gewählt)* — Stärke: PO Zeile 139 „nicht zum Schreibkonto“ ist im Bau sichtbar (zwei Halter, zwei Tabellen), eine Verwechslung der Konten unmöglich; Schwäche: eine zweite 429-Form neben der des Schreibkontos. (b) drittes Eimerpaar im Schreibkonto (A00014) — Stärke: ein Ort für alle Bremsen; Schwäche: ein Fehlgriff bucht Signale am Konto der Nachrichten — genau das schließt Zeile 139 aus.
- **F7 Technischer Name**: **`aktivitaet`** für Route, Ereignis, Modul und Baustein; der Fachbegriff bleibt „Zustand“ (R00045) und ist das Feld `zustand` darin *(gewählt)* — weil `zustand` im Browser und im Server schon etwas anderes heißt (Kontext oben). Verworfen: `zustand` (Verwechslung in `web/live.js`/`web/verteiler.js` und `src/http/mod.rs`), `tippen` (deckt `denkt nach` nicht).

## Entscheidung

**Gewählt: Option B** mit F1 (a), F2 (a), F3 (a), F4 (a), F5 (a), F6 (a), F7.

Es überwiegt, dass ein flüchtiges Signal die Zustellung der Nachrichten unter keiner Last verdrängen kann und ein überholter Leser dafür nur Anzeigen verliert statt nachzuladen — QA2 ist durch den Bau getragen, nicht durch eine Rechnung — und dass „weckt nicht“ strukturell gilt. In Kauf genommen: zwei Abonnements je Strom, keine garantierte Reihenfolge zwischen einer Nachricht und dem Ende, das sie auslöst (Regel 6, Regel 7 c), ein zweiter Ort für „sichtbar für“, und eine Regel, die für `offline` die selbst gewählte `art` liest (F5).

Regeln:

1. **Baustein** [Aktivität](../Architektur/A00019-aktivitaet.md) (`src/aktivitaet/`): einziger Halter der Aktivitäten und ihrer Bremse, nur im Speicher; Uhr über den Provider ([X00007](../Architektur/Aspekte/X00007-zeit.md)); ein `std::sync::Mutex` um Tabelle und Eimer, jede Operation kurz, ohne Ein-/Ausgabe ([X00008](../Architektur/Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)). Den Bus kennt er nicht: veröffentlicht wird über eine eingesetzte Funktion, die er **unter seiner Sperre** ruft (wie Anwesenheit, ADR-0016 Ergänzungen) — die Reihenfolge auf dem Signalbus ist damit je (Kennung, Gespräch) die der Änderungen.
2. **Datenmodell**: `Tabelle: HashMap<(Kennung, Gespraech), Eintrag { zustand: Zustand, bis: Zeit, gueltig: Duration }>` — **ein** Eintrag je Kennung und Gespräch; ein neues Setzen ersetzt den alten Eintrag (auch `denkt_nach` ersetzt `tippt`). `Zustand` ist ein Enum mit genau `Tippt` | `DenktNach`, auf der Leitung `"tippt"` | `"denkt_nach"` (fester Wortlaut, kein Freitext; jeder andere Text 422 `regel: "zustand"`). Höchstens **16** Einträge je Kennung; ein 17. Setzen in einem weiteren Gespräch → 422 `regel: "aktivitaeten"`, ohne Wirkung.
3. **Gültigkeit und Ablauf**: Standard je Zustand — `tippt` **6 s**, `denkt_nach` **30 s**; angebbar als `gueltig_s`, ganze Zahl **1 bis 120**; 0, negativ, keine ganze Zahl, über 120 → 422 `regel: "gueltigkeit"`, ohne Wirkung. Jedes Setzen gilt ab **seinem Eingang** (`bis = jetzt + gueltig`, unter der Sperre gelesen), Gültigkeiten werden nie addiert. Ein Eintrag gilt, solange `jetzt < bis`. Abgelaufene Einträge erzeugen **kein** Ereignis; sie werden beim Lesen übergangen und beim nächsten Zugriff auf die Tabelle entfernt (beim Setzen die Einträge der Kennung, zusätzlich ein ganzer Durchgang, wenn mehr als 1.024 Einträge bestehen). **Uhr rückwärts**: ein Eintrag mit `bis − jetzt > gueltig` gilt als abgelaufen (lieber weg als zu lang).
4. **Route** an der [HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), Sitzung nötig (Extraktor `Teilnehmer`, jeder Aufruf ist ein Lebenszeichen wie jede Anfrage mit Token, ADR-0012); die Kennung kommt **nur** aus der Sitzung:
   - `PUT /api/channels/{name}/aktivitaet` mit `{"zustand": "tippt" | "denkt_nach", "gueltig_s"?: 1..120}` (JSON, Körper höchstens 1.024 Bytes, Hausregeln des Körpers; ein weiteres Feld — auch `kennung` — ist 422 `regel: "feld"`) → **200** `{"zustand", "bis", "jetzt"}` (`bis`, `jetzt` RFC 3339 mit Millisekunden in Serverzeit, wie `weg_um` und `jetzt` in ADR-0019).
   - `DELETE /api/channels/{name}/aktivitaet` (kein Körper) → **204**, auch wenn nichts aktiv war (wiederholbar ohne Fehler).
   - `GET /api/channels/{name}/aktivitaet` → **200** `{"jetzt", "aktivitaeten": [{"kennung", "zustand", "bis"}]}` — nur gültige Einträge, nach Kennung sortiert, **ohne die eigene Kennung**.
   - Reihenfolge der Prüfungen (wie Frist, ADR-0031): Grenzschutz 403 (`PUT`, `DELETE` sind nicht lesend, ADR-0014) → Sitzung 401 → Körper 415/413/400 → Gespräch: unbekannt 404, nicht sichtbar **wie unbekannt** 404 (kein Orakel; archiviert und Direktgespräch nach I0022/I0012, Regel 10) → Wert 422 (`zustand`, `gueltigkeit`, `feld`, `aktivitaeten`) → Bremse 429 → Wirkung.
   - In der API-Beschreibung (`/api/openapi.json`) und der Anleitung (I0019) mit Beispiel `curl -X PUT …/aktivitaet -d '{"zustand":"denkt_nach","gueltig_s":60}'`.
5. **Ereignis** auf dem **Signalbus** ([Live-Kanal](../Architektur/A00011-live-kanal.md)): `Signal::Aktivitaet { channel, kennung, aktiv: Option<(Zustand, bis)> }`, im Strom
   - gesetzt oder erneuert: `{"art":"aktivitaet","channel":"allgemein","kennung":"anna@10.0.0.7","zustand":"tippt","bis":"…","jetzt":"…"}` — **jede** Erneuerung ist ein Ereignis (neues `bis`);
   - beendet: `{"art":"aktivitaet","channel":"allgemein","kennung":"anna@10.0.0.7","zustand":null}` — nur, wenn vorher etwas aktiv und nicht abgelaufen war;
   - überholt: `{"art":"aktivitaet_luecke"}` (Regel 6).
   Ohne Stand, ohne Ordnungszahl, nie im Stand beim Öffnen des Stroms, nie im Nachholen, nie in `warten`.
6. **Signalbus und Strom**: ein zweiter `broadcast` mit Kapazität **1.024** neben dem Ereignisbus (`KAPAZITAET` bleibt für den Ereignisbus). Der Handler von `/api/ereignisse` abonniert beide und wählt mit `tokio::select! { biased; … }` in der Folge **Ende-Signal → Ereignisbus → Signalbus → Herzschlag** — liegen Nachricht und Signal zugleich an, geht die Nachricht zuerst. Filter je Strom: (a) die **eigene** Kennung des Stroms wird nie gesendet (man sieht sich nie selbst); (b) „sichtbar für“ wie für Nachrichten (Channels: alle mit Sitzung; Direktgespräche: nur Beteiligte, mit I0012). Überholt ein Strom den Signalbus (`Lagged`), sendet er **ein** `aktivitaet_luecke` und weiter nichts — kein `NeuerStand`, keine Episode (flüchtig; ein überholter Ereignisbus meldet sich ohnehin über die Episode `live`).
7. **Ende** (sofort, unter der Sperre der Aktivität, je mit Ende-Ereignis nach Regel 5):
   - a) **Löschen** über `DELETE` (Regel 4).
   - b) **Senden**: nach einer gelungenen Nachricht (201, auch mit Anhängen) dieser Kennung in diesem Gespräch ruft die Integration im Sende-Handler `beenden(kennung, gespraech)` — **nach** dem Veröffentlichen der Nachricht, außerhalb der Schreibsperre (die Aktivität kennt die Schreibsperre nicht). Bearbeiten und Löschen einer Nachricht beenden nichts.
   - c) **Im Tab**: kommt eine `nachricht` von Kennung K im Channel C an, blendet der Tab die Anzeige von K in C sofort aus, ohne auf das Ende-Ereignis zu warten (Reihenfolge über zwei Busse, Option B Nachteil); ein späteres Setzen derselben Kennung zeigt sie wieder.
   - d) **`weg`** (Verlassen, Gnadenfrist): `beenden_alle(kennung)`.
   - e) **`offline` einer Sitzung der Art `mensch`**: `beenden_alle(kennung)`; für die Art `agent` ist `offline` der Normalzustand zwischen zwei Warte-Aufrufen und beendet nichts (F5).
   - d) und e) hängen an der Funktion, mit der die Integration Anwesenheits-Wechsel veröffentlicht (`live::bus::anwesenheit_veroeffentlichen`): nach dem Ereignis `anwesenheit` ruft sie die Aktivität. Sperrfolge damit **Anwesenheit → Aktivität**; die Aktivität ruft die Anwesenheit nie (die Kennung kommt fertig aus dem Extraktor) — keine Verklemmung.
   - f) **Channel-Wechsel und leeres Feld**: der Browser ruft `DELETE` (Regel 8); der Server kennt den geöffneten Channel eines Tabs nicht.
8. **Browser** ([Weboberfläche](../Architektur/A00005-weboberflaeche.md)):
   - **Melden** (`web/tippen.js`, reine Funktion über Zeit und Feldinhalt): Eingabe in das Eingabefeld des Gesprächs, Feld danach nicht leer (`/^\p{White_Space}*$/u` ist leer, wie ADR-0038) → `PUT {"zustand":"tippt"}` sofort, danach höchstens alle 3 s, solange weiter getippt wird; nach einem Ende (gesendet, geleert, gelöscht) ist das nächste Setzen wieder sofort erlaubt. Feld wird leer → `DELETE`, falls eine Meldung aktiv ist. Channel-Wechsel → `DELETE` für den alten Channel, falls aktiv. Nach dem Senden kein `DELETE` (der Server beendet, Regel 7 b). Das Bearbeiten-Feld einer Nachricht meldet nichts (PO Zeile 136 nennt die Eingabe). Beim Schließen eines Tabs kein Aufruf (läuft ab, wie ADR-0018 „nie beim Schließen“). Antwortet das Setzen 429, wird bis `Retry-After` nicht gemeldet; Fehler zeigen keinen Hinweis (die Anzeige ist Beiwerk).
   - **Anzeigen** (`web/aktivitaetsanzeige.js`): je offenem Channel eine Tabelle Kennung → lokaler Ablauf = Empfang + (`bis` − `jetzt`) (Versatz-frei wie ADR-0019); Zeile an der Eingabe nur für den geöffneten Channel, Platz immer reserviert; Text aus einer reinen Funktion (Namen, Zählung ab drei; Wortlaut, Ansage und Zeilen beim `ux-reviewer`); Erneuerungen ändern den Text nicht; gezeichnet höchstens einmal je Frame (wie ADR-0037).
   - **Verbindung**: bei jedem Beginn eines Stroms (erster und jeder Wiederverbinden) und bei `aktivitaet_luecke` leert der Tab alle Anzeigen — nach einem Abriss wird nichts nachgeholt (PO Zeile 138). Der `SharedWorker` reicht `aktivitaet` nur durch und merkt sich nichts; ein neuer Tab beginnt leer.
9. **Bremse** (im Baustein, Eimer-Typ aus `src/schreibkonto/eimer.rs`, eigene Tabellen, **nicht** das Schreibkonto): nur **Setzen** kostet, 1 Einheit, gebucht nach allen 4xx-Prüfungen; `DELETE` und `GET` kosten nichts (Löschen erzeugt höchstens ein Ereignis je vorherigem Setzen, Lesen keines).

   | Eimer | Vorrat | Nachschub | Dauerlast höchstens |
   |---|---|---|---|
   | je Kennung **und Gespräch** | 10 | 1 je Sekunde | 1 Setzen/s je Gespräch |
   | je Herkunft | 120 | 12 je Sekunde | 12 Setzen/s für alle Kennungen einer Adresse |

   429 `{"fehler", "grenze": "gespraech" | "herkunft", "wieder_in_ms"}` mit `Retry-After` (Form wie ADR-0036 Regel 4); Text „Zu viele Aktivitätsmeldungen von `claude@10.0.0.5` in `# allgemein` — wieder möglich in 1 s.“ bzw. „… von `10.0.0.5` (alle Teilnehmer dieser Adresse zusammen) …“. Eine abgewiesene Meldung ändert nichts — die bestehende Anzeige bleibt bis zu ihrem Ablauf. Volle Eimer werden entfernt (wie ADR-0036 Regel 6). Herleitung: ein Mensch setzt höchstens alle 3 s und nach jeder gesendeten Nachricht einmal sofort (≤ 1/s auch im schnellen Wechsel), ein Agent erneuert alle 10–20 s oder setzt vor jeder Nachricht (≤ 8/s je Herkunft durch das Schreibkonto); 50 Agenten einer Herkunft mit Erneuerung alle 20 s und Setzen vor jeder ihrer höchstens 8 Nachrichten/s brauchen ≈ 10,5/s < 12.
10. **Sichtbarkeit**: Setzen, Löschen und Lesen gehen nur für ein Gespräch, das der Aufrufer sehen darf; sonst antworten sie wie für ein unbekanntes (404). Channels: jeder mit Sitzung. **Archivierte Channels** (I0022): Setzen und Lesen wie bei Senden und Verlauf auf einen archivierten Channel; beim Archivieren beendet die Integration alle Aktivitäten dieses Channels (Ende-Ereignisse). **Direktgespräche** (I0012): der Pfad folgt der Nachrichten-Route, die I0012 festlegt; Filter „sichtbar für“ im Strom wie für Nachrichten. Bis dahin prüft ein Unit-Test die Regel (R00045).
11. **Betrieb und Protokoll**: kein Eintrag je Signal auf irgendeiner Stufe, wenn es gelingt (`DEBUG` eingeschlossen; die Middleware schreibt nur Abweisungen, `src/http/anfrageprotokoll.rs`); einzelne 4xx als `DEBUG` wie jede Route (Regelschlüssel `zustand`, `gueltigkeit`, `feld`, `aktivitaeten`, `channel_unbekannt`); 429 als **Episode** `WARNING aktivitaet` „Aktivität gebremst: zu viele Meldungen“, Quelle `kennung`, Beginn zusätzlich `channel`, `grenze`, Ende Zählung je Grenze — Katalogzeile in [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) (Ergänzung „Karte #77“) und Testzeile im ausführbaren Katalog ([ADR-0050](0050-episodenauftreten-als-eigener-typ-quelle-pflicht-einzelwerte-nur-im-beginn.md) Regel 6). Kein neues Feld im Health-Check, keine Einstellung, kein Testknopf (Ablauf wird mit kleinen `gueltig_s` und der gestellten Uhr geprüft).
12. **Nicht berührt**: Tageslog, Verlaufsindex, Plattenquote, Anhangquote, Schreibkonto, `warten` (abonniert den Signalbus nie), Teilnehmerliste und Seitenleiste.

### Lastnachweis (gerechnet; gemessen wird P11)

- **Signalrate im Lastprofil**: Setzen durch Menschen 20 × 1/3 s ≈ 6,7/s; Erneuern durch Agenten 50 / 20 s = 2,5/s; Ende durch Senden ≤ Nachrichtenrate, solange der Absender etwas aktiv hatte: Spitze ≤ 30/s, dauerhaft ≤ 5/s; Ende durch leeres Feld oder Wechsel ≤ Setzen der Menschen ≈ 6,7/s. **Spitze ≈ 46 Signale/s, dauerhaft ≈ 21/s** (PO Zeile 139 rechnete ≈ 23/s ohne Enden).
- **Bus**: Signalbus 1.024 Plätze ≈ 22 s Spitze für einen langsamen Leser; der Ereignisbus behält seine ≈ 34 s (A00001), weil kein Signal ihn belegt.
- **Strom**: ≈ 160 Bytes je Signal mit SSE-Rahmen → ≈ 7,4 KB/s je Strom in der Spitze; 20 Browser + 50 Agenten mit Strom = 70 Ströme ≈ 0,5 MB/s im LAN. Je Signal und Strom eine JSON-Serialisierung: ≈ 46 × 70 ≈ 3.200/s, Mikrosekunden je Stück → unter 1 % eines Kerns.
- **Zustellung der Nachrichten**: der einzige geteilte Weg ist die Schreibschleife des Stroms; mit `biased` wartet eine Nachricht höchstens auf das Schreiben **eines** Signals, das schon unterwegs war (Mikrosekunden). Erwartet: Zustellung unverändert wenige ms; Grenze 1 s.
- **Speicher**: ≤ 16 Einträge × 512 Sitzungen = 8.192 Einträge ≈ 1,2 MB; Eimer in derselben Größenordnung.
- **Flut**: eine Herkunft erzeugt dauerhaft ≤ 12 Setzen/s + ≤ 12 Enden/s = 24 Signale/s an jeden Strom (Lehre #5: 2.456 Ereignisse/s froren einen Tab; ADR-0036 S4: 8 Nachrichten/s); viele Herkünfte (IPv6) umgehen den Eimer je Herkunft — dieselbe angenommene Schwäche wie ADR-0021 und ADR-0036.

## Konsequenzen

**Positiv:**
- Nachrichten bleiben unter jeder Signallast vorn; ein Überholen durch Signale kostet keinen Neuladen.
- „Flüchtig“, „kein Nachholen“ und „weckt nicht“ gelten durch den Bau: kein Speicherort außer einer Tabelle im Speicher, kein Stand im Strom, `warten` hört den Signalbus nicht.
- Eine Route, ein Wortlaut, eine Regel für Mensch und Agent; der Browser ist nur ein Aufrufer mit Standardwerten.

**Negativ / Inkauf zu nehmen:**
- Zwei Abonnements je Strom; das Ende kann vor der Nachricht ankommen, die es auslöst (Regel 7 c fängt es im Tab).
- Wer den Strom roh liest, muss den Ablauf aus `bis` und `jetzt` selbst rechnen.
- Die `offline`-Regel liest die selbst gewählte `art` (F5); ein Mensch, der per API mit `art: agent` beitritt, „tippt“ nach einem Abriss bis zum Ablauf weiter.
- Zwei Tabs desselben Browsers im selben Channel teilen einen Eintrag: wechselt der eine den Channel, löscht er auch das Tippen des anderen, bis dessen nächste Meldung (≤ 3 s).
- Ein Agent, der „denkt nach“ in Schleife erneuert, hält seine Sitzung am Leben (jede Anfrage ist ein Lebenszeichen) — gewollt, aber kein Beweis, dass er arbeitet.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … Nachrichten unter Last spät kommen, weil der Strom Signale abarbeitet. Früh-Indikator: P11 max > 0,25 s oder Episode `live` „überholt“ nach Einführung häufiger. Gegenmaßnahme: eigener Bus (Option B), `biased` mit Ereignisbus vor Signalbus, Mutante M3.
2. … „anna tippt …“ nach dem Senden stehen bleibt. Früh-Indikator: E2E „Verschwinden sofort“ wackelt; Anzeige verschwindet erst nach 6 s. Gegenmaßnahme: Ende serverseitig nach dem Senden (7 b), im Tab bei Ankunft der Nachricht (7 c), lokaler Ablauf als Netz, Leeren bei Lücke und neuem Strom.
3. … „claude denkt nach …“ nach einer Sekunde verschwindet, weil der Agent zwischen zwei Warte-Aufrufen `offline` ist. Früh-Indikator: Integration P7 (Agent ohne Strom setzt, liest nach 5 s noch). Gegenmaßnahme: Regel 7 e (offline beendet nur `mensch`), Mutante M6.
4. … Bildschirmleser bei jedem Tastendruck reden. Früh-Indikator: E2E „Bildschirmleser“ zählt Textwechsel der Ansage-Region. Gegenmaßnahme: Erneuerungen ändern den Text nicht (Regel 8), Wortlaut und Ansage beim `ux-reviewer`.
5. … eine Schleife vieler Kennungen die Tabelle wachsen lässt. Früh-Indikator: Speicher je Sitzung. Gegenmaßnahme: 16 je Kennung, Sitzungen ≤ 512, Aufräumen beim Zugriff.

**Folgeentscheidungen / Aufgaben** (Vorschläge an den planer, keine Arbeitsliste des architekt):
- [ ] I0040: Baustein Aktivität (Regeln 1–3, 9), Signalbus und Strom (5, 6), `PUT`/`DELETE` (4), Ende-Haken (7 a–e), Browser (8), Katalogzeile (11) — Bausteine und Mini-Eignung siehe Bericht Karte #77.
- [ ] I0041: `GET` (4), `gueltig_s` (3), `denkt_nach` an der API (2), API-Beschreibung und Anleitung, Lastprobe P11.
- [ ] I0012, I0022: Regel 10 nachziehen (Pfad im Direktgespräch, Ende beim Archivieren).

### Abnahme-Proben (Test zuerst; „heute“ jeweils rot, weil nichts gebaut ist)

- **P1 Ablauf (Unit, gestellte Uhr t0)**: `setzen(anna, allgemein, tippt, None)` → `bis = t0 + 6 s`; `lesen(fuer: ben)` bei t0 + 5,999 s → `[anna]`, bei t0 + 6,000 s → `[]`. `setzen(claude, denkt_nach, None)` → t0 + 30 s; erneut bei t0 + 20 s → t0 + 50 s (nicht 60); `Some(120)` → t0 + 120 s; `Some(121)`, `Some(0)` → `Err(Gueltigkeit)` ohne Wirkung. Uhr auf t0 − 10 s → Eintrag gilt als abgelaufen.
- **P2 Ein Eintrag je Kennung und Gespräch (Unit)**: `tippt` dann `denkt_nach` → ein Eintrag `denkt_nach`; Veröffentlichen-Attrappe zählt zwei Ereignisse, das zweite unter der Sperre (`try_lock` besetzt). 16 Gespräche → ok, 17. → `Err(ZuViele)`.
- **P3 Bremse (Unit, gestellte Uhr)**: 10 × Setzen `(anna, allgemein)` bei t0 → ok, 11. → `Gebremst{gespraech, 1.000 ms}`; t0 + 999 ms gebremst, t0 + 1.000 ms ok; 10 × in `# technik` danach ok (anderes Gespräch). Herkunft: 120 Setzen über 12 Kennungen × 10 Gespräche → ok, 121. → `Gebremst{herkunft}`. `beenden` und `lesen` nie gebremst.
- **P4 Strom (Integration, echtes Programm)**: `anna`, `ben` lesen den Strom; `anna` `PUT tippt` → `ben` liest `aktivitaet` mit `zustand: "tippt"` binnen 1 s nach der 200 (wie Z10); `anna`s Strom liest binnen 500 ms **kein** `aktivitaet`. `anna` sendet → `ben` liest `nachricht`, danach `aktivitaet` mit `zustand: null`, in dieser Reihenfolge.
- **P5 Inhaber (Integration)**: `cora` `PUT {"zustand":"tippt","kennung":"anna@127.0.0.1"}` → 422 `regel: "feld"`, `GET` von `ben` leer; `cora` `PUT tippt` → nur `cora`; `anna` `PUT tippt`; `cora` `DELETE` → `GET` von `ben` nennt nur `anna`.
- **P6 Flüchtig (Integration)**: 30 Setzen und Löschen über 10 s → `sha256sum` über `channels/` gleich, Betriebsprotokoll auf Stufe Debug ohne Zeile mit `aktivitaet` außer Abweisungen; Strom neu verbinden → erstes Ereignis ist `teilnehmer`, kein `aktivitaet` vor dem nächsten Setzen.
- **P7 Agent offline (Integration)**: `claude` tritt mit `art: agent` bei, hat keinen Strom, `PUT denkt_nach` → `GET` von `ben` nach 5 s nennt `claude`. `anna` (`art: mensch`) mit Strom `PUT tippt`, Strom schließen → `ben` liest binnen 2 s `zustand: null` für `anna`. `claude` `POST /api/verlassen` → `zustand: null` binnen 1 s.
- **P8 Fehlerfälle (Integration)**: `zustand` `"schreibt"`, `"denkt nach"` (mit Leerzeichen), `"Tippt"` → 422 `zustand`; `gueltig_s` `0`, `-1`, `1.5`, `"30"`, `121` → 422 `gueltigkeit`; fremder `Origin` bei `PUT`/`DELETE` → 403; unbekannter Channel → 404; ohne Sitzung → 401; jeweils keine Wirkung (`GET` leer).
- **P9 Bremse an der Route (Integration)**: 11 × `PUT` in < 1 s → zehnmal 200, einmal 429 `grenze: "gespraech"` mit `Retry-After: 1`; Anzeige bei `ben` bleibt; nach 1 s wieder 200; danach `POST …/nachrichten` → 201 (Schreibkonto unberührt); im Protokoll genau eine Beginn-Zeile `WARNING aktivitaet`.
- **P10 Kein Wecken (Integration, sobald I0016 gebaut)**: `codex` wartet mit `bis=5`; `anna` setzt und erneuert 5 s → `warten` antwortet nach 5 s leer; eine Nachricht von `anna` weckt.
- **P11 Last (Gate in I0041, Lastprofil Team-LAN)**: wie R00045 „Last“; zusätzlich: keine Episode `live`, kein `aktivitaet_luecke` bei einem zügigen Leser; im Bericht Leerlauf-p90, p50, p90, max der Zustellung.

### Mutanten (jeder muss mindestens eine Probe rot machen)

| # | Mutante | rot in |
|---|---|---|
| M1 | Strom filtert die eigene Kennung nicht | P4 |
| M2 | `GET` liefert die eigene Kennung mit | R00045 „Lesen“ |
| M3 | Signal auf den Ereignisbus statt auf den Signalbus (oder `biased` weg, Signalbus zuerst) | P4-Reihenfolge, P11, P10 |
| M4 | `bis` addiert statt ab Eingang | P1 |
| M5 | `gueltig_s > 120` begrenzt statt abgewiesen | P1, P8 |
| M6 | `offline` beendet auch Agenten | P7 |
| M7 | Senden beendet nicht | P4 |
| M8 | Kennung aus dem Körper statt aus der Sitzung (Feld erlaubt) | P5 |
| M9 | Bremse aus (Eimer je Gespräch) bzw. Eimer je Herkunft aus | P3, P9 |
| M10 | Setzen bucht am Schreibkonto | P9 (Senden danach 429) |
| M11 | Sichtbarkeitsprüfung aus (nicht sichtbar → Wirkung) | Unit Sichtbarkeit (bis I0012), R00045 „Sichtbarkeit“ |
| M12 | Worker merkt sich `aktivitaet` für neue Tabs / Tab leert nicht bei neuem Strom | E2E „Flüchtig“ (Strom neu, keine Anzeige) |
| M13 | Erneuerung ändert den Ansage-Text | E2E „Bildschirmleser“ |
| M14 | Uhr rückwärts verlängert Einträge | P1 |

## Quellen / Bezuege

- [A00019 Aktivität](../Architektur/A00019-aktivitaet.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md), [A00014 Schreibkonto](../Architektur/A00014-schreibkonto.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md)
- ADR-0007, ADR-0012, ADR-0013, ADR-0014, ADR-0015, ADR-0016, ADR-0018, ADR-0019, ADR-0028, ADR-0031, ADR-0036, ADR-0037, ADR-0038, ADR-0042, ADR-0050
- R00045, WBS I0040, I0041, I0012, I0016, I0022; PO-Entscheidungen Zeilen 88, 133–141; Vision „Wer tippt oder nachdenkt, ist sichtbar“, Leitplanke „Lastprofil Team-LAN“
- Referenzen (Verhalten, nichts übernommen; aus der Karte #77): Slack `assistant.threads.setStatus` (endet mit der Antwort, leerem Status oder nach 2 Minuten), Discord Trigger Typing Indicator (verfällt nach 10 s, ohne Zustand im Server)

## Ergänzungen

- **2026-10-30 (Karte #13, I0012) — Verweis:** Regel 10 „Direktgespräche (I0012)“ eingelöst in [ADR-0055](0055-direktgespraeche-paar-aus-zwei-kennungen-gegenueber-im-pfad-anlegen-beim-ersten-senden-eine-sichtbarkeitsregel-sichtbare-marke.md) Regeln 3, 5 und 6: Route `/api/direkt/{kennung}/aktivitaet`, `sichtbar_fuer` ersetzt `aktivitaet::sichtbar`, Ausgabe im Strom nur an das Gegenüber — Vorgeschlagen.
