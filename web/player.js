// Video und Audio im Verlauf (R00030 F0165, B0460): der Player des Browsers, ohne Autoplay und ohne Anfrage vor
// „Abspielen“ (preload none), im Ursprung der Instanz eingebettet (App-CSP `media-src 'self'`). Das Video steht im Rahmen
// 16 : 9 — breit wie der Anhang, höchstens 360 px —, das Audio in einer Zeile fester Höhe; beide behalten ihre Höhe,
// auch wenn der Browser das Medium nicht abspielen kann.

// Der Player eines Anhangs der Form 'video' oder 'audio'; rahmen: { breite, videoHoehe } des vollen Rahmens.
export function playerZeichnen(anhang, form, rahmen) {
  const medium = Object.assign(document.createElement(form), { controls: true, preload: 'none' });
  medium.setAttribute('aria-label', anhang.name);
  if (form === 'video') {
    medium.playsInline = true;
    medium.width = rahmen.breite;
    medium.height = rahmen.videoHoehe;
  }
  const fehler = Object.assign(document.createElement('span'), {
    className: 'lc-player-fehler', textContent: 'Kann im Browser nicht abgespielt werden — „Laden“ nutzen.',
  });
  const player = Object.assign(document.createElement('div'), { className: `lc-player lc-player-${form}` });
  player.dataset.anhang = anhang.id;
  player.append(medium, fehler);
  medium.addEventListener('error', () => {
    player.dataset.zustand = 'fehler';
  });
  medium.src = anhang.url;
  return player;
}

// Spielt ein Video oder Audio unter `wurzel`? (B0733: solange folgt der Verlauf neuen fremden Nachrichten nicht.) Bis
// Pause oder Ende gilt die Wiedergabe — am Ende setzt der Browser `paused` selbst; ein entferntes Element zählt nicht mehr.
export function mediumSpielt(wurzel) {
  return Array.from(wurzel.querySelectorAll('video, audio')).some((medium) => !medium.paused);
}

// Eine Nachricht wird ersetzt (B0734, R00041): jeder Player des alten Knotens, dessen Anhang auch der neue zeigt, zieht
// samt seinem Element in den neuen Knoten um — Wiedergabe, Stelle und Fokus bleiben, nichts lädt neu. Zeigt der neue ihn
// nicht mehr (gelöscht), hält er an und lädt nichts mehr nach. Der Umzug geschieht, bevor `neu` den alten Knoten ersetzt,
// im selben Zug — so hängt das Element nie über eine Aufgabe hinaus außerhalb des Dokuments und läuft weiter.
export function medienUebernehmen(alt, neu) {
  for (const bisher of alt.querySelectorAll('.lc-player')) {
    const platz = neu.querySelector(`.lc-player[data-anhang="${CSS.escape(bisher.dataset.anhang)}"]`);
    if (platz === null) {
      bisher.querySelectorAll('video, audio').forEach(anhalten);
    } else {
      platz.replaceWith(bisher);
    }
  }
}

function anhalten(medium) {
  medium.pause();
  medium.removeAttribute('src');
  medium.load();
}
