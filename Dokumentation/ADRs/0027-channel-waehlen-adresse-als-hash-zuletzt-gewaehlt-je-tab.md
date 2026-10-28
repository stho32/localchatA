# ADR-0027: Channel wählen — Adresse `#/c/<name>`, Seitenleiste aus Links, zuletzt gewählter Channel je Tab; Name und Beschreibung in der Kopfzeile

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #7 I0008)
**Kontext-Anforderung**: — (WBS I0008; nimmt F0026 auf; prägt I0012, I0013, I0011)
**Erprobt durch**: — (folgt dem Vorbild kanbanD — Hash-Adressen `#/board/<id>` und `sessionStorage` je Tab in `web/app.js`, Quelle Q04, nur gelesen — und der gebauten Seitenleiste `web/seitenleiste.js`, ADR-0024)

## Kontext

- Fertig-Kriterium I0008: „Auswahl zeigt den Verlauf …“; Auftrag: wählen per Klick oder Tastatur, Adresse für Neuladen und Links, zuletzt gewählter Channel je Tab.
- Designsystem: Seitenleiste — aktiver Eintrag auf `kd-accent` mit `aria-current="page"`; `1-tastatur.md` — `Alt ↑`/`Alt ↓` voriger/nächster Channel „überall“, `Strg P` Schnellwechsel (später), `Strg K` mit `#`-Sprung (I0011); `components/Kopfzeile` — Zone „Ort“: Instanzname, aktueller Channel in `lc-title`, **danach die Channel-Beschreibung abgeschnitten mit Ellipse**, unter 720 px entfällt sie (Vorschau: `lc-head-desc`); `components/Hauptansicht` — leerer Channel „Noch keine Nachrichten in # name“, darunter wie Agenten hier schreiben.
- [ADR-0024](0024-seitenleiste-und-anlegen-dialog-mitte-bleibt-bis-i0008.md): Einträge sind bis I0008 keine Links, kein `aria-current`; Regel 8 legt den sichtbaren Ort der Beschreibung „in den Kopf des Channels in I0008“.
- [ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md): ein Strom je Browser über einen `SharedWorker`; mehrere Tabs teilen eine Sitzung (`localStorage`). Zwei Tabs sollen verschiedene Channels zeigen können.
- [ADR-0020](0020-channelname-ist-ordnername-erlaubnisliste.md): Namen sind `[a-z0-9-]` — im Hash ohne Kodierung.
- Beschreibung (F0026): Regel in [ADR-0022](0022-channel-einstellungen-je-channel-als-toml.md) Regel 5 entschieden; die API weist heute jede gesetzte Beschreibung mit 422 `regel: "beschreibung"` ab (`src/http/channels.rs`).

## Optionen

### Option A: Hash-Adresse `#/c/<name>`; zuletzt gewählter Channel je Tab in `sessionStorage`
- Vorteile: kein Server-Code — die Seite bleibt `/`, Neuladen und Lesezeichen funktionieren; Zurück/Vor des Browsers wechselt zwischen Channels; Mittelklick öffnet einen Channel im neuen Tab; Einträge sind echte Links (Tastatur, Kontextmenü „Link kopieren“); dasselbe Muster wie kanbanD (gleiches Bedienkonzept, Leitplanke der Vision).
- Nachteile: Adressen mit `#` wirken altmodisch; ein Link enthält nur den Channel, nicht die Stelle im Verlauf.

### Option B: Pfad `/c/<name>` mit History-API
- Vorteile: saubere Adressen; später `/c/<name>/<zeit>` als Permalink naheliegend.
- Nachteile: der Server muss jede Client-Adresse kennen und `index.html` dafür ausliefern (eine Route je Oberflächenpfad, Kopplung von HTTP-Schnittstelle an die Navigation der Oberfläche); Grenzschutz und CSP für einen weiteren Pfad; Mehrwert gegenüber A nur in der Optik.

