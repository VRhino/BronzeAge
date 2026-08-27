import type { ProyeccionJugador } from '../apiCliente';
import { faccionDelJugador, renderPestanaFaccion } from './pestanaFaccion';
import { renderPestanaAsentamientos } from './pestanaAsentamientos';
import { renderPestanaInformacion } from './pestanaInformacion';
import { estadoCliente } from './estadoCliente';

export function renderPanelInteraccion(
  proyeccion: ProyeccionJugador,
  escaparHtml: (valor: string) => string
): string {
  const faccion = faccionDelJugador(proyeccion);
  const contenido = estadoCliente.pestanaInteraccion === 'asentamientos'
    ? renderPestanaAsentamientos(proyeccion, faccion, escaparHtml)
    : estadoCliente.pestanaInteraccion === 'informacion'
      ? renderPestanaInformacion()
      : renderPestanaFaccion(proyeccion, escaparHtml);

  return `
    <div class="interaction-tabs" role="tablist" aria-label="Opciones de interacción">
      <button class="interaction-tab ${estadoCliente.pestanaInteraccion === 'faccion' ? 'active' : ''}" data-tab="faccion" type="button" role="tab" aria-selected="${estadoCliente.pestanaInteraccion === 'faccion'}">Facción</button>
      <button class="interaction-tab ${estadoCliente.pestanaInteraccion === 'asentamientos' ? 'active' : ''}" data-tab="asentamientos" type="button" role="tab" aria-selected="${estadoCliente.pestanaInteraccion === 'asentamientos'}">Asentamientos</button>
      <button class="interaction-tab ${estadoCliente.pestanaInteraccion === 'informacion' ? 'active' : ''}" data-tab="informacion" type="button" role="tab" aria-selected="${estadoCliente.pestanaInteraccion === 'informacion'}">Información</button>
    </div>
    <div class="faction-panel-view ${estadoCliente.pestanaInteraccion === 'asentamientos' ? 'settlement-panel' : ''} ${estadoCliente.pestanaInteraccion === 'informacion' ? 'information-panel' : ''}">${contenido}</div>
  `;
}
