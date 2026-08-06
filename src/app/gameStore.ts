// Capa de aplicación: única puerta de entrada a la lógica de simulación (motor + dominio).
// Ninguna capa de interfaz debe importar nada de `../engine/*` ni de `../domain/types` para
// mutar estado o capturar errores — solo debe hablar con `gameStore` (acciones + getState +
// subscribe) y con tipos de datos para tipar lo que lee. Así, cambiar de interfaz (otra librería
// de UI, un cliente CLI, tests) no requiere tocar nada de esta carpeta ni de `engine/`.
import type {
  AcuerdoTrueque,
  Asentamiento,
  CargoTipo,
  Caravana,
  Faccion,
  OrdenMercado,
  RecursoTipo,
  RelacionPolitica,
  Titulo,
  World,
  ZonaInfluencia,
} from '../domain/types';
import { POLITICA_CATALOGO, WORLD_DEFAULT } from '../constants';
import { generateWorld } from '../engine/world';
import { fundarAsentamiento as fundarAsentamientoEngine, FundacionInvalidaError } from '../engine/settlement';
import { computeTodasLasZonas } from '../engine/zones';
import { avanzarSimulacion } from '../engine/simulation';
import { proponerTrueque as proponerTruequeEngine, TruequeInvalidoError } from '../engine/trade';
import { colocarOrdenMercado as colocarOrdenMercadoEngine, calcularPrecioReferencia, OrdenInvalidaError } from '../engine/market';
import { crearFaccion, comprarCasa as comprarCasaEngine, calcularCapFundacion, capacidadCasas, FaccionInvalidaError } from '../engine/faccion';
import { asignarRey as asignarReyEngine, asignarEmbajador as asignarEmbajadorEngine, asignarCargoLocal as asignarCargoLocalEngine, CargoInvalidoError } from '../engine/cargos';
import { activarPolitica as activarPoliticaEngine, PoliticaInvalidaError } from '../engine/politicas';
import {
  proponerVasallaje as proponerVasallajeEngine,
  proponerAlianza as proponerAlianzaEngine,
  romperRelacion as romperRelacionEngine,
  rebelionVasallo as rebelionVasalloEngine,
  DiplomaciaInvalidaError,
} from '../engine/diplomacia';
import { computeLigas, type LigaInfo } from '../engine/liga';
import { anexionar as anexionarEngine, fusionar as fusionarEngine, FusionInvalidaError } from '../engine/fusion';
import { reclutar as reclutarEngine, ReclutamientoInvalidoError } from '../engine/tropas';
import { construirManualmente as construirManualmenteEngine, ConstruccionManualInvalidaError } from '../engine/construction';
import { iniciarAsedio as iniciarAsedioEngine, combateCampoAbierto as combateCampoAbiertoEngine, interceptarCaravana as interceptarCaravanaEngine, CombateInvalidoError } from '../engine/combate';

export interface EventoLog {
  tick: number;
  mensaje: string;
}

export interface GameState {
  world: World;
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  caravanas: Caravana[];
  acuerdos: AcuerdoTrueque[];
  ordenes: OrdenMercado[];
  relaciones: RelacionPolitica[];
  titulos: Titulo[];
  tick: number;
  log: EventoLog[];
  historialJugadores: Record<string, EventoLog[]>;
}

/** Catálogos de referencia (listas fijas, sin comportamiento) que la interfaz necesita para construir formularios. */
export const CATALOGOS = {
  cargos: ['gobernador', 'tesorero', 'general', 'maestroObras', 'sacerdote'] as CargoTipo[],
  origenesTropa: ['pesants', 'artesanos', 'nobleza'] as const,
  recursosTrueque: ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock'] as RecursoTipo[],
  recursosMercado: ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'livestock'] as RecursoTipo[],
  politicas: POLITICA_CATALOGO,
};

type Listener = () => void;

