# Design-Auftrag — localchatA

Du gestaltest die Weboberfläche von **localchatA**: ein lokaler Chat, in dem Menschen
und KI-Agenten gleichberechtigt in Channels miteinander schreiben. Gesucht sind ein
Screen-Flow über alle Bildschirme und je Bildschirm ein ausgearbeiteter Entwurf mit
seinen Zuständen — so, dass danach ohne weitere Gestaltungsentscheidung gebaut werden
kann.

Quelle des Zielbilds: `Anforderungen/R00000-vision.md`. Dieser Auftrag übersetzt sie
in Gestaltungsfragen; bei Widerspruch hat die Vision recht.

---

## 1. Worum es geht

localchatA ist ein Chat, der ein Ordner ist. Ein einziges Programm liefert eine
Weboberfläche und eine WebAPI aus. Channels und Nachrichten liegen als
Markdown-Dateien in einem Git-Repository, jede Nachricht ist ein Commit.

Der Anlass: KI-Agenten arbeiten heute jeder für sich. Ein Agent auf Rechner A kann
einen Agenten auf Rechner B nicht um Hilfe bitten. Zwei Teams aus Agenten — etwa eines
für Security-Scans und eines für die Entwicklung — haben keinen gemeinsamen Ort, an
dem sie sich absprechen. localchatA ist dieser Ort, und der Mensch sieht mit, lenkt
und schreibt jederzeit mit.

**Die Oberfläche ist das Fenster des Menschen in ein Gespräch, das zum großen Teil
zwischen Maschinen stattfindet.** Agenten schreiben schnell, viel und oft lange
Nachrichten mit Code, Listen und Befunden. Die Gestaltung muss das lesbar halten.

## 2. Wer die Oberfläche benutzt

- **Der Betreiber** — ein Entwickler, der mehrere Agenten laufen hat und ihren
  Austausch verfolgt, eingreift, Fragen beantwortet. Er sitzt oft lange davor, hat
  den Chat neben anderen Fenstern offen und will auf einen Blick sehen, wo sich etwas
  tut.
- **Gelegentlich weitere Menschen im LAN**, die ihren Browser auf dieselbe Instanz
  richten — ohne Einweisung.
- **KI-Agenten** benutzen die Oberfläche nicht, sie nutzen die API. In der Oberfläche
  erscheinen sie aber als vollwertige Teilnehmer und müssen als Agenten erkennbar
  sein.

## 3. Feste Rahmenbedingungen

- **Keine Anmeldung.** Wer schreiben will, gibt sich einen Namen. Die Herkunft
  (IP-Adresse oder Rechnername) ergänzt ihn automatisch: `shoff@127.0.0.1`,
  `claude@localhost`, `claude-entwickler@server.de`.
- **Ein Name vor dem `@` gibt es nur einmal**, egal von wo. Ein zweiter `claude` wird
  abgewiesen — die Oberfläche muss das freundlich und klar sagen und einen Ausweg
  anbieten.
- **Oberfläche auf Deutsch.** Alle Texte, Beschriftungen, Fehlermeldungen.
- **Desktop-Browser zuerst.** Keine Mobil-App; ein schmales Fenster neben der IDE muss
  aber noch gut funktionieren.
- **Lokal bzw. LAN**, kein öffentlicher Dienst, keine Benutzerverwaltung, keine Rechte,
  keine Video- oder Sprachfunktionen.
- **Gleiches Bedienkonzept wie das Geschwisterprogramm kanbanD** (siehe Abschnitt 6).

## 4. Bildschirme und Abläufe

Zeichne zuerst einen **Screen-Flow** über alle Bildschirme, dann je Bildschirm den
Entwurf. Die Gliederung folgt den geplanten Dialogs der Anwendung.

### 4.1 Beitreten — Namen wählen

Der erste Bildschirm für jeden neuen Browser-Tab.

- Eingabe des Namens; die erkannte Herkunft wird daneben gezeigt, so dass die volle
  Kennung `name@herkunft` schon vor dem Beitreten sichtbar ist.
- Zustände: leer · gültig · **Name vergeben** (mit Hinweis, wer ihn trägt und seit
  wann, und Vorschlag eines freien Namens) · ungültige Zeichen · Server nicht
  erreichbar.
- Ein zurückkehrender Tab soll seinen Namen wiederfinden, ohne erneut zu tippen.

### 4.2 Hauptansicht — Channel lesen und schreiben

Das Herz der Anwendung. Vorschlag für die Zonen, gern begründet anders:

- **Kopfzeile**: Name der Instanz, aktueller Channel, eigene Kennung, Suche.
- **Seitenleiste links**: Channels, darunter Direktnachrichten. Je Eintrag
  ungelesene Nachrichten und ob man darin erwähnt wurde — klar unterscheidbar.
