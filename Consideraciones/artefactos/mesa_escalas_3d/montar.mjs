// Monta la mesa de escalas 3D: mete las ventanas de datos en plantilla.html y escribe la página entera.
// `datos-alpina.json` es una ventana de la región alpina (Tierras altas secas) del generador y `datos-costa.json`, una costa
// del generador (la península del oeste). Las ventanas «Antes» y «Generador nuevo» se quitaron el 2026-10-31 (ya revisadas).
// Uso: node montar.mjs [salida.html]   (por defecto, mesa-escalas-3d.html al lado; está en .gitignore)
import { readFileSync, writeFileSync } from 'node:fs';

const aqui = new URL('.', import.meta.url);
const leer = nombre => readFileSync(new URL(nombre, aqui), 'utf8');
const plantilla = leer('plantilla.html');
for (const marca of ['__DATOS_ALPINA__', '__DATOS_COSTA__']) {
  if (plantilla.split(marca).length !== 2) throw new Error(`plantilla.html tiene que llevar un único ${marca}`);
}
const salida = process.argv[2] ?? new URL('mesa-escalas-3d.html', aqui);
writeFileSync(salida, plantilla.replace('__DATOS_COSTA__', () => leer('datos-costa.json')).replace('__DATOS_ALPINA__', () => leer('datos-alpina.json')));
console.log(`escrita ${salida}`);
