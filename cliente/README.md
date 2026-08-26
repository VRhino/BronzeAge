# Cliente de depuración / administración — Bronze Age Collapse

Interfaz de navegador que hablaba con el backend cuando ambos vivían en el mismo repositorio. Se separa aquí
para poder inicializar con ella un repositorio propio (Fase C: este repo pasa a ser **solo servidor**, y los
clientes —jugador y administración— viven fuera).

Contiene dos aplicaciones independientes:

| Entrada | Qué es |
|---|---|
| `index.html` → `src/main.ts` | Cliente de la partida: habla con la API por HTTP (`src/app/apiCliente.ts`) y sirve a la vez de consola de administración (crear/regenerar mundo, avanzar tick). |
| `laboratorio.html` → `src/lab/` | Laboratorio visual del crecimiento de asentamientos. **No usa la API**: ejecuta el motor directamente en el navegador. Es una herramienta de desarrollo del motor, no un cliente. |

## La única dependencia que hay que resolver: `@motor/*`

Este cliente no es autónomo todavía. `src/app/gameStore.ts` (y todo `src/lab/`) importan el **motor** para
calcular consultas derivadas en el navegador — producción por tick, zonas de influencia, poder de escuadrón,
niveles, trazado urbano. Son 25 imports en `gameStore.ts` y 13 en `lab/`.

Todos pasan por un único alias, declarado en dos sitios:

- `vite.config.ts` → `resolve.alias`
- `tsconfig.json` → `compilerOptions.paths`

Hoy apunta a `../src` (el `src/` del repositorio del backend). **Al mover esta carpeta a su propio
repositorio, ese alias es lo único que hay que repuntar.** Tres opciones, de menos a más trabajo:

1. **Copiar el motor** (`domain/`, `engine/`, `world/`, `worldgen/`, `session/`, `constants.ts`) dentro de
   este repo. Funciona ya, pero se desincroniza en cuanto el backend evolucione.
2. **Submódulo git** o dependencia `file:`/workspace apuntando al repo del backend. Mantiene una sola fuente.
3. **Publicar el motor como paquete npm** desde el repo del backend. Lo más limpio si el cliente va a vivir
   con vida propia.

### La opción de fondo: dejar de necesitarlo

La razón por la que este cliente necesita el motor es que calcula en el navegador cosas que el servidor podría
mandar ya calculadas. `Docs/Arquitectura/8_Triaje_Consultas.md` (repo del backend) ya clasificó las 31
consultas de `GameStore`: 5 de ellas se convierten en **proyecciones de Fase C**. A medida que el servidor
exponga esos DTOs, este cliente puede ir soltando imports de `@motor/*` hasta quedarse solo con `domain/types`
(tipos, sin coste en tiempo de ejecución).

## Qué superficie habla este cliente

La de **administración** (`/v1/admin/*`): crea partidas, avanza el tick y lee el estado completo. Eso es lo
que siempre hizo; desde la Fase C3 del backend esos endpoints exigen identidad y rol. El cliente de
**jugador** (`/v1/jugador/*`) vive en otro repositorio.

Se identifica con el proveedor de desarrollo del backend (`Authorization: dev <sujeto>`) y guarda el
`sesionId` en memoria — se pierde al recargar y se vuelve a pedir solo. Es un apaño de desarrollo consciente:
sustituirlo por un login real es cambiar `iniciarSesion` en `src/app/apiCliente.ts`.

## Uso

Requiere el backend corriendo aparte, con este sujeto declarado como administrador:

```bash
ADMINISTRADORES='dev:jefa' npm run server
```

```bash
npm install
npm run dev
```

`vite.config.ts` proxya `/v1` entero hacia `:3000` para evitar CORS en desarrollo.

El sujeto se cambia con `VITE_USUARIO`; el que se use debe figurar en `ADMINISTRADORES` del servidor, o el
backend responderá 403 al crear la partida. Sin `ADMINISTRADORES` no hay ningún administrador y nadie puede
crear partidas — es el default deliberado del servidor.

> Al servir este cliente desde otro origen (ya sin el proxy de Vite), el servidor necesita CORS: configúralo
> con `ORIGENES_PERMITIDOS='https://tu-origen'` al arrancarlo — vacío por defecto, ningún origen cruzado pasa.
> El contrato completo (rutas, esquemas, qué exige sesión) está publicado en `GET /v1/openapi.json`.