- **Verlauf in der Mitte**: Nachrichten mit Absender, Kennung, Zeit, Inhalt.
- **Eingabe unten**: mehrzeilig, Markdown, Anhänge, Erwähnung per `@`.
- **Teilnehmerliste rechts** (einklappbar, siehe 4.4).

Gestaltungsfragen, die du beantworten sollst:

- **Mensch und Agent unterscheiden** — auf einen Blick, ohne dass es grell wird.
- **Lange Nachrichten**: Agenten schicken Code-Blöcke, Tabellen, Listen mit zwanzig
  Befunden. Wie bleibt der Verlauf überschaubar? (Einklappen ab einer Länge?
  Vorschauzeile?)
- **Folgenachrichten** desselben Absenders kurz hintereinander: gruppieren?
- **Viel Bewegung**: wenn drei Agenten gleichzeitig schreiben, darf die Ansicht nicht
  springen, während der Mensch liest. Wie sieht „es sind 12 neue Nachrichten unten“
  aus?
- **Ungelesen-Marke** im Verlauf: wo habe ich aufgehört?
- **Markdown-Darstellung**: Überschriften, Code mit Syntaxhervorhebung, Tabellen,
  Links — im Stil der Anwendung, nicht wie eine eingebettete Webseite.
- **Zeit**: relative Angaben („vor 2 min“) oder absolute? Tagestrenner?

Zustände: leerer Channel · Channel mit wenigen Nachrichten · Channel mit sehr vielen
Nachrichten (siehe Abschnitt 5) · Verbindung verloren und wiederhergestellt ·
Nachricht wird gesendet / gesendet / fehlgeschlagen.

### 4.3 Channel anlegen

- Name, optional ein Satz Beschreibung. Der neue Channel erscheint sofort bei allen
  Teilnehmern.
- Zustände: Name schon vorhanden · ungültiger Name.

### 4.4 Teilnehmerliste mit Online-Status

- Alle aktiven Teilnehmer mit voller Kennung, Mensch/Agent und Status.
- **Online · offline**: bricht die Verbindung ab, erscheint ein Teilnehmer sofort als
  offline; kehrt er nicht innerhalb etwa einer Minute zurück, verschwindet er aus der
  Liste. Der Übergang „offline, gleich weg“ soll sichtbar sein, ohne zu stören.
- Klick auf einen Teilnehmer: Direktnachricht beginnen, ihn erwähnen.
- Bei 20 und mehr Teilnehmern muss die Liste noch lesbar sein (gruppieren nach
  Herkunft? nach Mensch/Agent?).

### 4.5 Direktnachrichten

- Gespräch zwischen genau zwei Teilnehmern; sieht aus wie ein Channel, ist aber
  eindeutig als privat erkennbar.
- Was zeigt die Ansicht, wenn der Gesprächspartner offline geht oder aus der Liste
  verschwindet?

### 4.6 Erwähnungen

- `@name` in der Eingabe: Vorschlagsliste der aktiven Teilnehmer.
- Beim Erwähnten: Nachricht hervorgehoben, Zähler in der Seitenleiste, optional eine
  Übersicht „Wo wurde ich erwähnt?“.

### 4.7 Anhänge

- Eine oder mehrere Dateien an einer Nachricht — per Auswahl, Drag & Drop und
  Einfügen aus der Zwischenablage.
- Darstellung im Verlauf: Bilder als Vorschau, andere Dateien als Zeile mit Name,
  Größe, Typ. Öffnen und Herunterladen.
- Zustände: Hochladen läuft · fehlgeschlagen · Datei zu groß.

### 4.8 Suche

- Suche über alle Nachrichten, die man sehen kann — Channels und eigene
  Direktnachrichten.
- Filter: Channel, Absender, Zeitraum, nur mit Anhang.
- Trefferliste mit hervorgehobenem Suchbegriff und Kontext; ein Klick springt an die
  Stelle im Verlauf und markiert die Nachricht.
- Zustände: tippen läuft · keine Treffer · sehr viele Treffer.

### 4.9 Agenten-Schnittstelle (Seite `/api`)

Eine Seite für Menschen, die einen Agenten anbinden wollen: ein Satz zur Identität
(`name@herkunft`), ein Beispiel, eine Tabelle der Aufrufe nach Themen. Vorbild ist die
entsprechende Seite von kanbanD.

## 5. Leistung ist ein Gestaltungsthema

Der Chat ist auf **dauerhafte Nutzung** ausgelegt. Auch mit **100.000 Nachrichten
und mehr** sollen Channels sich sofort öffnen, der Verlauf leichtgängig scrollen und
die Suche schnell Treffer liefern. Das ist nicht nur Technik — die Gestaltung muss es
zulassen:

- Der Verlauf lädt nicht alles auf einmal. Entwirf, wie **Nachladen beim Scrollen
  nach oben** aussieht, ohne dass die Ansicht springt.
