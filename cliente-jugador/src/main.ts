import {
  ApiError,
  cargarSesionLocal,
  cerrarSesion,
  consultarProyeccion,
  ejecutarComando,
  guardarSesionLocal,
  loginConUsuario,
  obtenerMapa,
  unirseAPartida,
  type ProyeccionJugador,
} from './apiCliente';
import { pintarAsentamiento, pintarPrevisualizacionFundacion, pintarTerreno } from './render';
import type { MapaGenerado } from './terreno';
import { estadoCliente, TIPS_FUNDACION } from './ui/estadoCliente';
import { actualizarTip, resumenRecursosFundacion } from './ui/pestanaAsentamientos';
import { renderPanelInteraccion } from './ui/panelInteraccion';
import { renderPanelMapa } from './ui/panelMapa';

const app = document.querySelector<HTMLDivElement>('#app')!;

function escaparHtml(valor: string): string {
  return valor.replace(/[&<>'"]/g, (caracter) => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    "'": '&#39;',
    '"': '&quot;',
  })[caracter] ?? caracter);
}

function mensajeError(error: unknown): string {
  return error instanceof ApiError ? error.message : 'No se pudo completar la operación.';
}

function sincronizarEstado(): void {
  Object.assign(estadoCliente, {
    usuarioActivo: estadoCliente.usuarioActivo,
    gameIdActivo: estadoCliente.gameIdActivo,
    mapaCache: estadoCliente.mapaCache,
    modoVista: estadoCliente.modoVista,
    proyeccionUltima: estadoCliente.proyeccionUltima,
    modoPanelFaccion: estadoCliente.modoPanelFaccion,
    pestanaInteraccion: estadoCliente.pestanaInteraccion,
    modoFundacionActivo: estadoCliente.modoFundacionActivo,
    posicionFundacion: estadoCliente.posicionFundacion,
    puntoFundacionFijado: estadoCliente.puntoFundacionFijado,
    indiceTip: estadoCliente.indiceTip,
  });
}

function cambiarPestana(pestana: typeof estadoCliente.pestanaInteraccion): void {
  estadoCliente.pestanaInteraccion = pestana;
  estadoCliente.modoFundacionActivo = false;
  estadoCliente.posicionFundacion = null;
  estadoCliente.puntoFundacionFijado = false;
  sincronizarEstado();
  if (estadoCliente.proyeccionUltima) {
    renderizarPanelInteraccion(estadoCliente.proyeccionUltima);
    actualizarModoFundacion(estadoCliente.proyeccionUltima);
  }
}

