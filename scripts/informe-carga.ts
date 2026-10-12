// Informe de una prueba de carga (contrato 03 §14): percentiles, picos, bytes/min por cliente contra el sondeo y candidatos de §13.
//
//   npx tsx scripts/informe-carga.ts <directorio-de-partidas> <gameId> [--playtest id] [--desde iso] [--hasta iso] [--json]
//   npx tsx scripts/informe-carga.ts --url http://localhost:3000 <gameId> [--admin "dev jefa"] [...mismos filtros]
//
// Con directorio lee los JSONL (solo lectura); con --url pide `GET /admin/partidas/:gameId/{carga,auditoria}` con una sesión de administrador.
// La línea base de bytes del sondeo sale del tamaño de las fotos medidas (ver `informeDeCarga.ts`); para la base offline exacta, `medicion-tiempo-real.ts`.
import { crearAlmacenEnDisco } from '../src/server/almacen/enDisco';
import { leerAuditoria, type EntradaAuditoria } from '../src/server/auditoria';
import { aMarkdown, generarInforme } from '../src/server/informeDeCarga';
import { leerCarga, type MuestraDeCarga } from '../src/server/telemetria';

const args = process.argv.slice(2);
const CON_VALOR = new Set(['--url', '--admin', '--playtest', '--desde', '--hasta']);
const opcion = (nombre: string): string | undefined => {
  const i = args.indexOf(`--${nombre}`);
  return i >= 0 ? args[i + 1] : undefined;
};
const url = opcion('url');
const posicionales = args.filter((a, i) => !a.startsWith('--') && !CON_VALOR.has(args[i - 1] ?? ''));
const [origen, gameId] = url ? [url, posicionales[0]] : [posicionales[0], posicionales[1]];
if (!origen || !gameId) {
  console.error(
    'uso: npx tsx scripts/informe-carga.ts <directorio> <gameId> [--playtest id] [--desde iso] [--hasta iso] [--json]\n' +
      '     npx tsx scripts/informe-carga.ts --url <servidor> <gameId> [--admin "dev jefa"]'
  );
  process.exit(2);
}
const filtro = { desde: opcion('desde'), hasta: opcion('hasta'), playtestId: opcion('playtest') };

async function desdeDisco(): Promise<{ muestras: MuestraDeCarga[]; auditoria: EntradaAuditoria[] }> {
  const almacen = crearAlmacenEnDisco(origen!);
  const { lineas } = await leerCarga(almacen, gameId!, filtro);
  const muestras = lineas.filter((l): l is MuestraDeCarga => l.tipo === 'muestra');
  const auditoria = (await leerAuditoria(almacen, gameId!, { desde: filtro.desde ?? muestras[0]?.momento, hasta: filtro.hasta ?? muestras.at(-1)?.momento })).entradas;
  return { muestras, auditoria };
}

async function desdeServidor(): Promise<{ muestras: MuestraDeCarga[]; auditoria: EntradaAuditoria[] }> {
  const login = await fetch(`${origen}/v1/sesiones`, { method: 'POST', headers: { authorization: opcion('admin') ?? 'dev jefa' } });
  if (!login.ok) throw new Error(`login ${login.status}`);
  const headers = { authorization: `sesion ${((await login.json()) as { sesionId: string }).sesionId}` };
  const q = (o: Record<string, string | undefined>) => new URLSearchParams(Object.entries(o).filter((e): e is [string, string] => e[1] !== undefined)).toString();
  const pedir = async <T>(ruta: string, qs: string): Promise<T> => {
    const r = await fetch(`${origen}/v1/admin/partidas/${gameId}/${ruta}?${qs}`, { headers });
    if (!r.ok) throw new Error(`${ruta} ${r.status}`);
    return (await r.json()) as T;
  };
  const { muestras } = await pedir<{ muestras: MuestraDeCarga[] }>('carga', q(filtro));
  const { entradas } = await pedir<{ entradas: EntradaAuditoria[] }>('auditoria', q({ desde: filtro.desde ?? muestras[0]?.momento, hasta: filtro.hasta ?? muestras.at(-1)?.momento }));
  return { muestras, auditoria: entradas };
}

const { muestras, auditoria } = url ? await desdeServidor() : await desdeDisco();
if (muestras.length === 0) {
  console.error(`sin muestras de carga para '${gameId}' con ese filtro.`);
  process.exit(1);
}
const informe = generarInforme(muestras, auditoria);
console.log(args.includes('--json') ? JSON.stringify(informe, null, 2) : aMarkdown(informe, `Informe de carga — ${gameId}${filtro.playtestId ? ` (${filtro.playtestId})` : ''}`));
