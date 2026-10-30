// Monta la mesa de escalas 3D: mete datos.json en plantilla.html y escribe la página entera.
// Uso: node montar.mjs [salida.html]   (por defecto, mesa-escalas-3d.html al lado; está en .gitignore)
import { readFileSync, writeFileSync } from 'node:fs';

const aqui = new URL('.', import.meta.url);
const plantilla = readFileSync(new URL('plantilla.html', aqui), 'utf8');
const datos = readFileSync(new URL('datos.json', aqui), 'utf8');
if (plantilla.split('__DATOS__').length !== 2) throw new Error('plantilla.html tiene que llevar un único __DATOS__');
const salida = process.argv[2] ?? new URL('mesa-escalas-3d.html', aqui);
writeFileSync(salida, plantilla.replace('__DATOS__', () => datos));
console.log(`escrita ${salida}`);
