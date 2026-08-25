// Adaptador de DESARROLLO del puerto `ProveedorIdentidad` (`proveedorIdentidad.ts`): no verifica nada
// criptográficamente, confía en que quien manda la cabecera es quien dice ser. Existe para construir y
// probar el resto de la identidad (sesiones, membresías, autorización de Fase C2) sin esperar a integrar un
// proveedor real — y para que ESE día, sustituirlo sea escribir un adaptador nuevo y quitar este de
// `registroProveedores.ts`, sin tocar `servicioAutenticacion.ts` ni ninguna ruta HTTP.
//
// NUNCA debe quedar activo en producción — quien construya el registro de proveedores del proceso real es
// responsable de no incluirlo (ver `registroProveedores.ts`).
//
// Formato de la credencial: `Authorization: dev <sujetoId>[:<email>]`. `sujetoId` es lo único necesario para
// identificar de forma estable al mismo usuario entre requests.
import type { IdentidadExterna, ProveedorIdentidad } from './proveedorIdentidad';
import { CredencialInvalidaError } from './proveedorIdentidad';

export const ESQUEMA_DESARROLLO = 'dev';

export const proveedorDesarrollo: ProveedorIdentidad = {
  esquema: ESQUEMA_DESARROLLO,

  async autenticar(valor: string): Promise<IdentidadExterna> {
    const [sujetoId, email] = valor.trim().split(':');
    if (!sujetoId) {
      throw new CredencialInvalidaError("el proveedor de desarrollo exige 'sujetoId[:email]' no vacio");
    }
    return { proveedor: ESQUEMA_DESARROLLO, sujetoId, email };
  },
};
