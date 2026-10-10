// Subpestañas comunes de las pestañas del cliente admin (Facción, Jugadores, Asentamientos, Bots): los botones comparten
// clases y `data-sub-tab`; los paneles (`.subpanel[data-subpanel]`) se alternan sin repintar con `enlazarSubpestanas`.

export interface Subpestana {
  id: string;
  texto: string;
  /** Contador pequeño junto al texto. */
  insignia?: string | number;
}

/** La tira de botones. `etiqueta` es el aria-label de la tira. */
export function htmlSubpestanas(subs: readonly Subpestana[], activa: string, etiqueta = 'Secciones', idTira = ''): string {
  return `<div class="settlement-detail-tabs"${idTira ? ` id="${idTira}"` : ''} role="tablist" aria-label="${etiqueta}">${subs
    .map(
      (s) =>
        `<button type="button" role="tab" class="settlement-detail-tab${s.id === activa ? ' active' : ''}" data-sub-tab="${s.id}" aria-selected="${s.id === activa}">${s.texto}${s.insignia !== undefined && s.insignia !== '' ? ` <span class="badge">${s.insignia}</span>` : ''}</button>`
    )
    .join('')}</div>`;
}

/** Tira + un panel por subpestaña (todos pintados; solo el activo se ve). */
export function htmlSubpaneles(subs: readonly Subpestana[], activa: string, contenidos: Record<string, string>, etiqueta?: string): string {
  return (
    htmlSubpestanas(subs, activa, etiqueta) +
    subs.map((s) => `<div class="subpanel${s.id === activa ? ' active' : ''}" data-subpanel="${s.id}" role="tabpanel">${contenidos[s.id] ?? ''}</div>`).join('')
  );
}

const enlazados = new WeakSet<HTMLElement>();

/** Cablea los botones de `htmlSubpaneles` dentro de `cont`: alterna el panel y avisa para recordar la elección. Delegado: se llama
 * tras cada pintado y solo enlaza la primera vez (el contenedor no cambia, aunque su contenido se repinte). */
export function enlazarSubpestanas(cont: HTMLElement, alCambiar: (id: string) => void): void {
  if (enlazados.has(cont)) return;
  enlazados.add(cont);
  cont.addEventListener('click', (ev) => {
    const btn = (ev.target as Element).closest<HTMLButtonElement>('[data-sub-tab]');
    if (!btn || !cont.contains(btn)) return;
    const id = btn.dataset.subTab!;
    alCambiar(id);
    cont.querySelectorAll<HTMLButtonElement>('[data-sub-tab]').forEach((t) => {
      t.classList.toggle('active', t === btn);
      t.setAttribute('aria-selected', String(t === btn));
    });
    cont.querySelectorAll<HTMLElement>('[data-subpanel]').forEach((p) => p.classList.toggle('active', p.dataset.subpanel === id));
  });
}
