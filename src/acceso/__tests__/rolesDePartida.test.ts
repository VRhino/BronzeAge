// Política de superficies: quién administra y quién juega. Negocio puro — sin HTTP, sin Fastify, sin
// adaptadores. Si estas reglas cambian, es por una decisión de diseño, no por un cambio de framework.
import { describe, expect, it } from 'vitest';
import {
  esVigente,
  puedeAdministrar,
  puedeCrearPartida,
  puedeDescartarPartida,
  puedeJugar,
  rolEnPartida,
  type ActorDeInstancia,
} from '../rolesDePartida';
import type { Membresia, RolTecnico } from '../tipos';

const AHORA = '2026-01-01T00:00:00.000Z';

function membresia(rol: RolTecnico, extra: Partial<Membresia> = {}): Membresia {
  return { usuarioId: 'u1', gameId: 'g1', jugadorId: 'u1', rol, desde: '2025-01-01T00:00:00.000Z', ...extra };
}

function actor(rol: RolTecnico | undefined, esAdministradorGlobal = false): ActorDeInstancia {
  return { usuarioId: 'u1', esAdministradorGlobal, membresia: rol ? membresia(rol) : undefined };
}

describe('esVigente', () => {
  it('sin `hasta` la membresia sigue viva', () => {
    expect(esVigente(membresia('jugador'), AHORA)).toBe(true);
  });

  it('con `hasta` en el futuro sigue viva; en el pasado ya no concede nada', () => {
    expect(esVigente(membresia('jugador', { hasta: '2027-01-01T00:00:00.000Z' }), AHORA)).toBe(true);
    expect(esVigente(membresia('jugador', { hasta: '2025-06-01T00:00:00.000Z' }), AHORA)).toBe(false);
  });
});

describe('puedeAdministrar', () => {
  it('lo cumplen administrador_partida, moderador y el administrador global', () => {
    expect(puedeAdministrar(actor('administrador_partida'))).toBe(true);
    expect(puedeAdministrar(actor('moderador'))).toBe(true);
    expect(puedeAdministrar(actor(undefined, true))).toBe(true);
  });

  it('no lo cumplen jugador, observador ni quien no tiene membresia', () => {
    expect(puedeAdministrar(actor('jugador'))).toBe(false);
    expect(puedeAdministrar(actor('observador'))).toBe(false);
    expect(puedeAdministrar(actor(undefined))).toBe(false);
  });
});

describe('puedeJugar', () => {
  it('solo el rol jugador', () => {
    expect(puedeJugar(actor('jugador'))).toBe(true);
    expect(puedeJugar(actor('observador'))).toBe(false);
    expect(puedeJugar(actor(undefined))).toBe(false);
  });

  it('un administrador NO puede jugar por serlo: acceso tecnico no es tener personaje', () => {
    expect(puedeJugar(actor('administrador_partida'))).toBe(false);
    expect(puedeJugar(actor(undefined, true))).toBe(false);
  });

  it('un administrador global que ADEMAS es jugador de esa partida, si puede', () => {
    expect(puedeJugar({ usuarioId: 'u1', esAdministradorGlobal: true, membresia: membresia('jugador') })).toBe(true);
  });
});

describe('puedeCrearPartida', () => {
  it('solo administrador global: en una partida inexistente no hay membresia posible', () => {
    expect(puedeCrearPartida(actor(undefined, true))).toBe(true);
    expect(puedeCrearPartida(actor('administrador_partida'))).toBe(false);
    expect(puedeCrearPartida(actor('jugador'))).toBe(false);
  });
});

describe('puedeDescartarPartida', () => {
  it('administrador_partida y global si; moderador NO (doc 5: sin regeneracion de mundo)', () => {
    expect(puedeDescartarPartida(actor('administrador_partida'))).toBe(true);
    expect(puedeDescartarPartida(actor(undefined, true))).toBe(true);
    expect(puedeDescartarPartida(actor('moderador'))).toBe(false);
    expect(puedeDescartarPartida(actor('jugador'))).toBe(false);
  });
});

describe('rolEnPartida', () => {
  it('la membresia concreta manda sobre ser administrador global (fix C8, 2026-08-26)', () => {
    // Caso real que produjo el bug: quien crea una partida recibe Membresia `administrador_partida`
    // (`otorgarAdministracion`) — antes del fix, esta función ignoraba esa membresia y devolvía
    // `administrador_global`, un rol que ninguna fila de `MATRIZ_AUTORIZACION` admite.
    expect(
      rolEnPartida({ usuarioId: 'u1', esAdministradorGlobal: true, membresia: membresia('administrador_partida') })
    ).toBe('administrador_partida');
    expect(rolEnPartida({ usuarioId: 'u1', esAdministradorGlobal: true, membresia: membresia('jugador') })).toBe('jugador');
  });

  it('sin membresia, cae al administrador global; sin ninguno de los dos, ninguno', () => {
    expect(rolEnPartida(actor(undefined, true))).toBe('administrador_global');
    expect(rolEnPartida(actor(undefined))).toBeUndefined();
  });

  it('sin admin global, el rol es el de la membresia', () => {
    expect(rolEnPartida(actor('moderador'))).toBe('moderador');
  });
});
