// Predicados de dominio reutilizados por `matriz.ts` (Docs/Arquitectura/5_Contratos_Identidad_Permisos.md,
// sección "Matriz de autorización por comando"): "Facción propia", "residente", "cargo de X". Viven en
// `server/` y no en `session/comandos/` porque necesitan `RolTecnico`/`Membresia` (conceptos de
// `server/identidad/`), y `session` no puede importar de `server` (frontera de capas,
// `src/__tests__/arquitectura.test.ts`) — solo al revés.
//
// CONVENCIÓN DELIBERADA: si la entidad referenciada por un id no existe, todas estas funciones devuelven
// `true` (dejan pasar). La comprobación de EXISTENCIA es responsabilidad del comando en sí (`exigirAsentamiento`
// et al. en `session/comandos/ayudas.ts`, con su propio código de error `*.no_existe`) — así un id inventado
// nunca resulta en un 403 engañoso ("no tienes permiso") cuando el problema real es "eso no existe" (que sí
// puede pasar limpiamente hasta el rechazo del comando, sin que ninguna acción no autorizada llegue a aplicarse).
import type { Asentamiento, CargoTipo, Caravana, Faccion } from '../../domain/types';
import type { GameSessionState } from '../../session/estado';

export function buscarAsentamiento(estado: GameSessionState, id: string): Asentamiento | undefined {
  return estado.asentamientos.find((a) => a.id === id);
}

export function buscarFaccion(estado: GameSessionState, id: string): Faccion | undefined {
  return estado.facciones.find((f) => f.id === id);
}

export function buscarCaravana(estado: GameSessionState, id: string): Caravana | undefined {
  return estado.caravanas.find((c) => c.id === id);
}

/** `actorFaccionId` es la Facción del `Jugador` del actor (`Membresia.faccionId`) — `null` si aún no se unió
 * a ninguna. */
export function esFaccionPropia(actorFaccionId: string | null, faccionObjetivoId: string): boolean {
  return actorFaccionId !== null && actorFaccionId === faccionObjetivoId;
}

export function esResidente(asentamiento: Asentamiento | undefined, jugadorId: string): boolean {
  if (!asentamiento) return true; // entidad inexistente: lo rechaza el comando, no la autorización
  return asentamiento.jugadoresFundadoresIds.includes(jugadorId) || asentamiento.casasCompradas.includes(jugadorId);
}

/** Es titular del `cargo` (gobernador/tesorero/general/maestroObras/sacerdote) de ESE asentamiento. */
export function tieneCargoLocal(asentamiento: Asentamiento | undefined, cargo: CargoTipo, jugadorId: string): boolean {
  if (!asentamiento) return true;
  const campo: Record<CargoTipo, keyof Asentamiento['cargos']> = {
    gobernador: 'gobernadorId',
    tesorero: 'tesoreroId',
    general: 'generalId',
    maestroObras: 'maestroObrasId',
    sacerdote: 'sacerdoteId',
  };
  return asentamiento.cargos[campo[cargo]] === jugadorId;
}

export function esReyDe(faccion: Faccion | undefined, jugadorId: string): boolean {
  if (!faccion) return true;
  return faccion.reyId === jugadorId;
}

export function esReyOEmbajadorDe(faccion: Faccion | undefined, jugadorId: string): boolean {
  if (!faccion) return true;
  return faccion.reyId === jugadorId || faccion.embajadorId === jugadorId;
}
