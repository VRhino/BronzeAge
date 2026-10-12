// Informe de una prueba de carga (03 §14): resume las muestras de `telemetria.ts` y la auditoría de comandos con
// percentiles, picos, bytes/min por cliente contra el sondeo, y qué mejoras de 03 §13 pide cada umbral.
//
// Puro (sin E/S) para poder probarlo; `scripts/informe-carga.ts` lo envuelve con lectura de disco o de URL.
//
// **Agregar ventanas.** Cada muestra ya trae p50/p95/máx de SU ventana. Aquí p50 = mediana de los p50, p95 = el
// percentil 95 de los p95 y máx = el máximo: es una cota honesta y barata, no el percentil exacto de todos los
// eventos (que no se conserva). Para decidir una mejora basta; para citar en un informe, dígase «p95 de ventanas».
import type { EntradaAuditoria } from './auditoria';
import type { MuestraDeCanal, MuestraDeCarga } from './telemetria';

const KIB = 1024;
/** Sondeo que sustituyen los canales (03 §12): el cliente admin pedía el estado completo cada 3 s; el jugador su proyección cada 5 s. */
export const SONDEO_ADMIN_S = 3;
export const SONDEO_JUGADOR_S = 5;

/** Umbrales que activan un candidato de 03 §13. Una constante, no configuración: se discuten en el doc y se cambian aquí. */
export const UMBRALES = {
  /** 13.4 dirty tracking: la proyección de una clave tarda más que esto (ms, p95 de ventanas). */
  msProyeccionP95: 5,
  /** 13.7 serialización fuera del hilo: serializar un mensaje tarda más que esto (ms, p95). */
  msSerializacionP95: 5,
  /** 13.2 compresión: una foto pesa más que esto (bytes). */
  bytesFotoMax: 100 * KIB,
  /** 13.5 / 13.2: un cliente admin recibe más que esto por minuto (bytes). */
  bytesMinPorCliente: 1024 * KIB,
  /** 13.3 reanudar con `desde`: fotos por resync/lento/visibilidad en la prueba. */
  fotosDeResync: 5,
  /** 13.6 ritmo adaptativo: p99 del retraso del bucle de eventos (ms) o tick lento (p95, ms). */
  bucleP99Ms: 50,
  tickP95Ms: 200,
  /** 13.8 presupuesto por socket: cualquier socket lento o cierre 4408. */
  socketsLentos: 1,
} as const;

export interface Agregado {
  p50: number;
  p95: number;
  max: number;
}

export interface InformeDeCanal {
  suscriptoresMax: number;
  clavesMax: number;
  flushes: number;
  /** Bytes empujados por minuto y por suscriptor (media de ventanas con suscriptores). */
  bytesMinPorCliente: number;
  bytesFlush: Agregado;
  bytesFotoMax: number;
  msProyeccion: Agregado;
  msDiff: Agregado;
  msSerializacion: Agregado;
  fotos: { alta: number; resync: number; lento: number; visibilidad: number };
  socketsLentos: number;
  cierres4408: number;
}

export interface InformeDeCarga {
  muestras: number;
  desde?: string;
  hasta?: string;
  duracionMin: number;
  playtestIds: string[];
  tickMs: Agregado;
  bucleMs: { p99: number; max: number };
  guardado: { msP95: number; msMax: number; bytesMax: number };
  conexionesMax: number;
  colaPendienteMax: number;
  memoriaMb: { rssInicial: number; rssFinal: number; rssMax: number };
  /** Lo que cuesta medir (del propio muestreo): debe ser << 1 % del tick. */
  costeMuestraMs: Agregado;
  canales: Record<string, InformeDeCanal>;
  /** Empujado contra sondeo, por cliente: bytes/min que enviaban los dos GET que sustituyen vs lo medido. */
  contraSondeo: { admin?: ComparacionSondeo; jugador?: ComparacionSondeo };
  auditoria?: InformeDeAuditoria;
  candidatos: Candidato[];
}

export interface ComparacionSondeo {
  sondeoBytesMin: number;
  empujadoBytesMin: number;
  /** empujado / sondeo (0,1 = el 10 % de los bytes). */
  razon: number;
}

