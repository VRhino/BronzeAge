// Punto de entrada: decide si la consola arranca conectada a una partida (`main.ts`, la de siempre) o en la pantalla «sin partida».
// Sin elección guardada se usa la partida del entorno y se crea si no existe, como hasta ahora. Una partida elegida que ya no está en
// el servidor (la borró otra consola) NO se recrea en silencio: se avisa desde «sin partida».
import { listarPartidas } from './app/apiCliente';
import { eleccionDePartida, tomarAviso } from './app/gameStore';

const eleccion = eleccionDePartida();
let aviso = tomarAviso();
let noExiste = false;
if (eleccion.tipo === 'elegida') {
  try {
    const { partidas } = await listarPartidas();
    noExiste = !partidas.some((p) => p.gameId === eleccion.gameId);
    if (noExiste) aviso = `La partida «${eleccion.gameId}» ya no existe en el servidor.`;
  } catch {
    // Sin la lista no se sabe: se intenta conectar como siempre, y `main.ts` dirá qué falló.
  }
}

if (eleccion.tipo === 'ninguna' || noExiste) await (await import('./sinPartida')).montarSinPartida(aviso);
else await import('./main');