### Option C: Keine Adresse; Auswahl nur im Speicher des Tabs (`sessionStorage`)
- Vorteile: am wenigsten Code.
- Nachteile: keine Links zwischen Menschen und Agenten („schau in `#/c/security-entwicklung`“), kein Zurück, kein Mittelklick.

### Option D: Zuletzt gewählter Channel je Browser (`localStorage`) statt je Tab
- Vorteile: ein neuer Tab öffnet gleich den gewohnten Channel.
- Nachteile: zwei Tabs mit verschiedenen Channels überschreiben sich gegenseitig den Merker; der Auftrag will „je Tab“.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass Links, Neuladen, Zurück und Mittelklick ohne eine Zeile Server-Code richtig funktionieren und das Muster dem Geschwister kanbanD folgt. In Kauf genommen: `#` in der Adresse, und ein neuer Tab ohne Adresse zeigt noch keinen Channel.

Regeln:

1. **Adresse** `#/c/<name>` wählt den Channel `name`. Die Auswahl ist eine **Ableitung** aus Adresse und Channel-Liste (reine Funktion `auswahl(hash, liste) → {name, bekannt} | keine`), kein eigener Zustand. Bekannt → Verlauf öffnen ([ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md) Regel 5). Unbekannt oder ungültig → die Mitte sagt „`# name` gibt es nicht.“; erscheint der Channel später in der Liste (live, ADR-0023), öffnet er sich, weil die Ableitung erneut läuft. Andere Hash-Formen lassen die Auswahl leer.
2. **Seitenleiste**: jeder Eintrag ist `<a href="#/c/<name>">` mit `# name`; der gewählte trägt `aria-current="page"` und die Fläche `kd-accent` (Designsystem). Klick, `Enter` und Mittelklick wirken nativ; geöffnet wird auf `hashchange` (und beim Start). Eine Wahl über Link ist ein neuer Eintrag im Browser-Verlauf (Zurück geht zum vorigen Channel).
3. **Zuletzt gewählt je Tab**: jede Auswahl schreibt `sessionStorage['localchatA.channel']`. Startet die Seite **ohne** Hash, setzt sie den gemerkten Namen mit `history.replaceState` (kein neuer Verlaufseintrag) — damit stellt Neuladen ohne Adresse den Channel des Tabs wieder her. Ohne Hash und ohne Merker ist **kein** Channel gewählt; die Mitte zeigt „Links einen Channel wählen.“ (bei leerer Liste nichts zusätzlich — die Seitenleiste sagt „Noch keine Channels.“, ADR-0024 Regel 2). Kein automatisches Wählen des ersten Channels.
4. **Tastatur**: `Alt ↑`/`Alt ↓` wählt den vorigen/nächsten Channel in der Reihenfolge der Seitenleiste (über die Adresse, wie ein Klick), überall außer bei offenem Dialog; am Rand nichts. Der Fokus bleibt, wo er ist. Ohne Auswahl wählt `Alt ↓` den ersten, `Alt ↑` den letzten.
5. **Kopfzeile, Zone „Ort“** (Designsystem `components/Kopfzeile`): nach dem Instanznamen `# name` in `lc-title`, danach die Beschreibung in `lc-head-desc` einzeilig mit Ellipse, isoliert (`<bdi>`, ADR-0022 Regel 5), vollständig als `title`; unter 720 px entfällt sie. `document.title` = „# name — localchatA“ (Tab-Leiste, Screenreader). Das ist der „Kopf des Channels“ aus ADR-0024 Regel 8 — es gibt keinen zweiten Kopf über dem Verlauf. Solange F0026 nicht gebaut ist, steht nur `# name`.
6. **Leerer Channel**: die Mitte zeigt „Noch keine Nachrichten in # name.“; der Hinweis, wie Agenten hier schreiben (Designsystem `Hauptansicht`), kommt mit der Senderoute (I0009) bzw. der Anleitung (I0019).
7. **Tab-Reihenfolge** bleibt Kopfzeile → Seitenleiste → Verlauf (fokussierbar) → Teilnehmerliste (ADR-0024 Regel 1).
8. **Nicht hier**: Zähler für Ungelesen und Erwähnt (spätere Interactions), Ausblenden von Channels (Designsystem-Frage 10: alle sehen alle; ein Ausblenden ist nicht geplant), Permalink auf eine Nachricht (wäre `#/c/<name>/<zeit>` mit ADR-0025 `nach`/`vor` — eine Erweiterung, kein Umbau).

