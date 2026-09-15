// Punto de arranque del proceso backend (Fastify + RunnerDePartida). Ejecutable con `npm run server`.
//
// Aquí, y solo aquí, se decide la configuración del despliegue: puerto, dónde viven las partidas y QUIÉN
// administra la instancia. `crearServidor()` no trae ningún administrador por defecto — si esta variable no
// se declara, nadie puede crear partidas. Es a propósito: un default que concede administración es de los
// que sobreviven hasta producción sin que nadie los vea.
//
// `ADMINISTRADORES` es una lista `proveedor:sujetoId` separada por comas, ej. `dev:jefa,oauth:1234`.
//
// `CODIGO_REGISTRO` protege el alta de cuentas locales (`POST /v1/registro`). Sin declararlo, el registro
// queda abierto. Con él, el cliente debe mandarlo en el cuerpo del alta.
//
// `ORIGENES_PERMITIDOS` (Fase C6, CORS) es una lista de orígenes separada por comas, ej.
// `https://jugador.ejemplo.com,https://admin.ejemplo.com`. Vacía por defecto: sin ella, ningún origen
// cruzado puede llamar a esta API — mismo criterio que `ADMINISTRADORES`.
//
// `INTERVALO_TICK_MS` (Fase C12) arranca el RELOJ DE MUNDO de cada partida que se abra (D5,
// `RunnerDePartida.iniciarRelojDeMundo`): un tick por cada tanto de reloj de pared, con catch-up en ráfaga de
// los ticks vencidos tras un reinicio. Sin declarar, `undefined` — ninguna partida avanza sola (ver el
// comentario de `RegistroDePartidas`). Con "mundo = tiempo real" (doc 10 §2) el valor es 60000 (=
// `SIMULACION.duracionTickMs`): un minuto real por tick.
//
// `SERVIDORES_BATALLA` (doc 02 §3.3) declara los servidores de batalla de Conquest que pueden hablar con esta
// instancia, como `servidorId:token` separados por comas. Es también el interruptor: sin ella, ninguna batalla llega a
// Unity y todas se resuelven con números, como antes de existir el ciclo de batalla.
//
// `ALMACEN_URL` elige el backend de persistencia (`server/almacen/`): sin declararlo, disco local bajo
// `DIRECTORIO_PARTIDAS`; con una URL de libSQL (`libsql://…`, `file:…`) usa esa base — es lo que hace falta
// para desplegar donde el disco es efímero. `ALMACEN_TOKEN` es el authToken si el proveedor lo pide (Turso).
import { crearServidor } from './api';
import { crearRegistroProveedores } from '../acceso/proveedorIdentidad';
import type { AlmacenDeObjetos } from './almacen/almacenDeObjetos';
import { crearAlmacenEnDisco } from './almacen/enDisco';
import { crearAlmacenEnLibsql } from './almacen/enLibsql';
import { parsearAdministradores } from './identidad/administradoresGlobales';
import { parsearServidoresDeBatalla } from './identidad/servidoresDeBatalla';
import { proveedoresDeProceso } from './identidad/proveedoresActivos';
import { crearRepositorioIdentidadPersistente } from './identidad/repositorioPersistente';

// `PORT` primero: es lo que inyecta la mayoría de PaaS. `PUERTO` se mantiene para el uso local ya existente.
const PUERTO = Number(process.env.PORT ?? process.env.PUERTO ?? 3000);
const ALMACEN_URL = process.env.ALMACEN_URL?.trim() || undefined;
const DIRECTORIO_PARTIDAS = process.env.DIRECTORIO_PARTIDAS ?? './partidas';
const ADMINISTRADORES = parsearAdministradores(process.env.ADMINISTRADORES);
// Código de invitación para `POST /v1/registro` (alta de cuenta local con contraseña). Sin declararlo, el
// registro queda ABIERTO — mismo criterio de "opt-in explícito" que el resto: para un playtest privado va,
// para un servidor público conviene ponerlo y pasárselo a los jugadores.
const CODIGO_REGISTRO = process.env.CODIGO_REGISTRO?.trim() || undefined;
const ORIGENES_PERMITIDOS = (process.env.ORIGENES_PERMITIDOS ?? '')
  .split(',')
  .map((o) => o.trim())
  .filter((o) => o !== '');
const INTERVALO_TICK_MS = process.env.INTERVALO_TICK_MS ? Number(process.env.INTERVALO_TICK_MS) : undefined;
const SERVIDORES_BATALLA = parsearServidoresDeBatalla(process.env.SERVIDORES_BATALLA);
// Mantenimiento (Fase E2): respaldos y poda automaticos. Opt-in, como los ticks y los administradores: sin
// `MANTENIMIENTO_INTERVALO_MS` no se respalda ni se borra nada por su cuenta. Los otros dos solo tienen
// efecto si ese esta puesto, asi que no hace falta un flag aparte para encenderlo.
const MANTENIMIENTO = process.env.MANTENIMIENTO_INTERVALO_MS
  ? {
      intervaloMs: Number(process.env.MANTENIMIENTO_INTERVALO_MS),
      respaldosAConservar: process.env.RESPALDOS_A_CONSERVAR ? Number(process.env.RESPALDOS_A_CONSERVAR) : undefined,
      retencionAuditoriaDias: process.env.RETENCION_AUDITORIA_DIAS ? Number(process.env.RETENCION_AUDITORIA_DIAS) : undefined,
    }
  : undefined;

