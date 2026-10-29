---
id: R00032
status: Neu
datum: 2026-10-29
---

# R00032: Doppelte Schlüssel im Körper abweisen

## Beschreibung

Ein JSON-Körper, der denselben Schlüssel zweimal enthält, wird abgewiesen statt still umgeformt. Heute sendet `POST …/nachrichten` mit `{"text":"a","text":"b"}` die Nachricht `b`, und `PUT …/frist` mit `{"tage":1,"tage":3650}` setzt 3650 — der letzte Wert gewinnt, ohne dass der Absender es merkt. Danach gilt:

- **Doppelt heißt 422, nichts geändert**: Senden, Frist setzen, Beitreten und Channel anlegen antworten auf einen doppelten Schlüssel der obersten Ebene mit 422 `{fehler, regel: "feld", feld: "<name>", grund: "doppelt"}` und dem Text „Das Feld <name> steht zweimal im Körper — jedes Feld nur einmal schicken.“ (wörtlich wie Anmelden, R00029). Keine Nachricht, keine Frist, keine Sitzung, kein Ordner.
- **Ein Ort für alle**: der Körperleser weist das Doppelte ab, bevor die Route den Körper sieht — direkt nach den Körperfehlern 413/415/400/408, beim Senden auch vor der 404 eines unbekannten Channels. Jede künftige Route über denselben Leser erbt die Regel.
- **Senden nennt das unbekannte Feld zuerst**: beim Senden kommt ein weiteres Feld vor dem Typfehler von `text` — `{"x":1}` antwortet `feld`/`x` statt `text`; so nennt die Antwort den Tippfehler.
- **Die Betreiberin sieht es auf Debug**: je Abweisung eine `DEBUG`-Zeile mit Grund `feld_doppelt`, nie mit Feldname oder Wert; kein neuer Zähler im Health-Check.

