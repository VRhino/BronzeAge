import type { ProyeccionJugador } from '../apiCliente';
import type { Faccion } from '../tiposDominio';
import { CAP_FUNDACION_POR_NIVEL, estadoCliente, TIPS_FUNDACION } from './estadoCliente';

export function resumenRecursosFundacion(escaparHtml: (valor: string) => string): string {
  const posicion = estadoCliente.posicionFundacion;
  const mapa = estadoCliente.mapaCache?.mapa;
  if (!posicion || !mapa) return '';
  const radio = 30;
  const nodos = mapa.nodos.filter((nodo) => {
    const dx = nodo.posicion.x - posicion.x;
    const dy = nodo.posicion.y - posicion.y;
    return dx * dx + dy * dy <= radio * radio;
  });
  const bosques = mapa.bosques.filter((bosque) => {
    const dx = bosque.centro.x - posicion.x;
    const dy = bosque.centro.y - posicion.y;
    const alcance = radio + bosque.radio;
    return dx * dx + dy * dy <= alcance * alcance;
  });
  return `
    <div class="foundation-resources">
      <strong>Recursos en la zona inicial</strong>
      <span class="resource-summary-label">${nodos.length > 0 ? `${nodos.length} nodo${nodos.length === 1 ? '' : 's'} dentro del alcance` : 'Ningún nodo de recurso dentro del alcance'}</span>
      ${nodos.length > 0 ? `<div class="resource-summary-list">${nodos.map((nodo) => `<span><b>${escaparHtml(nodo.tipo)}</b> · ${escaparHtml(nodo.rareza)} · ${nodo.cantidadInicial} uds.</span>`).join('')}</div>` : ''}
      <span class="forest-summary">${bosques.length > 0 ? `Acceso a ${bosques.length} bosque${bosques.length === 1 ? '' : 's'} · madera disponible` : 'Sin acceso a bosque · no habrá madera inicial'}</span>
    </div>
  `;
}

export function actualizarTip(): void {
  const numero = document.querySelector<HTMLElement>('#tip-number');
  const texto = document.querySelector<HTMLElement>('#tip-text');
  const contador = document.querySelector<HTMLElement>('#tip-counter');
  if (numero) numero.textContent = `0${estadoCliente.indiceTip + 1}`;
  if (texto) texto.textContent = TIPS_FUNDACION[estadoCliente.indiceTip]!;
  if (contador) contador.textContent = `${estadoCliente.indiceTip + 1} / ${TIPS_FUNDACION.length}`;
}

