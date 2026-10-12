// Validador de los mensajes del WebSocket de tiempo real contra `contratos.schema.json`, compartido por el test de los fixtures y por el del difusor.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';

const AQUI = dirname(fileURLToPath(import.meta.url));
const ajv = new Ajv({ allErrors: true, strict: true, strictRequired: false, allowUnionTypes: true });
ajv.addSchema(JSON.parse(readFileSync(join(AQUI, '..', 'contratos.schema.json'), 'utf8')) as object, 'v1');

export function erroresDeMensaje(definicion: string, dato: unknown) {
  const validar = ajv.getSchema(`v1#/definitions/${definicion}`);
  if (!validar) throw new Error(`El schema no define ${definicion}`);
  return validar(dato) ? [] : validar.errors;
}

const DEFINICION_POR_TIPO: Record<string, string> = {
  suscrito: 'MensajeSuscrito',
  evento: 'MensajeEvento',
  foto: 'MensajeFoto',
  delta: 'MensajeDelta',
  resync: 'MensajeResync',
  revocado: 'MensajeRevocado',
  error: 'MensajeErrorTiempoReal',
};

/** Los errores de un mensaje que sale del servidor, según su `tipo` (vacío = cumple). */
export function erroresDeMensajeDelServidor(mensaje: { tipo: string }) {
  const definicion = DEFINICION_POR_TIPO[mensaje.tipo];
  return definicion ? erroresDeMensaje(definicion, mensaje) : [{ message: `tipo sin definición: ${mensaje.tipo}` }];
}