export interface InformeDeAuditoria {
  comandos: number;
  rechazos: Record<string, number>;
  ms: Agregado;
  bytes: Agregado;
  /** Los comandos más caros por `ms` máximo. */
  masCaros: { comando: string; n: number; msP50: number; msMax: number; bytesMax: number }[];
}

export interface Candidato {
  id: string;
  mejora: string;
  motivo: string;
}

const mediana = (xs: number[]): number => percentil(xs, 0.5);
function percentil(xs: readonly number[], p: number): number {
  if (xs.length === 0) return 0;
  const o = [...xs].sort((a, b) => a - b);
  return o[Math.min(o.length - 1, Math.floor((o.length - 1) * p))]!;
}
const maximo = (xs: readonly number[]): number => xs.reduce((m, x) => (x > m ? x : m), 0);
const suma = (xs: readonly number[]): number => xs.reduce((a, b) => a + b, 0);
const r2 = (x: number): number => Math.round(x * 100) / 100;

/** p50 = mediana de p50s, p95 = p95 de p95s, max = máx. Ignora ventanas sin datos (n = 0). */
function agregar(res: readonly { n: number; p50: number; p95: number; max: number }[]): Agregado {
  const v = res.filter((x) => x.n > 0);
  return { p50: r2(mediana(v.map((x) => x.p50))), p95: r2(percentil(v.map((x) => x.p95), 0.95)), max: r2(maximo(v.map((x) => x.max))) };
}

const CANAL_VACIO: MuestraDeCanal = {
  flushes: 0, bytesEnviados: 0, bytesFotoVida: 0,
  bytesFlush: { n: 0, p50: 0, p95: 0, max: 0 }, bytesFoto: { n: 0, p50: 0, p95: 0, max: 0 },
  msProyeccion: { n: 0, p50: 0, p95: 0, max: 0 }, msDiff: { n: 0, p50: 0, p95: 0, max: 0 }, msSerializacion: { n: 0, p50: 0, p95: 0, max: 0 },
  fotos: { alta: 0, resync: 0, lento: 0, visibilidad: 0 }, cambios: { upsert: 0, patch: 0, remove: 0, set: 0 },
  socketsLentos: 0, cierres4408: 0, suscriptores: 0, claves: 0,
};

function informeDeCanal(muestras: MuestraDeCarga[], tipo: string): InformeDeCanal {
  const cs = muestras.map((m) => m.canales[tipo] ?? CANAL_VACIO);
  const porCliente: number[] = [];
  muestras.forEach((m, i) => {
    const c = cs[i]!;
    if (m.ventanaMs > 0 && c.suscriptores > 0) porCliente.push(c.bytesEnviados / (m.ventanaMs / 60_000) / c.suscriptores);
  });
  const fotos = { alta: 0, resync: 0, lento: 0, visibilidad: 0 };
  for (const c of cs) for (const k of Object.keys(fotos) as (keyof typeof fotos)[]) fotos[k] += c.fotos[k];
  return {
    suscriptoresMax: maximo(cs.map((c) => c.suscriptores)),
    clavesMax: maximo(cs.map((c) => c.claves)),
    flushes: suma(cs.map((c) => c.flushes)),
    bytesMinPorCliente: Math.round(porCliente.length === 0 ? 0 : suma(porCliente) / porCliente.length),
    bytesFlush: agregar(cs.map((c) => c.bytesFlush)),
    bytesFotoMax: maximo(cs.map((c) => Math.max(c.bytesFoto.max, c.bytesFotoVida ?? 0))),
    msProyeccion: agregar(cs.map((c) => c.msProyeccion)),
    msDiff: agregar(cs.map((c) => c.msDiff)),
    msSerializacion: agregar(cs.map((c) => c.msSerializacion)),
    fotos,
    socketsLentos: suma(cs.map((c) => c.socketsLentos)),
    cierres4408: suma(cs.map((c) => c.cierres4408)),
  };
}