async function arrancar(): Promise<void> {
  // Un solo almacén para TODA la persistencia (snapshots, eventos, auditoría, identidad). Disco si no hay
  // `ALMACEN_URL`; si la hay, libSQL (Turso u otro) — para desplegar donde el disco no persiste.
  const almacen: AlmacenDeObjetos = ALMACEN_URL
    ? await crearAlmacenEnLibsql({ url: ALMACEN_URL, authToken: process.env.ALMACEN_TOKEN })
    : crearAlmacenEnDisco(DIRECTORIO_PARTIDAS);

  // El dominio de acceso (usuarios, sesiones, membresías, credenciales) se persiste junto a las partidas: sin
  // esto, un reinicio del proceso deja a todos sin sesión y sin membresía (cierre de Fase C).
  const identidad = await crearRepositorioIdentidadPersistente(almacen);

  const app = crearServidor({
    directorio: DIRECTORIO_PARTIDAS,
    almacen,
    administradoresGlobales: ADMINISTRADORES,
    origenesPermitidos: ORIGENES_PERMITIDOS,
    intervaloTickMs: INTERVALO_TICK_MS,
    mantenimiento: MANTENIMIENTO,
    identidad: {
      proveedores: crearRegistroProveedores(proveedoresDeProceso(identidad.repositorio)),
      repositorio: identidad.repositorio,
    },
    codigoRegistro: CODIGO_REGISTRO,
    servidoresBatalla: SERVIDORES_BATALLA,
    // Cerrar el servidor drena las escrituras de identidad pendientes (ver `alCerrar` en `api.ts`).
    alCerrar: () => identidad.esperarEscrituras(),
  });

  // Apagado limpio: `app.close()` ya drena las escrituras de identidad por el gancho `alCerrar`, así que
  // aquí no hace falta ordenarlo a mano — que era justo el detalle fácil de olvidar.
  for (const senal of ['SIGINT', 'SIGTERM'] as const) {
    process.once(senal, () => {
      void app.close().finally(() => process.exit(0));
    });
  }

  await app.listen({ port: PUERTO, host: '0.0.0.0' });
  console.log(`servidor escuchando en :${PUERTO}`);
  if (ALMACEN_URL) {
    console.log(`persistencia: libSQL (${ALMACEN_URL.replace(/\/\/[^@/]*@/, '//***@')})`);
  } else {
    console.log(`persistencia: disco local en '${DIRECTORIO_PARTIDAS}'`);
    console.warn('AVISO: sin ALMACEN_URL — la persistencia es de disco local. En un host con disco efimero se pierde en cada reinicio.');
  }
  if (ADMINISTRADORES.length === 0) {
    console.warn('AVISO: sin ADMINISTRADORES configurados — nadie puede crear partidas.');
    console.warn("       ej: ADMINISTRADORES='dev:jefa' npm run server");
  } else {
    console.log(`administradores: ${ADMINISTRADORES.map((a) => `${a.proveedor}:${a.sujetoId}`).join(', ')}`);
  }
  if (MANTENIMIENTO === undefined) {
    console.warn('AVISO: sin MANTENIMIENTO_INTERVALO_MS — no se hacen respaldos automaticos ni se poda la auditoria.');
    console.warn("       ej: MANTENIMIENTO_INTERVALO_MS=3600000 npm run server");
  } else {
    const { intervaloMs, respaldosAConservar, retencionAuditoriaDias } = MANTENIMIENTO;
    console.log(`mantenimiento cada ${intervaloMs} ms — respaldos: ${respaldosAConservar ?? 7}, auditoria: ${retencionAuditoriaDias ?? 30} dias`);
  }
  if (CODIGO_REGISTRO === undefined) {
    console.warn('AVISO: sin CODIGO_REGISTRO — POST /v1/registro esta abierto, cualquiera puede crear una cuenta.');
  }
  if (ORIGENES_PERMITIDOS.length === 0) {
    console.warn('AVISO: sin ORIGENES_PERMITIDOS configurados — CORS desactivado, ningún origen cruzado puede llamar a esta API.');
  } else {
    console.log(`origenes CORS permitidos: ${ORIGENES_PERMITIDOS.join(', ')}`);
  }
  if (SERVIDORES_BATALLA.length === 0) {
    console.log('batallas: sin SERVIDORES_BATALLA — todas se resuelven con números, sin Unity.');
  } else {
    console.log(`batallas en Unity: servidores ${SERVIDORES_BATALLA.map((s) => s.id).join(', ')}`);
  }
  if (INTERVALO_TICK_MS === undefined) {
    console.warn('AVISO: sin INTERVALO_TICK_MS configurado — ninguna partida avanza sola, solo con POST .../tick a mano.');
  } else {
    console.log(`reloj de mundo: un tick cada ${INTERVALO_TICK_MS} ms reales, con catch-up en ráfaga tras un reinicio.`);
  }
}

arrancar().catch((err: unknown) => {
  console.error(err);
  process.exitCode = 1;
});
