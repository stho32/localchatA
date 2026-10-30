// Menü der Teilnehmerliste (R00050 F0178 B0785, V5, V6; Designsystem components/Teilnehmerliste): die Liste ist ein
// Tab-Stopp mit Roving-Tabindex über die Knöpfe der anderen Teilnehmer (↑/↓, Home/Ende); Klick, Enter oder Leertaste
// öffnen ein `role=menu` mit der Zeile „Direktnachricht“ (die Zeile „Erwähnen“ ergänzt I0013). Im Menü ↑/↓, Home/Ende;
// Escape schließt und gibt den Fokus an den Knopf zurück, Tab schließt und geht weiter, ein Klick daneben schließt.
// Das Menü hängt an der Region, nicht an der Zeile: zeichnet die Liste neu, bleibt es offen, solange die Zeile da ist,
// und schließt sich, wenn sie geht. Eine Kennung frei einzutippen gibt es nicht (P5).
// zeilen: der Behälter der Zeilen (teilnehmerliste.zeichnen) · direktnachricht(kennung): die Auswahl.
export function teilnehmermenueEinrichten({ zeilen, direktnachricht, signal }) {
  const region = zeilen.closest('.lc-members');
  const eintrag = element('button', { type: 'button', className: 'lc-menue-zeile', textContent: 'Direktnachricht', tabIndex: -1 });
  eintrag.setAttribute('role', 'menuitem');
  const menue = element('div', { className: 'lc-menue', id: 'lc-teilnehmermenue' }, [eintrag]);
  menue.setAttribute('role', 'menu');
  let offenFuer = null;
  let aktiveKennung = null;

  function knoepfe() {
    return [...zeilen.querySelectorAll('.lc-person-knopf')];
  }

  function knopfVon(kennung) {
    return knoepfe().find((knopf) => kennungDes(knopf) === kennung);
  }

  function rovingAbgleichen() {
    const alle = knoepfe();
    let aktiv = knopfVon(aktiveKennung);
    if (aktiv === undefined) {
      aktiv = alle[0];
    }
    alle.forEach((knopf) => {
      knopf.tabIndex = -1;
    });
    if (aktiv !== undefined) {
      aktiv.tabIndex = 0;
    }
    let regionImTabFluss = 0;
    if (alle.length > 0) {
      regionImTabFluss = -1;
    }
    region.tabIndex = regionImTabFluss;
  }

  function oeffnen(knopf) {
    offenFuer = kennungDes(knopf);
    knopf.setAttribute('aria-expanded', 'true');
    knopf.setAttribute('aria-controls', menue.id);
    region.append(menue);
    menueAnlegen(knopf);
    eintrag.focus();
  }

  function schliessen() {
    if (offenFuer === null) {
      return;
    }
    knopfVon(offenFuer)?.setAttribute('aria-expanded', 'false');
    offenFuer = null;
    menue.remove();
  }

  // Unter dem Knopf, in Koordinaten der Region (sie scrollt, das Menü scrollt mit).
  function menueAnlegen(knopf) {
    const zeile = knopf.closest('.lc-person');
    menue.style.top = `${zeile.offsetTop + zeile.offsetHeight}px`;
  }

  // Nach jedem Neuzeichnen der Liste: Roving-Tabindex wiederherstellen; ist die Zeile des offenen Menüs noch da, gilt
  // es ihrem neuen Knopf, sonst schließt es — lag der Fokus darin, geht er auf die Region.
  function nachDemZeichnen() {
    rovingAbgleichen();
    if (offenFuer === null) {
      return;
    }
    const knopf = knopfVon(offenFuer);
    if (knopf === undefined) {
      const fokusImMenue = menue.contains(document.activeElement);
      offenFuer = null;
      menue.remove();
      if (fokusImMenue) {
        region.focus();
      }
      return;
    }
    knopf.setAttribute('aria-expanded', 'true');
    knopf.setAttribute('aria-controls', menue.id);
    menueAnlegen(knopf);
  }

  const beobachter = new MutationObserver(nachDemZeichnen);
  beobachter.observe(zeilen, { childList: true });
  signal.addEventListener('abort', () => beobachter.disconnect());

  zeilen.addEventListener('focusin', (ereignis) => {
    const knopf = ereignis.target.closest('.lc-person-knopf');
    if (knopf !== null) {
      aktiveKennung = kennungDes(knopf);
      rovingAbgleichen();
    }
  }, { signal });

  zeilen.addEventListener('click', (ereignis) => {
    const knopf = ereignis.target.closest('.lc-person-knopf');
    if (knopf === null) {
      return;
    }
    const warOffen = offenFuer === kennungDes(knopf);
    schliessen();
    if (!warOffen) {
      oeffnen(knopf);
    }
  }, { signal });

  zeilen.addEventListener('keydown', (ereignis) => {
    const knopf = ereignis.target.closest('.lc-person-knopf');
    const ziel = zielDerTaste(knoepfe(), knopf, ereignis.key);
    if (knopf === null || ziel === null) {
      return;
    }
    ereignis.preventDefault();
    schliessen();
    ziel.focus();
  }, { signal });

  menue.addEventListener('keydown', (ereignis) => {
    const knopf = knopfVon(offenFuer);
    if (ereignis.key === 'Escape') {
      ereignis.preventDefault();
      ereignis.stopPropagation();
      schliessen();
      knopf?.focus();
      return;
    }
    if (ereignis.key === 'Tab') {
      knopf?.focus();
      schliessen();
      return;
    }
    const ziel = zielDerTaste([...menue.querySelectorAll('[role="menuitem"]')], ereignis.target, ereignis.key);
    if (ziel !== null) {
      ereignis.preventDefault();
      ziel.focus();
    }
  }, { signal });

  eintrag.addEventListener('click', () => {
    const kennung = offenFuer;
    schliessen();
    direktnachricht(kennung);
  }, { signal });

  document.addEventListener('pointerdown', (ereignis) => {
    const daneben = !menue.contains(ereignis.target) && ereignis.target.closest('.lc-person-knopf') === null;
    if (daneben) {
      schliessen();
    }
  }, { signal });
}

// ↑/↓ zum vorigen und nächsten, Home und Ende zum ersten und letzten, am Rand bleibt es stehen. → Element | null
export function zielDerTaste(elemente, aktuell, taste) {
  const stelle = elemente.indexOf(aktuell);
  if (stelle === -1) {
    return null;
  }
  const ziele = {
    ArrowDown: elemente[Math.min(stelle + 1, elemente.length - 1)],
    ArrowUp: elemente[Math.max(stelle - 1, 0)],
    Home: elemente[0],
    End: elemente[elemente.length - 1],
  };
  if (!(taste in ziele)) {
    return null;
  }
  return ziele[taste];
}

function kennungDes(knopf) {
  return knopf.closest('.lc-person').dataset.kennung;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
