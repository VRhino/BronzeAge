# Generación del mundo

> **Reescrito el 2026-10-13: este documento describe el generador del motor TypeScript (`typescript-final`) y pasa a ser la referencia de sus reglas, no su implementación.** En la reconstrucción el mundo es un **artefacto** inmutable por hash producido fuera de línea por el generador regional (TypeScript, de Codex) a **2.667 × 2.000** (factor 12 sobre Unity), con la **economía del mundo** del canon 1.9c: campamentos de mercenarios a 600 (8-10, centro de cada foco), tres anillos de tierra (foco hasta 150, frontera 150-250, tierra lejana), estaño solo fuera de 200 de un campamento y en vetas guardadas por bandidos, bosques mezclados y cuotas exactas. El servidor Rust **carga** el artefacto y no genera nada: de este documento sigue vigente la fachada `Mapa` (stock, regeneración por evento, coste de movimiento, Leñeras) y las reglas de campamentos, alijos y bandidos con los valores calibrados (alijos 20-40 / 40-90 / 250-300; bandidos a ≥ 10 del borde de su zona; protección de campamento 10). Abiertos para la fase 4: el oro (O1) y las Leñeras por bosque grande (O2).

## 1. Alcance y fuentes

Cubre todo lo que **existe antes de que nadie juegue** y lo que de ello cambia durante la partida: el **mapa** (relieve, fertilidad, bioma, ríos, bosques, yacimientos y ganado), las **regiones** que sesgan el relieve, la fachada `Mapa` por la que
el motor lo consulta (estado de lo extraído, regeneración, coste de movimiento, Leñeras), y las tres capas que se colocan **sobre** el mapa: los **campamentos de mercenarios** y los **alijos** (se colocan una vez, al crear la
partida) y los **campamentos de bandidos** (aparecen por eventos, no por un barrido).

- **Verificado contra:** `e77c2a1` (2026-10-12), más la corrección de un comentario de `worldgen/nodos.ts` y la ampliación de `mecanicas/sincronizar.ts` (regiones y versión del generador) que trae este documento.
- **Fuentes:** `worldgen/{index,config,types,ruido,elevacion,fertilidad,biomas,rios,bosques,nodos,colocacion,regiones,costeMovimiento}.ts`, `world/{mapa,geometria}.ts`, `engine/{mercenarios,alijos,bandidos}.ts`,
  `session/gameSession.ts` (`crear`, `importar`), `session/estado.ts` (`idDeMapa`), `server/persistenciaPartida.ts` (`cargarPartida`), `session/comandos/mercenarios.ts` (`abrirAlijo`).
- **Canon:** [`Docs/Game/1_Sistema_de_Mundo_y_Territorio.md`](../Game/1_Sistema_de_Mundo_y_Territorio.md) (1.0a escala, 1.0b agua, 1.1 generación, 1.4 recursos, 1.9 bandidos, 1.9b mercenarios).
- **No está aquí** (está en su documento):
  - **El generador aleatorio y su semilla** (decisión del autor, 2026-10-09: fuera de alcance). Este documento dice **qué se pide al generador, cuántas veces y en qué orden** (§3.2), no cómo se calcula el flujo de números (`worldgen/rng.ts`).
  - El **grafo de navegación**, el A\* y los vados (`world/grafoNavegacion.ts`, `world/rutas.ts`) y `COSTE_MOVIMIENTO`/`NAVEGACION` → [`Comercio_Caravanas.md`](Comercio_Caravanas.md) §7.1.
  - La **zona de influencia** de un asentamiento, la fundación y la viabilidad de un sitio → [`Economia_Construccion.md`](Economia_Construccion.md) §3. La producción de los extractores y su cupo → `Economia_Construccion.md` §5–§6 (aquí solo está el stock del yacimiento).
  - El **mercado**, la población, la tecnología y el reclutamiento de un campamento de mercenarios → [`Comercio_Caravanas.md`](Comercio_Caravanas.md) §9 y [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md); su **planta** (edificios y calles) → [`Trazado_Urbano.md`](Trazado_Urbano.md); residir en él → [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md).
  - Qué ve cada cual de un alijo o de un campamento → [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md). Atacar un campamento de bandidos y su botín → [`Combate.md`](Combate.md) §7.2.
  - Los **contornos fusionados** de bosques y zonas (`world/poligonos.ts`, `Mapa.contornosBosques`): campo de distancia con signo + marching squares, **solo para dibujar**; ninguna regla los consulta.

Quién llama a qué: `GameSession.crear` genera el mapa (`generarMapa`) y, sobre él, los campamentos y los alijos; el tick llama a `mapa.avanzarRegeneracion` (paso 7) y a `avanzarSpawnBandidos` (paso 8) y `avanzarAtaquesBandidos` (paso 9); ver [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md).

## 2. Modelo de datos

### 2.1 Qué se guarda y qué se regenera

| Dato | Dónde vive | Se guarda en el snapshot |
|---|---|---|
| `WorldConfig` = `{ ancho, alto, seed, region? }` | `MapaGenerado.config` | **Sí**: es lo único del mapa que se guarda. `ancho`/`alto` son siempre `MAPA_DEFAULT`. |
| `MapaGenerado` (bosques, nodos, campos de ruido, ríos) | `estado.mapa` | **No**: `cargarPartida` hace `estado.mapa = generarMapa(estado.mapa.config)`. |
| `EstadoMapa` = `{ extraido, regeneraEn }` | `estado.estadoMapa` | Sí. |
| `campamentosMercenarios`, `alijos` | `estado` | Sí (se colocan una vez y luego son estado). |
| `campamentosBandidos` | `estado` | Sí; **empieza vacío**. |

Consecuencias, en este orden de gravedad:

1. **Cambiar el algoritmo cambia el mundo de toda seed ya guardada**, así que `cargarPartida` **rechaza** (`WorldgenVersionNoCoincideError`, sin migración) un snapshot cuyo `worldgenVersion` ≠ `WORLDGEN_VERSION`. Toda modificación del pipeline, de `worldgen/config.ts`
   o del **orden o número de tiradas** sube esa versión y regenera el snapshot de caracterización a conciencia. `worldgen/config.ts` está **congelado** (`Object.freeze` recursivo) y no es parte del balance editable en caliente: por eso no vive en `constants.ts`.
2. **Los campamentos de mercenarios y los alijos no se regeneran al cargar**: están en el estado. Cambiar `colocarCampamentosIniciales` o `colocarAlijos` **no** exige subir `WORLDGEN_VERSION` y **no** altera las partidas guardadas, pero sí
   las nuevas. (Y viceversa: si el mapa cambiara sin subir la versión, los campamentos guardados quedarían sobre un relieve distinto.)
3. `idDeMapa(mapa) = "v{version}-s{seed}"` más `-{region}` si hay región. Es legible a propósito, no un hash, y nunca se guarda.

### 2.2 Entidades del mapa

```
NodoRecurso { id, tipo: piedra|cobre|hierro|estano|oro|livestock, rareza: comun|intermedio|raro, posicion: Point, cantidadInicial }
ZonaBosque  { id: "bosque-{i}", centro: Point, radio, densidad }            círculo; densidad 0..1
RioZona     { id, puntos: Point[], terminaEnLago, navegable }               polilínea
TerrenoTipo = agua | costa | llano | colina | montana | cima               (bandas de elevación)
BiomaTipo   = agua | costa | estepa | llanuraFertil | colina | montana | cima     (el «llano» se parte en estepa / llanuraFertil)
```

- Trigo y madera **no** son nodos: el trigo sale del campo de fertilidad (más la Granja) y la madera, de los bosques.
- `cantidadInicial` es lo que el yacimiento tiene **al nacer**; lo que queda es `cantidadInicial − extraido[id]` (§10). Nunca se escribe en el nodo.
- `CampoRuido { permutacion[512], octavas, frecuenciaBase, lacunaridad, persistencia, desplazamientos[octavas]{dx,dy}, amplitudTotal }`, datos puros.
- `CampoElevacion { ruido, region?, borde?: { ruido, limites } }`. `region` va **ya resuelta** a unidades de mapa (§6). `borde` existe **solo sin región**.
- `MapaGenerado { version, config, bosques, nodos, fertilidad, elevacion, rios }`. No existe ningún campo de relieve, de bioma ni de coste guardado como rejilla: todo se **evalúa en el punto** pedido.

