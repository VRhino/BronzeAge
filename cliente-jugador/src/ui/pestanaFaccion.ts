import type { ProyeccionJugador } from '../apiCliente';
import type { Faccion } from '../tiposDominio';
import { estadoCliente } from './estadoCliente';

export function faccionDelJugador(proyeccion: ProyeccionJugador): Faccion | undefined {
  return proyeccion.facciones.find((faccion) => faccion.id === proyeccion.faccionId);
}

export function renderPestanaFaccion(
  proyeccion: ProyeccionJugador,
  escaparHtml: (valor: string) => string
): string {
  const faccion = faccionDelJugador(proyeccion);
  const vista = faccion ? 'detalle' : estadoCliente.modoPanelFaccion;

  if (faccion) {
    return `
      <div class="faction-detail-header">
        <span class="faction-kicker">Tu facción</span>
        <h2>${escaparHtml(faccion.nombre)}</h2>
        <span class="faction-id">${escaparHtml(faccion.id)}</span>
      </div>
      <div class="faction-stats">
        <div><span>Nivel</span><strong>${faccion.nivel}</strong></div>
        <div><span>Reputación</span><strong>${faccion.reputacion ?? 0}</strong></div>
        <div><span>Ciudadanos</span><strong>${faccion.ciudadanosIds?.length ?? 0}</strong></div>
      </div>
      <div class="faction-roles">
        <div><span>Rey</span><strong>${escaparHtml(faccion.reyId ?? 'Sin designar')}</strong></div>
        <div><span>Embajador</span><strong>${escaparHtml(faccion.embajadorId ?? 'Sin designar')}</strong></div>
      </div>
    `;
  }

  if (vista === 'inicio') {
    return `
      <div class="faction-empty-state">
        <span class="faction-kicker">Organización política</span>
        <h2>Elige tu facción</h2>
        <p>Construye una nueva identidad o únete a una facción existente.</p>
        <div class="faction-choice-grid">
          <button id="btn-unirse-faccion" class="faction-choice" type="button"><span class="choice-icon">↗</span><strong>Unirse a una facción</strong><span>Explora las facciones del mundo.</span></button>
          <button id="btn-crear-faccion" class="faction-choice" type="button"><span class="choice-icon">✦</span><strong>Crear facción</strong><span>Funda una nueva casa política.</span></button>
        </div>
      </div>
    `;
  }

  if (vista === 'crear') {
    return `
      <div class="faction-form-view">
        <button class="back-button" id="btn-volver-faccion" type="button" aria-label="Volver a elegir facción">←</button>
        <span class="faction-kicker">Nueva identidad</span><h2>Crear facción</h2>
        <p>El nombre será visible para todos los jugadores.</p>
        <form id="form-crear-faccion" class="faction-form">
          <label for="input-nombre-faccion">Nombre de la facción</label>
          <input id="input-nombre-faccion" class="form-input" type="text" maxlength="60" required autocomplete="off" placeholder="Ej. Casa de Micenas" />
          <p id="error-faccion" class="faction-error" role="alert"></p>
          <button id="btn-submit-crear-faccion" class="btn-primary" type="submit">Crear facción</button>
        </form>
      </div>
    `;
  }

  return `
    <div class="faction-form-view faction-join-view">
      <button class="back-button" id="btn-volver-faccion" type="button" aria-label="Volver a elegir facción">←</button>
      <span class="faction-kicker">Facciones del mundo</span><h2>Unirse a una facción</h2>
      <input id="input-buscar-faccion" class="form-input" type="search" placeholder="Buscar por nombre..." autocomplete="off" />
      <div id="lista-facciones" class="faction-list"></div>
    </div>
  `;
}
