# Laboratorio visual de trazado urbano

El motor **real** corriendo en el navegador: un único asentamiento, sin partida, sin API y **sin red**. Se
funda, se le dan materiales infinitos y se avanza el tick a mano para ver crecer la ciudad y el árbol de anclas
en vivo.

```bash
npm run lab
```

Abre <http://localhost:5180>. `LAB_PORT` cambia el puerto. Recargar el navegador reconstruye el bundle, así que
cualquier cambio en `src/` se ve recargando — no hay proceso de watch que mantener.

## Para qué

Los dos bugs más caros de la historia de este módulo no los encontró la suite, los encontró el usuario mirando
la pantalla: el Mercado pegado al Centro Urbano (doc trazado §5.7) y las anclas huérfanas (Etapa 5). La suite
prueba invariantes sobre UNA ciudad de fixture; el laboratorio deja mirar cualquier otra.

Y funciona: al recuperarlo tras la Etapa 6 encontró en su primera corrida un edificio construido encima de una
calle, invariante que la suite daba por bueno porque su ciudad no lo reproduce (doc trazado §E6.16).

## La Facción del lab nace a nivel máximo

`fundar()` le pone `experiencia` por encima del último umbral de `NIVEL_FACCION.xpParaNivel` (nivel 10). Sin
esto, `CUPO_NIVEL_ASENTAMIENTO.maxNivel3` vale 0 para una Facción de nivel 1 y `avanzarNivelAsentamiento` nunca
promueve el único asentamiento del lab de nivel 2 a 3 aunque cumpla todos los gates — se queda "elegible,
esperando cupo" para siempre. Ese cupo modela cuántas ciudades grandes sostiene una Facción en la partida, no
el crecimiento urbano que el lab estudia. (`nivel` suelto no basta: `calcularNivelFaccion` lo recalcula desde
la experiencia en cada tick.)

## Qué se ve

- **Línea de estado** (bajo los botones de tick): tick, nivel, nº de edificios, nº de anclas de árbol, y **qué
  le falta al asentamiento para subir de nivel** — población actual/requerida de cada rol y los edificios que
  aún faltan del gate (`progresoNivelAsentamiento`, mismos gates que `calcularNivelAsentamiento`). El cupo de
  nivel de la partida real no aplica: el laboratorio tiene un solo asentamiento.
- **Mapa**: calles y caminos como ÁREAS (desde la Etapa 6 la calle ocupa celdas), huella real de cada edificio,
  rejilla de fondo conmutable. Contorno rojo = obra sin terminar. **Al pasar el cursor por un edificio**, su
  ficha con la verdad del motor: huella, celda que ocupa, código de árbol si es ancla, y el estado de su
  próxima mejora.
- **Anclas**: código de árbol, nivel, distancia a su raíz, y los tres estados que importan — semilla activa,
  semilla saturada (Lógica 1) y ancla llena (Lógica 2). Debajo, la **vista árbol** como diagrama abstracto;
  pasar el cursor por un ancla en el mapa la resalta en el árbol y al revés.
- **Edificios**: todos los del asentamiento numerados en orden de construcción — tipo, nivel, estado, huella,
  celda — más el recuento por tipo.
- **Construcción manual**: encolar cualquier tipo a mano para forzar demanda; y **mejora manual**, que sube de
  nivel un edificio elegido con los mismos gates y costo que `avanzarMejoras` (útil para llevar una Granja a
  nivel 4 en dos clics y ver cómo se muda a las afueras).
- **Parámetros**: edita EN CALIENTE las tablas reales de `constants.ts`, en tres secciones colapsables:
  *Dimensiones de edificios* (agrupadas por categoría de ancla asociada: residencial, mercado, militar, industria, almacenaje, afueras; anclas primero en cada grupo), *Trazado y anclas*
  (separación mínima y zona de seguridad entre anclas, ancho de calle, caps de corredor, radio y banda de
  afueras, largo de fila), y *Piezas dependientes* (formas de puesto de mercado, cuántos puestos añade cada
  nivel de Mercado, talleres de la Carpintería). Nada cambia hasta "Aplicar y refundar"; "Restaurar valores"
  vuelve a los de `constants.ts`. Todo campo que difiera de su valor original se marca en **amarillo**
  (`input.lab-cambiado`) — para no perder de vista qué se ha tocado.

## Desde la consola

`window.__lab` expone el estado REAL del motor: `{ tick, estado, asentamiento, trazado, red, mapa,
celdasDeEdificio }`. Sirve para medir invariantes sobre la ciudad que estés mirando sin instrumentar nada —
así se aisló el hallazgo de §E6.16.

## Ojo con la paleta

El fondo del mapa es VERDE (`#93c26b`), el mismo contra el que se eligió `EDIFICIO_COLOR`. No es decorativo:
un primer intento usó un fondo crema y la Vivienda —`#e8e2d0`, casi el mismo color— desaparecía. Parecía que
el motor no construía viviendas; se estaban dibujando perfectamente, no se veían. Por eso todo edificio lleva
además contorno: ningún tipo puede volver a confundirse con el fondo ni con su vecino.

## Por qué vive aquí y no en `cliente/`

Se eliminó en `73a12dfb` precisamente porque ejecuta el motor y por tanto *"no era un cliente y no podía
aislarse por red ni acompañar a `cliente/` a otro repositorio"*. Aquel commit lo dejó en el historial *"por si
se rescata como herramienta de desarrollo de este repo"* — que es lo que es ahora.

Su renderer (`src/render.ts`) es una copia deliberada, no una importación de `cliente/`: depender de un cliente
que va a salir del repositorio reintroduciría el problema, y además el laboratorio dibuja para DEPURAR, no para
jugar. El trazado, en cambio, NO se recalcula: llega ya resuelto de `trazadoParaAsentamiento`, el mismo
contrato que sirve el backend — así dibujar el laboratorio ejercita ese contrato de verdad.

## Bundler

esbuild, no vite: `cliente/` se lleva vite cuando salga a su repositorio y el backend no debería heredar un
bundler entero por una herramienta de depuración.