### 2.3 Capas colocadas sobre el mapa

```
CampamentoMercenarios { id: "mercenarios-{i}", posicion, origen, edificios[], residentesIds[], eligieronComoInicial, bandidosEn?, poblacion, poblacionEn, mercado, fondos, creadoEn, … }
Alijo                 { id: "alijo-{i}", posicion, oro }
CampamentoBandido     { id, posicion, bosqueId, asentamientoId? | campamentoMercenariosId?, nivel: 1|2|3, poder }
```

## 3. El pipeline `generarMapa(config)`

### 3.1 Orden de las capas

```
rng = createRng(config.seed)                      ← un solo generador para todo el mapa
1. elevacion  = generarCampoElevacion(rng, limites, region)      campo de ruido + (sin región) campo de ruido del borde
2. rios       = generarRios(rng, limites, elevacion)             nacimientos, descenso, navegabilidad, troncales
3. fertilidad = generarCampoFertilidad(rng)
4. bosques    = generarBosques(rng, limites, elevacion, fertilidad)
5. nodos      = piedra → cobre → estaño → oro → livestock → hierro → estaño extra            (§9)
                (`colocadosGlobal` se comparte y crece entre llamadas; `bosques` los esquivan)
return { version: WORLDGEN_VERSION, config, bosques, nodos, fertilidad, elevacion, rios }
```

El orden **es contrato** y tiene motivos: la elevación va primero porque ríos, bosques y nodos la consultan; los ríos van antes que bosques y nodos porque el bioma usa la cercanía a un río como humedad; la fertilidad va antes que los bosques
(v6) porque la densidad de cada bosque la pondera; los bosques van antes que los nodos porque estos los esquivan; hierro y estaño extra van **los últimos** para no desplazar nada ya calibrado.

### 3.2 Qué pide cada capa al generador

(El flujo es el de `rng.ts`; aquí solo se cuenta cuántas tiradas y en qué orden. «x, y» = dos tiradas, primero `x`.)

| Capa | Tiradas, en orden |
|---|---|
| Campo de ruido (`generarCampoRuido`) | **255** del barajado de Fisher-Yates (`i = 255…1`, `j = floor(r·(i+1))`, se intercambian `base[i]` y `base[j]`), luego `dx = r·256`, `dy = r·256` por octava. Total = 255 + 2·octavas. |
| Elevación | campo `ELEVACION` (265), y **si no hay región** el campo del borde `ELEVACION_BORDE` (259) |
| Ríos | nacimientos (§7.1); luego, para cada río troncal de la región, **una** tirada por punto de su trazo (§7.3) |
| Fertilidad | 261 |
| Bosques | por bosque, en este orden: centro (x, y por intento, hasta `COLOCACION.intentosPorPunto`; si ninguno vale, un centro uniforme más), `senalAzar`, `radio` |
| Nodos | por tipo: **todas sus posiciones** (§9.1) y **después** una `cantidadInicial` por nodo; el tipo siguiente empieza cuando termina el anterior |

Hay que respetar además el detalle de cada paso (qué se evalúa antes de gastar una tirada), que está en su sección. `GameSession.crear` arranca el generador **de la partida** con `createRng(seed)`; es una instancia distinta del de `generarMapa` (misma seed,
estado independiente).

## 4. Ruido fractal (`ruido.ts`)

Ruido de gradiente 2D (Perlin) sumado en octavas (fBm). Es **puro** y está definido en todo el plano (también fuera del mapa: los muestreos en anillo se salen del borde).

- **Gradientes** (8, todos unitarios, `S = √½`): `[1,0] [S,S] [0,1] [−S,S] [−1,0] [−S,−S] [0,−1] [S,−S]`, elegidos con `hash & 7`.
- **Ruido en `(x, y)`**: `xi = floor(x)`, `yi = floor(y)`, `xf = x − xi`, `yf = y − yi`, `cx = xi & 255`, `cy = yi & 255` (la rejilla envuelve en los dos sentidos). `u = suavizar(xf)`, `v = suavizar(yf)` con
  `suavizar(t) = t³·(t·(6t − 15) + 10)`. Con `p = permutacion` (512 entradas = la permutación repetida):
  `aa = p[p[cx] + cy]`, `ab = p[p[cx] + cy + 1]`, `ba = p[p[cx + 1] + cy]`, `bb = p[p[cx + 1] + cy + 1]`. El aporte de una esquina es `g[hash & 7] · (dx, dy)` y se interpola:
  `sup = lerp(aporte(aa, xf, yf), aporte(ba, xf − 1, yf), u)`, `inf = lerp(aporte(ab, xf, yf − 1), aporte(bb, xf − 1, yf − 1), u)`, resultado `lerp(sup, inf, v)`. Rango teórico `[−√½, +√½]`.
- **Campo completo** `evaluarRuido(campo, p)`: `valor = Σᵢ ruido(p.x·fᵢ + dxᵢ, p.y·fᵢ + dyᵢ)·aᵢ` con `f₀ = frecuenciaBase`, `fᵢ₊₁ = fᵢ·lacunaridad`, `a₀ = 1`, `aᵢ₊₁ = aᵢ·persistencia`; se normaliza con la **cota teórica**
  (`amplitudTotal·S`, no con el rango observado): `clamp((valor / (amplitudTotal·S) + 1) / 2, 0, 1)`. Por eso los valores se concentran alrededor de 0,5 y los umbrales de `ELEVACION`/`BIOMA` están calibrados contra la distribución real, no repartidos por 0–1.
- **Campo parcial** `evaluarRuidoParcial(campo, p, k)`: lo mismo con solo las `min(k, octavas)` primeras octavas (la misma tabla y los mismos desplazamientos), normalizando con **la suma de amplitudes de esas octavas**.

## 5. Elevación, terreno, bioma, fertilidad y coste

### 5.1 Elevación (`evaluarElevacion`)

```
ruido = evaluarRuido(campo.ruido, p)
e     = región ? guía(p)·pesoGuia + ruido·(1 − pesoGuia) : ruido                              (§6)

# Borde natural (solo sin región)
si campo.borde:
    distBorde = min(x, ancho − x, y, alto − y)                       (puede ser negativa fuera del mapa → factor 1)
    anchoBorde = ELEVACION_BORDE.anchoFraccion · (ancho + alto) / 2
    fBorde = 1 − smoothstep(0, anchoBorde, distBorde)
    si fBorde > 0:
        tipo     = evaluarRuido(campo.borde.ruido, p)
        objetivo = objetivoCosta + tipo · (objetivoMontana − objetivoCosta)
        e        = e + (objetivo − e) · fBorde

# Suavizado de la banda jugable
pesoBajo = smoothstep(umbralAgua, umbralCosta, e) ;  pesoAlto = 1 − smoothstep(umbralMontana, umbralCima, e)
pesoSuave = pesoBajo · pesoAlto · (1 − fBorde) · ELEVACION_SUAVIZADO.pesoMaximo
si pesoSuave > 0:
    suave = evaluarRuidoParcial(campo.ruido, p, octavasSuaves)   # con región: guía·pesoGuia + suave·(1 − pesoGuia)
    e = e + (suave − e) · pesoSuave
devolver clamp(e, 0, 1)
```

`smoothstep(a, b, x) = t²(3 − 2t)` con `t = clamp((x − a)/(b − a), 0, 1)`. El suavizado conserva la ondulación ancha y descarta el detalle fino; **nunca aplana del todo** (el gradiente de los ríos nunca es exactamente cero). El suavizado se apaga a medida que
crece el borde (`1 − fBorde`) para que la pared de costa/montaña del perímetro no se atenúe.

**Gradiente** (`gradienteElevacion(campo, p, paso)`): diferencias centradas, `dx = (e(x+paso, y) − e(x−paso, y)) / (2·paso)`, y análogo en `y`. Lo usan los ríos con `paso = RIOS.pasoGradiente`.

