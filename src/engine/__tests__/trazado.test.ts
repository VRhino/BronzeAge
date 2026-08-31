// Invariantes del TRAZADO URBANO DINÁMICO de la Vista de Asentamiento.
//
// Especificación: `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md`. Estos tests cubren las tres cosas
// que el enfoque anterior (rejilla de calles pre-generada al fundar) rompía y que motivaron el rediseño:
// edificios encima de calles, calles encima de calles y granjas pegadas al Centro Urbano. Se comprueban sobre
// una simulación real de 300 ticks, no sobre casos armados a mano, porque los tres bugs solo aparecían con la
// ciudad ya crecida.
import { beforeEach, describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { TRAZADO } from '../../constants';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import {
  celdaMinimaDeEdificio,
  celdasDeEdificio,
  edificiosInternos,
  esDeAfueras,
  redDeCalles,
  tamanoDeEdificio,
} from '../trazado';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  posicionRecomendable,
} from './fixtures';

const SEED = 99;
const TICKS = 300;

function simular(): Asentamiento[] {
  const mapa = crearMapaDeterminista(SEED);
  const facciones = crearFacciones();
  const posicion = posicionRecomendable(mapa);
  const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', [], 0, posicion);

  let estado = crearEstadoDeTest([asentamiento], faccionesTrasFundar);
  const rng = createRng(SEED);
  for (let tick = 1; tick <= TICKS; tick++) estado = avanzarSimulacion(estado, mapa, contextoDeTest(tick, rng));
  return estado.asentamientos;
}

