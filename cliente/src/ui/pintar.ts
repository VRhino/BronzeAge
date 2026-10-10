// Repintado incremental de los paneles. `pintar(el, html)` sustituye a `el.innerHTML = html`: si el HTML es idéntico al último no toca
// el DOM, y si cambió reconcilia el árbol existente con el nuevo (cambia solo texto, atributos y nodos distintos) en vez de tirarlo
// entero. Así se conservan el desplazamiento, el foco, los `<details>` abiertos y lo que se esté escribiendo, y el navegador solo
// recalcula lo que cambió.
//
// Los nodos se reutilizan, así que los eventos NO se enlazan tras pintar (se duplicarían): se delegan UNA vez en el contenedor con
// `delegar`. Comparación posicional, sin claves: vale para paneles de tablas y fichas, que es lo que hay aquí.

const ultimoHtml = new WeakMap<Element, string>();

export function pintar(el: Element, html: string): void {
  if (ultimoHtml.get(el) === html) return;
  ultimoHtml.set(el, html);
  const plantilla = document.createElement('template');
  plantilla.innerHTML = html;
  reconciliar(el, plantilla.content);
}

/** Un solo `addEventListener` en el contenedor para todo lo que case con `selector`, ahora o cuando se pinte. */
export function delegar<K extends keyof HTMLElementEventMap>(
  cont: Element,
  tipo: K,
  selector: string,
  manejador: (elemento: HTMLElement, ev: HTMLElementEventMap[K]) => void,
  captura = false
): void {
  cont.addEventListener(
    tipo,
    (ev) => {
      const el = (ev.target as Element | null)?.closest?.(selector) as HTMLElement | null;
      if (el && cont.contains(el)) manejador(el, ev as HTMLElementEventMap[K]);
    },
    captura
  );
}

function mismoTipo(a: Node, b: Node): boolean {
  return a.nodeType === b.nodeType && (a.nodeType !== Node.ELEMENT_NODE || (a as Element).tagName === (b as Element).tagName);
}

function reconciliar(viejo: Node, nuevo: Node): void {
  const vs = Array.from(viejo.childNodes);
  const ns = Array.from(nuevo.childNodes);
  ns.forEach((n, i) => {
    const v = vs[i];
    if (!v) viejo.appendChild(n);
    else if (!mismoTipo(v, n)) viejo.replaceChild(n, v);
    else if (n.nodeType === Node.ELEMENT_NODE) {
      sincronizarAtributos(v as Element, n as Element);
      reconciliar(v, n);
    } else if (v.nodeValue !== n.nodeValue) v.nodeValue = n.nodeValue;
  });
  for (let i = ns.length; i < vs.length; i++) viejo.removeChild(vs[i]!);
}

function sincronizarAtributos(v: Element, n: Element): void {
  for (const { name } of Array.from(v.attributes)) {
    // `open` de un <details> lo maneja quien lo abre; `value` de un campo, quien lo escribe.
    if (!n.hasAttribute(name) && name !== 'open' && name !== 'value') v.removeAttribute(name);
  }
  for (const { name, value } of Array.from(n.attributes)) {
    if (name === 'open' && v.tagName === 'DETAILS') continue;
    if (v.getAttribute(name) !== value) v.setAttribute(name, value);
  }
}