### 5.2 Terreno (`evaluarTerreno`)

`e < umbralAgua` → `agua`; `< umbralCosta` → `costa`; `< umbralColina` → `llano`; `< umbralMontana` → `colina`; `< umbralCima` → `montana`; si no, `cima` (inhabitable: ni se funda ni se colocan recursos). El agua es obstáculo
(`esTransitable(p) = terreno ≠ agua`, Doc 1.0b); la cima es transitable pero cara.

### 5.3 Fertilidad y bioma

`evaluarFertilidad(p) = evaluarRuido(campoFertilidad, p)` (campo `FERTILIDAD`, 3 octavas, manchas de ~500 u). `evaluarBioma(p)`: si `terreno ≠ llano` el bioma **es** el terreno; si es llano, `llanuraFertil` cuando
`fertilidad(p) ≥ BIOMA.umbralFertilLlanura` **o** `distanciaARioMasCercano(rios, p) < BIOMA.radioHumedadRio`, y `estepa` si no. `distanciaARioMasCercano` es el mínimo de la distancia punto-segmento a todos los segmentos de todos los ríos
(un río de un solo punto no tiene segmentos y no aporta).

### 5.4 Coste de movimiento

`costeEnPunto(campo, p)` (worldgen) es la entrada de `COSTE_MOVIMIENTO` del terreno de `p` (el del agua queda declarado pero no se consulta: lo que no es transitable se descarta antes). La fachada `Mapa.costeEnPunto(p)` lo multiplica por el bosque:

```
densidad = max{ b.densidad : b cubre la celda de p (índice de 100×100) ∧ distancia(b.centro, p) < b.radio }   (0 si ninguno)
coste(p) = costeTerreno(p) · (1 + COSTE_MOVIMIENTO.bosquePorDensidad · densidad)
```

## 6. Regiones (`regiones.ts`)

`config.region` (`greciaContinental | anatolia | egeo | nilo | mesopotamia`, o ausente = **mundo libre**) sesga el relieve hacia una silueta reconocible con una **guía** hecha a mano, mezclada con el mismo ruido. No importa datos geográficos reales.

- **Resolución** (una vez, en `generarCampoElevacion`): con `escala = (ancho + alto)/2`, los puntos de cada rasgo se multiplican por `ancho`/`alto`, y `anchoInfluencia` y `radio` por `escala`. `REGIONES` se autora en fracciones 0–1, así que la guía no depende del tamaño del mundo.
- **Guía** `evaluarGuia(región, p)`, con `S(t) = t²(3 − 2t)` y `f(d, w) = S(clamp(1 − d/w, 0, 1))`:
  ```
  valor = baseElevacion
  para cada rasgo, en orden:
      cresta:  valor += alturaPico · f(distanciaASegmento(p, a, b), anchoInfluencia)
      bahía:   valor −= profundidad · f(distanciaASegmento(p, a, b), anchoInfluencia)
  para cada rasgo, en orden (segunda pasada):
      cuenca:  valor += (alturaObjetivo − valor) · f(distancia(p, centro), radio)
      valle:   valor += (alturaObjetivo − valor) · f(distanciaASegmento(p, a, b), anchoInfluencia)
  devolver clamp(valor, 0, 1)
  ```
  Cuencas y valles se aplican **al final y aparte**: aplanan hacia su objetivo por encima de lo que hayan hecho crestas y bahías cercanas. Una cresta con `a = b` es un bulto circular (así se hacen las islas del Egeo).
- **Con región no hay campo de borde**: las regiones definen su propio perímetro. Por eso un mundo con región gasta 259 tiradas menos y todo lo posterior sale distinto.
- **Ríos troncales** (Nilo, Tigris, Éufrates): ver §7.3.

