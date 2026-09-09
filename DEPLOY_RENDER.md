# Despliegue en Render (free) + Turso

Runbook para levantar el backend **gratis del todo** para el playtest. Lo genérico (Dockerfile, adaptador
`AlmacenDeObjetos`/libSQL, `/salud`, lectura de `PORT`) está en `main`; esta rama solo añade
[`render.yaml`](render.yaml) y este documento.

## Por qué esta combinación

Render free tiene dos límites:

1. **Disco efímero** — se borra en cada deploy y en cada arranque tras dormir. Por eso la persistencia NO va
   a disco sino a **Turso** (SQLite en red, free tier siempre encendido) vía `ALMACEN_URL`. El backend usa el
   adaptador `enLibsql` automáticamente en cuanto esa variable existe.
2. **Duerme a los 15 min sin tráfico** — no se pierde nada (el reloj de mundo re-ancla a "ahora" al
   despertar, `RunnerDePartida`), pero el tiempo de juego se queda atrás mientras nadie juega. Se resuelve
   con un pinger externo (abajo) o se acepta.

Coste real: **$0**. Render free web service + Turso free.

## 1. Base de datos (Turso)

```bash
# instalar: https://docs.turso.tech/cli/installation
turso auth login
turso db create bronze-age
turso db show bronze-age --url          # -> ALMACEN_URL  (libsql://bronze-age-<org>.turso.io)
turso db tokens create bronze-age       # -> ALMACEN_TOKEN
```

## 2. Servicio en Render

1. Repo conectado a Render → **New > Blueprint** → detecta `render.yaml`.
2. Rellena los secretos que `render.yaml` deja como `sync: false`:
   - `ALMACEN_URL`, `ALMACEN_TOKEN` — del paso 1.
   - `ADMINISTRADORES` — p.ej. `dev:jefa`.
   - `CODIGO_REGISTRO` — el código que les pasas a los 5 jugadores.
   - `ORIGENES_PERMITIDOS` — solo si el cliente de jugador se sirve desde otro dominio (Cloudflare Pages,
     Netlify). Formato: `https://tu-cliente.pages.dev`.
3. Deploy. Render construye el `Dockerfile` y arranca. `PORT` lo inyecta Render solo.

Comprobación: `curl https://<servicio>.onrender.com/salud` → `{"ok":true}`. En los logs debe decir
`persistencia: libSQL (...)`, no `disco local`.

## 3. El sleep de 15 minutos

**Opción A — pinger (recomendada).** Un cron externo que llama a `/salud` cada 10 min mantiene el servicio
despierto y el reloj de mundo al día:

- [cron-job.org](https://cron-job.org) (gratis, intervalos exactos): nueva tarea → URL
  `https://<servicio>.onrender.com/salud`, cada 10 min.
- O el workflow de GitHub Actions incluido en esta rama
  ([`.github/workflows/keep-warm.yml`](.github/workflows/keep-warm.yml)): pon la URL en
  *Settings → Secrets and variables → Actions → Variables* como `RENDER_URL`. Aviso: los cron de Actions se
  retrasan bajo carga y se desactivan tras 60 días sin commits — cron-job.org es más fiable.

**Opción B — aceptarlo.** Si el playtest es por sesiones acordadas, deja que duerma: al entrar el primer
jugador arranca (~1 min de cold start) y el mundo sigue desde donde estaba.

## 4. Mantenimiento y respaldos

**No** configures `MANTENIMIENTO_INTERVALO_MS` en Render: `respaldos.ts` es de disco y en Render no hay disco.
El respaldo lo da Turso (`turso db shell bronze-age .dump`, o los point-in-time del plan de pago).

## 5. Cliente de jugador

Es estático. Cloudflare Pages / Netlify, gratis:

```bash
# en el repo BronzeAgeClient
VITE_API_BASE=https://<servicio>.onrender.com npm run build
npx wrangler pages deploy dist        # o: netlify deploy --prod --dir dist
```

Y en Render: `ORIGENES_PERMITIDOS = https://<tu-pages>.pages.dev`.

## Desmontar

- Render: dashboard → el servicio → Settings → Delete.
- Turso: `turso db destroy bronze-age`.
