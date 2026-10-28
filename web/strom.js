// Ein Strom je Browser (B0094, ADR-0015 Regel 2): Modul-SharedWorker, den alle Tabs teilen; er hält mit dem
// Halter aus live.js höchstens einen Strom und verteilt ihn über den Verteiler an alle Tabs.
import { halten } from './live.js';
import { verteilerAnlegen } from './verteiler.js';

const verteiler = verteilerAnlegen({ halten });

self.onconnect = (ereignis) => verteiler.verbinden(ereignis.ports[0]);