<!-- parametros:REGIONES -->
```json
{
  "greciaContinental": {
    "pesoGuia": 0.68,
    "baseElevacion": 0.63,
    "rasgos": [
      {
        "tipo": "cresta",
        "a": {
          "x": 0.38,
          "y": 0.05
        },
        "b": {
          "x": 0.45,
          "y": 0.95
        },
        "anchoInfluencia": 0.14,
        "alturaPico": 0.18
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.32,
          "y": 0.72
        },
        "b": {
          "x": 0.55,
          "y": 0.92
        },
        "anchoInfluencia": 0.11,
        "alturaPico": 0.15
      },
      {
        "tipo": "bahia",
        "a": {
          "x": 1,
          "y": 0.28
        },
        "b": {
          "x": 0.55,
          "y": 0.35
        },
        "anchoInfluencia": 0.16,
        "profundidad": 0.48
      },
      {
        "tipo": "bahia",
        "a": {
          "x": 1,
          "y": 0.62
        },
        "b": {
          "x": 0.5,
          "y": 0.68
        },
        "anchoInfluencia": 0.16,
        "profundidad": 0.48
      },
      {
        "tipo": "bahia",
        "a": {
          "x": 0.05,
          "y": 0.7
        },
        "b": {
          "x": 0.75,
          "y": 0.66
        },
        "anchoInfluencia": 0.14,
        "profundidad": 0.5
      },
      {
        "tipo": "bahia",
        "a": {
          "x": 0,
          "y": 0.45
        },
        "b": {
          "x": 0.25,
          "y": 0.5
        },
        "anchoInfluencia": 0.16,
        "profundidad": 0.38
      },
      {
        "tipo": "cuenca",
        "centro": {
          "x": 0.55,
          "y": 0.22
        },
        "radio": 0.1,
        "alturaObjetivo": 0.48
      },
      {
        "tipo": "cuenca",
        "centro": {
          "x": 0.4,
          "y": 0.85
        },
        "radio": 0.07,
        "alturaObjetivo": 0.48
      }
    ]
  },
  "anatolia": {
    "pesoGuia": 0.55,
    "baseElevacion": 0.5,
    "rasgos": [
      {
        "tipo": "cresta",
        "a": {
          "x": 0,
          "y": 0.1
        },
        "b": {
          "x": 1,
          "y": 0.14
        },
        "anchoInfluencia": 0.2,
        "alturaPico": 0.32
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.05,
          "y": 0.88
        },
        "b": {
          "x": 0.95,
          "y": 0.85
        },
        "anchoInfluencia": 0.2,
        "alturaPico": 0.34
      },
      {
        "tipo": "bahia",
        "a": {
          "x": 0,
          "y": 0.35
        },
        "b": {
          "x": 0.22,
          "y": 0.4
        },
        "anchoInfluencia": 0.14,
        "profundidad": 0.35
      },
      {
        "tipo": "bahia",
        "a": {
          "x": 0,
          "y": 0.62
        },
        "b": {
          "x": 0.2,
          "y": 0.58
        },
        "anchoInfluencia": 0.14,
        "profundidad": 0.35
      },
      {
        "tipo": "cuenca",
        "centro": {
          "x": 0.55,
          "y": 0.5
        },
        "radio": 0.28,
        "alturaObjetivo": 0.49
      }
    ]
  },
  "egeo": {
    "pesoGuia": 0.7,
    "baseElevacion": 0.25,
    "rasgos": [
      {
        "tipo": "cresta",
        "a": {
          "x": 0.5,
          "y": 0.25
        },
        "b": {
          "x": 0.5,
          "y": 0.25
        },
        "anchoInfluencia": 0.16,
        "alturaPico": 0.55
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.68,
          "y": 0.85
        },
        "b": {
          "x": 0.68,
          "y": 0.85
        },
        "anchoInfluencia": 0.2,
        "alturaPico": 0.5
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.2,
          "y": 0.35
        },
        "b": {
          "x": 0.2,
          "y": 0.35
        },
        "anchoInfluencia": 0.09,
        "alturaPico": 0.4
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.75,
          "y": 0.2
        },
        "b": {
          "x": 0.75,
          "y": 0.2
        },
        "anchoInfluencia": 0.07,
        "alturaPico": 0.38
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.35,
          "y": 0.6
        },
        "b": {
          "x": 0.35,
          "y": 0.6
        },
        "anchoInfluencia": 0.08,
        "alturaPico": 0.4
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.55,
          "y": 0.55
        },
        "b": {
          "x": 0.55,
          "y": 0.55
        },
        "anchoInfluencia": 0.06,
        "alturaPico": 0.35
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.85,
          "y": 0.55
        },
        "b": {
          "x": 0.85,
          "y": 0.55
        },
        "anchoInfluencia": 0.07,
        "alturaPico": 0.38
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.15,
          "y": 0.75
        },
        "b": {
          "x": 0.15,
          "y": 0.75
        },
        "anchoInfluencia": 0.06,
        "alturaPico": 0.35
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 0.45,
          "y": 0.15
        },
        "b": {
          "x": 0.45,
          "y": 0.15
        },
        "anchoInfluencia": 0.06,
        "alturaPico": 0.33
      }
    ]
  },
  "nilo": {
    "pesoGuia": 0.55,
    "baseElevacion": 0.47,
    "rasgos": [
      {
        "tipo": "valle",
        "a": {
          "x": 0.52,
          "y": 0.02
        },
        "b": {
          "x": 0.48,
          "y": 0.35
        },
        "anchoInfluencia": 0.035,
        "alturaObjetivo": 0.46
      },
      {
        "tipo": "valle",
        "a": {
          "x": 0.48,
          "y": 0.35
        },
        "b": {
          "x": 0.55,
          "y": 0.65
        },
        "anchoInfluencia": 0.035,
        "alturaObjetivo": 0.46
      },
      {
        "tipo": "valle",
        "a": {
          "x": 0.55,
          "y": 0.65
        },
        "b": {
          "x": 0.5,
          "y": 0.98
        },
        "anchoInfluencia": 0.035,
        "alturaObjetivo": 0.46
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 1,
          "y": 0.3
        },
        "b": {
          "x": 0.95,
          "y": 0.7
        },
        "anchoInfluencia": 0.15,
        "alturaPico": 0.22
      }
    ],
    "riosTroncales": [
      {
        "id": "nilo",
        "puntos": [
          {
            "x": 0.52,
            "y": 0.02
          },
          {
            "x": 0.48,
            "y": 0.35
          },
          {
            "x": 0.55,
            "y": 0.65
          },
          {
            "x": 0.5,
            "y": 0.98
          }
        ]
      }
    ]
  },
  "mesopotamia": {
    "pesoGuia": 0.5,
    "baseElevacion": 0.46,
    "rasgos": [
      {
        "tipo": "valle",
        "a": {
          "x": 0.35,
          "y": 0.02
        },
        "b": {
          "x": 0.45,
          "y": 0.5
        },
        "anchoInfluencia": 0.03,
        "alturaObjetivo": 0.45
      },
      {
        "tipo": "valle",
        "a": {
          "x": 0.45,
          "y": 0.5
        },
        "b": {
          "x": 0.55,
          "y": 0.98
        },
        "anchoInfluencia": 0.03,
        "alturaObjetivo": 0.45
      },
      {
        "tipo": "valle",
        "a": {
          "x": 0.55,
          "y": 0.02
        },
        "b": {
          "x": 0.58,
          "y": 0.5
        },
        "anchoInfluencia": 0.03,
        "alturaObjetivo": 0.45
      },
      {
        "tipo": "valle",
        "a": {
          "x": 0.58,
          "y": 0.5
        },
        "b": {
          "x": 0.56,
          "y": 0.98
        },
        "anchoInfluencia": 0.03,
        "alturaObjetivo": 0.45
      },
      {
        "tipo": "cresta",
        "a": {
          "x": 1,
          "y": 0
        },
        "b": {
          "x": 0.92,
          "y": 1
        },
        "anchoInfluencia": 0.18,
        "alturaPico": 0.3
      }
    ],
    "riosTroncales": [
      {
        "id": "tigris",
        "puntos": [
          {
            "x": 0.35,
            "y": 0.02
          },
          {
            "x": 0.45,
            "y": 0.5
          },
          {
            "x": 0.55,
            "y": 0.98
          }
        ]
      },
      {
        "id": "eufrates",
        "puntos": [
          {
            "x": 0.55,
            "y": 0.02
          },
          {
            "x": 0.58,
            "y": 0.5
          },
          {
            "x": 0.56,
            "y": 0.98
          }
        ]
      }
    ]
  }
}
```
<!-- /parametros -->

## 7. Ríos (`rios.ts`)

### 7.1 Nacimientos

`colocarConEspaciado(rng, limites, RIOS.cantidad, RIOS.espacioMinimoEntreNacimientos, [], [], aceptar)` (§9.1, sin bosques que esquivar ni puntos previos) con el predicado `aceptar(p) = terreno(p) = montana ∧ |∇e(p)| ≥ RIOS.gradienteMinimo`
(gradiente con `pasoGradiente`). Siempre salen **exactamente `RIOS.cantidad`** nacimientos: lo que no encuentra sitio cae al respaldo (primer candidato que cumple el predicado pero no el espaciado) o, en último caso, a un punto uniforme sin comprobar nada.
Un nacimiento de ese último tipo puede producir un río de un solo punto.

### 7.2 Descenso por máxima pendiente (sin tiradas)

```
puntos = [fuente]; actual = fuente; terminaEnLago = true
repetir hasta RIOS.pasosMax veces:
    (dx, dy) = ∇e(actual, pasoGradiente) ;  m = |(dx, dy)|
    si m < gradienteMinimo:  terminaEnLago = true ;  parar
    siguiente = actual − (dx/m, dy/m)·pasoDescenso
    si siguiente sale de [0, ancho]×[0, alto]:  añadir siguiente recortado al borde ;  terminaEnLago = false ;  parar
    añadir siguiente ; actual = siguiente
    si terreno(actual) = agua:  terminaEnLago = false ;  parar
(agotar los pasos deja terminaEnLago = true)
id = "rio-{i}" (i = orden de nacimiento)
```

**Navegabilidad** (sin tiradas): candidatos = ríos con `terminaEnLago = false`, ordenados por longitud total de la polilínea **descendente** (orden estable: a igual longitud, el de menor `i`); se marcan `navegable` los `round(candidatos · proporcionNavegable)` primeros. Solo cuenta
entre los ríos aleatorios.

### 7.3 Ríos troncales (solo con región que los define)

Se generan **después** y aparte de los aleatorios (así las regiones sin ellos consumen el generador igual que antes de existir esta pieza). Para cada `RioTroncalDef` (puntos ya resueltos a unidades de mapa):

```
meandro = min(ancho, alto) · 0,015
para cada tramo (a → b):  largo = |b − a| (o 1 si es 0) ;  n = (−dy, dx)/largo
    para paso en (tramo = 0 ? 0 : 1) … 8:         ← el primer tramo incluye su extremo inicial; los demás lo omiten
        t = paso / 8
        desplazamiento = (r·2 − 1) · meandro · sin(π·t)               ← una tirada por punto
        punto = a + (b − a)·t + n·desplazamiento
RioZona { id: def.id, puntos, terminaEnLago: false, navegable: true }
```

Un troncal tiene `8·tramos + 1` puntos. Son navegables por definición y **no** compiten por el cupo de `proporcionNavegable`; se suman a los `RIOS.cantidad` (la lista final es aleatorios ++ troncales).

## 8. Bosques (`bosques.ts`)

Para `i = 0 … BOSQUE.cantidad − 1`:

1. **Centro**: hasta `COLOCACION.intentosPorPunto` intentos con `(x, y)` uniformes en el mapa; el primero cuyo `terreno ∈ BOSQUE_TERRENO_PERMITIDO` (llano o colina) se queda. Si ninguno vale, un punto uniforme más, sin comprobar (el mapa «saturado»: no bloquea la generación).
2. `senalFertilidad = fertilidad(centro)`; `senalAzar = r` (una tirada); `mezcla = senalFertilidad·0,7 + senalAzar·0,3`; `densidad = densidadMin + mezcla·(densidadMax − densidadMin)`. (0,7 es una constante local de `bosques.ts`.)
3. `radio = radioMin + r·(radioMax − radioMin)` (una tirada).

