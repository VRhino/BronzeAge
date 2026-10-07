// Pestaña «Bots» del cliente admin: administra el servicio de bots por su canal de control (`clienteControl.ts`). Solo red,
// como pide el README del cliente: no toca el motor ni el backend del juego.
//
// El esqueleto se pinta una vez (para no perder el foco de los campos ni los filtros al llegar un estado nuevo) y cada sección
// se repinta por separado, con un pequeño retardo si lo que llega es un flujo (la actividad en vivo).
import type { AccionDeBot, BotInfo, PizarraInfo, ConfigBots, EstadoServicio, FaseBot, FaseServicio, ModoBot, PerfilBot, TipoPerfil } from '@motor/bots/control/contrato';
import type { Sigilo } from '@motor/contratos/v1/dto';
import { ClienteControl, type EstadoConexion } from './clienteControl';
import { esc } from '../ui/html';
import { htmlSubpestanas } from '../ui/subpestanas';
import { htmlNombreConSigilo } from '../sigilo/sigilo';
import './panelBots.css';

const CLAVE_URL = 'bots.url';
const CLAVE_TOKEN = 'bots.token';
const URL_POR_DEFECTO = import.meta.env.VITE_BOTS_URL || 'ws://localhost:4000/control';
const MAX_ACTIVIDAD = 300;
const MAX_FILAS_ACTIVIDAD = 120;
const MAX_REGISTRO = 100;
const ARMADO_MS = 5000;
const MINUTOS_DIA = 1440;
/** Cuántos llegan juntos en un grupo de amigos. Copia de `AMIGOS_POR_GRUPO` (`src/bots/llegadas.ts`): el panel solo habla por red, así que
 * no importa la lógica del servicio; si cambia allí, cambia aquí. */
const AMIGOS_POR_GRUPO = 5;

const FASE_SERVICIO: Record<FaseServicio, { texto: string; tono: string }> = {
  inactivo: { texto: 'Inactivo', tono: 'apagado' },
  arrancando: { texto: 'Arrancando…', tono: 'aviso' },
  corriendo: { texto: 'Corriendo', tono: 'ok' },
  pausado: { texto: 'Pausado', tono: 'aviso' },
  parando: { texto: 'Parando…', tono: 'aviso' },
  error: { texto: 'Error', tono: 'error' },
};
const CONEXION: Record<EstadoConexion, { texto: string; tono: string }> = {
  desconectado: { texto: 'Sin conexión', tono: 'apagado' },
  conectando: { texto: 'Conectando…', tono: 'aviso' },
  conectado: { texto: 'Conectado', tono: 'ok' },
  reintentando: { texto: 'Reconectando…', tono: 'aviso' },
};
const FASE_BOT: Record<FaseBot, string> = { 'sin-plaza': 'Sin plaza', 'en-casa': 'En casa', 'en-columna': 'En columna', 'sin-datos': '—' };
const PERFIL: Record<TipoPerfil, string> = { amigos: 'Amigos', solitario: 'Solitario', tardio: 'Tardío' };
const MODO: Record<ModoBot, string> = { auto: 'Según su horario', conectado: 'Forzado conectado', desconectado: 'Forzado desconectado', congelado: 'Congelado (sin pensar)' };

type Orden = Parameters<ClienteControl['pedir']>[0];
type FormularioConfig = Omit<ConfigBots, 'codigoRegistroBots'> & { codigoRegistroBots: string };
type ColumnaOrden = 'nombre' | 'fase' | 'pensamientos' | 'errores' | 'ultima';

const guardado = (almacen: Storage, clave: string): string => {
  try {
    return almacen.getItem(clave) ?? '';
  } catch {
    return '';
  }
};
const guardar = (almacen: Storage, clave: string, valor: string): void => {
  try {
    almacen.setItem(clave, valor);
  } catch {
    // Sin almacenamiento (navegación privada): se pierde al recargar y ya.
  }
};

/** «hace 40 s», «hace 3 min», «hace 2 h». */
function hace(ahora: number, en: number | undefined): string {
  if (en === undefined) return '—';
  const s = Math.max(0, Math.round((ahora - en) / 1000));
  return s < 60 ? `hace ${s} s` : s < 3600 ? `hace ${Math.round(s / 60)} min` : `hace ${Math.round(s / 3600)} h`;
}

