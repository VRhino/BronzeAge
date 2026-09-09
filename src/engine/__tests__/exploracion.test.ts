// La rejilla de exploración (niebla de guerra, Paso 2). Es la pieza más "opaca" de la mecánica —un bitmap en
// hexadecimal— así que lo que aquí se congela es sobre todo que la opacidad no esconda errores: que lo
// marcado se lea marcado, que lo de al lado no, y que el formato aguante el ida y vuelta.
import { describe, expect, it } from 'vitest';
import { EXPLORACION, VISION } from '../../constants';
import { celdasExploradas, estaExplorado, marcarVisto, rejillaDe, SIN_EXPLORAR } from '../exploracion';

const MUNDO = { ancho: 2000, alto: 2000 };
const REJILLA = rejillaDe(MUNDO);

describe('rejillaDe', () => {
  it('trocea el mundo entero, sin dejar borde fuera', () => {
    expect(REJILLA.tamanoCelda).toBe(EXPLORACION.tamanoCelda);
    expect(REJILLA.columnas * REJILLA.tamanoCelda).toBeGreaterThanOrEqual(MUNDO.ancho);
    expect(REJILLA.filas * REJILLA.tamanoCelda).toBeGreaterThanOrEqual(MUNDO.alto);
  });

  it('un mundo que no es multiplo de la celda redondea HACIA ARRIBA: ninguna esquina se queda sin celda', () => {
    // 2010/25 = 80,4 y 999/25 = 39,96: si se redondease hacia abajo, la ultima franja de mapa no tendria
    // celda donde marcarse y mirarla no dejaria rastro.
    const rara = rejillaDe({ ancho: 2010, alto: 999 });
    expect(rara.columnas).toBe(81);
    expect(rara.filas).toBe(40);
    expect(estaExplorado(marcarVisto(SIN_EXPLORAR, rara, { x: 2009, y: 998 }, 30), rara, { x: 2009, y: 998 })).toBe(true);
  });
});

describe('marcarVisto / estaExplorado', () => {
  it('sin explorar, no hay nada explorado', () => {
    expect(estaExplorado(SIN_EXPLORAR, REJILLA, { x: 1000, y: 1000 })).toBe(false);
    expect(celdasExploradas(SIN_EXPLORAR)).toBe(0);
  });

  it('lo que cae bajo el ojo queda marcado, y lo que queda lejos no', () => {
    const tras = marcarVisto(SIN_EXPLORAR, REJILLA, { x: 1000, y: 1000 }, VISION.ejercito);

    expect(estaExplorado(tras, REJILLA, { x: 1000, y: 1000 })).toBe(true);
    expect(estaExplorado(tras, REJILLA, { x: 1000 + VISION.ejercito - 30, y: 1000 })).toBe(true);
    // Mas de una celda fuera del radio: el margen evita depender de por donde caiga la retícula.
    expect(estaExplorado(tras, REJILLA, { x: 1000 + VISION.ejercito + 60, y: 1000 })).toBe(false);
    expect(estaExplorado(tras, REJILLA, { x: 100, y: 100 })).toBe(false);
  });

  it('el area marcada se parece a un circulo, no a su cuadrado envolvente', () => {
    const radio = 150;
    const tras = marcarVisto(SIN_EXPLORAR, REJILLA, { x: 1000, y: 1000 }, radio);
    const celdas = celdasExploradas(tras);
    const teoricas = (Math.PI * radio * radio) / (REJILLA.tamanoCelda * REJILLA.tamanoCelda);

    expect(celdas).toBeGreaterThan(teoricas * 0.9);
    expect(celdas).toBeLessThan(teoricas * 1.1);
    // La esquina del cuadrado envolvente queda FUERA, que es la diferencia entre un circulo y un cuadrado.
    expect(estaExplorado(tras, REJILLA, { x: 1000 + radio - 5, y: 1000 + radio - 5 })).toBe(false);
  });

  it('explorar es acumulativo: lo de antes no se pierde al mirar a otro sitio', () => {
    const uno = marcarVisto(SIN_EXPLORAR, REJILLA, { x: 200, y: 200 }, 100);
    const dos = marcarVisto(uno, REJILLA, { x: 1800, y: 1800 }, 100);

    expect(estaExplorado(dos, REJILLA, { x: 200, y: 200 })).toBe(true);
    expect(estaExplorado(dos, REJILLA, { x: 1800, y: 1800 })).toBe(true);
    expect(celdasExploradas(dos)).toBe(celdasExploradas(uno) * 2);
  });

  it('volver a mirar lo mismo devuelve la MISMA cadena: un tick sin novedad no ensucia el estado', () => {
    const uno = marcarVisto(SIN_EXPLORAR, REJILLA, { x: 1000, y: 1000 }, 150);
    expect(marcarVisto(uno, REJILLA, { x: 1000, y: 1000 }, 150)).toBe(uno);
    // Y mirar desde dentro de lo ya explorado, con menos alcance, tampoco cambia nada.
    expect(marcarVisto(uno, REJILLA, { x: 1010, y: 1010 }, 50)).toBe(uno);
  });

  it('el ojo marca SIEMPRE la celda que pisa, aunque su radio no llegue al centro de esa celda', () => {
    // La regla general es "celda vista = celda cuyo centro entra en el circulo". Sola, dejaria a un ojo
    // diminuto pegado al borde de su celda sin marcar ni el suelo que pisa.
    const rincon = { x: 501, y: 501 }; // celda 20,20 -> su centro cae en (512.5, 512.5), a ~16 de distancia
    const tras = marcarVisto(SIN_EXPLORAR, REJILLA, rincon, 1);

    expect(estaExplorado(tras, REJILLA, rincon)).toBe(true);
    expect(celdasExploradas(tras)).toBe(1);
  });

  it('un ojo pegado al borde del mundo no se sale ni revienta', () => {
    const esquina = marcarVisto(SIN_EXPLORAR, REJILLA, { x: 5, y: 5 }, 200);
    expect(estaExplorado(esquina, REJILLA, { x: 5, y: 5 })).toBe(true);
    // Fuera del mapa nunca esta explorado, aunque el circulo lo cubriese.
    expect(estaExplorado(esquina, REJILLA, { x: -50, y: 5 })).toBe(false);
    expect(estaExplorado(esquina, REJILLA, { x: 5, y: -50 })).toBe(false);
    expect(estaExplorado(esquina, REJILLA, { x: 2500, y: 2500 })).toBe(false);
  });

  it('el formato aguanta el ida y vuelta por JSON, que es como viaja de verdad', () => {
    const original = marcarVisto(marcarVisto(SIN_EXPLORAR, REJILLA, { x: 333, y: 777 }, 90), REJILLA, { x: 1500, y: 250 }, 120);
    const tras = JSON.parse(JSON.stringify({ e: original })).e as string;

    expect(tras).toBe(original);
    expect(estaExplorado(tras, REJILLA, { x: 333, y: 777 })).toBe(true);
    expect(estaExplorado(tras, REJILLA, { x: 1500, y: 250 })).toBe(true);
    expect(celdasExploradas(tras)).toBe(celdasExploradas(original));
  });

  it('cuesta menos de 1 KB tener el mundo entero explorado', () => {
    // El argumento entero de guardarlo como bitmap es este: el tope no depende de cuanto se juegue.
    const todo = marcarVisto(SIN_EXPLORAR, REJILLA, { x: 1000, y: 1000 }, 4000);
    expect(celdasExploradas(todo)).toBe(REJILLA.columnas * REJILLA.filas);
    expect(todo.length / 2).toBeLessThan(1024);
  });
});