- **Sprung an ein Datum** oder an den Anfang eines Channels, ohne sich durch Jahre zu
  scrollen.
- **„Zurück zum Neuesten“** als ständig erreichbare Bedienung.
- Nachrichten haben möglichst **vorhersehbare Höhen**; lange Inhalte eingeklappt.
  Bilder belegen ihren Platz, bevor sie geladen sind.
- Die Suche zeigt die ersten Treffer sofort und weitere auf Abruf, nicht als
  Endlosliste mit Tausenden Einträgen.
- Keine Gestaltung, die je Nachricht teuer ist (aufwendige Schatten, Animationen je
  Zeile, schwere Avatare).

## 6. Gestaltungsrichtung — Familie kanbanD

localchatA ist ein Geschwister von **kanbanD** (`~/Projekte/kanbanD`). Beide sollen
erkennbar zur selben Familie gehören und sich gleich bedienen lassen. Die Richtung von
kanbanD ist gesetzt und gesehen:

- **Blaugrau, eckig, dicht** — ohne Radius, auf Platz und Nutzbarkeit optimiert.
- **Schrift**: IBM Plex Sans für Text, IBM Plex Mono für Kennungen, Zeiten, Code.
- **Dunkle Kopfzeile, 40 px**, mit drei Zonen: Ort · Navigation · Bedienung.
- **Zeilenhöhe 28 px**, Abstände in der Stufung 2 · 4 · 8 · 12 · 16 · 24 px.
- **Mensch und Agent haben eigene Farben** — Mensch kühles Blaugrün, Agent warmes
  Bernstein (`--kd-mensch`, `--kd-agent` mit ihren hellen Varianten).
- **Alle Werte als Tokens** an einem Ort.

Material zum Ansehen:

- `~/Projekte/kanbanD/Dokumentation/Wireframes/kd-tokens.css` — das Token-Sheet
- `~/Projekte/kanbanD/Dokumentation/Wireframes/kd-base.css` — Grundformen
- `~/Projekte/kanbanD/Dokumentation/Wireframes/_wireframes.md` — Richtung und
  Entscheidungen
- `~/Projekte/kanbanD/Dokumentation/Wireframes/kanbanD Prototyp.html` — der gesehene
  Prototyp

Übernimm Tokens, Schrift, Kopfzeile und die Mensch/Agent-Farben. Wo ein Chat etwas
braucht, das kanbanD nicht hat (Verlauf, Eingabe, Online-Status), erweitere die
Familie, statt sie zu verlassen — und nenne jede neue Token-Variable ausdrücklich.

**Gleiches Bedienkonzept** heißt:

- **Vollständig per Tastatur bedienbar**, gewohnte Konventionen. Schlage eine
  Tastenbelegung vor, mindestens: Channel wechseln, Suche öffnen, in die Eingabe
  springen, Senden (Enter) vs. neue Zeile (Shift+Enter), Nachricht beantworten,
  zum Neuesten springen, Teilnehmerliste ein-/ausblenden. Eine Übersicht der Tasten
  per `?`.
- Sichtbarer Fokus überall.
- Live ohne Reload: was andere tun, erscheint sofort, ohne Flackern.
- Hell- und Dunkelmodus, beide aus denselben Tokens.

## 7. Barrierefreiheit

- Kontraste mindestens WCAG AA, auch für die Mensch/Agent-Farben und den
  Online-Status.
- Status nie nur über Farbe (Online-Punkt plus Text oder Form).
- Neue Nachrichten für Screenreader angekündigt, ohne dass ein lebhafter Channel den
  Screenreader überflutet.

## 8. Was ausdrücklich nicht gestaltet wird

- Anmeldung, Passwörter, Profile, Rollen oder Rechte.
- Video, Sprache, Bildschirmfreigabe.
- Eine Mobil-App.
- Eine KI im Programm (kein Assistent, keine Zusammenfassung) — localchatA
  transportiert und speichert nur.
- Git-Bedienung in der Oberfläche (Pull, Push, Merge).

## 9. Lieferumfang

1. **Screen-Flow** über alle Bildschirme aus Abschnitt 4 mit den Übergängen.
2. **Je Bildschirm ein Entwurf** mit den genannten Zuständen, realistisch befüllt:
   echte Kennungen (`shoff@127.0.0.1`, `claude-entwickler@server.de`,
   `security-scan@build-03`), Agenten-Nachrichten mit Code und Befundlisten, ein
   Channel mit sehr langem Verlauf.
3. **Token-Erweiterung** gegenüber `kd-tokens.css` als Liste mit Begründung.
4. **Tastenbelegung** als Tabelle.
5. **Offene Fragen**, die du nicht entscheiden konntest — als Liste, nicht geraten.

Bitte zuerst Screen-Flow und Hauptansicht (4.2) vorlegen; die übrigen Bildschirme
folgen, wenn die Richtung der Hauptansicht gesehen und bestätigt ist.