function informeDeAuditoria(entradas: EntradaAuditoria[]): InformeDeAuditoria {
  const rechazos: Record<string, number> = {};
  const porComando = new Map<string, { ms: number[]; bytes: number[] }>();
  const todosMs: number[] = [];
  const todosBytes: number[] = [];
  for (const e of entradas) {
    if (e.resultado === 'rechazado') rechazos[e.causa ?? 'sin_causa'] = (rechazos[e.causa ?? 'sin_causa'] ?? 0) + 1;
    if (e.ms === undefined) continue;
    todosMs.push(e.ms);
    if (e.bytes !== undefined) todosBytes.push(e.bytes);
    const c = porComando.get(e.comando) ?? { ms: [], bytes: [] };
    c.ms.push(e.ms);
    if (e.bytes !== undefined) c.bytes.push(e.bytes);
    porComando.set(e.comando, c);
  }
  const ag = (xs: number[]): Agregado => ({ p50: r2(percentil(xs, 0.5)), p95: r2(percentil(xs, 0.95)), max: r2(maximo(xs)) });
  return {
    comandos: entradas.length,
    rechazos,
    ms: ag(todosMs),
    bytes: ag(todosBytes),
    masCaros: [...porComando.entries()]
      .map(([comando, c]) => ({ comando, n: c.ms.length, msP50: r2(percentil(c.ms, 0.5)), msMax: r2(maximo(c.ms)), bytesMax: maximo(c.bytes) }))
      .sort((a, b) => b.msMax - a.msMax)
      .slice(0, 5),
  };
}

export function generarInforme(muestras: MuestraDeCarga[], auditoria?: EntradaAuditoria[]): InformeDeCarga {
  const tipos = [...new Set(muestras.flatMap((m) => Object.keys(m.canales)))].sort();
  const canales = Object.fromEntries(tipos.map((t) => [t, informeDeCanal(muestras, t)]));
  const conVentana = muestras.filter((m) => m.ventanaMs > 0);
  const duracionMin = r2(suma(conVentana.map((m) => m.ventanaMs)) / 60_000);
  const rss = muestras.map((m) => m.memoriaMb.rss);

  const sondeoBytesMin = (bytesFoto: number, cadaS: number): number => Math.round((bytesFoto * 60) / cadaS);
  const contraSondeo: InformeDeCarga['contraSondeo'] = {};
  const admin = canales['admin/partida'];
  if (admin && admin.bytesFotoMax > 0) {
    const s = sondeoBytesMin(admin.bytesFotoMax, SONDEO_ADMIN_S);
    contraSondeo.admin = { sondeoBytesMin: s, empujadoBytesMin: admin.bytesMinPorCliente, razon: r2(admin.bytesMinPorCliente / s) };
  }
  const mapa = canales['vista/mapa'];
  const heroe = canales['vista/heroe'];
  if (mapa && heroe && mapa.bytesFotoMax + heroe.bytesFotoMax > 0) {
    const s = sondeoBytesMin(mapa.bytesFotoMax + heroe.bytesFotoMax, SONDEO_JUGADOR_S);
    const e = mapa.bytesMinPorCliente + heroe.bytesMinPorCliente;
    contraSondeo.jugador = { sondeoBytesMin: s, empujadoBytesMin: e, razon: r2(e / s) };
  }

  const informe: InformeDeCarga = {
    muestras: muestras.length,
    desde: muestras[0]?.momento,
    hasta: muestras[muestras.length - 1]?.momento,
    duracionMin,
    playtestIds: [...new Set(muestras.map((m) => m.playtestId).filter((x): x is string => !!x))],
    tickMs: agregar(muestras.map((m) => m.tickMs)),
    bucleMs: { p99: r2(maximo(muestras.map((m) => m.bucleMs.p99))), max: r2(maximo(muestras.map((m) => m.bucleMs.max))) },
    guardado: {
      msP95: r2(percentil(muestras.filter((m) => m.guardadoMs.n > 0).map((m) => m.guardadoMs.p95), 0.95)),
      msMax: r2(maximo(muestras.map((m) => m.guardadoMs.max))),
      bytesMax: maximo(muestras.map((m) => m.guardadoBytes)),
    },
    conexionesMax: maximo(muestras.map((m) => m.conexiones)),
    colaPendienteMax: maximo(muestras.map((m) => m.colaPendiente)),
    memoriaMb: { rssInicial: rss[0] ?? 0, rssFinal: rss[rss.length - 1] ?? 0, rssMax: maximo(rss) },
    costeMuestraMs: agregar(muestras.map((m) => ({ n: 1, p50: m.costeMuestraMs, p95: m.costeMuestraMs, max: m.costeMuestraMs }))),
    canales,
    contraSondeo,
    auditoria: auditoria ? informeDeAuditoria(auditoria) : undefined,
    candidatos: [],
  };
  informe.candidatos = candidatosDe(informe);
  return informe;
}