`id = "bosque-{i}"`. Pueden solaparse entre sí y salirse del borde: solo el centro queda dentro del mapa. Un bosque nunca nace en agua, costa, montaña ni cima (salvo el respaldo).

## 9. Yacimientos y ganado (`nodos.ts`, `colocacion.ts`)

### 9.1 `colocarConEspaciado` (rejection sampling)

Para colocar `cantidad` puntos con espaciado mínimo `E` (`todos` = lo ya colocado + los que se van colocando):

```
para i en 0…cantidad−1:
    colocado = ninguno ; respaldo = ninguno
    para intento en 0…intentosPorPunto−1:
        c = (x uniforme, y uniforme)                                  ← dos tiradas por intento
        si c está dentro de algún bosque (distancia < radio)  o  (hay predicado de terreno y falla):  siguiente intento
        si ∀ q ∈ todos: distancia(q, c) ≥ E:  colocado = c ; parar
        respaldo ??= c                                                ← el PRIMER candidato en terreno válido que no cumplió el espaciado
    final = colocado ?? respaldo ?? (punto uniforme sin comprobar nada)      ← el último, 2 tiradas más
```

- El predicado de terreno **solo se evalúa si el punto no está en un bosque**.
- **Se sacrifica el espaciado, nunca el terreno**: «los metales salen en la montaña» es una regla del mundo; el espaciado solo reparte. Es el camino habitual de estaño y oro, porque cuando les toca el turno (`espacioMinimo` 120) ya hay cientos de nodos comunes.
- El último recurso es la única vía por la que un nodo acaba **dentro de un bosque o fuera de su bioma**.

### 9.2 Cuántos, de qué y en qué orden

`generarNodosDeRareza(rareza, tipos)`: cantidad por tipo = `round(RECURSO_RAREZA[rareza].cantidadBase · factor)`, espaciado = `RECURSO_RAREZA[rareza].espacioMinimo`. El predicado de terreno es `RECURSO_BIOMA_PERMITIDO[tipo]` (`evaluarBioma(p) ∈ lista`); un tipo sin
entrada acepta todo. Tras sortear todas las posiciones de **un tipo**, cada una se añade a `colocadosGlobal` y recibe `cantidadInicial = round(min + r·(max − min))` con el rango de `RECURSO_CANTIDAD_NODO[tipo]`; después empieza el tipo siguiente (que ya ve los anteriores).

| Paso | Rareza | Tipos y cantidad | Espaciado |
|---|---|---|---|
| 1 | común | piedra 240 | 20 |
| 2 | intermedio | cobre 80 | 40 |
| 3 | raro | estaño 24, luego oro 24 | 120 |
| 4 | (ganado) | livestock 100 (`LIVESTOCK`), rareza `comun`, bioma `livestock`, espaciado 30, `cantidadInicial = floor(10 + r·31)` (entero entre `cantidadPorManada.min` y `.max`) | 30 |
| 5 | intermedio | hierro `round(80 · 1,5) = 120` (`HIERRO`) | 40 |
| 6 | intermedio | estaño extra `round(80 · 0,5) = 40` (`ESTANO_EXTRA`), rareza `intermedio` | 40 |

Total: **628** nodos. `id = "{tipo}-{nodos.length}-{round x}-{round y}"`, donde `nodos.length` es el índice **dentro de esa llamada** (por eso hierro empieza en 0 y el estaño extra repite índices del estaño raro; la unicidad la dan las coordenadas) y, para el ganado,
`"livestock-{i}-{round x}-{round y}"`. El orden del array `nodos` (el de la tabla) es **observable**: la fachada lo usa para desempatar.

## 10. La fachada `Mapa` (`world/mapa.ts`)

La única vía por la que el motor consulta el mundo. Se construye por comando sobre una **copia** del `EstadoMapa`; quien la crea decide si adopta `estadoActual()` o la descarta. Los índices derivados del mundo (nodos por id, celdas, bosques por celda, orden de generación) se cachean por `MapaGenerado`.

### 10.1 Estado de partida y yacimientos

```
EstadoMapa { extraido: { nodoId → unidades sacadas }, regeneraEn: { nodoId → Instante } }
stock(id)   = nodo.cantidadInicial − (extraido[id] ?? 0)          (0 si el nodo no existe)
extraer(id, n): si el nodo no existe o n ≤ 0 → 0 ; disponible = stock(id) ; si ≤ 0 → 0 ;
                sacado = min(n, disponible) ; extraido[id] += sacado ; mutaciones++ ; devolver sacado
```

**Regeneración** (un **evento** al fin del cooldown de cada nodo; antes `avanzarRegeneracion`), para cada nodo **en orden de generación** con `stock ≤ 0`:

1. Sin entrada en `regeneraEn`: se agenda `instante + cooldown` (`livestock` = `REGENERACION_NODOS.livestock.cooldownMinutos`; **todo lo demás**, también la piedra, = `.metales`). El cooldown cuenta desde el tick en que se **detecta** el agotamiento, no desde que ocurrió. Esto también cura partidas viejas.
2. Con entrada y `instante ≥ regeneraEn`: se borran `extraido[id]` y `regeneraEn[id]` (stock completo) y se emite `mapa.yacimiento_regenerado { nodoId, recurso }`.

### 10.2 Consultas espaciales

- Índice de celdas de **100×100**: los nodos se apuntan en la celda de su posición; los bosques, en **todas** las celdas que toca su caja (`floor((cx − r)/100) … floor((cx + r)/100)`, igual en y).
- `nodosEnRadio(c, r, opciones)`: nodos de las celdas de la caja `[c ± r]` con `distancia ≤ r` (inclusive). `nodosEnPoligono`: caja envolvente + `pointInPolygon` (par-impar). Ambos devuelven los nodos **en orden de generación** (se reordena tras recoger celdas) y después aplican `opciones`:
  `tipo`, `conStock` (stock > 0), `excluir` (ids) y `ordenarPorCercaniaA` (orden estable por distancia: a igual distancia, el de menor orden de generación). **El orden de generación es contrato**: sin él, dos nodos a igual distancia se desempatarían distinto y la simulación divergiría.
- `mejorPorFertilidad(candidatos)`: el de mayor fertilidad; empate → el primero (comparación estricta `>`, parte de −1). `rioMasCercano(p)`: el río con menor distancia al trazo; empate → el primero; `null` sin ríos. `dentroDelMapa(p)`: `0 ≤ x ≤ ancho ∧ 0 ≤ y ≤ alto` (inclusive).
- `hayBosqueEnRadio(c, r)`: algún bosque con `distancia(centro, c) < r + b.radio` (estricto: el criterio único de «el borde del bosque entra en el círculo»). `hayBosqueLibreEnRadio` añade que el bosque tenga Leñeras `< capacidadLeneras`.

### 10.3 Leñeras en un bosque

`capacidadLeneras(bosque)`: `radio ≥ LENERA_POR_BOSQUE.umbral3` → 3; `≥ umbral2` → 2; si no, 1. Con los radios de `BOSQUE` (45–120) la rama «1» no se da nunca: ~29 % de los bosques admiten 2 y ~71 %, 3 (decidido así el 2026-10-12).

`puntoDeTrabajoEnBosque(bosqueId, polígono, ocupadas)`: punto preferido = el **centro** si `ocupadas = 0`; si no, `centro + radio·0,4·(cos(2π·ocupadas/3), sin(2π·ocupadas/3))`. Si cae dentro del polígono, ese. Si no, se prueban **anillos** de radio `radio·k/3`, `k = 1, 2, 3`, con 12 muestras cada uno
(ángulo `2π·i/12`, `i = 0…11`) y se devuelve la primera dentro; ninguna → `null`.

