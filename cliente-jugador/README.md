# Cliente de jugador — boilerplate

Punto de partida para un cliente de jugador real, no un cliente completo. Nace de una decisión concreta
(2026-08-26, ver [`Docs/Arquitectura/3_Plan_Evolucion_Roadmap.md`](../Docs/Arquitectura/3_Plan_Evolucion_Roadmap.md)
hito **C11b**): en vez de que el backend rasterice el terreno y lo sirva como imagen/rejilla, este cliente
lleva su **propia copia** de las funciones puras de evaluación de terreno (`src/terreno/`) y lo recalcula él
mismo a partir de los parámetros públicos que ya sirve `GET /jugador/partidas/:gameId/mapa/:mapaId`
(Fase C11a).

## Por qué duplicar en vez de importar

A diferencia de [`cliente/`](../cliente/) (el cliente de administración/depuración, que todavía importa el
motor del backend vía el alias `@motor/*`), **esta carpeta no tiene ningún acceso al código fuente del
servidor** — ni alias, ni `paths` en `tsconfig.json`, ni import de `../src`. Es la prueba de que la Fase C
puede cerrarse: un cliente que solo habla con el backend por HTTP.

Duplicar el evaluador de terreno (no importarlo) es una decisión informada, no una improvisación — ver
[`Docs/Arquitectura/9_Reglas_vs_Simulacion.md`](../Docs/Arquitectura/9_Reglas_vs_Simulacion.md): las funciones
de `src/terreno/` de aquí son **T2a** ("regla de entrada propia/no privilegiada, el cliente puede calcularla
sin viaje de red") — el terreno lo ve todo el mundo por igual, no es información privilegiada de ningún
jugador ni de ningún rival. El coste aceptado es el mismo que en cualquier T2a: la fórmula existe dos veces
(servidor por autoridad — aunque hoy el servidor tampoco la ejecuta para nada propio, solo la generó una vez —
y cliente por presentación) y puede divergir si una cambia sin la otra. Ver `src/terreno/README.md` para el
detalle y la disciplina de mantenimiento.

## Qué NO hace todavía

Es un boilerplate, no un cliente completo:

- Pinta terreno (bioma + ríos + bosques fusionados). No dibuja asentamientos, zonas de influencia ni trazado
  urbano — esos ya llegan calculados en la proyección (`zonas`/`zonasFusionadas`/`trazadoPorAsentamiento`,
  Fase C10) y son triviales de pintar encima del canvas existente, pero no está hecho aquí. **Ojo con las
  zonas de Facción si se implementan luego**: a diferencia de terreno y bosques, fusionarlas con
  `unirFormas`/`unirPoligonos` NO se puede duplicar aquí — su entrada (posición de asentamientos rivales) es
  privilegiada (T2b, doc 9), así que el resultado ya filtrado por Facción tiene que seguir viniendo calculado
  del servidor, como hoy.
- No soporta partidas creadas con `region` (ver la limitación documentada en `src/terreno/elevacion.ts`).
- Sin formularios de comando: `ejecutarComando` existe en `apiCliente.ts` pero no hay UI que la invoque más
  allá del ejemplo comentado en `main.ts`.
- No crea partidas (eso es administración): asume que una partida con el `gameId` indicado ya existe.

## Uso

Requiere el backend corriendo aparte, y una partida ya creada por un administrador:

```bash
ADMINISTRADORES='dev:jefa' npm run server
```

```bash
npm install
npm run dev
```

`VITE_USUARIO` (sujeto del login de desarrollo, `dev <sujeto>`) y `VITE_GAME_ID` (partida a la que unirse) se
configuran como variables de entorno de Vite — por defecto `ana` y `local`.

`vite.config.ts` proxya `/v1` hacia `:3000` para evitar CORS en desarrollo, igual que `cliente/`.
