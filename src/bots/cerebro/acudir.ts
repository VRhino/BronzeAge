// Acudir a donde los suyos combaten o se juntan (Doc 5.14.4, 5.15.1b): una columna personal parada y sin plan se une a una batalla de su Facción
// o a una formación de ejército de sus compañeros. Como un jugador: lo ve en su proyección, va hasta allí y lo pide.
//
// Solo hay batallas de Unity con servidores de batalla, y formaciones cuando un humano organiza una: sin ellos esto no hace nada, y el batch y los
// bots de siempre juegan igual.
import { LOGISTICA, VISION } from '../../constants';
import { distancia } from '../../world/geometria';
import type { Point } from '../../domain/types';
import type { ContextoBot } from '../runner';
import { estaHerido } from './comun';

const ACTIVAS = new Set(['convocando', 'asignada', 'en_curso']);
/** Cuánto espera tras un rechazo antes de volver a pedir lo mismo: lo que cambia es la batalla o la formación, no el bot. */
const ESPERA_MS = 10 * 60_000;

/**
 * La batalla de su Facción que más cerca tiene, sin contar las persecuciones (donde cualquiera elige bando: un bot no toma partido en
 * lo que no es de los suyos) ni una batalla campal (no admite a nadie). Si no hay, la formación de un compañero más cercana.
 */
function adonde(ctx: ContextoBot, desde: Point): { clave: string; punto: Point; pedir: () => ReturnType<ContextoBot['intentar']> } | undefined {
  const { vista, yo } = ctx;
  const cerca = <T extends { punto: Point }>(lista: readonly T[]): T | undefined => [...lista].sort((a, b) => distancia(a.punto, desde) - distancia(b.punto, desde))[0];

  const batalla = cerca(
    (vista.batallas ?? []).filter(
      (b) =>
        b.ladoPropio === undefined &&
        ACTIVAS.has(b.estado) &&
        (b.contexto.tipo === 'asedio' || b.contexto.tipo === 'caravana' || b.contexto.tipo === 'campamento_bandidos') &&
        (b.bandos.atacante.faccionId === vista.faccionId || b.bandos.defensor.faccionId === vista.faccionId)
    )
  );
  if (batalla) return { clave: `batalla:${batalla.battleId}`, punto: batalla.punto, pedir: () => ctx.intentar(`unirse-batalla:${batalla.battleId}`, 'unirseABatalla', { heroeId: yo, battleId: batalla.battleId }, ESPERA_MS) };

  const formacion = cerca(
    vista.ejercitos
      .filter((e) => e.formacion && !e.participantes.some((p) => p.heroeId === yo) && distancia(e.posicionActual, desde) <= VISION.ejercito)
      .map((e) => ({ id: e.id, punto: e.posicionActual }))
  );
  if (formacion) return { clave: `formacion:${formacion.id}`, punto: formacion.punto, pedir: () => ctx.intentar(`unirse-formacion:${formacion.id}`, 'unirseEnCampo', { ejercitoId: formacion.id, heroeId: yo }, ESPERA_MS) };
  return undefined;
}

/** `true` si hizo algo: pidió unirse, o se puso en marcha hacia allí. */
export async function acudirALosSuyos(ctx: ContextoBot): Promise<boolean> {
  const { vista, yo } = ctx;
  const columna = vista.ejercitos.find((e) => e.participantes.some((p) => p.heroeId === yo));
  // Solo una columna personal que va sola y está parada: un ejército no cambia de rumbo, y quien marcha tiene algo que hacer.
  if (!columna || columna.tipo !== 'personal' || columna.participantes.length !== 1 || columna.formacion || columna.estado !== 'estacionado' || estaHerido(vista)) return false;
  const destino = adonde(ctx, columna.posicionActual);
  if (!destino) return false;

  if (distancia(columna.posicionActual, destino.punto) <= LOGISTICA.radioEncuentro) return (await destino.pedir())?.ok === true;
  const marcha = await ctx.intentar(`acudir:${destino.clave}`, 'marcharA', { heroeId: yo, objetivo: { tipo: 'punto', punto: destino.punto } }, ESPERA_MS);
  return marcha?.ok === true;
}