`bosqueParaLenera(polígono, ocupación, cercaDe)`: polígono vacío → `null`. Si no: `centroide` = media de los vértices, `radioEnvolvente` = distancia al vértice más lejano. Candidatos = bosques con `distancia(centro, centroide) ≤ radioEnvolvente + radio` (descarte conservador:
no pierde ningún bosque que pudiera dar punto), ordenados por distancia de su centro a `cercaDe` (orden estable sobre el de `bosques`); se devuelve el primero con `ocupadas < capacidad` y un punto de trabajo válido: `{ posicion, fuenteId }`. Cuántas Leñeras hay y cuántas se cuentan: [`Economia_Construccion.md`](Economia_Construccion.md) §5.

## 11. Campamentos de mercenarios (`engine/mercenarios.ts`)

Se colocan **una vez**, al crear la partida (`colocarCampamentosIniciales(mapa, instanteDeTick(0))`), y no aparecen ni desaparecen más.

```
rng = createRng(seed XOR MERCENARIOS.salSemilla)            ← semilla derivada: NO consume el generador de la partida ni el del mapa
m   = MERCENARIOS.margenBorde
puestos = []
repetir MERCENARIOS.intentosColocacion veces:                (siempre todas: no hay salida anticipada)
    p = pegadoABosque( (m + r·(ancho − 2m),  m + r·(alto − 2m)) )          ← x primero, luego y
    si terrenoValido(p) ∧ ∀ q ∈ puestos: distancia(p, q) ≥ separacion:  puestos += p
devolver puestos.map(nuevoCampamento(índice, p, instante))
```

- `terrenoValido(p)`: dentro del mapa ∧ `esTransitable` ∧ terreno ∉ {agua, cima}.
- `pegadoABosque(p)`: recorre **todos** los bosques en orden; para cada uno `alBorde = distancia(p, centro) − radio`; lo ignora si `alBorde > margenBosque` o si ya hay un mejor con `alBorde` menor o igual. Para el elegido, el punto es `p` si ya estaba dentro (`alBorde ≤ 0`) y, si no,
  `centro + dir·(radio + pegadoAlBorde)` con `dir` el vector unitario de centro a `p` (`(1, 0)` si coinciden). Se usa solo si ese punto es `terrenoValido`; si no, **se queda `p`** (el original, no un punto intermedio). Es una preferencia, no un requisito.
- **Ficha** (`nuevoCampamento`): `huella = |floor(x)·31 + floor(y)·17 + índice|`; `militar = edificiosMilitares[huella mod 3]`; `origen = floor(huella / 3) mod origenes`; `edificios = [taberna, vivienda ×10, mercado, militar]`; `poblacion` = viviendas × `poblacionPorVivienda` (nace lleno, 100);
  `id = "mercenarios-{índice}"` con el índice **en el orden de aceptación**; `mercado = mercadoInicial()`; `fondos = {}`; `residentesIds = []`; `eligieronComoInicial = 0`. Aspecto y edificio militar salen de la posición, no del azar de la partida.
- Con la seed 1, 42 y 7 caben **5, 4 y 5** campamentos (separación 900 en 2000×2000); el número depende de qué intentos caen en tierra firme y no está garantizado.

## 12. Alijos (`engine/alijos.ts`)

Colocados una vez, justo después de los campamentos (`colocarAlijos(mapa, campamentos)`), con semilla derivada `createRng(seed XOR ALIJOS.salSemilla)`:

```
para cada campamento c (en orden), para cada banda de ALIJOS.bandas (en orden), repetir banda.cuantos veces:
    hasta 40 intentos:
        angulo = r·2π ;  radio = desde + r·(hasta − desde) ;  p = c.posicion + radio·(cos, sin)
        descartar si: fuera del mapa  ∨  no transitable  ∨  terreno = agua
                      ∨  p está en la protección de algún campamento (distancia ≤ MERCENARIOS.radioProteccion)
                      ∨  otro campamento (≠ c) está estrictamente más cerca de p que c
        si vale:  alijos += { id: "alijo-{alijos.length}", posicion: p, oro: banda.oro } ;  parar
    (si los 40 fallan, ese alijo no se pone)
```

Así la banda lejana cae en los huecos entre campamentos, y cada alijo tiene su campamento como el más cercano. Un campamento con todo colocado suma 30 de oro (3·3 + 2·5 + 1·11); con 5 campamentos salen 30 alijos y con 4, 24 en las seeds medidas.

**Abrir** (`abrirAlijo`, comando `abrirAlijo`, rol jugador): el héroe es de una Facción **sin asentamiento** o **sin Facción** (`buscaAlijos`); no lo ha abierto ya (`heroe.alijosAbiertos`); su columna está a `≤ MOVIMIENTO.radioPuerta` del alijo. Efecto: el id se añade a `alijosAbiertos` del héroe y su `oro`
se carga **en el carro de la columna** con `cargarBotin` (`min(capacidad − lo que ya lleva, oro)`; lo que no cabe **se pierde**). El alijo **no desaparece**: es **por héroe** (cada uno lo abre una vez; nadie se lo quita a otro). Verlo: `alijosALaVista` (columna a `≤ VISION.jugadorSolo`, sin abrir).
Evento `alijo.abierto { heroeId, alijoId, oro }`.

## 13. Campamentos de bandidos (`engine/bandidos.ts`)

**No hay ninguno al crear el mundo** (`campamentosBandidos = []`): aparecen por un evento programado al fin del plazo de cada plaza, y solo mientras existan asentamientos o campamentos de mercenarios que los atraigan. Los dos orígenes los evalúa `avanzarSpawnBandidos` cada tick (paso 8, consume el generador de la partida):

**Nivel** (`nivelAlAzar`): `t = r`; para los niveles 1, 2, 3 en ese orden, `t −= peso`; el primero que deja `t < 0`. Si ninguno, nivel 1. `poder` = `niveles[nivel].poder`.

**A. Uno por asentamiento, siempre.** Para cada asentamiento en orden: salta si ya tiene un campamento (`asentamientoId`) o si `instante < asentamiento.bandidosReaparecenEn`. Elige el **bosque no reclamado más cercano** a la posición del asentamiento
(bosque sin campamento ya encima — por `bosqueId` de **cualquier** campamento existente, incluidos los que se acaban de crear este tick — y con el **centro fuera de todo polígono de zona**; empate de distancia → el primero). Sin bosque, nada (y no gasta tirada). Si lo hay:
`{ id: "campamento-{asentamiento.id}", posicion: centro del bosque, bosqueId, asentamientoId, nivel }` (**una tirada** de nivel) y evento `bandidos.campamento_aparece`. Al destruirse, `agendarReaparicionBandidos` fija `asentamiento.bandidosReaparecenEn = instante + respawnMinutos`.

**B. En el anillo de cada campamento de mercenarios, según la demanda.** Para cada uno en orden: salta si `instante < bandidosEn`. `tocan = clamp(ceil(residentesSinPlaza / residentesPorBandido), minimo, maximo)`, donde `residentesSinPlaza` cuenta a los residentes cuya Facción **no tiene asentamientos** o que no tienen Facción.
Si ya hay `≥ tocan` campamentos suyos (`campamentoMercenariosId`), nada. Si no, `puntoDelAnillo`: hasta 20 intentos con `angulo = r·2π` y `radio = radioMin + r·(radioMax − radioMin)` (dos tiradas por intento), válido si está dentro del mapa, es transitable, no es agua, **no** cae en ninguna zona de influencia y no está en la
protección de ningún campamento de mercenarios. Sin punto: no se hace nada (ni se agenda), de modo que **se reintenta en el siguiente evento de aparición** y gasta tiradas cada vez. Con punto: `{ id: "bandidos-{merc.id}-{instante}", bosqueId: "", campamentoMercenariosId, nivel }` (una tirada de nivel) y
`bandidosEn = instante + reaparicionMinutos`.