/** Qué mejoras de 03 §13 activan los umbrales. 13.1 está hecha; 13.2-13.8 son candidatas, no se implementan sin luz verde del usuario. */
export function candidatosDe(i: InformeDeCarga): Candidato[] {
  const c: Candidato[] = [];
  const canales = Object.entries(i.canales);
  const peor = (campo: (x: InformeDeCanal) => number): { tipo: string; valor: number } =>
    canales.reduce((m, [tipo, x]) => (campo(x) > m.valor ? { tipo, valor: campo(x) } : m), { tipo: '-', valor: 0 });

  const proy = peor((x) => x.msProyeccion.p95);
  if (proy.valor > UMBRALES.msProyeccionP95) c.push({ id: '13.4', mejora: 'Seguimiento de cambios (dirty tracking)', motivo: `ms de proyección p95 = ${proy.valor} en ${proy.tipo} (umbral ${UMBRALES.msProyeccionP95})` });

  const ser = peor((x) => x.msSerializacion.p95);
  if (ser.valor > UMBRALES.msSerializacionP95) c.push({ id: '13.7', mejora: 'Serialización fuera del hilo principal', motivo: `ms de serialización p95 = ${ser.valor} en ${ser.tipo} (umbral ${UMBRALES.msSerializacionP95})` });

  const foto = peor((x) => x.bytesFotoMax);
  const cliente = peor((x) => x.bytesMinPorCliente);
  if (foto.valor > UMBRALES.bytesFotoMax || cliente.valor > UMBRALES.bytesMinPorCliente) {
    c.push({ id: '13.2', mejora: 'Compresión permessage-deflate (al menos fotos)', motivo: `foto máx ${Math.round(foto.valor / KIB)} KiB en ${foto.tipo}; ${Math.round(cliente.valor / KIB)} KiB/min por cliente en ${cliente.tipo}` });
  }
  if (cliente.valor > UMBRALES.bytesMinPorCliente) {
    c.push({ id: '13.5', mejora: 'Suscripción por visibilidad / ritmo lento del admin', motivo: `${Math.round(cliente.valor / KIB)} KiB/min por cliente en ${cliente.tipo} (umbral ${UMBRALES.bytesMinPorCliente / KIB} KiB)` });
  }

  const resync = suma(canales.map(([, x]) => x.fotos.resync + x.fotos.lento + x.fotos.visibilidad));
  if (resync >= UMBRALES.fotosDeResync) c.push({ id: '13.3', mejora: 'Reanudar con `desde`', motivo: `${resync} fotos de resync/lento/visibilidad (umbral ${UMBRALES.fotosDeResync})` });

  if (i.bucleMs.p99 > UMBRALES.bucleP99Ms || i.tickMs.p95 > UMBRALES.tickP95Ms) {
    c.push({ id: '13.6', mejora: 'Ritmo adaptativo del coalescer', motivo: `bucle p99 ${i.bucleMs.p99} ms (umbral ${UMBRALES.bucleP99Ms}), tick p95 ${i.tickMs.p95} ms (umbral ${UMBRALES.tickP95Ms})` });
  }

  const lentos = suma(canales.map(([, x]) => x.socketsLentos + x.cierres4408));
  if (lentos >= UMBRALES.socketsLentos) c.push({ id: '13.8', mejora: 'Presupuesto de bytes por socket', motivo: `${lentos} sockets lentos o cierres 4408` });
  return c;
}