export function renderPestanaAsentamientos(
  proyeccion: ProyeccionJugador,
  faccion: Faccion | undefined,
  escaparHtml: (valor: string) => string
): string {
  if (!faccion) return '<div class="interaction-empty">Crea o únete a una facción para gestionar asentamientos.</div>';
  const propios = proyeccion.asentamientos.filter((item) => item.faccionId === faccion.id);
  const indiceCap = Math.min(CAP_FUNDACION_POR_NIVEL.length, Math.max(1, faccion.nivel)) - 1;
  const cap = CAP_FUNDACION_POR_NIVEL[indiceCap] ?? CAP_FUNDACION_POR_NIVEL[CAP_FUNDACION_POR_NIVEL.length - 1]!;
  const puedeFundar = propios.length < cap;
  const asentamiento = propios[0];
  const poblacion = asentamiento?.poblacion;
  const totalPoblacion = poblacion ? poblacion.pesants + poblacion.artesanos + poblacion.nobleza : null;
  const edificios = asentamiento?.edificios ?? [];
  const almacen = asentamiento?.almacen ?? {};
  const contenidoDetalle = asentamiento
    ? estadoCliente.asentamientoDetalleTab === 'general'
      ? `<div class="settlement-detail-grid"><div><span>Nivel</span><strong>${asentamiento.nivel}</strong></div><div><span>Nivel operativo</span><strong>${asentamiento.nivelActual ?? asentamiento.nivel}</strong></div><div><span>Población</span><strong>${totalPoblacion ?? 'No disponible'}</strong></div><div><span>Mantenimiento</span><strong>${asentamiento.mantenimiento ?? 'No disponible'}</strong></div></div>`
      : estadoCliente.asentamientoDetalleTab === 'edificios'
        ? `<div class="settlement-building-list">${edificios.length > 0 ? edificios.map((edificio) => `<div><strong>${escaparHtml(edificio.tipo)}</strong><span>${edificio.estado.replace('_', ' ')}</span></div>`).join('') : '<span>No hay edificios visibles.</span>'}</div>`
        : estadoCliente.asentamientoDetalleTab === 'almacen'
          ? `<div class="storage-resource-list">${Object.entries(almacen).filter(([, recurso]) => recurso.cantidad > 0 || recurso.capacidad > 0).map(([tipo, recurso]) => { const porcentaje = recurso.capacidad > 0 ? Math.min(100, Math.round((recurso.cantidad / recurso.capacidad) * 100)) : 0; return `<div class="storage-resource-card"><div class="storage-resource-heading"><span>${escaparHtml(tipo)}</span><strong>${Math.floor(recurso.cantidad)} <small>/ ${Math.floor(recurso.capacidad)}</small></strong></div><div class="storage-capacity-track"><span style="width:${porcentaje}%"></span></div></div>`; }).join('') || '<span>Almacén vacío.</span>'}</div>`
        : estadoCliente.asentamientoDetalleTab === 'produccion'
          ? `<div class="settlement-detail-copy"><strong>Producción y recursos</strong><p>La producción se calcula a partir de los edificios activos, la población y los recursos disponibles del asentamiento.</p><span>Edificios activos: ${edificios.filter((edificio) => edificio.estado === 'activo').length}</span></div>`
          : `<div class="settlement-detail-copy"><strong>Defensa y tropas</strong><p>La información militar detallada aparecerá aquí cuando la proyección incluya escuadrones y guarniciones del asentamiento.</p></div>`
    : '<div class="interaction-empty">Todavía no tienes un asentamiento fundado.</div>';
  return `
    <div class="settlement-panel-view">
      <span class="faction-kicker">Expansión territorial</span><h2>Asentamientos</h2>
      ${puedeFundar ? `<div class="settlement-action ${estadoCliente.modoFundacionActivo ? 'settlement-action-active' : ''}">
        <div class="settlement-action-heading"><span class="choice-icon">⌂</span><strong>Fundar asentamiento</strong></div>
        <p>Elige una posición libre en el mapa y revisa el alcance inicial de su zona de influencia.</p>
        <div id="foundation-resource-summary">${resumenRecursosFundacion(escaparHtml)}</div>
        <div class="foundation-actions">
          <button id="btn-fundar-asentamiento" class="btn-primary ${estadoCliente.posicionFundacion ? 'is-confirmation' : ''}" type="button">${estadoCliente.posicionFundacion ? 'Fundar' : estadoCliente.modoFundacionActivo ? 'Selecciona punto de fundación' : 'Presiona aquí para elegir dónde quieres fundar'}</button>
          ${estadoCliente.modoFundacionActivo ? '<button id="btn-cancelar-fundacion" class="btn-cancel" type="button">Cancelar</button>' : ''}
        </div>
      </div>
      <div class="tips-box"><span class="faction-kicker">Tips</span>
        <div class="tip-carousel" aria-live="polite"><strong id="tip-number" class="tip-number">0${estadoCliente.indiceTip + 1}</strong><p id="tip-text">${TIPS_FUNDACION[estadoCliente.indiceTip]}</p></div>
        <div class="tip-controls"><button id="btn-tip-anterior" class="tip-control" type="button" aria-label="Tip anterior">←</button><span id="tip-counter">${estadoCliente.indiceTip + 1} / ${TIPS_FUNDACION.length}</span><button id="btn-tip-siguiente" class="tip-control" type="button" aria-label="Tip siguiente">→</button></div>
      </div>
      <div class="settlement-explanation"><strong>Antes de fundar</strong><p>Un asentamiento necesita una posición libre del mapa. Al fundarlo, tu facción obtiene una nueva zona de influencia y una base para crecer.</p></div>` : `<div class="settlement-cap-note"><strong>Capacidad de expansión alcanzada</strong><p>Tu facción controla ${propios.length} de ${cap} asentamiento${cap === 1 ? '' : 's'} permitidos en el nivel ${faccion.nivel}.</p></div>`}
      ${asentamiento ? `<section class="settlement-detail"><div class="settlement-detail-heading"><span class="faction-kicker">Detalle</span><h3>${escaparHtml(asentamiento.nombre ?? asentamiento.id)}</h3></div><div class="settlement-detail-tabs" role="tablist" aria-label="Detalle del asentamiento">${(['general', 'edificios', 'almacen', 'produccion', 'militar'] as const).map((tab) => `<button class="settlement-detail-tab ${estadoCliente.asentamientoDetalleTab === tab ? 'active' : ''}" data-settlement-detail-tab="${tab}" type="button" role="tab" aria-selected="${estadoCliente.asentamientoDetalleTab === tab}">${tab[0]!.toUpperCase()}${tab.slice(1)}</button>`).join('')}</div><div class="settlement-detail-content">${contenidoDetalle}</div></section>` : ''}
      <button id="btn-ir-informacion" class="text-link" type="button">Consulta la información del mundo →</button>
    </div>
  `;
}
