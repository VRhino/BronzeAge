// Registro de "valores de balance": todos los números arbitrarios/placeholder de `constants.ts`
// (documentados allí mismo y en Consideraciones/Preguntas_Abiertas.md), expuestos para editarse
// en caliente desde la interfaz sin tocar la lógica del motor.
//
// Cómo funciona: los objetos de `constants.ts` son los MISMOS objetos que importa cada archivo de
// `engine/*` — y ese código siempre los lee por propiedad dentro de funciones (`EDIFICIO_CATALOGO[t].costo`,
// `MILITAR.racionPorSoldadoPorTick`, etc.), nunca los desestructura a nivel de módulo. Por eso basta con
// mutar la propiedad en el objeto real (no una copia) para que el motor vea el nuevo valor en su
// próxima lectura — no hace falta tocar ninguna firma de función del motor.
// `as const` en constants.ts es solo un candado de TIPOS (para que el motor no pueda escribir por error);
// este módulo es la única puerta explícita que sí puede escribir, vía casteos a `any` acotados a este archivo.
import {
  ALMACEN,
  ASCENSO_TROPA,
  BOSQUE,
  CAP_FUNDACION_POR_NIVEL,
  CIUDADANIA,
  COMISION,
  EDIFICIO_CATALOGO,
  FERTILIDAD,
  FUNDACION,
  LIVESTOCK,
  MANTENIMIENTO,
  MILITAR,
  NECESIDADES,
  NIVEL_ASENTAMIENTO,
  NIVEL_FACCION,
  POBLACION,
  POLITICAS,
  POLITICA_CATALOGO,
  PRECIO_BASE,
  PRECIO_REFERENCIA,
  RECLUTAMIENTO,
  RECURSO_CANTIDAD_NODO,
  RECURSO_RAREZA,
  REPUTACION,
  RESERVA_CONSTRUCCION,
  SITIO,
  TROPA_CATALOGO,
  TRUEQUE,
  WORLD_DEFAULT,
  ZONA_INFLUENCIA,
} from '../constants';

export interface CampoBalance {
  path: string;
  grupo: string;
  etiqueta: string;
  valor: number;
  defecto: number;
}

interface CampoInterno {
  path: string;
  grupo: string;
  etiqueta: string;
  defecto: number;
  get: () => number;
  set: (v: number) => void;
}

/**
 * Grupos en el orden en que se documentan en constants.ts: [etiqueta legible para la UI, identificador
 * real de la constante (única por construcción — son exports de un mismo módulo), objeto].
 * El identificador real se usa como prefijo del `path` para que nunca colisione entre grupos aunque
 * compartan nombres de clave interna (p. ej. NIVEL_FACCION y NIVEL_ASENTAMIENTO comparten "nivelMaximo").
 */