function renderizarPanelInteraccion(proyeccion: ProyeccionJugador): void {
  const panel = document.querySelector<HTMLDivElement>('#panel-interaccion');
  if (!panel) return;
  sincronizarEstado();
  panel.innerHTML = renderPanelInteraccion(proyeccion, escaparHtml);
  const animacion = panel.querySelector<HTMLElement>('.faction-panel-view');
  animacion?.classList.add('is-entering');
  requestAnimationFrame(() => animacion?.classList.remove('is-entering'));

  panel.querySelectorAll<HTMLButtonElement>('[data-tab]').forEach((boton) => {
    boton.addEventListener('click', () => cambiarPestana(boton.dataset.tab as typeof estadoCliente.pestanaInteraccion));
  });
  panel.querySelector('#btn-ir-informacion')?.addEventListener('click', () => cambiarPestana('informacion'));
  panel.querySelector('#btn-crear-faccion')?.addEventListener('click', () => {
    estadoCliente.modoPanelFaccion = 'crear';
    renderizarPanelInteraccion(proyeccion);
  });
  panel.querySelector('#btn-unirse-faccion')?.addEventListener('click', () => {
    estadoCliente.modoPanelFaccion = 'unirse';
    renderizarPanelInteraccion(proyeccion);
  });
  panel.querySelector('#btn-volver-faccion')?.addEventListener('click', () => {
    estadoCliente.modoPanelFaccion = 'inicio';
    renderizarPanelInteraccion(proyeccion);
  });
  panel.querySelector('#btn-tip-anterior')?.addEventListener('click', () => {
    estadoCliente.indiceTip = (estadoCliente.indiceTip + TIPS_FUNDACION.length - 1) % TIPS_FUNDACION.length;
    actualizarTip();
  });
  panel.querySelector('#btn-tip-siguiente')?.addEventListener('click', () => {
    estadoCliente.indiceTip = (estadoCliente.indiceTip + 1) % TIPS_FUNDACION.length;
    actualizarTip();
  });
  panel.querySelectorAll<HTMLButtonElement>('[data-settlement-detail-tab]').forEach((boton) => {
    boton.addEventListener('click', () => {
      const tab = boton.dataset.settlementDetailTab;
      if (tab === 'general' || tab === 'edificios' || tab === 'almacen' || tab === 'produccion' || tab === 'militar') {
        estadoCliente.asentamientoDetalleTab = tab;
        if (estadoCliente.proyeccionUltima) renderizarPanelInteraccion(estadoCliente.proyeccionUltima);
      }
    });
  });
  panel.querySelector('#btn-fundar-asentamiento')?.addEventListener('click', () => {
    if (!estadoCliente.modoFundacionActivo) {
      estadoCliente.modoFundacionActivo = true;
      estadoCliente.puntoFundacionFijado = false;
      actualizarModoFundacion(proyeccion);
      renderizarPanelInteraccion(proyeccion);
    } else if (estadoCliente.posicionFundacion) {
      void confirmarFundacion(proyeccion, estadoCliente.posicionFundacion);
    }
  });
  panel.querySelector('#btn-cancelar-fundacion')?.addEventListener('click', () => {
    estadoCliente.modoFundacionActivo = false;
    estadoCliente.posicionFundacion = null;
    estadoCliente.puntoFundacionFijado = false;
    actualizarModoFundacion(proyeccion);
    renderizarPanelInteraccion(proyeccion);
  });
  const form = panel.querySelector<HTMLFormElement>('#form-crear-faccion');
  form?.addEventListener('submit', async (evento) => {
    evento.preventDefault();
    const input = panel.querySelector<HTMLInputElement>('#input-nombre-faccion');
    const boton = panel.querySelector<HTMLButtonElement>('#btn-submit-crear-faccion');
    const error = panel.querySelector<HTMLParagraphElement>('#error-faccion');
    if (!input || !boton) return;
    boton.disabled = true;
    boton.textContent = 'Creando...';
    try {
      const respuesta = await ejecutarComando(estadoCliente.gameIdActivo, 'crearFaccion', { nombre: input.value.trim() });
      if (!respuesta.resultado.ok) throw new ApiError(409, respuesta.resultado.codigoError ?? 'El servidor rechazó la operación.');
      estadoCliente.modoPanelFaccion = 'inicio';
      estadoCliente.pestanaInteraccion = 'asentamientos';
      await refrescarDatosJuego();
    } catch (err) {
      boton.disabled = false;
      boton.textContent = 'Crear facción';
      if (error) error.textContent = mensajeError(err);
    }
  });
  const lista = panel.querySelector<HTMLDivElement>('#lista-facciones');
  const busqueda = panel.querySelector<HTMLInputElement>('#input-buscar-faccion');
  if (lista && busqueda) {
    const pintarLista = (): void => {
      const termino = busqueda.value.trim().toLowerCase();
      const facciones = proyeccion.facciones.filter((faccion) => faccion.nombre.toLowerCase().includes(termino));
      lista.innerHTML = facciones.length > 0 ? facciones.map((faccion) => `<div class="faction-list-item"><div><strong>${escaparHtml(faccion.nombre)}</strong><span>Nivel ${faccion.nivel}</span></div><button class="btn-join-faction" type="button" data-faccion-id="${escaparHtml(faccion.id)}">Unirse</button></div>`).join('') : '<p class="faction-empty-list">No hay facciones que coincidan.</p>';
      lista.querySelectorAll<HTMLButtonElement>('.btn-join-faction').forEach((boton) => boton.addEventListener('click', async () => {
        boton.disabled = true;
        boton.textContent = 'Uniendo...';
        try {
          const respuesta = await ejecutarComando(estadoCliente.gameIdActivo, 'unirseAFaccion', { faccionId: boton.dataset.faccionId });
          if (!respuesta.resultado.ok) throw new ApiError(409, respuesta.resultado.codigoError ?? 'El servidor rechazó la operación.');
          await refrescarDatosJuego();
        } catch (err) {
          boton.disabled = false;
          boton.textContent = 'Unirse';
          const aviso = document.createElement('p');
          aviso.className = 'faction-error';
          aviso.textContent = mensajeError(err);
          boton.parentElement?.appendChild(aviso);
        }
      }));
    };
    busqueda.addEventListener('input', pintarLista);
    pintarLista();
  }
}

