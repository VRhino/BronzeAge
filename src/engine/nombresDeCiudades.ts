// Nombres para los asentamientos que se fundan: ciudades reales de Grecia y el Egeo, Chipre, Anatolia, Mesopotamia, el Levante y Egipto, de la
// Edad del Bronce hasta Alejandro Magno. Cada partida va sacando de aquí sin repetir (`nombreDeCiudadLibre`); cuando se agotan, el asentamiento
// se queda con su id, como antes. Es solo presentación: ningún nombre cambia lo que hace una plaza.

export const NOMBRES_DE_CIUDADES: readonly string[] = [
  // Grecia y el Egeo
  'Micenas', 'Tirinto', 'Pilos', 'Orcómeno', 'Gla', 'Argos', 'Esparta', 'Atenas', 'Corinto', 'Eleusis',
  'Lerna', 'Asine', 'Midea', 'Yolcos', 'Dimini', 'Lefkandi', 'Calcis', 'Eretria', 'Delfos', 'Olimpia',
  'Mégara', 'Egina', 'Naxos', 'Delos', 'Akrotiri', 'Filakopi', 'Platea', 'Queronea', 'Pela', 'Olinto',
  'Potidea', 'Dodona', 'Cnosos', 'Festos', 'Malia', 'Zakros', 'Gurnia', 'Cidonia',
  // Chipre
  'Enkomi', 'Kition', 'Salamina', 'Pafos',
  // Anatolia
  'Troya', 'Hattusa', 'Alacahöyük', 'Kanesh', 'Purushanda', 'Tarso', 'Mileto', 'Éfeso', 'Colofón', 'Focea',
  'Esmirna', 'Halicarnaso', 'Cnido', 'Sardes', 'Gordio', 'Pérgamo', 'Sinope', 'Lámpsaco', 'Cícico', 'Aspendos',
  'Side', 'Janto', 'Pátara', 'Telmeso', 'Milasa', 'Ancira', 'Pesinunte',
  // Mesopotamia
  'Ur', 'Uruk', 'Lagash', 'Girsu', 'Umma', 'Nippur', 'Kish', 'Eridú', 'Larsa', 'Isin',
  'Babilonia', 'Borsippa', 'Sippar', 'Acad', 'Assur', 'Nínive', 'Calah', 'Mari', 'Terqa', 'Eshnunna',
  'Nagar', 'Harrán', 'Susa', 'Dur-Kurigalzu', 'Opis', 'Shuruppak', 'Adab', 'Carquemish',
  // Levante
  'Ugarit', 'Biblos', 'Tiro', 'Sidón', 'Arvad', 'Sarepta', 'Beirut', 'Alalakh', 'Ebla', 'Qatna',
  'Cades', 'Damasco', 'Hamat', 'Alepo', 'Hazor', 'Meguido', 'Jericó', 'Jerusalén', 'Siquem', 'Laquis',
  'Gezer', 'Ascalón', 'Gaza', 'Asdod', 'Jaffa', 'Samaria', 'Betseán', 'Dor', 'Ecrón', 'Aco', 'Tadmor',
  // Egipto y Nubia
  'Menfis', 'Tebas', 'Heliópolis', 'Hermópolis', 'Abidos', 'Ajetatón', 'Pi-Ramsés', 'Avaris', 'Tanis', 'Bubastis',
  'Sais', 'Mendes', 'Busiris', 'Buto', 'Elefantina', 'Edfu', 'Nejeb', 'Hieracómpolis', 'Coptos', 'Licópolis',
  'Heracleópolis', 'Crocodilópolis', 'Naucratis', 'Pelusio', 'Dendera', 'Esna', 'Kom Ombo', 'Giza', 'Buhen', 'Kerma', 'Napata',
];

const normalizar = (nombre: string): string => nombre.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

/** Hash FNV-1a de 32 bits: solo hace falta un reparto estable, no criptografía. */
function hash(texto: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < texto.length; i++) {
    h ^= texto.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/**
 * Un nombre de la lista que `usados` (los asentamientos de la partida) no lleve ya, elegido a partir de `semilla` (el id del asentamiento nuevo):
 * el mismo id sobre las mismas plazas da el mismo nombre, sin gastar el azar de la partida. `undefined` si ya se han usado todos.
 */
export function nombreDeCiudadLibre(usados: Iterable<string | undefined>, semilla: string): string | undefined {
  const ocupados = new Set<string>();
  for (const nombre of usados) if (nombre) ocupados.add(normalizar(nombre));
  const libres = NOMBRES_DE_CIUDADES.filter((n) => !ocupados.has(normalizar(n)));
  return libres.length === 0 ? undefined : libres[hash(semilla) % libres.length];
}
