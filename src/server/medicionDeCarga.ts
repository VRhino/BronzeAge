// Interruptor de la telemetría de carga (03 §14). Módulo hoja y a propósito global: lo leen el difusor, el runner y
// la ruta de comandos en su camino caliente, y lo enciende/apaga `RegistroDeCarga`. Apagado (por defecto) esos sitios
// no acumulan nada ni miden nada extra: el coste es una lectura de un booleano.
// ponytail: global de proceso; si algún día hay varias instancias de servidor en un proceso, pasarlo por dependencias.
export const medicion = { activa: false };