**Acoso** (`avanzarAtaquesBandidos`, paso 9). Para cada caravana en orden, salvo las `disponible`, `preparando` y `aparcada`: toma el **primer** campamento (en el orden de la lista) a `≤ radioAtaqueCaravana`. No actúa si no hay ninguno, si la caravana está dentro de **cualquier** zona de influencia
(inmunidad, sin mirar de quién) o en la protección de un campamento de mercenarios. Si actúa gasta **una** tirada: `gana = poder_campamento · (1 + (2r − 1)·varianzaCombate) > defensa`, con `defensa` = `poderTotal(escuadras del ejército al que va adjunta, cohesión)` si va enganchada a una columna; si no,
`poderTotal(escolta sin héroe, cohesión)` si tiene escolta; si no, `MILITAR.defensaBaseCaravana`. Gana → la caravana **desaparece** (no hay quien reciba la carga), su escolta sin héroe vuelve a 0 al campamento de su héroe (`alCampamento`) y evento `bandidos.caravana_interceptada`. Pierde →
sigue su camino con evento `bandidos.caravana_escapa`. La escolta sin héroe sufre bajas (`aplicarBajas`) del 25 % si el bandido gana y del 5 % si no; el ejército escolta **no** sufre nada aquí.

## 14. Parámetros

Todos los de `worldgen/config.ts` están **congelados** (no se editan en caliente). Las constantes de reglas (`MERCENARIOS`, `ALIJOS`, `CAMPAMENTOS_BANDIDOS`, `REGENERACION_NODOS`, `LENERA_POR_BOSQUE`) viven en `constants.ts`; estas dos últimas y `MERCENARIOS` están en
[`Economia_Construccion.md`](Economia_Construccion.md) y [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md) (de `MERCENARIOS` solo intervienen aquí `separacion`, `margenBorde`, `intentosColocacion`, `salSemilla`, `origenes`, `edificiosFijos`, `edificiosMilitares`, `margenBosque`, `pegadoAlBorde`, `poblacionPorVivienda` y `radioProteccion`).

<!-- parametros:WORLDGEN_VERSION -->
```json
17
```
<!-- /parametros -->

<!-- parametros:MAPA_DEFAULT -->
```json
{
  "ancho": 2000,
  "alto": 2000
}
```
<!-- /parametros -->

<!-- parametros:ELEVACION -->
```json
{
  "octavas": 5,
  "escala": 0.00167,
  "lacunaridad": 2,
  "persistencia": 0.5,
  "umbralAgua": 0.375,
  "umbralCosta": 0.42,
  "umbralColina": 0.545,
  "umbralMontana": 0.6,
  "umbralCima": 0.685
}
```
<!-- /parametros -->

<!-- parametros:ELEVACION_SUAVIZADO -->
```json
{
  "octavasSuaves": 2,
  "pesoMaximo": 0.9
}
```
<!-- /parametros -->

<!-- parametros:ELEVACION_BORDE -->
```json
{
  "anchoFraccion": 0.09,
  "octavas": 2,
  "escala": 0.0012,
  "lacunaridad": 2,
  "persistencia": 0.5,
  "objetivoCosta": 0.12,
  "objetivoMontana": 0.97
}
```
<!-- /parametros -->

<!-- parametros:FERTILIDAD -->
```json
{
  "octavas": 3,
  "escala": 0.002,
  "lacunaridad": 2,
  "persistencia": 0.5
}
```
<!-- /parametros -->

<!-- parametros:BIOMA -->
```json
{
  "umbralFertilLlanura": 0.52,
  "radioHumedadRio": 40
}
```
<!-- /parametros -->

<!-- parametros:RIOS -->
```json
{
  "cantidad": 24,
  "espacioMinimoEntreNacimientos": 150,
  "pasoDescenso": 8,
  "pasoGradiente": 50,
  "gradienteMinimo": 0.0002,
  "pasosMax": 700,
  "proporcionNavegable": 0.4,
  "anchoMetros": 10,
  "profundidadMetros": 3,
  "anchoNavegableMetros": 30,
  "profundidadNavegableMetros": 6
}
```
<!-- /parametros -->

<!-- parametros:BOSQUE -->
```json
{
  "cantidad": 170,
  "radioMin": 45,
  "radioMax": 120,
  "densidadMin": 0.55,
  "densidadMax": 1
}
```
<!-- /parametros -->

<!-- parametros:BOSQUE_TERRENO_PERMITIDO -->
```json
[
  "llano",
  "colina"
]
```
<!-- /parametros -->

<!-- parametros:COLOCACION -->
```json
{
  "intentosPorPunto": 40
}
```
<!-- /parametros -->

<!-- parametros:RECURSO_RAREZA -->
```json
{
  "comun": {
    "cantidadBase": 240,
    "espacioMinimo": 20
  },
  "intermedio": {
    "cantidadBase": 80,
    "espacioMinimo": 40
  },
  "raro": {
    "cantidadBase": 24,
    "espacioMinimo": 120
  }
}
```
<!-- /parametros -->

<!-- parametros:RECURSO_TIPOS_POR_RAREZA -->
```json
{
  "comun": [
    "piedra"
  ],
  "intermedio": [
    "cobre"
  ],
  "raro": [
    "estano",
    "oro"
  ]
}
```
<!-- /parametros -->

<!-- parametros:RECURSO_CANTIDAD_NODO -->
```json
{
  "piedra": {
    "min": 600,
    "max": 1200
  },
  "cobre": {
    "min": 100,
    "max": 300
  },
  "hierro": {
    "min": 100,
    "max": 300
  },
  "estano": {
    "min": 50,
    "max": 150
  },
  "oro": {
    "min": 150,
    "max": 400
  }
}
```
<!-- /parametros -->

<!-- parametros:RECURSO_BIOMA_PERMITIDO -->
```json
{
  "piedra": [
    "colina",
    "montana",
    "llanuraFertil",
    "estepa"
  ],
  "cobre": [
    "colina",
    "montana"
  ],
  "hierro": [
    "colina",
    "montana"
  ],
  "estano": [
    "colina",
    "montana"
  ],
  "oro": [
    "montana"
  ],
  "livestock": [
    "llanuraFertil",
    "estepa"
  ]
}
```
<!-- /parametros -->

<!-- parametros:HIERRO -->
```json
{
  "rareza": "intermedio",
  "factorCantidad": 1.5
}
```
<!-- /parametros -->

<!-- parametros:ESTANO_EXTRA -->
```json
{
  "rareza": "intermedio",
  "factorCantidad": 0.5
}
```
<!-- /parametros -->

<!-- parametros:LIVESTOCK -->
```json
{
  "cantidadBase": 100,
  "espacioMinimo": 30,
  "cantidadPorManada": {
    "min": 10,
    "max": 40
  }
}
```
<!-- /parametros -->

<!-- parametros:ALIJOS -->
```json
{
  "bandas": [
    {
      "desde": 100,
      "hasta": 200,
      "cuantos": 3,
      "oro": 3
    },
    {
      "desde": 200,
      "hasta": 300,
      "cuantos": 2,
      "oro": 5
    },
    {
      "desde": 300,
      "hasta": 450,
      "cuantos": 1,
      "oro": 11
    }
  ],
  "salSemilla": 1634494826
}
```
<!-- /parametros -->

<!-- parametros:CAMPAMENTOS_BANDIDOS -->
```json
{
  "niveles": {
    "1": {
      "poder": 20,
      "unidades": 10,
      "oroPorHeroe": 30,
      "peso": 0.5
    },
    "2": {
      "poder": 60,
      "unidades": 30,
      "oroPorHeroe": 33,
      "peso": 0.3
    },
    "3": {
      "poder": 120,
      "unidades": 60,
      "oroPorHeroe": 36,
      "peso": 0.2
    }
  },
  "rendimientos": {
    "ventanaHoras": 24,
    "completas": 8,
    "caidaPorCada": 0.15,
    "soloExperienciaDesde": 14
  },
  "anillo": {
    "radioMin": 150,
    "radioMax": 250,
    "residentesPorBandido": 2,
    "minimo": 1,
    "maximo": 6,
    "reaparicionMinutos": 10
  },
  "radioAtaqueCaravana": 40,
  "respawnMinutos": 60
}
```
<!-- /parametros -->

## 15. Invariantes