describe('trazado urbano dinámico', () => {
  let asentamientos: Asentamiento[];

  beforeEach(() => {
    asentamientos = simular();
  });

  it('la simulación produce ciudades con material suficiente para juzgar el trazado', () => {
    expect(asentamientos.length).toBeGreaterThan(0);
    const internos = asentamientos.flatMap((a) => edificiosInternos(a.edificios));
    expect(internos.length).toBeGreaterThan(5);
  });

  // El bug histórico "edificios encima de la calle" se resolvió por construcción (las calles corren sobre
  // aristas, no sobre celdas), pero el solapamiento ENTRE EDIFICIOS dejó de ser gratis al pasar de una celda
  // por edificio a huellas de hasta 6x6: esto ya sí hay que comprobarlo.
  it('ningún par de edificios comparte una celda', () => {
    for (const asentamiento of asentamientos) {
      const ocupadas = new Map<string, Edificio>();
      for (const edificio of edificiosInternos(asentamiento.edificios)) {
        for (const celda of celdasDeEdificio(edificio)) {
          const clave = `${celda.col},${celda.row}`;
          const previo = ocupadas.get(clave);
          expect(
            previo,
            `${edificio.tipo} (${edificio.id}) pisa la celda ${clave} de ${previo?.tipo} (${previo?.id})`
          ).toBeUndefined();
          ocupadas.set(clave, edificio);
        }
      }
    }
  });

  // Etapa 6 (§E6.12): con las calles sobre CELDAS, "un edificio encima de una calle" vuelve a ser posible y
  // deja de estar garantizado por construcción. Es una de las dos garantías que pasaron de gratuitas a
  // obligación al abandonar las aristas.
  it('ningún edificio pisa una celda de calle', () => {
    for (const asentamiento of asentamientos) {
      const red = redDeCalles(asentamiento.id, asentamiento.edificios);
      for (const edificio of edificiosInternos(asentamiento.edificios)) {
        for (const celda of celdasDeEdificio(edificio)) {
          const clave = `${celda.col},${celda.row}`;
          const enRed = red.calles.has(clave) || red.caminos.has(clave);
          expect(enRed, `${edificio.tipo} (${edificio.id}) ocupa la celda de calle ${clave}`).toBe(false);
        }
      }
    }
  });

  // Reemplaza al viejo "toca la red por alguna ARISTA de su perímetro", que se cumplía aunque al otro lado
  // hubiera otro edificio pegado — medido: el 51% de aquella red eran calles de ancho cero, y solo el 33% de
  // los edificios tenía delante algo por lo que se pudiera caminar (§E6.1).
  it('todo edificio interno tiene una CELDA de calle o camino ortogonalmente adyacente', () => {
    for (const asentamiento of asentamientos) {
      const red = redDeCalles(asentamiento.id, asentamiento.edificios);
      for (const edificio of edificiosInternos(asentamiento.edificios)) {
        const min = celdaMinimaDeEdificio(edificio);
        const tamano = tamanoDeEdificio(edificio);
        let conFrente = false;
        for (let dc = 0; dc < tamano.ancho && !conFrente; dc++) {
          for (const row of [min.row - 1, min.row + tamano.alto]) {
            const clave = `${min.col + dc},${row}`;
            if (red.calles.has(clave) || red.caminos.has(clave)) conFrente = true;
          }
        }
        for (let dr = 0; dr < tamano.alto && !conFrente; dr++) {
          for (const col of [min.col - 1, min.col + tamano.ancho]) {
            const clave = `${col},${min.row + dr}`;
            if (red.calles.has(clave) || red.caminos.has(clave)) conFrente = true;
          }
        }
        expect(conFrente, `${edificio.tipo} (${edificio.id}) quedó sin salida real a la calle`).toBe(true);
      }
    }
  });

  // EL invariante que el modelo de aristas no podía ni formular, y el que 3D necesita de verdad: se puede ir
  // andando de cualquier punto de la red a cualquier otro. Sale gratis por inducción (§E6.4) — las celdas de
  // calle están en `ocupadas`, así que nadie puede partir un corredor existente, y cada edificio o ya toca la
  // red o abre un corredor, que es un camino. Este test es lo que congela esa propiedad.
  it('la red es un ÚNICO componente conexo, alcanzable a pie desde el Centro Urbano', () => {
    for (const asentamiento of asentamientos) {
      const red = redDeCalles(asentamiento.id, asentamiento.edificios);
      const todas = new Set([...red.calles, ...red.caminos]);
      if (todas.size === 0) continue;

      const centroUrbano = edificiosInternos(asentamiento.edificios).find((e) => e.tipo === 'centroUrbano')!;
      const minCU = celdaMinimaDeEdificio(centroUrbano);
      const tamCU = tamanoDeEdificio(centroUrbano);
      const arranque = [...todas].find((clave) => {
        const [col, row] = clave.split(',').map(Number) as [number, number];
        return col >= minCU.col - 1 && col <= minCU.col + tamCU.ancho && row >= minCU.row - 1 && row <= minCU.row + tamCU.alto;
      });
      expect(arranque, `${asentamiento.id}: la red no toca el Centro Urbano`).toBeDefined();

      const vistas = new Set<string>([arranque!]);
      const cola = [arranque!];
      for (let i = 0; i < cola.length; i++) {
        const [col, row] = cola[i]!.split(',').map(Number) as [number, number];
        for (const [dc, dr] of [[0, -1], [1, 0], [0, 1], [-1, 0]] as [number, number][]) {
          const vecina = `${col + dc},${row + dr}`;
          if (!todas.has(vecina) || vistas.has(vecina)) continue;
          vistas.add(vecina);
          cola.push(vecina);
        }
      }

      const huerfanas = [...todas].filter((c) => !vistas.has(c));
      expect(
        huerfanas.length,
        `${asentamiento.id}: ${huerfanas.length}/${todas.size} celdas de calle no se alcanzan a pie desde el Centro Urbano (p.ej. ${huerfanas.slice(0, 5).join(' ')})`
      ).toBe(0);
    }
  });

  it('Granja y Corral se quedan a las afueras, nunca pegados al Centro Urbano', () => {
    for (const asentamiento of asentamientos) {
      for (const edificio of edificiosInternos(asentamiento.edificios)) {
        if (!esDeAfueras(edificio.tipo)) continue;
        const distancia = Math.hypot(edificio.posicion.x, edificio.posicion.y);
        expect(distancia, `${edificio.tipo} (${edificio.id}) quedó a ${distancia.toFixed(1)} del centro`).toBeGreaterThanOrEqual(
          TRAZADO.radioAfuerasMin
        );
      }
    }
  });

  // La red no se guarda en ningún sitio: se deriva recorriendo los edificios en orden de construcción. Si esa
  // derivación no fuera estable, la ciudad se redibujaría distinta en cada frame y las partidas guardadas no
  // reproducirían su propio trazado al cargarlas.
  it('la red es determinista: mismo asentamiento, misma red', () => {
    for (const asentamiento of asentamientos) {
      const primera = redDeCalles(asentamiento.id, asentamiento.edificios);
      const segunda = redDeCalles(asentamiento.id, asentamiento.edificios);
      expect([...segunda.calles]).toEqual([...primera.calles]);
      expect([...segunda.caminos]).toEqual([...primera.caminos]);
    }
  });
});