function renderVistaLogin(errorMensaje?: string): void {
  app.innerHTML = `<div class="login-container"><div class="login-card"><div class="login-header"><h1 class="login-title">Bronze Age Collapse</h1><p class="login-subtitle">Cliente de Jugador — Inicio de Sesión</p></div>${errorMensaje ? `<div class="error-banner">⚠️ ${escaparHtml(errorMensaje)}</div>` : ''}<form id="form-login"><div class="form-group"><label class="form-label" for="input-usuario">Usuario / Nickname</label><input type="text" id="input-usuario" class="form-input" value="${escaparHtml(estadoCliente.usuarioActivo || 'ana')}" required autocomplete="off" /><div class="user-chips"><button type="button" class="chip-btn" data-user="ana">ana</button><button type="button" class="chip-btn" data-user="bruno">bruno</button><button type="button" class="chip-btn" data-user="carla">carla</button><button type="button" class="chip-btn" data-user="jefa">jefa (admin)</button></div></div><div class="form-group"><label class="form-label" for="input-gameid">ID de Partida</label><input type="text" id="input-gameid" class="form-input" value="${escaparHtml(estadoCliente.gameIdActivo)}" required autocomplete="off" /></div><button type="submit" id="btn-login-submit" class="btn-primary">Entrar a la Partida</button></form></div></div>`;
  document.querySelectorAll<HTMLButtonElement>('.chip-btn').forEach((chip) => chip.addEventListener('click', () => {
    const input = document.querySelector<HTMLInputElement>('#input-usuario');
    if (input) input.value = chip.dataset.user ?? '';
  }));
  document.querySelector<HTMLFormElement>('#form-login')?.addEventListener('submit', async (evento) => {
    evento.preventDefault();
    const usuario = document.querySelector<HTMLInputElement>('#input-usuario')?.value.trim();
    const gameId = document.querySelector<HTMLInputElement>('#input-gameid')?.value.trim() || 'local';
    if (!usuario) return;
    try {
      const login = await loginConUsuario(usuario);
      try { await unirseAPartida(gameId); } catch (err) { if (!(err instanceof ApiError) || err.status !== 409) throw err; }
      guardarSesionLocal(login.sesionId, usuario, gameId);
      estadoCliente.usuarioActivo = usuario;
      estadoCliente.gameIdActivo = gameId;
      renderVistaJuego();
      await refrescarDatosJuego();
    } catch (err) { renderVistaLogin(mensajeError(err)); }
  });
}