const kib = (b: number): string => `${(b / KIB).toFixed(1)} KiB`;
const ag = (a: Agregado, u = 'ms'): string => `p50 ${a.p50} · p95 ${a.p95} · máx ${a.max} ${u}`;

/** El informe como Markdown (lo que ve el usuario en el terminal y descarga el panel). */
export function aMarkdown(i: InformeDeCarga, titulo = 'Informe de carga'): string {
  const L: string[] = [`# ${titulo}`, ''];
  L.push(`- Muestras: **${i.muestras}** (${i.duracionMin} min) · ${i.desde ?? '—'} → ${i.hasta ?? '—'}`);
  if (i.playtestIds.length > 0) L.push(`- Pruebas: ${i.playtestIds.join(', ')}`);
  L.push(`- Conexiones máx: ${i.conexionesMax} · cola máx: ${i.colaPendienteMax}`);
  L.push(`- Tick del motor: ${ag(i.tickMs)}`);
  L.push(`- Bucle de eventos: p99 ${i.bucleMs.p99} ms · máx ${i.bucleMs.max} ms`);
  L.push(`- Guardado del snapshot: p95 ${i.guardado.msP95} ms · máx ${i.guardado.msMax} ms · ${kib(i.guardado.bytesMax)}`);
  L.push(`- Memoria rss: ${i.memoriaMb.rssInicial} → ${i.memoriaMb.rssFinal} MB (máx ${i.memoriaMb.rssMax})`);
  L.push(`- Coste de medir: ${ag(i.costeMuestraMs)}`, '');
  L.push('## Canales', '', '| Canal | Suscr./claves | KiB/min/cliente | Foto máx | Proyección ms (p95/máx) | Diff ms (p95/máx) | Serial. ms (p95/máx) | Fotos alta/resync/lento/vis. | Lentos/4408 |', '|---|---|---|---|---|---|---|---|---|');
  for (const [t, c] of Object.entries(i.canales)) {
    L.push(`| ${t} | ${c.suscriptoresMax}/${c.clavesMax} | ${(c.bytesMinPorCliente / KIB).toFixed(1)} | ${kib(c.bytesFotoMax)} | ${c.msProyeccion.p95}/${c.msProyeccion.max} | ${c.msDiff.p95}/${c.msDiff.max} | ${c.msSerializacion.p95}/${c.msSerializacion.max} | ${c.fotos.alta}/${c.fotos.resync}/${c.fotos.lento}/${c.fotos.visibilidad} | ${c.socketsLentos}/${c.cierres4408} |`);
  }
  L.push('', '## Empujado contra sondeo (por cliente, aproximado con el tamaño máximo de foto)', '');
  const fila = (n: string, s?: ComparacionSondeo, cadaS?: number) =>
    s ? L.push(`- ${n}: sondeo (${cadaS} s) ${kib(s.sondeoBytesMin)}/min · empujado ${kib(s.empujadoBytesMin)}/min · **${(s.razon * 100).toFixed(1)} %**`) : L.push(`- ${n}: sin datos`);
  fila('Admin', i.contraSondeo.admin, SONDEO_ADMIN_S);
  fila('Jugador', i.contraSondeo.jugador, SONDEO_JUGADOR_S);
  if (i.auditoria) {
    const a = i.auditoria;
    L.push('', '## Comandos (auditoría)', '', `- ${a.comandos} comandos · coste ${ag(a.ms)} · respuesta ${ag(a.bytes, 'B')}`);
    L.push(`- Rechazos: ${Object.keys(a.rechazos).length === 0 ? 'ninguno' : Object.entries(a.rechazos).map(([k, v]) => `${k} ${v}`).join(', ')}`);
    for (const c of a.masCaros) L.push(`  - ${c.comando}: n=${c.n}, p50 ${c.msP50} ms, máx ${c.msMax} ms, respuesta máx ${kib(c.bytesMax)}`);
  }
  L.push('', '## Candidatos de 03 §13 (no se implementan sin luz verde)', '');
  if (i.candidatos.length === 0) L.push('Ningún umbral superado.');
  for (const c of i.candidatos) L.push(`- **${c.id}** ${c.mejora}: ${c.motivo}`);
  return L.join('\n') + '\n';
}