function crearFaccionesIniciales(): Faccion[] {
  return [crearFaccion('faccion-1', 'Micenas'), crearFaccion('faccion-2', 'Troya'), crearFaccion('faccion-3', 'Ugarit')];
}

function idsNoVacios(csv: string): string[] {
  return csv
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

/**
 * Único dueño del estado de simulación y única puerta hacia el motor. Expone acciones de alto
 * nivel (una por intención de usuario) que ya resuelven validación/errores de dominio y dejan
 * constancia en el log — la interfaz nunca necesita conocer las clases de error del motor.
 */
export class GameStore {
  private state: GameState;
  private listeners = new Set<Listener>();
  private contadorAcciones = 0;

  constructor() {
    this.state = {
      world: generateWorld({ ...WORLD_DEFAULT, seed: 1 }),
      asentamientos: [],
      facciones: crearFaccionesIniciales(),
      caravanas: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      tick: 0,
      log: [],
      historialJugadores: {},
    };
    this.registrar('Mundo generado. Selecciona una facción y haz clic en el mapa para fundar.');
  }

  getState(): Readonly<GameState> {
    return this.state;
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify(): void {
    for (const listener of this.listeners) listener();
  }

  private registrar(mensaje: string): void {
    this.state.log = [{ tick: this.state.tick, mensaje }, ...this.state.log];
  }

  private registrarJugador(jugadorId: string, mensaje: string): void {
    if (!jugadorId) return;
    const lista = this.state.historialJugadores[jugadorId] ?? [];
    this.state.historialJugadores[jugadorId] = [{ tick: this.state.tick, mensaje }, ...lista];
  }

  // --- Derivados de solo lectura (evitan que la interfaz importe funciones del motor) ---

  getZonas(): ZonaInfluencia[] {
    return computeTodasLasZonas(this.state.asentamientos);
  }

  getLigas(): LigaInfo[] {
    return computeLigas(this.state.relaciones, this.state.facciones);
  }

  capFundacion(nivel: number): number {
    return calcularCapFundacion(nivel);
  }

  cupoVivienda(asentamiento: Asentamiento): number {
    return capacidadCasas(asentamiento);
  }

  precioReferencia(recurso: string): number {
    return calcularPrecioReferencia(recurso, this.state.asentamientos);
  }

  // --- Acciones (una por intención de usuario) ---

  fundarAsentamiento(faccionId: string, posicion: { x: number; y: number }, numJugadores: number): void {
    const n = Math.min(5, Math.max(1, numJugadores || 1));
    const jugadoresIds = Array.from({ length: n }, (_, i) => `jugador-${faccionId}-${i + 1}`);
    try {
      const resultado = fundarAsentamientoEngine(
        this.state.world,
        this.state.facciones,
        faccionId,
        posicion,
        jugadoresIds,
        this.state.asentamientos,
        this.state.tick
      );
      this.state.asentamientos = [...this.state.asentamientos, resultado.asentamiento];
      this.state.facciones = resultado.facciones;
      const nombreFaccion = this.state.facciones.find((f) => f.id === faccionId)?.nombre ?? faccionId;
      this.registrar(`${nombreFaccion} funda asentamiento en (${Math.round(posicion.x)}, ${Math.round(posicion.y)}).`);
      for (const jugadorId of jugadoresIds) {
        this.registrarJugador(jugadorId, `Funda ${resultado.asentamiento.id} (${nombreFaccion}) y recibe casa + ciudadanía.`);
      }
    } catch (err) {
      if (err instanceof FundacionInvalidaError) this.registrar(`Fundación rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  asignarRey(faccionId: string, jugadorId: string): void {
    try {
      const faccion = this.state.facciones.find((f) => f.id === faccionId)!;
      this.state.facciones = this.state.facciones.map((f) => (f.id === faccion.id ? asignarReyEngine(f, jugadorId) : f));
      this.registrar(`${faccion.nombre}: ${jugadorId} es el nuevo Rey.`);
      this.registrarJugador(jugadorId, `Nombrado Rey de ${faccion.nombre}.`);
    } catch (err) {
      if (err instanceof CargoInvalidoError) this.registrar(`Rey rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  asignarEmbajador(faccionId: string, jugadorId: string): void {
    try {
      const faccion = this.state.facciones.find((f) => f.id === faccionId)!;
      this.state.facciones = this.state.facciones.map((f) => (f.id === faccion.id ? asignarEmbajadorEngine(f, jugadorId) : f));
      this.registrar(`${faccion.nombre}: ${jugadorId} es el nuevo Embajador.`);
      this.registrarJugador(jugadorId, `Nombrado Embajador de ${faccion.nombre}.`);
    } catch (err) {
      if (err instanceof CargoInvalidoError) this.registrar(`Embajador rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  asignarCargoLocal(asentamientoId: string, cargo: CargoTipo, jugadorId: string): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const faccion = this.state.facciones.find((f) => f.id === asentamiento.faccionId)!;
      const actualizado = asignarCargoLocalEngine(asentamiento, faccion, cargo, jugadorId);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.registrar(`${asentamiento.id}: ${jugadorId} asignado como ${cargo}.`);
      this.registrarJugador(jugadorId, `Asignado como ${cargo} en ${asentamiento.id}.`);
    } catch (err) {
      if (err instanceof CargoInvalidoError) this.registrar(`Cargo rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  comprarCasa(asentamientoId: string, jugadorId: string): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const resultado = comprarCasaEngine(this.state.facciones, asentamiento, jugadorId);
      this.state.facciones = resultado.facciones;
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === asentamiento.id ? resultado.asentamiento : a));
      this.registrar(`${jugadorId} compra casa en ${asentamiento.id} y obtiene ciudadanía.`);
      this.registrarJugador(jugadorId, `Compra casa en ${asentamiento.id} y obtiene ciudadanía.`);
    } catch (err) {
      if (err instanceof FaccionInvalidaError) this.registrar(`Compra de casa rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  activarPolitica(asentamientoId: string, cargo: CargoTipo, politicaId: string): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const faccion = this.state.facciones.find((f) => f.id === asentamiento.faccionId)!;
      const actualizado = activarPoliticaEngine(asentamiento, faccion, cargo, politicaId, this.state.tick, this.contadorAcciones++);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.registrar(`${asentamiento.id}: política "${politicaId}" activada por ${cargo}.`);
    } catch (err) {
      if (err instanceof PoliticaInvalidaError) this.registrar(`Política rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  proponerRelacion(
    tipo: 'vasallaje' | 'alianza',
    faccionAId: string,
    faccionBId: string,
    tributoRecurso: string,
    tributoCantidad: number
  ): void {
    try {
      const nueva =
        tipo === 'vasallaje'
          ? proponerVasallajeEngine(
              this.state.facciones,
              this.state.relaciones,
              faccionAId,
              faccionBId,
              tributoRecurso,
              tributoCantidad,
              this.state.tick,
              this.contadorAcciones++
            )
          : proponerAlianzaEngine(this.state.facciones, this.state.relaciones, faccionAId, faccionBId, this.state.tick, this.contadorAcciones++);
      this.state.relaciones = [...this.state.relaciones, nueva];
      this.registrar(`Relación propuesta: ${nueva.id}.`);
    } catch (err) {
      if (err instanceof DiplomaciaInvalidaError) this.registrar(`Relación rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  romperRelacion(relacionId: string, iniciadorFaccionId: string): void {
    if (!relacionId) return;
    const resultado = romperRelacionEngine(this.state.facciones, this.state.relaciones, relacionId, iniciadorFaccionId);
    this.state.facciones = resultado.facciones;
    this.state.relaciones = resultado.relaciones;
    this.registrar(`Relación ${relacionId} rota voluntariamente.`);
    this.notify();
  }

  rebelionVasallo(relacionId: string): void {
    if (!relacionId) return;
    try {
      const resultado = rebelionVasalloEngine(this.state.facciones, this.state.relaciones, this.state.acuerdos, this.state.asentamientos, relacionId);
      this.state.facciones = resultado.facciones;
      this.state.relaciones = resultado.relaciones;
      this.state.acuerdos = resultado.acuerdos;
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof DiplomaciaInvalidaError) this.registrar(`Rebelión rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  anexionar(faccionAId: string, faccionBId: string): void {
    try {
      const resultado = anexionarEngine(this.state.facciones, this.state.asentamientos, faccionAId, faccionBId);
      this.state.facciones = resultado.facciones;
      this.state.asentamientos = resultado.asentamientos;
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof FusionInvalidaError) this.registrar(`Anexión rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  fusionar(faccionAId: string, faccionBId: string, nuevoNombre: string, nuevoReyId: string): void {
    try {
      const resultado = fusionarEngine(
        this.state.facciones,
        this.state.asentamientos,
        faccionAId,
        faccionBId,
        nuevoNombre || 'Facción Fusionada',
        nuevoReyId,
        this.state.tick
      );
      this.state.facciones = resultado.facciones;
      this.state.asentamientos = resultado.asentamientos;
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof FusionInvalidaError) this.registrar(`Fusión rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  proponerTrueque(
    asentamientoAId: string,
    recursoA: string,
    cantidadA: number,
    asentamientoBId: string,
    recursoB: string,
    cantidadB: number
  ): void {
    try {
      const nuevo = proponerTruequeEngine(
        this.state.asentamientos,
        asentamientoAId,
        asentamientoBId,
        recursoA,
        recursoB,
        cantidadA,
        cantidadB,
        this.state.tick,
        this.contadorAcciones++
      );
      this.state.acuerdos = [...this.state.acuerdos, nuevo];
      this.registrar(`Trueque propuesto: ${nuevo.id}.`);
    } catch (err) {
      if (err instanceof TruequeInvalidoError) this.registrar(`Trueque rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  colocarOrdenMercado(asentamientoId: string, tipo: 'compra' | 'venta', recurso: string, cantidad: number, precio: number | undefined): void {
    try {
      const nueva = colocarOrdenMercadoEngine(this.state.asentamientos, asentamientoId, tipo, recurso, cantidad, this.state.tick, precio, this.contadorAcciones++);
      this.state.ordenes = [...this.state.ordenes, nueva];
      this.registrar(`Orden de mercado colocada: ${nueva.id} (${nueva.tipo} ${nueva.cantidad} ${nueva.recurso} @ ${nueva.precioUnitario.toFixed(2)}).`);
    } catch (err) {
      if (err instanceof OrdenInvalidaError) this.registrar(`Orden rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  reclutar(asentamientoId: string, origen: 'pesants' | 'artesanos' | 'nobleza', cantidad: number): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const actualizado = reclutarEngine(asentamiento, origen, cantidad, this.state.tick, this.contadorAcciones++);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.registrar(`${asentamiento.id}: recluta ${cantidad} de ${origen}.`);
    } catch (err) {
      if (err instanceof ReclutamientoInvalidoError) this.registrar(`Reclutamiento rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  construirManualmente(asentamientoId: string, tipo: 'fundicion' | 'granFundicion'): void {
    try {
      const asentamiento = this.state.asentamientos.find((a) => a.id === asentamientoId)!;
      const faccion = this.state.facciones.find((f) => f.id === asentamiento.faccionId)!;
      const zona = this.getZonas().find((z) => z.asentamientoId === asentamiento.id);
      const actualizado = construirManualmenteEngine(asentamiento, faccion, zona?.poligono ?? [], tipo, this.contadorAcciones++);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === actualizado.id ? actualizado : a));
      this.registrar(`${asentamiento.id}: se encola ${tipo} (construcción manual).`);
    } catch (err) {
      if (err instanceof ConstruccionManualInvalidaError) this.registrar(`Construcción rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  iniciarAsedio(atacanteId: string, defensorId: string, escuadronesCsv: string): void {
    try {
      const atacante = this.state.asentamientos.find((a) => a.id === atacanteId)!;
      const defensor = this.state.asentamientos.find((a) => a.id === defensorId)!;
      const resultado = iniciarAsedioEngine(atacante, defensor, idsNoVacios(escuadronesCsv), this.state.facciones, this.state.relaciones, this.state.tick);
      this.state.asentamientos = this.state.asentamientos.map((a) => {
        if (a.id === resultado.atacante.id) return resultado.atacante;
        if (a.id === resultado.defensor.id) return resultado.defensor;
        return a;
      });
      this.state.facciones = resultado.facciones;
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof CombateInvalidoError) this.registrar(`Asedio rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  combateCampoAbierto(asentamientoAId: string, escuadronesACsv: string, asentamientoBId: string, escuadronesBCsv: string): void {
    try {
      const asentamientoA = this.state.asentamientos.find((a) => a.id === asentamientoAId)!;
      const asentamientoB = this.state.asentamientos.find((a) => a.id === asentamientoBId)!;
      const resultado = combateCampoAbiertoEngine(
        asentamientoA,
        idsNoVacios(escuadronesACsv),
        asentamientoB,
        idsNoVacios(escuadronesBCsv),
        this.state.facciones,
        this.state.relaciones,
        this.state.tick
      );
      this.state.asentamientos = this.state.asentamientos.map((a) => {
        if (a.id === resultado.asentamientoA.id) return resultado.asentamientoA;
        if (a.id === resultado.asentamientoB.id) return resultado.asentamientoB;
        return a;
      });
      this.state.facciones = resultado.facciones;
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof CombateInvalidoError) this.registrar(`Combate rechazado: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  interceptarCaravana(atacanteId: string, escuadronesCsv: string, caravanaId: string): void {
    try {
      const atacante = this.state.asentamientos.find((a) => a.id === atacanteId)!;
      const caravana = this.state.caravanas.find((c) => c.id === caravanaId)!;
      const resultado = interceptarCaravanaEngine(atacante, idsNoVacios(escuadronesCsv), caravana, this.state.tick);
      this.state.asentamientos = this.state.asentamientos.map((a) => (a.id === resultado.atacante.id ? resultado.atacante : a));
      if (resultado.caravanaCapturada) this.state.caravanas = this.state.caravanas.filter((c) => c.id !== caravana.id);
      for (const e of resultado.eventos) this.registrar(e);
    } catch (err) {
      if (err instanceof CombateInvalidoError) this.registrar(`Intercepción rechazada: ${err.message}`);
      else throw err;
    }
    this.notify();
  }

  avanzarTick(): void {
    this.state.tick += 1;
    const resultado = avanzarSimulacion(
      {
        asentamientos: this.state.asentamientos,
        facciones: this.state.facciones,
        caravanas: this.state.caravanas,
        acuerdos: this.state.acuerdos,
        ordenes: this.state.ordenes,
        relaciones: this.state.relaciones,
        titulos: this.state.titulos,
      },
      this.state.world,
      this.state.tick
    );
    this.state.asentamientos = resultado.asentamientos;
    this.state.facciones = resultado.facciones;
    this.state.caravanas = resultado.caravanas;
    this.state.acuerdos = resultado.acuerdos;
    this.state.ordenes = resultado.ordenes;
    this.state.relaciones = resultado.relaciones;
    this.state.titulos = resultado.titulos;
    for (const evento of resultado.eventos) this.registrar(evento);
    this.notify();
  }

  regenerarMundo(seed: number): void {
    this.state = {
      world: generateWorld({ ...WORLD_DEFAULT, seed }),
      asentamientos: [],
      facciones: crearFaccionesIniciales(),
      caravanas: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      tick: 0,
      log: [],
      historialJugadores: {},
    };
    this.registrar(`Mundo regenerado con seed ${seed}.`);
    this.notify();
  }
}

/** Instancia única que usa la interfaz. */
export const gameStore = new GameStore();