| Invariante | Test que lo vigila |
|---|---|
| Misma seed ⇒ mismo mundo, bit a bit (snapshot de las seeds 1, 42 y 7 con 6 decimales); seeds distintas ⇒ mundos distintos | `engine/__tests__/worldgen_caracterizacion.test.ts` |
| Ids de nodo y de bosque únicos; todo nodo dentro del mapa y con cantidad > 0; el nº de nodos por tipo es el de la configuración; trigo y madera no generan nodos | `worldgen_caracterizacion.test.ts` |
| Bosques: cantidad, rango de radio y de densidad | `worldgen_caracterizacion.test.ts` |
| Fertilidad y elevación en [0, 1] en todo el plano (también fuera del mapa) y con variación real; terreno y bioma siempre definidos | `worldgen_caracterizacion.test.ts` |
| Hay exactamente `RIOS.cantidad` ríos con todos sus puntos dentro del mapa; solo los que desembocan pueden ser navegables | `worldgen_caracterizacion.test.ts` |
| Sin región, el mundo sale idéntico al de antes de existir las regiones; Grecia/Anatolia más montañosas y Egeo mucho más acuático que el mundo libre; los troncales (Nilo, Tigris/Éufrates) no rompen el cupo de ríos | `worldgen_caracterizacion.test.ts` |
| La fachada responde lo mismo que el acceso lineal anterior (`nodo`, `bosque`, radio, polígono, orden de generación, bosque en radio, capacidad y punto de Leñera, `bosqueParaLenera` incluido el bosque grande y lejano y el polígono vacío, fertilidad, `dentroDelMapa`) | `world/__tests__/mapa.test.ts` |
| `extraer` descuenta y devuelve lo realmente sacado, nunca toca el mundo ni el estado recibido; `estadoActual` es independiente; dos fachadas no se contaminan; fuentes inexistentes o cantidades ≤ 0 son inocuas | `mapa.test.ts` |
| Elevación, terreno, bioma y ríos de la fachada coinciden con las funciones de `worldgen`; `rioMasCercano` coincide con el recorrido lineal | `mapa.test.ts` |
| Regeneración: no antes del cooldown, a `cantidadInicial` completa al cumplirse; el ganado antes que el mineral | `world/__tests__/regeneracion_nodos.test.ts` |
| Contorno fusionado de bosques (solo dibujo) | `world/__tests__/contornos_bosques.test.ts`, `poligonos.test.ts` |
| Campamentos: separación mínima, lejos del borde y caben varios; misma seed ⇒ mismos; nunca sobre agua; pegados al borde de un bosque si lo hay cerca | `engine/__tests__/mercenarios.test.ts` |
| Alijos: los de cada banda alrededor de cada campamento, con su oro, fuera de la protección y con ese campamento como el más cercano; misma seed ⇒ mismos; ver y abrir (una vez por héroe, oro al carro, solo Facción sin asentamiento) | `engine/__tests__/alijos.test.ts` |
| Bandidos: uno por asentamiento aunque estén juntos; reaparición justo al cumplirse el plazo; anillo según residentes sin plaza, al menos uno; botín por nivel y rendimientos decrecientes | `engine/__tests__/bandidos.test.ts` |
| Un snapshot con otra `worldgenVersion` se rechaza al cargar | `server/__tests__/persistenciaPartida.test.ts` |
| La madera de una fundación se mira en la zona real, no en el círculo | `engine/__tests__/viabilidadZonaReal.test.ts` |

**Sin test propio** (solo los guarda el snapshot, es decir, un cambio se ve pero no se explica): el cálculo del ruido, el borde y el suavizado de la elevación, el descenso de los ríos, el respaldo de la colocación, `pegadoABosque`, el reintento por tick del anillo de bandidos sin sitio y el acoso a caravanas.

## 16. Casos límite y decisiones que parecen raras

- **Medidas de referencia** (`e77c2a1`, mundo libre 2000×2000, rejilla 200², seeds 1 / 42 / 7):

  | Terreno | % |
  |---|---|
  | llano | 46,8 / 46,3 / 42,0 |
  | colina | 17,3 / 16,3 / 14,7 |
  | montaña | 16,5 / 15,0 / 18,7 |
  | cima | 1,6 / 0,6 / 1,8 |
  | costa | 13,9 / 11,4 / 14,0 |
  | agua | 3,8 / 10,4 / 8,8 |

  Siempre 628 nodos (piedra 240, cobre 80, estaño 64, oro 24, ganado 100, hierro 120) y 170 bosques; 24 ríos, de los que 4 / 4 / 5 son navegables y 15 / 14 / 11 acaban en lago; 5 / 4 / 5 campamentos de mercenarios y 30 / 24 / 30 alijos.
  Los bosques nacieron siempre en llano o colina; por seed, **1–2** nodos de ganado (y alguno de cobre) quedaron fuera de su bioma o dentro de un bosque.
- **El espaciado de los raros casi nunca se cumple**: los 24 de oro, en las tres seeds, quedaron a menos de 120 de otro nodo ya colocado (en montaña: se sacrificó el espaciado); estaño 55–60 de 64; hierro 65–79 de 120; cobre 8–15 de 80; ganado 6–9 de 100; piedra 0 de 240. Es la prioridad terreno > espaciado
  de §9.1, no un fallo.
- **El primer intento válido no es el mejor**: `colocarConEspaciado` acepta el primer candidato que cumple todo, no el más separado; el resultado depende de la **secuencia** de tiradas.
- **Un mismo `x, y` se tira siempre en ese orden**; cambiar el orden (aunque sea «y primero») mueve todo el mundo.
- **`pasosMax` agotado deja el río como lago** (`terminaEnLago = true`), igual que un gradiente casi nulo: no hay un estado «sin terminar».
- **Un río de un solo punto** (nacimiento de respaldo o gradiente insuficiente) existe y se cuenta, pero no tiene segmentos: no aporta humedad y su distancia a cualquier punto es infinita.
- **La Leñera «de centro»**: el primer puesto de un bosque es siempre su centro; los siguientes giran a 0,4·radio con ángulos de 120° y 240°. Si el punto preferido cae fuera de la zona, se buscan anillos dentro del propio bosque (un bosque puede tocar la zona por un punto que no es su centro).
- **La regeneración empieza al detectar**, no al agotar. Un yacimiento que se agota y se detecta en el mismo instante tarda exactamente el cooldown; uno agotado entre dos ejecuciones del paso 7 se retrasa a la siguiente.
- **El anillo de bandidos sin sitio gasta tiradas todos los ticks** (hasta 40) mientras no encuentre un punto: es la única vía por la que el generador de la partida avanza sin que ocurra nada visible.
- **Los bandidos de un asentamiento miran el centro del bosque**, no su círculo, para decidir si está reclamado (misma simplificación que `posicionLibreParaFundar`): un bosque cuyo centro queda fuera de toda zona pero cuyo borde la toca cuenta como libre.
- **El campamento de bandidos de un asentamiento no bloquea el bosque que ocupa**; solo amenaza las caravanas que pasen a menos de 40.
- **`margenBorde` se aplica al punto sorteado, antes de pegarlo a un bosque**: pegarse al borde de un bosque puede acercarlo al perímetro del mapa unas decenas de unidades por debajo del margen (solo se exige `dentroDelMapa`). La separación mínima, en cambio, se comprueba sobre el punto **ya pegado**.
- **Fuera del mapa todo está definido**: ruido, elevación, fertilidad y bioma se pueden evaluar en cualquier punto (los muestreos en anillo se salen). `Mapa.dentroDelMapa` es la única comprobación de límites y es inclusiva.
- **El agua infranqueable no está en `costeEnPunto`**: el coste 15 del agua no se usa; el grafo descarta cualquier arista con un punto no transitable.

## 17. Divergencias

No hay divergencias abiertas. Resueltas el 2026-10-12 al escribir este documento: se borró del canon la sección de chokepoints (1.5, sin renumerar) y la línea del spawn de jugadores; la regeneración del ganado se documenta como 3 frente a 10 minutos; se aceptó que un bosque admita 2 o 3 Leñeras (nunca 1) y se actualizaron los comentarios de calibración; la tabla de oro por nivel del canon 1.9 se corrigió a 30/33/36.