const GRUPOS: Array<[string, string, unknown]> = [
  ['Mundo', 'WORLD_DEFAULT', WORLD_DEFAULT],
  ['Recursos — rareza', 'RECURSO_RAREZA', RECURSO_RAREZA],
  ['Recursos — cantidad por nodo', 'RECURSO_CANTIDAD_NODO', RECURSO_CANTIDAD_NODO],
  ['Livestock', 'LIVESTOCK', LIVESTOCK],
  ['Bosques', 'BOSQUE', BOSQUE],
  ['Fertilidad', 'FERTILIDAD', FERTILIDAD],
  ['Zona de influencia', 'ZONA_INFLUENCIA', ZONA_INFLUENCIA],
  ['Fundación', 'FUNDACION', FUNDACION],
  ['Población', 'POBLACION', POBLACION],
  ['Edificios', 'EDIFICIO_CATALOGO', EDIFICIO_CATALOGO],
  ['Almacén', 'ALMACEN', ALMACEN],
  ['Necesidades (auto-construcción)', 'NECESIDADES', NECESIDADES],
  ['Colocación de edificios', 'SITIO', SITIO],
  ['Trueque', 'TRUEQUE', TRUEQUE],
  ['Precio base', 'PRECIO_BASE', PRECIO_BASE],
  ['Precio de referencia', 'PRECIO_REFERENCIA', PRECIO_REFERENCIA],
  ['Comisión de comercio', 'COMISION', COMISION],
  ['Nivel de Facción', 'NIVEL_FACCION', NIVEL_FACCION],
  ['Cap de fundación por nivel', 'CAP_FUNDACION_POR_NIVEL', CAP_FUNDACION_POR_NIVEL],
  ['Ciudadanía', 'CIUDADANIA', CIUDADANIA],
  ['Políticas — slots', 'POLITICAS', POLITICAS],
  ['Catálogo de políticas', 'POLITICA_CATALOGO', POLITICA_CATALOGO],
  ['Catálogo de tropas', 'TROPA_CATALOGO', TROPA_CATALOGO],
  ['Reclutamiento', 'RECLUTAMIENTO', RECLUTAMIENTO],
  ['Ascenso de tropa', 'ASCENSO_TROPA', ASCENSO_TROPA],
  ['Militar', 'MILITAR', MILITAR],
  ['Nivel de asentamiento', 'NIVEL_ASENTAMIENTO', NIVEL_ASENTAMIENTO],
  ['Mantenimiento', 'MANTENIMIENTO', MANTENIMIENTO],
  ['Reserva mínima de construcción', 'RESERVA_CONSTRUCCION', RESERVA_CONSTRUCCION],
  ['Reputación', 'REPUTACION', REPUTACION],
];

/**
 * Rutas excluidas a propósito: son "claves foráneas" hacia otra tabla (el tier con el que nace una
 * tropa, usado como índice directo en TROPA_CATALOGO), no un número de balance libre — ponerles un
 * valor que no exista en TROPA_CATALOGO rompería el reclutamiento en tiempo de ejecución.
 */
const RUTAS_EXCLUIDAS = new Set([
  'RECLUTAMIENTO.pesants.tierInicial',
  'RECLUTAMIENTO.artesanos.tierInicial',
  'RECLUTAMIENTO.nobleza.tierInicial',
]);

const campos: CampoInterno[] = [];

function recorrer(grupo: string, path: string, etiqueta: string, padre: any, clave: string | number): void {
  if (RUTAS_EXCLUIDAS.has(path)) return;
  const valor = padre[clave];
  if (typeof valor === 'number') {
    campos.push({
      path,
      grupo,
      etiqueta,
      defecto: valor,
      get: () => padre[clave],
      set: (v: number) => {
        padre[clave] = v;
      },
    });
    return;
  }
  if (Array.isArray(valor)) {
    valor.forEach((_, i) => recorrer(grupo, `${path}[${i}]`, `${etiqueta}[${i}]`, valor, i));
    return;
  }
  if (valor && typeof valor === 'object') {
    for (const clave2 of Object.keys(valor)) {
      recorrer(grupo, `${path}.${clave2}`, `${etiqueta} › ${clave2}`, valor, clave2);
    }
  }
}

for (const [grupo, prefijo, obj] of GRUPOS) {
  if (obj && typeof obj === 'object') {
    for (const clave of Object.keys(obj)) {
      recorrer(grupo, `${prefijo}.${clave}`, clave, obj, clave);
    }
  }
}

/** Todos los campos de balance editables, con su valor actual (en vivo) y su valor original de fábrica. */
export function listarCamposBalance(): CampoBalance[] {
  return campos.map((c) => ({ path: c.path, grupo: c.grupo, etiqueta: c.etiqueta, valor: c.get(), defecto: c.defecto }));
}

/** Aplica un nuevo valor al campo `path`. Devuelve `false` si el path no existe o el valor no es numérico. */
export function actualizarCampoBalance(path: string, valor: number): boolean {
  const campo = campos.find((c) => c.path === path);
  if (!campo || Number.isNaN(valor)) return false;
  campo.set(valor);
  return true;
}

/** Restaura todos los campos a su valor original de fábrica (el que tenían al cargar la página). */
export function restaurarBalancePorDefecto(): void {
  for (const c of campos) c.set(c.defecto);
}
