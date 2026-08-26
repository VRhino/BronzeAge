// Sirve el balance como DATOS, no como código (Fase C7, doc 9 — patrón *Static Data Export*): las 39 tablas
// de `constants.ts` completas, más `version` (`BALANCE_VERSION`) para que un cliente sepa cuándo invalidar lo
// que tenga cacheado. Ninguna de las 39 es privilegiada (T1 en doc 9: regla pública, no estado de un rival),
// así que a diferencia de `/admin/*` y `/jugador/*` esta ruta no exige sesión — mismo criterio que
// `GET /openapi.json`: es lo primero que un cliente nuevo puede necesitar, antes incluso de poder hacer login.
//
// NO está bajo `/partidas/:gameId/`: hoy el balance es un único valor de proceso, no hay overrides por
// partida que justifiquen el prefijo (ver el comentario de `BALANCE_VERSION` en `constants.ts`).
//
// Sin `schema.response`: publicar 39 tablas con su forma completa en JSON Schema es una segunda fuente de
// verdad que mantener sincronizada con `constants.ts` a cada cambio de balance, por el mismo motivo que
// `esquemas.ts` ya deja sin modelar los cuerpos grandes o de forma variable — ver su cabecera.
import type { FastifyInstance } from 'fastify';
import {
  ALMACEN,
  ASIGNACION_CARAVANA,
  BALANCE_VERSION,
  CAMPAMENTOS_BANDIDOS,
  CAP_FUNDACION_POR_NIVEL,
  CARAVANA_CATALOGO,
  CARAVANA_COOLDOWN,
  CIUDADANIA,
  COMISION,
  CUPO_NIVEL_ASENTAMIENTO,
  EDIFICIO_CATALOGO,
  EDIFICIO_TAMANO,
  EXTRACCION_MAXIMOS,
  EXTRACTOR_DESEMPATE,
  FUNDACION,
  LENERA_POR_BOSQUE,
  LINEAS_PRODUCCION,
  MANTENIMIENTO,
  MERCADO_PUESTOS_POR_NIVEL,
  MILITAR,
  NECESIDADES,
  NIVEL_ASENTAMIENTO,
  NIVEL_FACCION,
  POBLACION,
  POLITICAS,
  POLITICA_CATALOGO,
  PRECIO_BASE,
  PRECIO_REFERENCIA,
  PUESTO_MERCADO_FORMA,
  REGENERACION_NODOS,
  REJILLA_ASENTAMIENTO,
  REPUTACION,
  RESERVA_CONSTRUCCION,
  SCORE_BANDAS,
  SIMULACION_AUTO_COMERCIO,
  SITIO,
  TRAZADO,
  TROPAS_RECLUTABLES,
  TRUEQUE,
  ZONA_INFLUENCIA,
} from '../../constants';

export const ESQUEMA_BALANCE = {
  description:
    'Las 39 tablas de balance del proceso (recursos, edificios, economía, población, construcción, combate, ' +
    'política, mundo), más `version` (Fase C7, patrón Static Data Export). Sin autenticar: es regla pública, ' +
    'no estado de partida. Cuerpo no modelado en este esquema por su tamaño — ver esquemas.ts.',
  tags: ['balance'],
} as const;

/** Agrupado por el mismo criterio que la tabla "por consumidor de interfaz" del doc 9, para que quien lea la
 * respuesta cruda pueda ubicar una tabla sin memorizarse las 39. El agrupado es solo presentación: cada tabla
 * sigue siendo el mismo objeto exportado por `constants.ts`, sin transformar. */
function balancePublicado() {
  return {
    version: BALANCE_VERSION,
    catalogos: { EDIFICIO_CATALOGO, POLITICA_CATALOGO, TROPAS_RECLUTABLES, CARAVANA_CATALOGO },
    cuposYNiveles: { NIVEL_FACCION, NIVEL_ASENTAMIENTO, CAP_FUNDACION_POR_NIVEL, CUPO_NIVEL_ASENTAMIENTO, POLITICAS, CIUDADANIA },
    costesYEconomia: { MANTENIMIENTO, ALMACEN, NECESIDADES, PRECIO_BASE, PRECIO_REFERENCIA, COMISION, TRUEQUE, RESERVA_CONSTRUCCION },
    geometriaUrbana: { REJILLA_ASENTAMIENTO, EDIFICIO_TAMANO, TRAZADO, SITIO, PUESTO_MERCADO_FORMA, MERCADO_PUESTOS_POR_NIVEL },
    mundoYMilitar: { ZONA_INFLUENCIA, FUNDACION, POBLACION, MILITAR, LENERA_POR_BOSQUE },
    caravanas: { CARAVANA_COOLDOWN, ASIGNACION_CARAVANA },
    reputacion: { REPUTACION },
    internas: { CAMPAMENTOS_BANDIDOS, REGENERACION_NODOS, SCORE_BANDAS, EXTRACTOR_DESEMPATE, LINEAS_PRODUCCION, EXTRACCION_MAXIMOS, SIMULACION_AUTO_COMERCIO },
  };
}

export function registrarRutaDeBalance(app: FastifyInstance): void {
  app.get('/balance', { schema: ESQUEMA_BALANCE }, async () => balancePublicado());
}
