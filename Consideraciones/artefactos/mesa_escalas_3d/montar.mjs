// Monta la mesa de escalas 3D: mete las dos ventanas de datos en plantilla.html y escribe la página entera.
// `datos.json` es el generador de antes (relieve regional liso) y `datos-nuevo.json`, el del relieve a escala de pantalla.
// Uso: node montar.mjs [salida.html]   (por defecto, mesa-escalas-3d.html al lado; está en .gitignore)
import { readFileSync, writeFileSync } from 'node:fs';

const aqui = new URL('.', import.meta.url);
const leer = nombre => readFileSync(new URL(nombre, aqui), 'utf8');
const plantilla = leer('plantilla.html');
for (const marca of ['__DATOS__', '__DATOS_NUEVO__']) {
  if (plantilla.split(marca).length !== 2) throw new Error(`plantilla.html tiene que llevar un único ${marca}`);
}
const salida = process.argv[2] ?? new URL('mesa-escalas-3d.html', aqui);
writeFileSync(salida, plantilla.replace('__DATOS_NUEVO__', () => leer('datos-nuevo.json')).replace('__DATOS__', () => leer('datos.json')));
console.log(`escrita ${salida}`);