function renderVistaJuego(): void {
  app.innerHTML = `<div class="game-container"><header class="top-bar"><div class="brand-section"><span class="brand-title">Bronze Age Collapse</span><span class="brand-badge">Cliente Jugador</span></div><div class="user-section"><div class="user-info"><div class="user-avatar">${escaparHtml(estadoCliente.usuarioActivo.substring(0, 2).toUpperCase())}</div><div class="details-group"><button id="btn-toggle-proyeccion" class="user-name" type="button" aria-expanded="false" aria-controls="panel-proyeccion">${escaparHtml(estadoCliente.usuarioActivo)}</button><span class="game-id">Partida: <strong>${escaparHtml(estadoCliente.gameIdActivo)}</strong></span></div></div><button id="btn-toggle-vista" class="btn-secondary">🏙️ Ver Ciudad</button><button id="btn-refrescar" class="btn-secondary">🔄 Refrescar</button><button id="btn-logout" class="btn-secondary">Cerrar Sesión</button></div></header><div class="main-content"><div id="panel-interaccion" class="card-panel interaction-panel"><div class="interaction-loading">Cargando opciones de interacción...</div></div><div id="panel-proyeccion" class="card-panel projection-panel" hidden><h2 class="panel-title">Proyección del Jugador (JSON / HTTP)</h2><pre id="proyeccion">Cargando proyección...</pre></div>${renderPanelMapa()}</div></div>`;
  document.querySelector('#btn-logout')?.addEventListener('click', () => { cerrarSesion(); estadoCliente.usuarioActivo = ''; renderVistaLogin(); });
  document.querySelector('#btn-toggle-proyeccion')?.addEventListener('click', () => {
    const boton = document.querySelector<HTMLButtonElement>('#btn-toggle-proyeccion');
    const panel = document.querySelector<HTMLDivElement>('#panel-proyeccion');
    if (!boton || !panel) return;
    panel.hidden = !panel.hidden;
    boton.setAttribute('aria-expanded', String(!panel.hidden));
  });
  document.querySelector('#btn-refrescar')?.addEventListener('click', () => void refrescarDatosJuego());
  document.querySelector('#btn-toggle-vista')?.addEventListener('click', () => {
    estadoCliente.modoVista = estadoCliente.modoVista === 'mundo' ? 'asentamiento' : 'mundo';
    const boton = document.querySelector<HTMLButtonElement>('#btn-toggle-vista');
    if (boton) boton.textContent = estadoCliente.modoVista === 'mundo' ? '🏙️ Ver Ciudad' : '🗺️ Ver Mapa Mundo';
    if (estadoCliente.proyeccionUltima) void dibujarPantallaSegunModo(estadoCliente.proyeccionUltima);
  });
}

async function sincronizarMapa(mapaId: string): Promise<MapaGenerado> {
  if (estadoCliente.mapaCache?.id === mapaId) return estadoCliente.mapaCache.mapa;
  const mapa = await obtenerMapa(estadoCliente.gameIdActivo, mapaId);
  estadoCliente.mapaCache = { id: mapaId, mapa };
  return mapa;
}

async function dibujarPantallaSegunModo(proyeccion: ProyeccionJugador): Promise<void> {
  const canvas = document.querySelector<HTMLCanvasElement>('#mapa');
  const titulo = document.querySelector<HTMLHeadingElement>('#titulo-mapa');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  if (estadoCliente.modoVista === 'mundo') {
    if (titulo) titulo.textContent = 'Mapa del Mundo (Evaluación T2a)';
    const mapa = await sincronizarMapa(proyeccion.mapaId);
    pintarTerreno(ctx, mapa, proyeccion, canvas.width / mapa.config.ancho);
  } else {
    if (titulo) titulo.textContent = 'Vista de Asentamiento (Geometría Urbana T2a)';
    const asentamiento = proyeccion.asentamientos[0];
    if (asentamiento) pintarAsentamiento(ctx, asentamiento, proyeccion.trazadoPorAsentamiento?.[asentamiento.id]);
    else { ctx.clearRect(0, 0, canvas.width, canvas.height); ctx.fillText('No perteneces a ningún asentamiento aún.', 20, 40); }
  }
  if (estadoCliente.modoFundacionActivo && estadoCliente.posicionFundacion && estadoCliente.mapaCache?.mapa) {
    pintarPrevisualizacionFundacion(ctx, estadoCliente.mapaCache.mapa, estadoCliente.posicionFundacion, canvas.width / estadoCliente.mapaCache.mapa.config.ancho);
  }
}

