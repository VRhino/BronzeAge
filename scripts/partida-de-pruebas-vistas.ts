// Genera una partida pequeña para probar en vivo los canales de vista (contrato 03): un usuario de CLAVE CONOCIDA que es Rey de una plaza con
// obras en curso y está DENTRO de ella, y una segunda plaza propia a la que entrar y de la que salir (para forzar `revocado`).
//
//   npx tsx scripts/partida-de-pruebas-vistas.ts <directorio> <gameId> [nick] [clave]
//
// Escribe en `<directorio>` (partidas, identidad): no lo apuntes al de una partida viva. Después se arranca el servidor con ese directorio:
//   DIRECTORIO_PARTIDAS=<directorio> ADMINISTRADORES=dev:admin PORT=3200 INTERVALO_TICK_MS=4000 npm run server
// y se abre la partida con `POST /v1/admin/partidas { gameId, seed: 42 }` (sesión de `dev admin`); el usuario entra con `Authorization: clave <nick>:<clave>`.
import { crearRegistroProveedores } from '../src/acceso/proveedorIdentidad';
import { crearAlmacenEnDisco } from '../src/server/almacen/enDisco';
import { crearServidor } from '../src/server/api';
import { proveedoresDeProceso } from '../src/server/identidad/proveedoresActivos';
import { crearRepositorioIdentidadPersistente } from '../src/server/identidad/repositorioPersistente';
import { cargarPartida, guardarPartida } from '../src/server/persistenciaPartida';
import { salirAlMundo } from '../src/session/comandos/presencia';
import { enPie } from '../src/session/__tests__/fixtures';
import { fundarAsentamiento } from '../src/session/__tests__/fundarDePrueba';

const [directorio, gameId, nick = 'rey', clave = 'clave-de-prueba'] = process.argv.slice(2);
if (!directorio || !gameId) {
  console.error('uso: npx tsx scripts/partida-de-pruebas-vistas.ts <directorio> <gameId> [nick] [clave]');
  process.exit(2);
}

const almacen = crearAlmacenEnDisco(directorio);
const identidad = await crearRepositorioIdentidadPersistente(almacen);
const app = crearServidor({
  directorio,
  almacen,
  administradoresGlobales: [{ proveedor: 'dev', sujetoId: 'admin' }],
  identidad: { proveedores: crearRegistroProveedores(proveedoresDeProceso(identidad.repositorio)), repositorio: identidad.repositorio },
  alCerrar: () => identidad.esperarEscrituras(),
});
await app.ready();

const sesionDe = async (autorizacion: string) =>
  `sesion ${(await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: autorizacion } })).json().sesionId}`;
const comando = (auth: string, tipo: string, params: unknown) =>
  app.inject({ method: 'POST', url: `/v1/jugador/partidas/${gameId}/comandos`, headers: { authorization: auth }, payload: { tipo, params } });

// 1. La cuenta, la partida y el héroe, por la misma vía que un jugador de verdad.
const registro = await app.inject({ method: 'POST', url: '/v1/registro', payload: { nick, clave } });
if (registro.statusCode !== 201 && registro.statusCode !== 409) throw new Error(`registro: ${registro.statusCode} ${registro.body}`);
const admin = await sesionDe('dev admin');
await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: { authorization: admin }, payload: { gameId, seed: 42 } });
const auth = await sesionDe(`clave ${nick}:${clave}`);
await app.inject({ method: 'POST', url: `/v1/jugador/partidas/${gameId}/membresia`, headers: { authorization: auth } });
const creado = await comando(auth, 'crearHeroe', {
  displayName: nick,
  campamentoId: 'mercenarios-0',
  classDefinitionId: 'Spear',
  genero: 'femenino',
  avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' },
});
const heroeId = creado.json().resultado.datos.heroeId as string;
await comando(auth, 'salirDelCampamento', { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: {} });
const faccion = await comando(auth, 'crearFaccion', { nombre: 'Reino de pruebas' });
const faccionId = faccion.json().resultado.datos.faccionId as string;
await app.close();