Zahlt ein auf: [Vision](R00000-vision.md) — „Mensch und KI sind gleichberechtigt“ (ein Agent, dessen Werkzeug versehentlich doppelte Schlüssel erzeugt, bekommt eine Antwort, die Feld und Ausweg nennt, statt dass still ein anderer Text im Log landet) und „eine gemeinsame LAN-Instanz“ (eine mehrdeutige Anfrage kann nicht je nach Leser anders wirken). Kein Widerspruch zur Vision; der architekt hat keine Richtungsfrage gesehen (ADR-0034 Ergänzung „Karte #47“).

Herkunft: Karte #47 — Security-Gate #9 (security-reviewer und architekt, 2026-10-29), Vorschlag L9 aus [ADR-0046](../Dokumentation/ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Security-Gate“. Entschieden in [ADR-0034](../Dokumentation/ADRs/0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) Ergänzung „Karte #47“ (U2 zentral im Körperleser; Reihenfolge doppelt → weiteres Feld → Typ; 422 `feld`/`doppelt`; Debug `feld_doppelt`).

Weiche (`rollen/planer/befunde-einplanen.md`): **fehlt**, nicht falsch gebaut — R00011 und die übrigen Anforderungen sagen zu doppelten Schlüsseln nichts zu; „abgewiesen, nie umgeformt“ (R00011 F0041) ist die Haltung, die diese Anforderung auf den doppelten Schlüssel ausdehnt.

Slice (WBS `Dokumentation/Planung/localchata.md`, unter I0009 Nachricht senden): F0123 Doppelte Schlüssel im Körper abgewiesen.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00010, R00011, R00016 und R00029: Körper, Körperfehler, Körperleser, Feld, Regel (`regel` der 422), Sitzung, Kennung, Herkunft, Schreibkonto, Einheit, Vorab.

## Geschäftlicher Nutzen

Ein Agent, dessen Werkzeug einen Körper aus zwei Quellen zusammensetzt, schickt `{"text":"Entwurf","text":"Endfassung"}`. Heute steht „Endfassung“ im Log, ohne dass irgendwer erfährt, dass es zwei Werte gab — welcher gewinnt, entscheidet der Leser, nicht der Absender. Bei der Frist ist es schärfer: `{"tage":1,"tage":3650}` setzt zehn Jahre, ein anderer Leser hätte einen Tag gesetzt und Daten gelöscht. Mit R00032 bekommt der Absender eine 422, die das Feld nennt, und nichts ändert sich.

## Funktionale Anforderungen

### Doppelte Schlüssel im Körper abgewiesen (F0123; ADR-0034 Ergänzung „Karte #47“)

- **Erkennen**: der Körperleser (`json_lesen`, `json_lesen_auf_werkbank`) findet den ersten Schlüssel der obersten Ebene, der ein zweites Mal vorkommt. Verglichen werden die **dekodierten** Schlüssel (`"text"` ist `text`). Verschachtelte Objekte prüft er nicht.
- **Reihenfolge**: Körperfehler (413, 415, 400, 408) → **doppelt** → was die Route prüft. Beim Senden heißt das: doppelt vor der 404 des Channels; danach **weiteres Feld** (422 `{regel: "feld", feld}`) vor **`text` fehlt oder ist keine Zeichenkette** (422 `text`).
- **Antwort**: 422 `{"fehler": "Das Feld <name> steht zweimal im Körper — jedes Feld nur einmal schicken.", "regel": "feld", "feld": "<name>", "grund": "doppelt"}`. Nichts geändert, nichts gemerkt (kein Wiederholschlüssel). Kosten wie jeder Körperfehler: nur die Vorab-Einheit am Schreibkonto (ADR-0036 Regel 3); Beitreten hat kein Konto.
- **Protokoll**: eine `DEBUG`-Zeile je Abweisung wie jede 422 (ADR-0042 Regel 2): Route, Status 422, Grund `feld_doppelt`, Kennung falls bekannt, Herkunft; **nie** Feldname oder Wert. Kein neuer Zähler in `/api/gesundheit`.
- **Unverändert**: Frist, Beitreten und Channel anlegen dulden weiter **weitere** (unbekannte) Felder; nur das Doppelte weisen sie ab. Anmelden (`felder_lesen`) prüft weiter selbst, mit derselben Antwort.

## Nicht-funktionale Anforderungen

- **Sicherheit** (CWE-436, mehrdeutige Eingabe): kein Körper mit doppeltem Schlüssel der obersten Ebene ändert über diese vier Routen einen Zustand.
- **Betrieb**: die Abweisung ist auf Debug sichtbar und verrät keinen Inhalt.
- **Kosten**: der Leser parst den Körper ein zweites Mal (Paare in Reihenfolge); beim Senden auf der Werkbank, höchstens 131.072 Bytes. Kein neues Zeitkriterium.

## Akzeptanzkriterien

Gemeinsam: Start, Beitritte per `curl` wie in R00011; Channel `allgemein` existiert; „mit Sitzung“ heißt mit dem Token einer beigetretenen Kennung. Jedes Integrationskriterium startet eine eigene Instanz mit `--protokoll-stufe debug`.

### F0123 Doppelte Schlüssel im Körper abgewiesen

- [ ] **Körperleser** (Unit, rein; erster Test aus ADR-0034 Ergänzung „Karte #47“, wörtlich): „`{"a":1,"a":2}` → Fehlerfall doppelt mit `a`; `{"a":1,"a":2}` → doppelt; `{"a":1,"b":2}` → `Value` wie bisher; `[1]` und kaputtes JSON wie bisher.“ Dazu dekodiert: `{"a":1,"a":2}` → doppelt mit `a`. Die Mutante „letzter Wert gewinnt“ (Prüfung im Leser abgeschaltet) wird rot.
- [ ] **Sendekörper** (Unit, rein; wörtlich aus der ADR): „Unit `sendekoerper_lesen`: `{"x":1}` → `Feld{x}`, `{"text":1}` → `Text`.“ Dazu `{"text":1,"x":1}` → `Feld{x}`; die Fälle aus R00011 („Sendekörper“: `{}`, `{"text":5}`, `{"text":null}`, `[1]`, `"Hallo"` → `text`; `{"text":"Hallo","antwort":"x"}` → `feld`/`antwort`) bleiben grün.
- [ ] **Abnahme-Probe über alle vier Routen** (Integration, echtes Programm; Kriterium aus ADR-0034 Ergänzung „Karte #47“, wörtlich): „Gegeben eine laufende Instanz mit Channel `allgemein` und einer Sitzung: `POST /api/channels/allgemein/nachrichten` mit `{"text":"a","text":"b"}` → 422 `{regel: "feld", feld: "text", grund: "doppelt"}`, keine Nachricht in Tagesdatei, Verlauf und Strom; mit `{"text":"a","text":"b"}` → dasselbe; mit `{"text":"a","x":1,"x":2}` → `feld: "x"`, `grund: "doppelt"`; mit `{"x":1}` → 422 `{regel: "feld", feld: "x"}` ohne `grund`; mit `{"text":"a","text":"b"}` an `/api/channels/gibtsnicht/nachrichten` → 422 `doppelt` (nicht 404); `PUT /api/channels/allgemein/frist` mit `{"tage":1,"tage":7}` → 422 `feld`/`tage`/`doppelt`, Frist unverändert; `POST /api/beitreten` mit `{"name":"anna","name":"bob"}` → 422 `feld`/`name`/`doppelt`, keine Sitzung, kein Token; `POST /api/channels` mit `{"name":"neu","name":"neu2"}` → 422 `feld`/`name`/`doppelt`, kein Ordner. Je Abweisung genau eine `DEBUG`-Zeile mit Grund `feld_doppelt`, in keiner Zeile `anna`, `bob` oder `neu2`; jede Abweisung mit Sitzung kostet am Schreibkonto 1 Einheit (Beitreten hat kein Konto). Gültige Körper aller vier Routen antworten wie vorher.“
  Lesart des planer: die zweite Variante „→ dasselbe“ ist der dekodiert gleiche Schlüssel — im Test als roher Körper `{"text":"a","text":"b"}` (Escape im JSON, nicht in Rust), Antwort `feld: "text"`, `grund: "doppelt"`. Der `fehler` jeder Abweisung ist „Das Feld <name> steht zweimal im Körper — jedes Feld nur einmal schicken.“ mit dem gemeldeten Feld.
- [ ] **Kosten am Schreibkonto** (Integration; Teil der Probe, ausgeschrieben): eine Kennung schickt 40 Körper `{"text":"a","text":"b"}` an `allgemein` nacheinander so schnell wie möglich → zuerst nur 422, danach nur 429 `grenze: "kennung"`; die Zahl der 422 liegt zwischen 30 und 30 + ⌈Dauer der Folge in s⌉. Rechnung: Vorrat je Kennung 30 (R00011), je Abweisung 1 Einheit vorab, Nachschub 1 je s — dauert die Folge 0,4 s, sind es 30 oder 31 × 422 und 10 oder 9 × 429. Die Mutante „doppelt kostet 0“ (40 × 422) wird rot.
- [ ] **Nichts merkt sich** (Integration): nach der 422 auf `{"text":"a","text":"b"}` mit `Idempotency-Key: k47` (Wiederholschlüssel nach R00012/R00025) antwortet `{"text":"a"}` mit demselben Schlüssel 201 (nicht 422 `schluessel_anders`).
- [ ] **Unverändert gültig**: die Integrationstests aus R00003 (Beitreten), R00006 (Channel anlegen), R00008/R00019/R00021 (Frist), R00011/R00020/R00025 (Senden) und R00029 (Anmelden, `doppelt`) laufen grün; wo einer `{"x":1}` beim Senden als `text` erwartet hat, zieht er auf `feld`/`x` mit (Teständerung, gedeckt durch ADR-0034 Ergänzung „Karte #47“).

## Betroffene Verzeichnisstruktur

- Server: `src/http/koerper.rs` (Leser `json_lesen`, `json_lesen_auf_werkbank`, Fehlerfall doppelt, `koerperfehler_beantworten`), `src/http/felder.rs` (wiederverwendet: `felder_lesen`, `doppelt`), `src/http/senden.rs` (`sendekoerper_lesen`: weiteres Feld vor `text`), `src/http/anfrageprotokoll.rs` (Grund `feld_doppelt`, falls nicht schon vorhanden).
- Tests: Units in `src/http/koerper.rs` und `src/http/senden.rs`; Integration neu `tests/koerper_doppelt.rs`; bestehende `tests/senden.rs` u. a. ziehen mit, wo sie `{"x":1}` als `text` erwarten.
- Keine Oberfläche: `JSON.stringify` erzeugt nie doppelte Schlüssel.

## Technische Überlegungen

### Ablauf

1. Route → Grenzschutz → Sitzung → Vorab-Buchung (Routen mit Konto) → Körper lesen: 413 · 415 · 400 · 408 · **doppelt → 422 `feld`/`doppelt`** → Route.
2. Senden: Channel bekannt? (404) → **weiteres Feld** (422 `feld`) → `text` Zeichenkette? (422 `text`) → Regeln aus R00011.

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0123: B0546 Körperleser erkennt doppelte Schlüssel · B0547 Doppelt beantwortet und protokolliert · B0548 Senden nennt das weitere Feld vor `text` · B0549 Doppelte Schlüssel über alle vier Routen geprobt

Elemente (Namen entscheidet der Entwickler): `Koerperfehler::Doppelt { feld: String }` neben den bestehenden Körperfehlern; der Leser ruft nach dem Parsen `felder::felder_lesen` + `felder::doppelt` (ADR-0046 L9, gebaut mit B0427).

### Änderungen an bestehenden Klassen

- `http::koerper`: neuer Fehlerfall, Prüfung in beiden Lesern, Antwort und Debug-Grund in `koerperfehler_beantworten`.
- `http::senden::sendekoerper_lesen`: Schleife über die Schlüssel vor der `text`-Prüfung.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Unit (rein)**: Körperleser, Sendekörper.
- **Integration (echtes Programm)**: Abnahme-Probe über alle vier Routen, Kosten, nichts merkt sich.
- **E2E**: keine (keine Oberfläche).

## Abhängigkeiten

- Abhängig von: F0041 Senden über die API (gruen), B0427 Anmelden beantworten (Feldleser `src/http/felder.rs`; gebaut).
- Nicht parallel mit: Karte #9 (I0010, F0097) — `src/http/koerper.rs` und `src/http/senden.rs` werden dort geändert. Gebaut wird F0123, wenn beide Dateien in #9 nicht mehr offen sind (Hinweis architekt #47).
- Blockiert: —

## Offene Fragen

Keine an den Betreiber. Entschieden vom planer (Rückfragen mit Empfehlung beantwortet):

- **Zweite Variante der Probe**: die ADR nennt `{"text":"a","text":"b"}` zweimal; die zweite Zeile („→ dasselbe“) steht für den dekodiert gleichen Schlüssel (Antwort-Zeile der ADR: „Schlüssel dekodiert verglichen“) und wird als `{"text":"a","text":"b"}` geprobt. Wörtlich übernommen ist die Probe trotzdem.
- **Kosten belegt über die Erschöpfung**, nicht über einen Zähler: es gibt keinen Health-Wert für das Konto (ADR-0034 „Protokoll“: kein neuer Zähler).

## Manuelle Vorbereitungstätigkeiten

Keine.

## Manuelle Nachbereitungstätigkeiten

- Agenten-Anleitung (falls vorhanden, I0019): Satz „jedes Feld nur einmal schicken“ — nur, wenn sie Körperregeln aufzählt.

## Umfang

Gezählt aus der WBS (Bubbles); 0,4 h je klare Bubble ist die Untergrenze nach dem Skill `work-breakdown-structure` (Hypothese, ungemessen für diesen Typ); unklar als Spanne.

- F0123: 4 Bubbles · 3 klar (1,2 h) · 1 unklar (B0549, 2–4 h: vier Routen, Debug-Zeilen und Erschöpfung des Kontos im echten Programm)
- Zusammen: 4 Bubbles · 1,2 h klar + 2–4 h unklar

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: die Probe des security-reviewer im Gate #9 — `{"text":"a","text":"b"}` sendet still `b`.

- Der Leser prüft auf doppelte Schlüssel, bevor eine Route den Körper sieht. Deshalb gibt es keinen Körper mehr, dessen Bedeutung vom Leser abhängt, und keine Route kann die Prüfung vergessen — so war die Lücke beim Senden entstanden.
- Die Antwort nennt Feld und Ausweg. Deshalb repariert ein Agent sein Werkzeug, statt zu raten, welcher Wert angekommen ist.
- Senden nennt das weitere Feld vor dem Typfehler. Deshalb liest der Agent bei `{"txet":"…"}` den Tippfehler, nicht „text fehlt“.
- Die Debug-Zeile `feld_doppelt` ohne Werte. Deshalb sieht die Betreiberin ein Werkzeug, das plötzlich doppelt schickt (Pre-Mortem 1 der ADR), ohne dass Inhalte ins Protokoll gelangen.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).

## Notizen

- Bewusst außerhalb: weitere Felder bei Frist, Beitreten und Channel anlegen abweisen (U3 der ADR, verworfen); verschachtelte Objekte (keine Route nimmt eines an).
- Review-Punkt für den code-reviewer (Pre-Mortem 2 der ADR): Körper nur über `koerper::json_lesen*` oder `felder_lesen`.