## Konsequenzen

**Positiv:**
- Ein Agent kann einen Menschen mit `http://<instanz>/#/c/security-entwicklung` in den richtigen Channel schicken.
- Zwei Tabs eines Browsers zeigen zwei Channels, ohne sich zu stören; Neuladen behält den Channel des Tabs.
- Die Auswahl ist eine reine Funktion — ohne Browser testbar.

**Negativ / Inkauf zu nehmen:**
- Ein frischer Tab ohne Adresse zeigt keinen Verlauf, bis jemand wählt.
- Jeder Wechsel per Link füllt den Browser-Verlauf.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein Link auf einen Channel, der gerade angelegt wird, „gibt es nicht“ zeigt und so stehen bleibt. Früh-Indikator: E2E „Link vor dem Anlegen geöffnet“. Gegenmaßnahme: Regel 1 (Ableitung läuft bei jedem Channel-Ereignis neu).
2. … `Alt ↓` in einem Textfeld eine gewohnte Bewegung des Betriebssystems stört. Früh-Indikator: Meldung aus der Abnahme. Gegenmaßnahme: Kürzel als Operation getestet, abschaltbar durch Weglassen der Regel 4 ohne Folgen für den Rest.
3. … ein Wechsel per Zurück-Taste während eines laufenden Abrufs den falschen Verlauf zeichnet. Früh-Indikator: E2E mit verzögerter Route. Gegenmaßnahme: ADR-0026 Regel 8.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Node-Test (rein): `auswahl('#/c/allgemein', [allgemein, test])` → bekannt; `'#/c/neu'` → unbekannt `neu`; `'#/c/Neu'`, `'#/x'`, `''` → keine; Start ohne Hash mit Merker `test` → `#/c/test` per `replaceState`; `Alt ↓` von `allgemein` → `test`, vom letzten → nichts.
- [ ] Erster E2E-Test: Klick auf `# test` → Adresse `#/c/test`, `aria-current` am Eintrag, Kopfzeile `# test`, `document.title`; Neuladen → derselbe Channel; Zurück → voriger; zweiter Tab mit `#/c/allgemein` → beide Tabs zeigen je ihren Channel; Adresse eines noch nicht vorhandenen Channels, dann Anlegen durch einen anderen Kontext → öffnet sich ohne Neuladen.

## Quellen / Bezuege

- [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md)
- ADR-0015, ADR-0020, ADR-0022 (Regel 5), ADR-0023, ADR-0024 (Regeln 1, 2, 8), ADR-0026
- Designsystem `components/Seitenleiste/README.md`, `components/Kopfzeile/README.md` (und `preview.html`, `lc-head-desc`), `components/Hauptansicht/README.md`, `1-tastatur.md`, `3-offene-fragen.md` (Frage 10)
- kanbanD `web/app.js` (Hash-Adressen, `sessionStorage`), Quelle Q04, gelesen 2026-10-28

## Ergänzungen

- **2026-10-28 (Karte #8 I0009):** Regel 7 wird um die Eingabe ergänzt: Kopfzeile → Seitenleiste → Verlauf → **Eingabe** → Teilnehmerliste ([ADR-0038](0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md) Regel 8, Designsystem `Hauptansicht`). Regel 4 (Fokus bleibt beim Wechsel) gilt auch im Feld: es zeigt dann den Entwurf des neuen Channels.