// 2. Las plazas, sobre la partida guardada: fundar a pie ya no existe en el juego (ver `fundarDePrueba`), así que se funda con la fixture.
const { sesion } = (await cargarPartida(almacen, gameId))!;
let s = enPie(sesion, heroeId, { x: 400, y: 400 });
const primera = s.ejecutar(fundarAsentamiento, { faccionId }, { actor: heroeId });
if (!primera.ok) throw new Error(`fundar la primera plaza: ${primera.codigoError}`);
const plazaA = primera.datos!.asentamientoId;
// Obras en curso en la primera plaza: la auto-construcción las encola sola, así que basta con dejar correr el mundo hasta que haya algunas.
const obras: string[] = [];
for (let i = 0; i < 400 && obras.length < 2; i++) {
  s.avanzarTick();
  obras.splice(0, obras.length, ...s.getState().asentamientos.find((a) => a.id === plazaA)!.edificios.filter((e) => e.estado !== 'activo').map((e) => e.tipo));
}
// La segunda plaza propia: la Facción de nivel 1 no puede fundar otra, así que se le sube el nivel para el montaje.
const payload = s.exportar();
payload.state.facciones = payload.state.facciones.map((f) => (f.id === faccionId ? { ...f, nivel: 3 } : f));
s = (await import('../src/session/gameSession')).GameSession.importar(payload);
// Para fundar la segunda hay que estar fuera: se sale al mundo desde la primera y se anda hasta el sitio.
const salida = s.ejecutar(salirAlMundo, { asentamientoId: plazaA, heroeId, escuadronIds: [], carga: {} }, { actor: heroeId });
if (!salida.ok) throw new Error(`salir al mundo: ${salida.codigoError}`);
s = enPie(s, heroeId, { x: 520, y: 400 });
const segunda = s.ejecutar(fundarAsentamiento, { faccionId }, { actor: heroeId });
const plazaB = segunda.ok ? segunda.datos!.asentamientoId : undefined;

// 3. El héroe, dentro de la primera plaza.
const dentro = s.exportar();
dentro.state.heroes = dentro.state.heroes.map((h) => (h.id === heroeId ? { ...h, ubicacion: { tipo: 'asentamiento' as const, asentamientoId: plazaA } } : h));
dentro.state.ejercitos = dentro.state.ejercitos.filter((e) => !e.participantes.some((p) => p.heroeId === heroeId));
const final = (await import('../src/session/gameSession')).GameSession.importar(dentro);
await guardarPartida(almacen, final, new Date().toISOString(), { forzar: true });
await identidad.esperarEscrituras();

const estado = final.getState();
const enCurso = estado.asentamientos.find((a) => a.id === plazaA)!.edificios.filter((e) => e.estado !== 'activo');
console.log(`partida '${gameId}' en ${directorio}`);
console.log(`  usuario: nick '${nick}', clave '${clave}'  →  Authorization: clave ${nick}:${clave}   (héroe ${heroeId}, Facción ${faccionId})`);
console.log(`  plaza A ${plazaA}: héroe DENTRO, ${enCurso.length} edificios sin acabar (${enCurso.map((e) => `${e.tipo}:${e.estado}`).join(', ')}); obras pedidas: ${obras.join(', ') || 'ninguna'}`);
console.log(`  plaza B ${plazaB ?? '(no se pudo fundar: ' + (segunda.ok ? '' : segunda.codigoError) + ')'}`);
console.log(`  arranque: DIRECTORIO_PARTIDAS=${directorio} ADMINISTRADORES=dev:admin PORT=3200 INTERVALO_TICK_MS=4000 npm run server`);
console.log(`  luego: POST /v1/admin/partidas { "gameId": "${gameId}", "seed": 42 } con la sesión de 'dev admin'`);