function actualizarModoFundacion(proyeccion: ProyeccionJugador): void {
  const canvas = document.querySelector<HTMLCanvasElement>('#mapa');
  if (!canvas) return;
  if (!estadoCliente.modoFundacionActivo) { canvas.classList.remove('mapa-fundacion-activo'); canvas.onpointermove = null; canvas.onpointerleave = null; canvas.onclick = null; return; }
  canvas.classList.add('mapa-fundacion-activo');
  const punto = (evento: MouseEvent) => { const rect = canvas.getBoundingClientRect(); const mapa = estadoCliente.mapaCache?.mapa; return mapa ? { x: ((evento.clientX - rect.left) / rect.width) * mapa.config.ancho, y: ((evento.clientY - rect.top) / rect.height) * mapa.config.alto } : { x: 0, y: 0 }; };
  canvas.onpointermove = (evento) => { if (estadoCliente.puntoFundacionFijado) return; estadoCliente.posicionFundacion = punto(evento); actualizarPanelFundacion(); void dibujarPantallaSegunModo(proyeccion); };
  canvas.onpointerleave = () => { if (estadoCliente.puntoFundacionFijado) return; estadoCliente.posicionFundacion = null; actualizarPanelFundacion(); };
  canvas.onclick = (evento) => { if (estadoCliente.puntoFundacionFijado) return; estadoCliente.posicionFundacion = punto(evento); estadoCliente.puntoFundacionFijado = true; actualizarPanelFundacion(); void dibujarPantallaSegunModo(proyeccion); };
}

function actualizarPanelFundacion(): void {
  const panel = document.querySelector<HTMLDivElement>('#panel-interaccion');
  const boton = panel?.querySelector<HTMLButtonElement>('#btn-fundar-asentamiento');
  const resumen = panel?.querySelector<HTMLDivElement>('#foundation-resource-summary');
  if (!panel || !boton || !resumen) return;
  boton.classList.toggle('is-active', estadoCliente.modoFundacionActivo && !estadoCliente.posicionFundacion);
  boton.classList.toggle('is-confirmation', Boolean(estadoCliente.posicionFundacion));
  boton.textContent = estadoCliente.posicionFundacion
    ? 'Fundar'
    : estadoCliente.modoFundacionActivo
      ? 'Selecciona punto de fundación'
      : 'Presiona aquí para elegir dónde quieres fundar';
  resumen.innerHTML = resumenRecursosFundacion(escaparHtml);
}

async function confirmarFundacion(proyeccion: ProyeccionJugador, posicion: { x: number; y: number }): Promise<void> {
  try {
    const respuesta = await ejecutarComando(estadoCliente.gameIdActivo, 'fundarAsentamiento', { faccionId: proyeccion.faccionId, posicion });
    if (!respuesta.resultado.ok) throw new ApiError(409, respuesta.resultado.codigoError ?? 'La posición no es válida.');
    estadoCliente.modoFundacionActivo = false;
    estadoCliente.posicionFundacion = null;
    estadoCliente.puntoFundacionFijado = false;
    await refrescarDatosJuego();
  } catch (err) {
    const estado = document.querySelector<HTMLParagraphElement>('#estado');
    if (estado) estado.textContent = `Error al fundar: ${mensajeError(err)}`;
  }
}

async function refrescarDatosJuego(): Promise<void> {
  const proyeccion = await consultarProyeccion(estadoCliente.gameIdActivo);
  estadoCliente.proyeccionUltima = proyeccion;
  renderizarPanelInteraccion(proyeccion);
  await dibujarPantallaSegunModo(proyeccion);
  const salida = document.querySelector<HTMLParagraphElement>('#estado');
  const json = document.querySelector<HTMLPreElement>('#proyeccion');
  if (salida) salida.textContent = `Conectado a '${estadoCliente.gameIdActivo}' — Tick: ${proyeccion.tick} | Modo: ${estadoCliente.modoVista} | Facción: ${proyeccion.faccionId ?? '(Ninguna)'}`;
  if (json) json.textContent = JSON.stringify(proyeccion, null, 2);
}

function arrancar(): void {
  const sesion = cargarSesionLocal();
  if (!sesion) { renderVistaLogin(); return; }
  estadoCliente.usuarioActivo = sesion.usuario;
  estadoCliente.gameIdActivo = sesion.gameId;
  renderVistaJuego();
  void refrescarDatosJuego().catch(() => { cerrarSesion(); renderVistaLogin('La sesión previa expiró o el servidor fue reiniciado.'); });
}

arrancar();