/** Minutos de mundo como «2 h 10 min». */
function duracion(minutos: number): string {
  const m = Math.max(0, Math.round(minutos));
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h} h ${String(m % 60).padStart(2, '0')} min`;
  return `${Math.floor(h / 24)} d ${h % 24} h`;
}

const hhmm = (minutos: number): string => `${String(Math.floor((minutos % MINUTOS_DIA) / 60)).padStart(2, '0')}:${String(minutos % 60).padStart(2, '0')}`;
const horaLocal = (ts: number): string => new Date(ts).toLocaleTimeString('es-ES', { hour12: false });
const perfilTexto = (p: PerfilBot): string => PERFIL[p.tipo];

function numeroDe(nombre: string): number {
  return Number(/(\d+)$/.exec(nombre)?.[1] ?? 0);
}

/** Lo que el panel sabe del servidor del juego por el cliente admin (que ya lo lee): sus héroes bot, para cruzarlos con el registro. */
export interface OrigenDeHeroes {
  heroesBot(): { gameId: string; heroes: { id: string; nombre: string }[] } | undefined;
  /** Facciones y plazas de la partida, para poner nombre (y sigilo) a los ids de la memoria y las pizarras. */
  nombres(): { facciones: { id: string; nombre: string; sigilo?: Sigilo }[]; plazas: { id: string; nombre: string }[] };
}

type Sub = 'servicio' | 'flota' | 'llegadas' | 'facciones' | 'actividad';
const SUBS: { id: Sub; texto: string }[] = [
  { id: 'servicio', texto: 'Servicio' },
  { id: 'flota', texto: 'Flota' },
  { id: 'llegadas', texto: 'Llegadas' },
  { id: 'facciones', texto: 'Facciones' },
  { id: 'actividad', texto: 'Actividad' },
];
const CLAVE_SUB = 'bots.sub';

export interface PanelBots {
  /** Se llama al mostrar la pestaña: pinta lo que llegó mientras estaba oculta y, la primera vez, conecta si hay datos guardados. */
  alMostrar(): void;
  /**
   * Al borrar una partida: si los bots juegan en ella, paran, y su registro de cuentas se borra. Devuelve qué hizo, en una frase.
   * Sin conexión con el servicio no puede hacer nada y lo dice (el registro queda y se borra luego desde esta pestaña).
   */
  olvidarPartida(gameId: string): Promise<string>;
}

export function montarPanelBots(raiz: HTMLElement, origen: OrigenDeHeroes): PanelBots {
  let estado: EstadoServicio | undefined;
  let conexion: EstadoConexion = 'desconectado';
  let detalleConexion = '';
  let acciones: AccionDeBot[] = [];
  let registros: { nivel: 'info' | 'error'; texto: string; en: number }[] = [];
  let sub: Sub = SUBS.some((x) => x.id === guardado(sessionStorage, CLAVE_SUB)) ? (guardado(sessionStorage, CLAVE_SUB) as Sub) : 'servicio';
  let seleccionado: string | undefined;
  let memoriaVolcada: { heroeId: string; datos: Memoria } | undefined;
  let armado: { clave: string; timer: ReturnType<typeof setTimeout> } | undefined;
  let aviso: { ok: boolean; texto: string; timer: ReturnType<typeof setTimeout> } | undefined;
  let primeraVez = true;
  let sucio = false;
  let firmaConfig = '';
  const filtros = { fase: '', perfil: '', texto: '', problemas: false, orden: 'nombre' as ColumnaOrden, descendente: false };
  const actividad = { soloSeleccion: false, soloRechazos: false, congelada: false };
  let formulario: FormularioConfig = { servidor: '', partida: '', codigoRegistroBots: '', total: 30, diasLlegada: 3, semilla: 1, cadaMs: 15_000, horario: 'por-semilla' };
  let formularioSembrado = false;
  let perfilForzado: TipoPerfil = 'solitario';
  let pendienteDePintar: ReturnType<typeof setTimeout> | undefined;

  const cliente = new ClienteControl({
    conexion: (c, detalle) => {
      conexion = c;
      detalleConexion = detalle ?? '';
      pintar();
    },
    estado: (e) => {
      estado = e;
      if (!formularioSembrado) sembrarFormulario(e);
      pintar();
    },
    accion: (a) => {
      if (actividad.congelada) return;
      acciones.push(a);
      if (acciones.length > MAX_ACTIVIDAD) acciones.shift();
      pintarPronto();
    },
    registro: (r) => {
      registros.push(r);
      if (registros.length > MAX_REGISTRO) registros.shift();
      pintarPronto();
    },
    historial: (a, r) => {
      acciones = a.slice(-MAX_ACTIVIDAD);
      registros = r.slice(-MAX_REGISTRO);
      pintarPronto();
    },
  });

  function sembrarFormulario(e: EstadoServicio): void {
    formularioSembrado = true;
    const d = { ...e.porDefecto, ...e.config };
    formulario = {
      servidor: d.servidor ?? '',
      partida: d.partida ?? '',
      codigoRegistroBots: '',
      total: d.total ?? 30,
      diasLlegada: d.diasLlegada ?? 3,
      semilla: d.semilla ?? 1,
      cadaMs: d.cadaMs ?? 15_000,
      horario: d.horario ?? 'por-semilla',
    };
  }

  // --- Esqueleto ---

  raiz.innerHTML = `
    <div class="bots-root">
      <section class="detail-section bots-card" id="bots-conexion"></section>
      <div class="bots-aviso" id="bots-aviso" hidden></div>
      <div id="bots-subtabs"></div>
      <div class="bots-contenido" id="bots-contenido">
        <div class="bots-subpanel" data-sub="servicio" hidden>
          <div class="bots-kpis" id="bots-kpis"></div>
          <section class="detail-section bots-card" id="bots-config"></section>
        </div>
        <div class="bots-subpanel" data-sub="flota" hidden>
          <section class="detail-section bots-card" id="bots-huerfanos"></section>
          <section class="detail-section bots-card">
            <h3 data-i="🛡️">Flota <span class="badge" id="bots-flota-cuenta"></span></h3>
            <div class="bots-filtros">
              <input type="search" id="bots-f-texto" placeholder="Buscar por nombre o id" aria-label="Buscar bot" />
              <select id="bots-f-fase" aria-label="Filtrar por fase">
                <option value="">Todas las fases</option>
                ${(Object.keys(FASE_BOT) as FaseBot[]).filter((f) => f !== 'sin-datos').map((f) => `<option value="${f}">${FASE_BOT[f]}</option>`).join('')}
              </select>
              <select id="bots-f-perfil" aria-label="Filtrar por perfil">
                <option value="">Todos los perfiles</option>
                ${(Object.keys(PERFIL) as TipoPerfil[]).map((p) => `<option value="${p}">${PERFIL[p]}</option>`).join('')}
              </select>
              <label class="bots-check"><input type="checkbox" id="bots-f-problemas" /> Solo con errores</label>
            </div>
            <div class="table-scroll"><table class="mini-table bots-tabla" id="bots-tabla"></table></div>
          </section>
          <section class="detail-section bots-card" id="bots-detalle"></section>
        </div>
        <div class="bots-subpanel" data-sub="llegadas" hidden>
          <section class="detail-section bots-card" id="bots-llegadas"></section>
        </div>
        <div class="bots-subpanel" data-sub="facciones" hidden>
          <section class="detail-section bots-card" id="bots-pizarras"></section>
        </div>
        <div class="bots-subpanel bots-dos" data-sub="actividad" hidden>
          <section class="detail-section bots-card" id="bots-actividad"></section>
          <section class="detail-section bots-card" id="bots-registro"></section>
        </div>
      </div>
    </div>`;
  const $ = <T extends HTMLElement>(id: string) => raiz.querySelector<T>(`#${id}`)!;
  const elConexion = $('bots-conexion');
  const elAviso = $('bots-aviso');
  const elContenido = $('bots-contenido');

  // --- Pintado ---

  const oculta = () => raiz.closest('[hidden]') !== null;

  function pintar(): void {
    if (pendienteDePintar) clearTimeout(pendienteDePintar);
    pendienteDePintar = undefined;
    if (oculta()) {
      sucio = true;
      return;
    }
    sucio = false;
    pintarConexion();
    pintarSubtabs();
    elContenido.classList.toggle('bots-obsoleto', conexion !== 'conectado');
    pintarSub();
  }

  /** Solo se pinta la subpestaña a la vista: las demás se ponen al día al abrirlas. */
  function pintarSub(): void {
    elContenido.querySelectorAll<HTMLElement>('.bots-subpanel').forEach((p) => (p.hidden = p.dataset.sub !== sub));
    switch (sub) {
      case 'servicio':
        pintarKpis();
        return pintarConfig();
      case 'flota':
        pintarHuerfanos();
        pintarFlota();
        return pintarDetalle();
      case 'llegadas':
        return pintarLlegadas();
      case 'facciones':
        return pintarPizarras();
      case 'actividad':
        pintarActividad();
        return pintarRegistro();
    }
  }

  function pintarSubtabs(): void {
    const e = estado;
    const cuenta: Partial<Record<Sub, string>> = e
      ? { flota: String(e.bots.length), llegadas: `${e.llegadas.hechas}/${e.llegadas.plan.length}`, facciones: String(agruparPizarras().facciones.length) }
      : {};
    $('bots-subtabs').outerHTML = htmlSubpestanas(
      SUBS.map((x) => ({ ...x, insignia: cuenta[x.id] ?? '' })),
      sub,
      'Secciones del panel de bots',
      'bots-subtabs'
    );
  }

  /** Un flujo (la actividad en vivo) no repinta a cada mensaje: se agrupa. */
  function pintarPronto(): void {
    if (pendienteDePintar || oculta()) {
      if (oculta()) sucio = true;
      return;
    }
    pendienteDePintar = setTimeout(() => {
      pendienteDePintar = undefined;
      if (sub !== 'actividad') return;
      pintarActividad();
      pintarRegistro();
    }, 300);
  }

  function pill(texto: string, tono: string): string {
    return `<span class="bots-pill bots-${tono}"><i></i>${esc(texto)}</span>`;
  }

  function pintarConexion(): void {
    const c = CONEXION[conexion];
    const f = estado ? FASE_SERVICIO[estado.fase] : undefined;
    const conectado = conexion === 'conectado';
    const enMarcha = estado && (estado.fase === 'corriendo' || estado.fase === 'pausado');
    elConexion.innerHTML = `
      <h3 data-i="🤖">Servicio de bots</h3>
      <div class="bots-conexion-fila">
        <div class="bots-estado-grande">
          ${f && conectado ? pill(f.texto, f.tono) : pill(c.texto, c.tono)}
          ${enMarcha && estado?.config ? `<span class="bots-dato"><b>${esc(estado.config.partida)}</b> · tick <b>${estado.tick ?? '—'}</b> · <span id="bots-uptime">${esc(uptime())}</span></span>` : ''}
          ${detalleConexion ? `<span class="bots-dato bots-texto-error">${esc(detalleConexion)}</span>` : ''}
          ${estado?.error ? `<span class="bots-dato bots-texto-error">${esc(estado.error)}</span>` : ''}
        </div>
        <form class="bots-conectar" id="bots-form-conexion" autocomplete="off">
          <input type="text" id="bots-url" value="${esc(guardado(localStorage, CLAVE_URL) || URL_POR_DEFECTO)}" placeholder="ws://host:4000/control" aria-label="Dirección del servicio" ${conexion !== 'desconectado' ? 'disabled' : ''} />
          <input type="password" id="bots-token" value="${esc(guardado(sessionStorage, CLAVE_TOKEN))}" placeholder="Token de control" aria-label="Token de control" ${conexion !== 'desconectado' ? 'disabled' : ''} />
          ${conexion === 'desconectado' ? '<button type="submit">Conectar</button>' : '<button type="button" data-accion="desconectar">Desconectar</button>'}
        </form>
      </div>`;
  }

  function uptime(): string {
    return estado?.iniciadoEn ? duracion((Date.now() - estado.iniciadoEn) / 60_000) : '—';
  }

  function kpi(etiqueta: string, valor: string, sub = '', tono = ''): string {
    return `<div class="bots-kpi ${tono ? `bots-${tono}` : ''}"><span>${esc(etiqueta)}</span><strong>${valor}</strong><small>${sub}</small></div>`;
  }

  function pintarKpis(): void {
    const e = estado;
    const el = $('bots-kpis');
    if (!e) {
      el.innerHTML = '';
      return;
    }
    const vivos = e.bots.filter((b) => !b.retirado);
    const conectados = vivos.filter((b) => b.conectado).length;
    const s = e.salud;
    const cada = e.config?.cadaMs ?? 0;
    const ratio = s.vuelta && cada ? s.vuelta.ms / cada : 0;
    const tonoVuelta = ratio > 0.8 ? 'error' : ratio > 0.5 ? 'aviso' : '';
    const sinErrores = s.erroresMin === 0;
    el.innerHTML =
      kpi('Bots conectados', `${conectados}<em> / ${vivos.length}</em>`, e.bots.length - vivos.length ? `${e.bots.length - vivos.length} retirados` : 'según su horario') +
      kpi('Llegadas', `${e.llegadas.hechas}<em> / ${e.llegadas.plan.length}</em>`, e.llegadas.extras ? `+${e.llegadas.extras} forzadas` : 'del plan') +
      kpi(
        'Vuelta',
        s.vuelta ? `${s.vuelta.ms}<em> ms</em>` : '—',
        s.vuelta ? `${s.vuelta.pensaron} pensaron · cada ${cada / 1000} s${cada ? ` <i class="bots-barra"><b style="width:${Math.min(100, ratio * 100)}%"></b></i>` : ''}` : 'sin datos',
        tonoVuelta
      ) +
      kpi('Peticiones / min', String(s.peticionesMin), `p50 ${s.latenciaP50} ms · p95 ${s.latenciaP95} ms`) +
      kpi('Errores / min', String(s.erroresMin), sinErrores ? 'sin fallos' : 'de red o del servidor', sinErrores ? 'ok' : 'error') +
      kpi('Sockets', String(s.socketsAbiertos), s.sesionesRenovadas ? `${s.sesionesRenovadas} sesiones renovadas` : 'tiempo real abierto');
  }

  function boton(clave: string, texto: string, textoArmado: string, extra = ''): string {
    const esta = armado?.clave === clave;
    return `<button type="button" data-armable="${clave}" data-texto="${esc(texto)}" data-armado="${esc(textoArmado)}" class="${esta ? 'btn-armado' : ''} ${extra}">${esc(esta ? textoArmado : texto)}</button>`;
  }

  function pintarConfig(): void {
    const el = $('bots-config');
    const fase = estado?.fase ?? 'inactivo';
    const libre = fase === 'inactivo' || fase === 'error';
    const enMarcha = fase === 'corriendo' || fase === 'pausado';
    const conectado = conexion === 'conectado';
    // Los campos solo se rehacen si cambia lo que los condiciona: repintar a cada estado les quitaría el foco.
    const firma = `${fase}|${conectado}|${estado?.codigoEnEntorno}|${estado?.config?.cadaMs}|${armado?.clave ?? ''}`;
    if (firma === firmaConfig && el.innerHTML) return;
    firmaConfig = firma;
    const campo = (nombre: keyof FormularioConfig, etiqueta: string, tipo = 'number', extra = '') =>
      `<label>${etiqueta}<input type="${tipo}" data-campo="${nombre}" value="${esc(formulario[nombre])}" ${libre && conectado ? '' : 'disabled'} ${extra} /></label>`;
    el.innerHTML = `
      <h3 data-i="⚙️">Configuración</h3>
      <div class="bots-form">
        ${campo('servidor', 'Servidor del juego', 'text', 'placeholder="https://…"')}
        ${campo('partida', 'Partida (gameId)', 'text')}
        ${estado && !estado.codigoEnEntorno ? campo('codigoRegistroBots', 'Código de registro de bots', 'password') : ''}
        ${campo('total', 'Bots que llegan', 'number', 'min="1"')}
        ${campo('diasLlegada', 'Días de llegada', 'number', 'min="0.01" step="any"')}
        ${campo('semilla', 'Semilla', 'number', 'min="1"')}
        <label>Horario
          <select data-campo="horario" ${libre && conectado ? '' : 'disabled'}>
            <option value="por-semilla" ${formulario.horario === 'por-semilla' ? 'selected' : ''}>Por semilla (2–6 h al día)</option>
            <option value="siempre" ${formulario.horario === 'siempre' ? 'selected' : ''}>Siempre conectados (pruebas)</option>
          </select>
        </label>
        <label>Cada (ms)
          <input type="number" data-campo="cadaMs" min="1000" step="1000" value="${esc(formulario.cadaMs)}" ${conectado && (libre || enMarcha) ? '' : 'disabled'} />
        </label>
      </div>
      <div class="bots-botones">
        ${libre ? `<button type="button" data-accion="iniciar" ${conectado ? '' : 'disabled'}>▶ Iniciar</button>` : ''}
        ${fase === 'corriendo' ? '<button type="button" data-accion="pausar">⏸ Pausar</button>' : ''}
        ${fase === 'pausado' ? '<button type="button" data-accion="reanudar">▶ Reanudar</button>' : ''}
        ${enMarcha ? '<button type="button" data-accion="ajustar">Aplicar cadencia</button>' : ''}
        ${enMarcha ? boton('parar', '⏹ Parar', '⚠ Confirmar: desconecta a todos') : ''}
        ${libre && estado?.config ? boton('reiniciarRegistro', 'Borrar registro', '⚠ Confirmar: los héroes se quedan sin manejar') : ''}
      </div>
      <p class="legend-note">${
        enMarcha
          ? 'Pausar deja a los bots conectados pero sin pensar. La semilla y los días de llegada no se cambian en marcha: alterarían el plan de llegadas.'
          : 'Iniciar crea las cuentas de los bots en el servidor y los va dando de alta a lo largo de los días de llegada. El registro de cuentas permite retomarlos tras un reinicio.'
      }</p>`;
  }

  function pintarLlegadas(): void {
    const el = $('bots-llegadas');
    const l = estado?.llegadas;
    if (!estado || !l) {
      el.innerHTML = '<h3 data-i="🧭">Llegadas</h3><p class="legend-note">Sin datos todavía.</p>';
      return;
    }
    const activo = estado.fase === 'corriendo' || estado.fase === 'pausado';
    const span = Math.max(1, ...l.plan.map((g) => g.tick));
    const ahoraTick = estado.tick !== undefined && l.inicioTick !== undefined ? estado.tick - l.inicioTick : undefined;
    const siguiente = l.plan.find((g) => !g.hecha);
    const marcas = l.plan
      .map((g) => `<i class="bots-marca ${g.hecha ? 'hecha' : ''} bots-perfil-${g.perfil}" style="left:${(g.tick / span) * 100}%" title="${PERFIL[g.perfil]} ×${g.cuantos} · ${duracion(g.tick)}"></i>`)
      .join('');
    const cursor = ahoraTick !== undefined ? `<i class="bots-cursor" style="left:${Math.min(100, Math.max(0, (ahoraTick / span) * 100))}%" title="Ahora"></i>` : '';
    const faltan = siguiente && ahoraTick !== undefined ? `próxima en ${duracion(siguiente.tick - ahoraTick)}` : 'plan completo';
    el.innerHTML = `
      <h3 data-i="🧭">Llegadas <span class="badge">${l.hechas}/${l.plan.length}</span></h3>
      <div class="bots-pista">${marcas}${cursor}</div>
      <div class="bots-pista-leyenda"><span>0</span><span>${faltan}</span><span>${duracion(span)}</span></div>
      <div class="bots-forzar">
        <select id="bots-perfil-forzado" aria-label="Perfil de la llegada">
          ${(Object.keys(PERFIL) as TipoPerfil[]).map((p) => `<option value="${p}" ${p === perfilForzado ? 'selected' : ''}>${PERFIL[p]}${p === 'amigos' ? ` (×${AMIGOS_POR_GRUPO})` : ''}</option>`).join('')}
        </select>
        <button type="button" data-accion="forzarLlegada" ${activo && conexion === 'conectado' ? '' : 'disabled'}>Forzar llegada ahora</button>
      </div>
      <div class="table-scroll bots-lista-llegadas">
        <table class="mini-table">
          <tr><th>#</th><th>Perfil</th><th>Bots</th><th>A los</th><th></th></tr>
          ${l.plan.map((g) => `<tr class="${g.hecha ? '' : 'bots-pendiente'}"><td>${g.grupo + 1}</td><td>${PERFIL[g.perfil]}</td><td>${g.cuantos}</td><td>${duracion(g.tick)}</td><td>${g.hecha ? '✓' : ''}</td></tr>`).join('')}
        </table>
      </div>`;
  }

  // --- Flota ---

  function problema(b: BotInfo, ahora: number): boolean {
    return !!b.ultimoError && ahora - b.ultimoError.en < 10 * 60_000;
  }

  function botsVisibles(): BotInfo[] {
    const ahora = estado?.ahora ?? Date.now();
    const texto = filtros.texto.trim().toLowerCase();
    const lista = (estado?.bots ?? []).filter(
      (b) =>
        (!filtros.fase || b.fase === filtros.fase) &&
        (!filtros.perfil || b.perfil.tipo === filtros.perfil) &&
        (!filtros.problemas || problema(b, ahora)) &&
        (!texto || b.nombre.toLowerCase().includes(texto) || b.heroeId.toLowerCase().includes(texto))
    );
    const clave: Record<ColumnaOrden, (b: BotInfo) => number | string> = {
      nombre: (b) => numeroDe(b.nombre),
      fase: (b) => b.fase,
      pensamientos: (b) => b.pensamientos,
      errores: (b) => b.errores,
      ultima: (b) => b.ultimaAccion?.en ?? 0,
    };
    const f = clave[filtros.orden];
    const signo = filtros.descendente ? -1 : 1;
    return lista.sort((a, b) => (f(a) < f(b) ? -signo : f(a) > f(b) ? signo : 0));
  }

  function pintarFlota(): void {
    const tabla = $('bots-tabla');
    const todos = estado?.bots ?? [];
    const lista = botsVisibles();
    $('bots-flota-cuenta').textContent = lista.length === todos.length ? String(todos.length) : `${lista.length} de ${todos.length}`;
    const ahora = estado?.ahora ?? Date.now();
    const th = (col: ColumnaOrden, texto: string) =>
      `<th data-orden="${col}" class="bots-ordenable ${filtros.orden === col ? 'activo' : ''}">${texto}${filtros.orden === col ? (filtros.descendente ? ' ▾' : ' ▴') : ''}</th>`;
    const filas = lista
      .map((b) => {
        const a = b.ultimaAccion;
        const dot = b.retirado ? 'retirado' : b.conectado ? 'on' : 'off';
        const modo = b.modo !== 'auto' ? `<span class="badge badge-modified" title="${MODO[b.modo]}">${b.modo}</span>` : '';
        return `<tr class="bots-fila ${b.heroeId === seleccionado ? 'seleccionada' : ''} ${b.retirado ? 'bots-retirado' : ''} ${problema(b, ahora) ? 'fila-deficit' : ''}" data-bot="${esc(b.heroeId)}" tabindex="0">
          <td><span class="bots-dot bots-dot-${dot}" title="${b.retirado ? 'Retirado' : b.conectado ? 'Conectado' : 'Desconectado'}"></span><b>${esc(b.nombre)}</b> ${modo}</td>
          <td>${perfilTexto(b.perfil)}</td>
          <td><span class="badge">${FASE_BOT[b.fase]}</span></td>
          <td class="bots-plan">${esc(b.plan ?? '—')}</td>
          <td class="bots-num">${b.pensamientos}</td>
          <td>${a ? `<span class="${a.ok ? 'bots-ok-txt' : 'bots-rechazo-txt'}">${a.ok ? '✓' : '✗'}</span> ${esc(a.tipo)}${a.motivo ? ` <small>${esc(a.motivo)}</small>` : ''} <small class="bots-hace">${hace(ahora, a.en)}</small>` : '—'}</td>
          <td class="bots-num ${b.errores ? 'valor-negativo' : ''}">${b.errores || ''}</td>
        </tr>`;
      })
      .join('');
    tabla.innerHTML = `<thead><tr>${th('nombre', 'Bot')}<th>Perfil</th>${th('fase', 'Fase')}<th>Plan</th>${th('pensamientos', 'Pensó')}${th('ultima', 'Última acción')}${th('errores', 'Err.')}</tr></thead><tbody>${
      filas || `<tr><td colspan="7" class="bots-vacio">${todos.length ? 'Ningún bot coincide con el filtro.' : 'Todavía no hay bots: llegan según el plan de llegadas.'}</td></tr>`
    }</tbody>`;
  }

  /**
   * Los héroes bot que el servidor tiene y que nadie maneja: el registro de cuentas del servicio y el mundo son dos listas
   * distintas, y si el registro se pierde o se borra, sus héroes siguen en el mundo sin nadie detrás. Se cruzan con los héroes
   * que el cliente admin ya lee del servidor (`controlador: 'bot'`).
   */
  function pintarHuerfanos(): void {
    const el = $('bots-huerfanos');
    const cab = '<h3 data-i="🔗">Registro frente al servidor</h3>';
    const partida = estado?.config?.partida;
    const lado = origen.heroesBot();
    if (!estado || !partida) {
      el.innerHTML = `${cab}<p class="legend-note">Configura e inicia el servicio para cruzar sus bots con los héroes del servidor.</p>`;
      return;
    }
    if (!lado || lado.gameId !== partida) {
      el.innerHTML = `${cab}<p class="legend-note">Este cliente admin está en la partida «${esc(lado?.gameId ?? '—')}» y los bots juegan en «${esc(partida)}»: para cruzarlos hay que abrir la misma.</p>`;
      return;
    }
    const activos = new Set(estado.bots.filter((b) => !b.retirado).map((b) => b.heroeId));
    const retirados = new Set(estado.bots.filter((b) => b.retirado).map((b) => b.heroeId));
    const enMundo = new Set(lado.heroes.map((h) => h.id));
    const sinManejar = lado.heroes.filter((h) => !activos.has(h.id) && !retirados.has(h.id));
    const retiradosEnMundo = lado.heroes.filter((h) => retirados.has(h.id));
    const sinHeroe = estado.bots.filter((b) => !enMundo.has(b.heroeId));
    const chips = (xs: { id: string; nombre: string }[]) => `<div class="chip-row">${xs.map((h) => `<span class="chip" title="${esc(h.id)}">${esc(h.nombre)} · ${esc(h.id)}</span>`).join('')}</div>`;
    el.innerHTML = `${cab}
      <div class="bots-cruce">
        <div class="bots-cruce-dato ${sinManejar.length ? 'bots-aviso-txt' : 'bots-ok-txt'}"><strong>${sinManejar.length}</strong><span>héroes bot sin manejar</span></div>
        <div class="bots-cruce-dato"><strong>${retiradosEnMundo.length}</strong><span>retirados que siguen en el mundo</span></div>
        <div class="bots-cruce-dato ${sinHeroe.length ? 'bots-aviso-txt' : ''}"><strong>${sinHeroe.length}</strong><span>del registro sin héroe en el servidor</span></div>
        <div class="bots-cruce-dato"><strong>${activos.size}</strong><span>manejados ahora</span></div>
      </div>
      ${sinManejar.length ? `<div class="detail-sub"><span class="bots-sub">Huérfanos: el servidor los tiene como bots y el servicio no los maneja</span>${chips(sinManejar)}</div>` : '<p class="legend-note">Todos los héroes bot del servidor tienen quien los maneje.</p>'}
      ${sinHeroe.length ? `<div class="detail-sub"><span class="bots-sub">Están en el registro pero el servidor no tiene su héroe</span>${chips(sinHeroe.map((b) => ({ id: b.heroeId, nombre: b.nombre })))}</div>` : ''}`;
  }

  // --- Detalle de un bot ---

  function barraDia(b: BotInfo): string {
    const tick = estado?.tick;
    const segmentos = b.sesiones.flatMap(([ini, fin]) => {
      const i = ini % MINUTOS_DIA;
      return fin <= MINUTOS_DIA ? [[i, fin - ini]] : [[i, MINUTOS_DIA - i], [0, fin - MINUTOS_DIA]];
    });
    const partes = segmentos.map(([i, d]) => `<i class="bots-sesion" style="left:${(i! / MINUTOS_DIA) * 100}%;width:${(d! / MINUTOS_DIA) * 100}%"></i>`).join('');
    const ahora = tick !== undefined ? `<i class="bots-cursor" style="left:${((tick % MINUTOS_DIA) / MINUTOS_DIA) * 100}%" title="Ahora (hora de mundo)"></i>` : '';
    return `<div class="bots-pista bots-pista-dia">${partes}${ahora}</div><div class="bots-pista-leyenda"><span>00:00</span><span>12:00</span><span>24:00</span></div>`;
  }

  function pintarDetalle(): void {
    const el = $('bots-detalle');
    const b = estado?.bots.find((x) => x.heroeId === seleccionado);
    if (!b) {
      el.innerHTML = '<h3 data-i="🔎">Detalle del bot</h3><p class="legend-note">Elige un bot de la flota para ver su memoria, su horario y sus últimas acciones, y manejarlo.</p>';
      return;
    }
    const e = estado!;
    const propias = acciones.filter((a) => a.heroeId === b.heroeId).slice(-8).reverse();
    const lider = b.perfil.tipo === 'amigos' ? ` · líder ${esc(b.perfil.lider === b.heroeId ? 'él' : b.perfil.lider)}` : '';
    const esperas = b.esperas.length
      ? b.esperas.map((x) => `<span class="chip" title="no se reintenta hasta entonces">${esc(x.clave)}${b.vistaEn !== undefined ? ` · ${duracion((x.hasta - b.vistaEn) / 60_000)}` : ''}</span>`).join('')
      : '<span class="legend-note">ninguna</span>';
    const activo = e.fase === 'corriendo' || e.fase === 'pausado';
    const manejable = activo && conexion === 'conectado' && !b.retirado;
    el.innerHTML = `
      <h3 data-i="🔎">${esc(b.nombre)} <span class="badge">${perfilTexto(b.perfil)}</span> ${b.retirado ? '<span class="badge badge-modified">retirado</span>' : ''}</h3>
      <div class="kv-grid bots-kv">
        <div class="kv-row"><span>Héroe</span><strong>${esc(b.heroeId)}</strong></div>
        <div class="kv-row"><span>Perfil</span><strong>${perfilTexto(b.perfil)}${lider}</strong></div>
        <div class="kv-row"><span>Fase</span><strong>${FASE_BOT[b.fase]}</strong></div>
        <div class="kv-row"><span>Plan</span><strong>${esc(b.plan ?? '—')}</strong></div>
        <div class="kv-row"><span>Facción</span><strong>${b.faccionId ? mundo().faccion(b.faccionId) : '—'}</strong></div>
        <div class="kv-row"><span>Residencia</span><strong>${b.residenciaId ? esc(mundo().nombre(b.residenciaId)) : '—'}</strong></div>
        <div class="kv-row"><span>Columna</span><strong>${esc(b.columnaId ?? '—')}</strong></div>
        <div class="kv-row"><span>Pensó</span><strong>${b.pensamientos} ${b.pensamientos === 1 ? 'vez' : 'veces'}</strong></div>
      </div>
      <div class="detail-sub"><span class="bots-sub">Horario de juego</span>${barraDia(b)}<small class="legend-note">${b.sesiones.map(([i, f]) => (f - i >= MINUTOS_DIA ? 'todo el día' : `${hhmm(i)}–${hhmm(f)}`)).join(' · ')}</small></div>
      <div class="detail-sub"><span class="bots-sub">Esperas tras un rechazo</span><div class="chip-row">${esperas}</div></div>
      ${b.ultimoError ? `<div class="detail-sub"><span class="bots-sub">Último error</span><span class="bots-texto-error">${esc(b.ultimoError.mensaje)}</span> <small class="bots-hace">${hace(e.ahora, b.ultimoError.en)}</small></div>` : ''}
      <div class="detail-sub"><span class="bots-sub">Últimas acciones</span>${
        propias.length
          ? `<ul class="bots-acciones">${propias.map((a) => `<li><span class="${a.ok ? 'bots-ok-txt' : 'bots-rechazo-txt'}">${a.ok ? '✓' : '✗'}</span> ${esc(a.tipo)}${a.motivo ? ` <small>${esc(a.motivo)}</small>` : ''} <small class="bots-hace">${horaLocal(a.en)}</small></li>`).join('')}</ul>`
          : '<span class="legend-note">ninguna desde que se abrió el panel</span>'
      }</div>
      <div class="bots-botones">
        <label class="bots-modo">Modo
          <select data-modo ${manejable ? '' : 'disabled'}>${(Object.keys(MODO) as ModoBot[]).map((m) => `<option value="${m}" ${m === b.modo ? 'selected' : ''}>${MODO[m]}</option>`).join('')}</select>
        </label>
        <button type="button" data-accion="pensarYa" ${manejable && b.conectado ? '' : 'disabled'}>Pensar ya</button>
        <button type="button" data-accion="volcarMemoria" ${activo && conexion === 'conectado' ? '' : 'disabled'}>Ver memoria</button>
        ${manejable ? boton(`retirar:${b.heroeId}`, 'Retirar', '⚠ Confirmar retirada') : ''}
      </div>
      ${memoriaVolcada?.heroeId === b.heroeId ? `<div class="detail-sub"><span class="bots-sub">Memoria</span>${htmlMemoria(memoriaVolcada.datos)}</div>` : ''}`;
  }

  // --- Nombres: lo que el servicio llama por id, el panel lo dice por nombre ---

  const SIN_FACCION = 'sin-faccion:';

  function mundo() {
    const m = origen.nombres();
    const bots = new Map((estado?.bots ?? []).map((b) => [b.heroeId, b.nombre]));
    const facciones = new Map(m.facciones.map((f) => [f.id, f]));
    const plazas = new Map(m.plazas.map((x) => [x.id, x.nombre]));
    return {
      facciones,
      /** Nombre legible de un id de héroe, Facción o plaza; el resto (columnas, campamentos) se deja como viene. */
      nombre: (id: string): string => bots.get(id) ?? facciones.get(id)?.nombre ?? plazas.get(id) ?? id,
      faccion: (id: string): string => {
        const f = facciones.get(id);
        return f ? htmlNombreConSigilo({ nombre: esc(f.nombre), sigilo: f.sigilo }, 18) : esc(id);
      },
    };
  }

  /** El detalle de una pizarra (bandidos vistos, salidas, anillos explorados, listos y residencias), con nombres y tiempos legibles. */
  function htmlDetallePizarra(p: PizarraInfo, m: ReturnType<typeof mundo>): string {
    const d = p.detalle;
    // Referencia de mundo: lo último que vio cualquier bot.
    const ref = Math.max(0, ...(estado?.bots ?? []).map((b) => b.vistaEn ?? 0));
    const hace = (en: number) => `hace ${duracion(Math.max(0, ref - en) / 60_000)}`;
    const falta = (hasta: number) => (hasta <= ref ? 'vencida' : `faltan ${duracion((hasta - ref) / 60_000)}`);
    const tabla = (titulo: string, cab: string[], filas: string[][]) =>
      filas.length
        ? `<div class="detail-sub"><span class="bots-sub">${titulo} (${filas.length})</span><table class="mini-table"><thead><tr>${cab.map((c) => `<th>${c}</th>`).join('')}</tr></thead><tbody>${filas.map((f) => `<tr>${f.map((c) => `<td>${c}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`
        : '';
    return (
      tabla('Bandidos vistos', ['Campamento', 'Posición', 'Poder', 'Visto'], d.bandidosVistos.map((b) => [esc(b.id), `${Math.round(b.x)}, ${Math.round(b.y)}`, String(b.poder), hace(b.vistoEn)])) +
      tabla('Salidas abiertas', ['Campamento', 'Columna', 'Líder', 'Para', 'Espera'], d.salidasAbiertas.map((x) => [esc(x.campamentoId), esc(x.ejercitoId), esc(m.nombre(x.liderId)), x.para === 'fundar' ? 'fundar' : 'cazar', falta(x.hasta)])) +
      tabla('Listos para salir', ['Bot', 'Campamento', 'Ración', 'Vale'], d.listos.map((x) => [esc(m.nombre(x.heroeId)), esc(x.campamentoId), x.conRacion ? 'llena' : 'no', falta(x.hasta)])) +
      tabla('Anillos explorados', ['Campamento', 'Último explorador'], d.exploradosEn.map((x) => [esc(x.campamentoId), hace(x.en)])) +
      tabla('Residencias', ['Bot', 'Reside en'], d.residencias.map((x) => [esc(m.nombre(x.heroeId)), esc(m.nombre(x.plazaId))]))
    );
  }

  /** Las pizarras de Facciones de verdad, aparte de los bots solitarios sin Facción (una pizarra por bot) y de las vacías. */
  function agruparPizarras() {
    const lista = estado?.pizarras ?? [];
    const facciones = lista.filter((p) => !p.id.startsWith(SIN_FACCION) && p.bots.length > 0);
    const solitarios = lista.filter((p) => p.id.startsWith(SIN_FACCION)).flatMap((p) => p.bots);
    const vacias = lista.filter((p) => !p.id.startsWith(SIN_FACCION) && p.bots.length === 0).length;
    return { facciones, solitarios, vacias };
  }

  function pintarPizarras(): void {
    const el = $('bots-pizarras');
    const { facciones, solitarios, vacias } = agruparPizarras();
    const m = mundo();
    const chips = (ids: string[]) => `<div class="chip-row">${ids.map((id) => `<span class="chip" title="${esc(id)}">${esc(m.nombre(id))}</span>`).join('')}</div>`;
    const tarjeta = (p: (typeof facciones)[number]) => `<div class="bots-pizarra">
        <div class="bots-pizarra-cab"><b>${m.faccion(p.id)}</b> <span class="badge">${p.bots.length} bot${p.bots.length === 1 ? '' : 's'}</span></div>
        ${chips(p.bots)}
        <div class="bots-pizarra-datos"><span>Bandidos vistos <b>${p.bandidos}</b></span><span>Salidas abiertas <b>${p.salidas}</b></span><span>Anillos explorados <b>${p.explorados}</b></span></div>
        ${p.encargos.length ? `<div class="chip-row">${p.encargos.map((x) => `<span class="chip" title="encargo de ${esc(m.nombre(x.heroeId))}">${esc(x.clave)} → ${esc(m.nombre(x.heroeId))}</span>`).join('')}</div>` : ''}
        ${htmlDetallePizarra(p, m)}
      </div>`;
    el.innerHTML = `<h3 data-i="📋">Pizarras de Facción <span class="badge">${facciones.length}</span></h3>${
      facciones.length
        ? facciones.map(tarjeta).join('')
        : '<p class="legend-note">Los bots de una Facción se cuentan aquí lo que van viendo (bandidos, quién va a qué). Aún no hay ninguna Facción con bots.</p>'
    }${
      solitarios.length
        ? `<div class="bots-pizarra"><div class="bots-pizarra-cab"><b>Sin Facción</b> <span class="badge">${solitarios.length} bot${solitarios.length === 1 ? '' : 's'}</span></div>${chips(solitarios)}<p class="legend-note">Bots solitarios que aún no tienen Facción: cada uno lleva su propia pizarra hasta que funda o entra en una.</p></div>`
        : ''
    }${vacias ? `<p class="legend-note">${vacias} pizarra${vacias === 1 ? '' : 's'} sin bots, no se muestra${vacias === 1 ? '' : 'n'}.</p>` : ''}`;
  }

  // --- Memoria de un bot (la respuesta de `volcarMemoria`, pintada) ---

  type Memoria = {
    heroeId: string;
    conectado?: boolean;
    sesiones?: [number, number][];
    diag?: { nombre?: string; fase?: FaseBot; modo?: ModoBot; retirado?: boolean; faccionId?: string; vistaEn?: number; pensamientos?: number };
    memoria?: {
      plan?: Record<string, unknown> & { tipo: string };
      columnaId?: string;
      residenciaId?: string;
      esperas?: [string, number][];
      solicitud?: { faccionId: string; desde: number };
      esperaFundar?: number;
      dentroDe?: string;
      pizarra?: string;
    };
  };

  function textoDePlan(plan: NonNullable<NonNullable<Memoria['memoria']>['plan']>, nombre: (id: string) => string): string {
    const n = (v: unknown) => `<b>${esc(nombre(String(v)))}</b>`;
    switch (plan.tipo) {
      case 'cazar': return `Cazar el campamento de bandidos ${n(plan.campamentoId)}`;
      case 'campana': return `Campaña contra la plaza ${n(plan.plazaId)}`;
      case 'explorar': return `Explorar la plaza ${n(plan.plazaId)}`;
      case 'mudarse': return `Mudarse a ${n(plan.plazaId)}`;
      case 'anillo': return `Recorrer el anillo de bandidos de ${n(plan.campamentoId)}${plan.explorar ? ' (explorando sin tropa)' : ''}${plan.salio ? ' · ya salió de la puerta' : ''}`;
      case 'fundar': {
        const sitio = plan.sitio as { x: number; y: number } | undefined;
        const descartados = (plan.descartados as unknown[] | undefined)?.length ?? 0;
        return `Llevar la caravana ${n(plan.caravanaId)} a fundar${sitio ? ` en (${Math.round(sitio.x)}, ${Math.round(sitio.y)})` : ' (aún sin sitio)'}${descartados ? ` · ${descartados} sitio${descartados === 1 ? '' : 's'} descartado${descartados === 1 ? '' : 's'}` : ''}`;
      }
      case 'unirse': return `Unirse a la columna ${n(plan.ejercitoId)} para ${plan.para === 'fundar' ? 'fundar' : 'cazar'}${plan.salio ? ' · ya salió de la puerta' : ''}`;
      default: return esc(plan.tipo);
    }
  }

  function htmlMemoria(d: Memoria): string {
    const m = mundo();
    const mem = d.memoria ?? {};
    const diag = d.diag ?? {};
    // Referencia de tiempo: el último instante de mundo en que el bot miró (las esperas se cuentan contra él).
    const ref = diag.vistaEn;
    const falta = (hasta: number) => (ref === undefined ? '—' : hasta <= ref ? 'ya vencida' : `faltan ${duracion((hasta - ref) / 60_000)}`);
    const desde = (en: number) => (ref === undefined ? '—' : `hace ${duracion(Math.max(0, ref - en) / 60_000)}`);
    const fila = (k: string, v: string) => `<div class="kv-row"><span>${k}</span><strong>${v}</strong></div>`;
    const faccionId = diag.faccionId ?? (mem.pizarra && !mem.pizarra.startsWith(SIN_FACCION) ? mem.pizarra : undefined);
    const pizarra = (estado?.pizarras ?? []).find((p) => p.id === mem.pizarra);
    const esperas = [...(mem.esperas ?? [])].sort((a, b) => a[1] - b[1]);

    const pizarraHtml = pizarra
      ? `<div class="bots-pizarra-datos"><span>Compañeros <b>${pizarra.bots.length}</b></span><span>Bandidos vistos <b>${pizarra.bandidos}</b></span><span>Salidas abiertas <b>${pizarra.salidas}</b></span><span>Anillos explorados <b>${pizarra.explorados}</b></span></div>${
          pizarra.encargos.length
            ? `<div class="chip-row">${pizarra.encargos.map((x) => `<span class="chip${x.heroeId === d.heroeId ? ' bots-mio' : ''}" title="encargo de ${esc(m.nombre(x.heroeId))}">${esc(x.clave)} → ${x.heroeId === d.heroeId ? 'él' : esc(m.nombre(x.heroeId))}</span>`).join('')}</div>`
            : '<span class="legend-note">sin encargos repartidos</span>'
        }`
      : '<span class="legend-note">sin pizarra</span>';

    return `<div class="bots-memoria-panel">
      <div class="kv-grid bots-kv">
        ${fila('Plan', mem.plan ? textoDePlan(mem.plan, m.nombre) : '— (nada en curso)')}
        ${fila('Facción', faccionId ? m.faccion(faccionId) : 'sin Facción')}
        ${fila('Residencia', mem.residenciaId ? esc(m.nombre(mem.residenciaId)) : 'sin plaza')}
        ${fila('Columna', mem.columnaId ? esc(mem.columnaId) : '—')}
        ${fila('Dentro de', mem.dentroDe ? esc(m.nombre(mem.dentroDe)) : '—')}
        ${mem.solicitud ? fila('Solicitud pendiente', `entrar en ${m.faccion(mem.solicitud.faccionId)} · ${desde(mem.solicitud.desde)}`) : ''}
        ${mem.esperaFundar !== undefined ? fila('Espera a los compañeros para fundar', desde(mem.esperaFundar)) : ''}
      </div>
      <div class="detail-sub"><span class="bots-sub">Esperas tras un rechazo (${esperas.length})</span>${
        esperas.length
          ? `<table class="mini-table"><thead><tr><th>Acción</th><th>Hasta</th></tr></thead><tbody>${esperas.map(([clave, hasta]) => `<tr><td>${esc(clave)}</td><td>${falta(hasta)}</td></tr>`).join('')}</tbody></table>`
          : '<span class="legend-note">ninguna: puede intentarlo todo</span>'
      }</div>
      <div class="detail-sub"><span class="bots-sub">Pizarra de la Facción</span>${pizarraHtml}${pizarra ? `${htmlDetallePizarra(pizarra, m)}` : ''}</div>
    </div>`;
  }

  // --- Actividad y registro ---

  function pintarActividad(): void {
    const el = $('bots-actividad');
    const nombres = new Map((estado?.bots ?? []).map((b) => [b.heroeId, b.nombre]));
    const lista = acciones.filter((a) => (!actividad.soloSeleccion || a.heroeId === seleccionado) && (!actividad.soloRechazos || !a.ok));
    const rechazos = acciones.filter((a) => !a.ok).length;
    const filas = lista
      .slice(-MAX_FILAS_ACTIVIDAD)
      .reverse()
      .map(
        (a) =>
          `<div class="bots-linea ${a.ok ? '' : 'bots-linea-rechazo'}"><span class="bots-hora">${horaLocal(a.en)}</span><b>${esc(nombres.get(a.heroeId) ?? a.heroeId)}</b><span>${esc(a.tipo)}</span><span>${a.ok ? '✓' : `✗ ${esc(a.motivo ?? '')}`}</span><span class="bots-ms">${a.ms} ms</span></div>`
      )
      .join('');
    const lleno = el.querySelector('.bots-flujo');
    const scroll = lleno?.scrollTop ?? 0;
    el.innerHTML = `
      <h3 data-i="⚡">Actividad en vivo <span class="badge">${acciones.length}</span>${rechazos ? ` <span class="badge badge-modified">${rechazos} rechazos</span>` : ''}</h3>
      <div class="bots-filtros">
        <label class="bots-check"><input type="checkbox" data-act="rechazos" ${actividad.soloRechazos ? 'checked' : ''} /> Solo rechazos</label>
        <label class="bots-check"><input type="checkbox" data-act="seleccion" ${actividad.soloSeleccion ? 'checked' : ''} ${seleccionado ? '' : 'disabled'} /> Solo el bot elegido</label>
        <label class="bots-check"><input type="checkbox" data-act="congelar" ${actividad.congelada ? 'checked' : ''} /> Congelar la vista</label>
      </div>
      <div class="bots-flujo">${filas || '<p class="legend-note">Cada comando que mande un bot aparece aquí.</p>'}</div>`;
    const nuevo = el.querySelector('.bots-flujo');
    if (nuevo) nuevo.scrollTop = scroll;
  }

  function pintarRegistro(): void {
    const el = $('bots-registro');
    el.innerHTML = `
      <h3 data-i="🗒️">Registro del servicio <span class="badge">${registros.length}</span></h3>
      <div class="bots-flujo">${
        registros.length
          ? [...registros].reverse().map((r) => `<div class="bots-linea ${r.nivel === 'error' ? 'bots-linea-error' : ''}"><span class="bots-hora">${horaLocal(r.en)}</span><span>${esc(r.texto)}</span></div>`).join('')
          : '<p class="legend-note">Arranques, paradas y errores del servicio.</p>'
      }</div>`;
  }

  // --- Órdenes ---

  function avisar(ok: boolean, texto: string): void {
    if (aviso) clearTimeout(aviso.timer);
    aviso = { ok, texto, timer: setTimeout(() => ((aviso = undefined), (elAviso.hidden = true)), ok ? 4000 : 9000) };
    elAviso.hidden = false;
    elAviso.className = `bots-aviso ${ok ? 'bots-aviso-ok' : 'bots-aviso-error'}`;
    elAviso.textContent = `${ok ? '✓' : '✗'} ${texto}`;
  }

  async function ordenar(orden: Orden, exito: string): Promise<unknown> {
    try {
      const datos = await cliente.pedir(orden);
      avisar(true, exito);
      return datos;
    } catch (err) {
      avisar(false, err instanceof Error ? err.message : String(err));
      return undefined;
    }
  }

  function armar(clave: string): boolean {
    if (armado?.clave === clave) {
      clearTimeout(armado.timer);
      armado = undefined;
      firmaConfig = '';
      return true;
    }
    if (armado) clearTimeout(armado.timer);
    armado = {
      clave,
      timer: setTimeout(() => {
        armado = undefined;
        firmaConfig = '';
        pintar();
      }, ARMADO_MS),
    };
    firmaConfig = '';
    pintar();
    return false;
  }

  function configAIniciar(): ConfigBots {
    const f = formulario;
    return { servidor: f.servidor, partida: f.partida, total: f.total, diasLlegada: f.diasLlegada, semilla: f.semilla, cadaMs: f.cadaMs, horario: f.horario, ...(f.codigoRegistroBots ? { codigoRegistroBots: f.codigoRegistroBots } : {}) };
  }

  async function alAccion(accion: string): Promise<void> {
    switch (accion) {
      case 'desconectar':
        return cliente.desconectar();
      case 'iniciar':
        await ordenar({ accion: 'iniciar', config: configAIniciar() }, 'Servicio iniciado');
        formulario.codigoRegistroBots = '';
        return;
      case 'pausar':
        await ordenar({ accion: 'pausar' }, 'Pausado');
        return;
      case 'reanudar':
        await ordenar({ accion: 'reanudar' }, 'Reanudado');
        return;
      case 'ajustar':
        await ordenar({ accion: 'ajustar', cadaMs: formulario.cadaMs }, `Mira el mundo cada ${formulario.cadaMs / 1000} s`);
        return;
      case 'forzarLlegada': {
        const r = (await ordenar({ accion: 'forzarLlegada', perfil: perfilForzado }, 'Llegada dada de alta')) as { creados: number } | undefined;
        if (r) avisar(true, `${r.creados} bot${r.creados === 1 ? '' : 's'} ${r.creados === 1 ? 'dado' : 'dados'} de alta`);
        return;
      }
    }
    if (!seleccionado) return;
    if (accion === 'pensarYa') await ordenar({ accion: 'pensarYa', heroeId: seleccionado }, 'Pensó');
    if (accion === 'volcarMemoria') {
      const datos = await ordenar({ accion: 'volcarMemoria', heroeId: seleccionado }, 'Memoria leída');
      if (datos !== undefined) {
        memoriaVolcada = { heroeId: seleccionado, datos: datos as Memoria };
        pintarDetalle();
      }
    }
  }

  // --- Eventos (delegados: el DOM se repinta) ---

  raiz.addEventListener('submit', (ev) => {
    ev.preventDefault();
    const url = raiz.querySelector<HTMLInputElement>('#bots-url')!.value.trim();
    const token = raiz.querySelector<HTMLInputElement>('#bots-token')!.value.trim();
    if (!url || !token) return avisar(false, 'Falta la dirección o el token del servicio');
    guardar(localStorage, CLAVE_URL, url);
    guardar(sessionStorage, CLAVE_TOKEN, token);
    formularioSembrado = false;
    cliente.conectar(url, token);
  });

  raiz.addEventListener('click', (ev) => {
    const objetivo = ev.target as HTMLElement;
    const subtab = objetivo.closest<HTMLElement>('[data-sub-tab]');
    if (subtab) {
      sub = subtab.dataset.subTab as Sub;
      guardar(sessionStorage, CLAVE_SUB, sub);
      pintarSubtabs();
      return pintarSub();
    }
    const armable = objetivo.closest<HTMLElement>('[data-armable]');
    if (armable) {
      const clave = armable.dataset.armable!;
      if (!armar(clave)) return;
      if (clave === 'parar') void ordenar({ accion: 'parar' }, 'Servicio parado');
      else if (clave === 'reiniciarRegistro') void ordenar({ accion: 'reiniciarRegistro' }, 'Registro borrado');
      else if (clave.startsWith('retirar:')) void ordenar({ accion: 'retirarBot', heroeId: clave.slice(8) }, 'Bot retirado');
      return;
    }
    const botonAccion = objetivo.closest<HTMLElement>('[data-accion]');
    if (botonAccion) return void alAccion(botonAccion.dataset.accion!);
    const orden = objetivo.closest<HTMLElement>('[data-orden]');
    if (orden) {
      const col = orden.dataset.orden as ColumnaOrden;
      filtros.descendente = filtros.orden === col ? !filtros.descendente : false;
      filtros.orden = col;
      return pintarFlota();
    }
    const fila = objetivo.closest<HTMLElement>('[data-bot]');
    if (fila) {
      seleccionado = fila.dataset.bot;
      memoriaVolcada = undefined;
      pintarFlota();
      pintarDetalle();
      pintarActividad();
    }
  });

  raiz.addEventListener('keydown', (ev) => {
    const fila = (ev.target as HTMLElement).closest<HTMLElement>('[data-bot]');
    if (fila && (ev.key === 'Enter' || ev.key === ' ')) {
      ev.preventDefault();
      fila.click();
    }
  });

  raiz.addEventListener('input', (ev) => {
    const el = ev.target as HTMLInputElement;
    if (el.id === 'bots-f-texto') (filtros.texto = el.value), pintarFlota();
    else if (el.dataset.campo) {
      const campo = el.dataset.campo as keyof FormularioConfig;
      (formulario as unknown as Record<string, unknown>)[campo] = el.type === 'number' ? Number(el.value) : el.value;
    }
  });

  raiz.addEventListener('change', (ev) => {
    const el = ev.target as HTMLInputElement | HTMLSelectElement;
    if (el.id === 'bots-f-fase') (filtros.fase = el.value), pintarFlota();
    else if (el.id === 'bots-f-perfil') (filtros.perfil = el.value), pintarFlota();
    else if (el.id === 'bots-f-problemas') (filtros.problemas = (el as HTMLInputElement).checked), pintarFlota();
    else if (el.id === 'bots-perfil-forzado') perfilForzado = el.value as TipoPerfil;
    else if (el.dataset.campo === 'horario') formulario.horario = el.value as ConfigBots['horario'];
    else if ('modo' in el.dataset && seleccionado) void ordenar({ accion: 'modoBot', heroeId: seleccionado, modo: el.value as ModoBot }, `Modo: ${MODO[el.value as ModoBot]}`);
    else if (el.dataset.act) {
      const marcado = (el as HTMLInputElement).checked;
      if (el.dataset.act === 'rechazos') actividad.soloRechazos = marcado;
      if (el.dataset.act === 'seleccion') actividad.soloSeleccion = marcado;
      if (el.dataset.act === 'congelar') actividad.congelada = marcado;
      pintarActividad();
    }
  });

  setInterval(() => {
    const el = raiz.querySelector('#bots-uptime');
    if (el && !oculta()) el.textContent = uptime();
  }, 1000);
  // El cliente admin relee el servidor por su cuenta: el cruce de huérfanos se pone al día con él.
  setInterval(() => {
    if (sub === 'flota' && !oculta()) pintarHuerfanos();
  }, 5000);

  pintar();
  return {
    alMostrar() {
      if (sucio || primeraVez) pintar();
      if (primeraVez) {
        primeraVez = false;
        const url = guardado(localStorage, CLAVE_URL);
        const token = guardado(sessionStorage, CLAVE_TOKEN);
        if (url && token && conexion === 'desconectado') cliente.conectar(url, token);
      }
    },
    async olvidarPartida(gameId) {
      if (conexion !== 'conectado') return `Servicio de bots sin conectar: su registro de «${gameId}», si lo hay, sigue ahí (bórralo desde la pestaña Bots).`;
      const jugando = estado?.config?.partida === gameId && (estado.fase === 'corriendo' || estado.fase === 'pausado');
      try {
        if (jugando) await cliente.pedir({ accion: 'parar' });
        await cliente.pedir({ accion: 'reiniciarRegistro', partida: gameId });
        return jugando ? 'Los bots que jugaban en ella pararon y su registro se borró.' : 'El registro de bots de esa partida se borró.';
      } catch (err) {
        return `Servicio de bots: ${err instanceof Error ? err.message : err}.`;
      }
    },
  };
}
