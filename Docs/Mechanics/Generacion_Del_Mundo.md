# Generación del mundo

> **Reescrito el 2026-10-13: este documento describe el generador del motor TypeScript (`typescript-final`) y pasa a ser la referencia de sus reglas, no su implementación.** En la reconstrucción el mundo es un **artefacto** inmutable por hash producido fuera de línea por el generador regional (TypeScript, de Codex) a **2.667 × 2.000** (factor 12 sobre Unity), con la **economía del mundo** del canon 1.9c: campamentos de mercenarios a 600 (8-10, centro de cada foco), tres anillos de tierra (foco hasta 150, frontera 150-250, tierra lejana), estaño y oro por **gradiente** de anillo (foco 5 %, frontera 25 %, tierra lejana 70 %, Doc 1.9c, decidido el 2026-10-13) en vetas guardadas por bandidos, bosques mezclados y cuotas exactas. El servidor Rust **carga** el artefacto y no genera nada: de este documento sigue vigente la fachada `Mapa` (stock, regeneración por evento, coste de movimiento, Leñeras) y las reglas de campamentos, alijos y bandidos con los valores calibrados (alijos 20-40 / 40-90 / 250-300; bandidos a ≥ 10 del borde de su zona; protección de campamento 10). Resueltos (2026-10-13): el oro y el estaño por gradiente de anillo (O1, Doc 1.9c) y las Leñeras por parcelas de explotación (O2, Doc 1.9c).

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

> **Implementado en Rust (2026-11-08, tanda 69, T9):** el artefacto del mundo lleva **tres capas u8 más**, todas obligatorias ([`Docs/Coordinacion/06_Contrato_de_Mundo.md`](../Coordinacion/06_Contrato_de_Mundo.md)): `fertilidad_vid` y `fertilidad_oliva`, la fertilidad (0 a 1, 255 = 1,0) de cada cultivo de las aldeas de un cultivo ([`Economia_Construccion.md`](Economia_Construccion.md) §14.0), y `region`, el `id` de la región del relieve en cada celda (con la lista `regiones` de ids y nombres). Desde el 2026-11-09 hay otras dos, `fertilidad_cebada` y `fertilidad_sesamo` (también obligatorias): la fertilidad del trigo en la tierra húmeda (humedad de 0,55 a 0,65, rampa suave), la cebada donde además es alta (altura de 0,45 a 0,48 en adelante) y el sésamo donde es baja (por debajo de esa rampa, en todo el mapa); 0 en el agua; el crate `mundo` las lee con `fertilidad_cebada_en` y `fertilidad_sesamo_en`. La vid es fértil en laderas soleadas y secas (pendiente suave a media, humedad baja a media, 0 en el agua y en las cumbres) y la oliva en tierra baja cercana a la costa (humedad baja a media, 0 en el agua); sus cifras están en `generador/src/config.ts` (`CULTIVOS`), son funciones puras del relieve (no consumen azar) y `generador/src/cultivos.ts` las evalúa. `fertilidad` (la del trigo) no cambia. El crate `mundo` las lee (`fertilidad_vid_en`, `fertilidad_oliva_en`, `region_en`) y las valida al cargar; el núcleo toma la capa del cultivo de la aldea en `zonas.rs`. La fertilidad que usa el relieve de este documento sigue siendo la del trigo.

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

> **En la reconstrucción: relieve a escala de pantalla (2026-10-30; `generador/src/relieveDetalle.ts`, `RELIEVE_DETALLE`).** Con la mesa de escalas 3D (`Consideraciones/Escalas_Mundo_3D_Definicion.md`) el autor eligió un relieve con formas que caben en una o pocas pantallas: el regional de Codex tenía la anchura de las cordilleras bien, pero por dentro era una rampa lisa. El detalle se pone **el último**, encima de `evaluarElevacion` (cauces troncales y lagos ya aplicados, antes de la costa), cuando ya existen ríos, lagos y costas, así que no mueve la geografía:
> - **Fondo:** hasta el `suelo` (0,55, 550 m) la elevación regional tal cual; por encima, `suelo + (base − suelo) · 0,35`. Crece siempre con el regional, así que un río sigue bajando.
> - **Crestas** (macizo, `suave((base − suelo) / 0,08)`): ruido «ridged» de 4 octavas con longitud de onda 42 · 2,2 u, estirado de su rango útil (0,15-0,9) a 0-1 y afilado (^1,5), modulado por un ruido de 70 · 2,2 u (0,6-1); amplitud `0,4 + 1,0 · (base − suelo)` (los núcleos de las cordilleras, más altos).
> - **Colinas** en las tierras altas y el piedemonte (de 0,47 hasta donde empieza el macizo): hasta 79 m, longitud de onda 14 · 2,2 u. **Lomas** en el llano: ±8 m, 6 · 2,2 u, dos octavas, y nada por debajo de la orilla (0,40), así que la costa no se mueve.
> - **Valles de los ríos:** todo el detalle se multiplica por `1 − campana(distancia al río / semiancho)`, con semiancho de 4 · 2,2 u en el llano a 10 · 2,2 u en el macizo: en el cauce queda solo el fondo.
> - **Pasos de las barreras:** en el corredor del collado del relieve regional (y medio radio o medio ancho más) no hay detalle.
> - Por encima de 0,95, compresión suave (`tanh`) hasta 1: no hay mesetas recortadas.
>
> **Conexión** (`conexionPorCeldas`, `generador/src/conexion.ts`). Se mide en las celdas de 4 u del artefacto, con 8 vecinos (la diagonal solo con las dos ortogonales libres). La tierra principal es la mayor componente, y una zona está conectada si tiene tierra principal a menos del 2,5 % del lado del mapa de su centro. `tieneAcceso` (lo que usan campamentos, metales, recursos y bandidos) es estar en la tierra principal. Si alguna zona queda aislada, el detalle se repite con fuerza 0,8, 0,6, 0,4 y 0 (`fuerzaDetalle`); las semillas 1, 7, 42, 99 y 2026 conectan con la fuerza entera. La rejilla de celdas se calcula una sola vez en el relieve y la reutiliza `generar-mundo`.
>
> Medido en la ventana de la mesa (semilla 42, 320 × 240 u entre la cordillera norte y el campamento‑0): en el macizo, 46 m de desnivel por pantalla de mediana y 196 m en el percentil 90 (antes 50 y 110), y el 25 % de las laderas por encima de 35° (antes 13 %). Semilla 42 completa: hash `71697908…`, 8 campamentos, 7.360 recursos, grafo de 4 u (ver `06_Contrato_de_Mundo.md` §7).

> **El relieve fino para los clientes 3D** (2026-10-30): el generador evalúa `evaluarElevacion` también cada 0,5 u en todo el mapa y lo escribe aparte, en el paquete de relieve (`relieve-<hash>.bin`, `generador/src/paqueteRelieve.ts`, en 4 hilos). Es la misma función y la misma ejecución que los rasters de 4 u del artefacto, así que no cambia el artefacto (sigue en `71697908…`) ni ninguna regla: solo lo que se dibuja. Formato y descarga en `06_Contrato_de_Mundo.md` §8.

> **Región alpina (2026-10-31; `generador/src/regionAlpina.ts`, `REGION_ALPINA`).** El autor pidió una región entera hecha montaña habitable, como Suiza: desde fuera se va subiendo y dentro hay una cadena de valles en altura, con altiplanos a varias alturas unidos por pasos. Es la región **Tierras altas secas** (noreste; unas 380 u de radio en la semilla 42). Va al final de `aplicarDetalle`, encima de todo lo demás:
> - **Peso de la región:** 1 hasta el 80 % del radio y 0 al 112 %, con el borde bamboleado (±8 %, onda de 160 u). En el anillo, el terreno alpino se mezcla con el de fuera: esa es la subida.
> - **Montaña:** un suelo de 0,60 en el borde del núcleo a 0,65 en el centro, más crestas («ridged», onda de 80 u) de hasta 0,24, afiladas con ^1,6 para que las vaguadas sean anchas.
> - **Altiplanos:** el primero en el centro de la región (allí busca la conexión su acceso) y el resto por muestreo de disco, a 82 u como mínimo entre centros y a 25 u de los lagos. Elipses de 22 a 40 u de radio mayor, suelo entre 0,58 y 0,76 (más alto hacia el centro), lomas de ±6 m y un borde corto.
> - **Pasos:** el árbol de longitud mínima entre altiplanos, así que todos quedan unidos, y un paso más hacia el vecino más cercano sin paso si está a menos de 1,45 separaciones (con probabilidad 0,6). Cada paso es una curva con meandros, con fondo de 2,5 u a cada lado, laderas de 11 u y un collado de 0,012 a 0,035 sobre la recta entre los dos suelos.
> - **Valles de entrada:** hasta 4, desde los altiplanos de fuera hacia fuera de la región, separados en ángulo y acabando en tierra. Su fondo baja del suelo del altiplano al terreno de fuera a la vez que la región se apaga, y corta lo que haya en la subida.
> - **Ríos y lagos:** siguen donde estaban. Su valle (semiancho de 9 u, 30 u alrededor de los lagos) manda donde queda más bajo. Si el terreno alpino queda por debajo de un río, el cauce conserva su cota en 1,5 u, para que el río siga bajando.
> - Los pasos de las barreras regionales que caen dentro dejan de contar: allí se pasa por los pasos propios de la región. La conexión se sigue midiendo como siempre.
>
> Medido en la semilla 42: 25 altiplanos y 36 sendas (2 de entrada). Queda cerrado por pendiente el 33 % del núcleo y el 28 % de toda la región, frente al 52 % del macizo de la cordillera norte por encima de 600 m. Con las reglas del mismo día (solo frena la pendiente, §5.4, y se funda según la pendiente, §5.2), los altiplanos se andan como el llano y se pueden fundar aunque sean «montaña» o «cima». Semilla 42 con todo: hash `b93e8c18…`, 199.494 nodos en el grafo (el 99,08 % en la componente mayor, con los 8 campamentos).

> **Costas (2026-11-01; `generador/src/costasRegional.ts`, `COSTAS_REGIONALES`, revisión `costas-2`).** El autor vio en la mesa 3D un boceto de costa con acantilados, calas y otras formaciones («no todo es playa lisa») y pidió que el generador las hiciera a lo largo de todo el mapa. Antes la costa era una línea lisa, el 73 % playa en rampa de unos 17°, y el tipo salía de una máscara fija (roca al noroeste y al norte). La costa va la última en `evaluarElevacion`, encima del detalle y de la región alpina:
> - **Línea base:** donde el relieve regional sin detalle cruza el nivel del mar (0,375), trazada cada `muestreo` (100 del generador, unas 8 u). Solo se toca lo que está a menos de `alcance` (40 u) de esa línea; más allá, nada.
> - **Tipo de cada tramo:** acantilado, costa rocosa o playa, el 30 / 25 / 45 % de la longitud (`reparto`; los umbrales salen de cuantiles por longitud). Lo decide una puntuación: un ruido de onda 110 u a lo largo de la costa (la mitad), si el tramo es cabo o bahía (la tierra que hay en un anillo de 30 u; un cuarto: el cabo, roca; la bahía, playa) y lo alta que es la tierra a 25 u tierra adentro (un cuarto: la costa alta, acantilado). Los tipos de tramos vecinos se funden en unas 5 u, sin escalón.
> - **Contorno:** la orilla avanza o retrocede con tres ruidos (onda 45, 9 y 3,5 u; hasta 9, 2,5 y 1 u) en la roca y el acantilado, y con uno suave (onda 25 u, hasta 2 u) en la playa: cabos, ensenadas y bordes rotos. Se apaga entre 15 y 25 u de la línea base.
> - **Acantilado:** pared de 30 a 55 m que sube en unas 2,4 u; detrás, la tierra vuelve al relieve de dentro entre 5 y 37 u. Al pie, el fondo cae enseguida (hasta 40 m).
> - **Costa rocosa:** escalón quebrado de 7 a 12 m en unas 3 u; fondo hasta 28 m.
> - **Playa:** berma llana de 1,6 m y 8 u de ancho, dunas de 1,5 a 5 m hasta las 16 u y una franja llana de 6 a 14 u antes de la ladera de dentro, que se aparta (hasta 30 m) en vez de allanarse; donde la tierra de detrás es alta, una rampa de 8 u la lleva hasta el agua. Fondo somero (hasta 22 m).
> - **Calas** en la roca y el acantilado (separadas 45 u o más, con probabilidad 0,75 en cada sitio apto, donde hay tierra detrás): una entrada de 5 a 9 u a lo largo de la costa y 7 a 12 u tierra adentro, con una playita al fondo y un barranco que sube desde ella.
> - **Farallones** al pie de los cabos de acantilado (grupos separados 40 u o más, con probabilidad 0,6): de 1 a 4, a 2-7 u de la orilla, de 0,8 a 2,2 u de radio y hasta 30 m.
> - **Desembocaduras y estuarios:** a menos de 8 u de donde un río llega al mar la costa es playa, y la roca y el acantilado vuelven poco a poco hasta las 30 u. Si la tierra es baja (base por debajo de 0,39-0,405), los últimos 25 u del río se abren en estuario, de 0,5 u de ancho a 4 u en la boca.
> - **Lo que no toca:** a menos de 3 u de un río (y menguando hasta las 8 u) y en los pasos de las barreras regionales junto al mar no hay contorno, roca, acantilado, dunas ni retirada: los ríos siguen bajando hasta el mar y los pasos siguen transitables. Las calas y los estuarios se apagan entre 20 y 28 u de la línea base.
>
> Medido en la semilla 42: 7.413 u de costa, 29 % acantilado, 23 % roca y 48 % playa; 48 calas, 89 farallones y estuarios en los 7 ríos que llegan al mar por tierra baja. En ventanas de prueba, la orilla (de 0 a 4 u del agua) se puede fundar en el 40-52 % (antes, el 1-2 %, por la rampa), y entre 4 y 15 u tierra adentro queda cerrado por pendiente el 17-19 % (antes, el 1-2 %): son los acantilados. Versión del generador `regional-18-costas-2+estrategico-3`; semilla 42: hash `e5c637d4…`, 8 campamentos y 8.672 recursos (ver §9 para el bosque y los metales del mismo mundo).

### 5.2 Terreno (`evaluarTerreno`)

`e < umbralAgua` → `agua`; `< umbralCosta` → `costa`; `< umbralColina` → `llano`; `< umbralMontana` → `colina`; `< umbralCima` → `montana`; si no, `cima` (en el TypeScript, inhabitable: ni se funda ni se colocan recursos; en la reconstrucción ya no, ver abajo). El agua es obstáculo
(`esTransitable(p) = terreno ≠ agua`, Doc 1.0b); en el TypeScript la cima es transitable pero cara.

> **Desde el 2026-10-31 (decisión del autor, con la región alpina): la altura ya no decide ni el paso ni dónde se funda.** La clase sigue saliendo de la altura y se usa para pintar, para los metales (colina, montaña o cima, §9), el drenaje y los recursos (ni en agua; desde el mismo día, también en la cima), pero no frena (§5.4) ni impide fundar: en la cima se funda si la pendiente lo deja (`FUNDACION.pendienteMaximaParaFundar`, `Fundacion_Y_Refundacion.md`). Transitable es lo que no es agua y no pasa de 35° (`ESCALA_TERRENO.pendienteMaximaTransitable`).

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

> **En la reconstrucción (2026-10-31, decisión del autor): solo frena la pendiente** (`generador/src/costeMovimiento.ts`, `COSTE_MOVIMIENTO` de `generador/src/config.ts`). En tierra, `coste = 1 + min(2,5; 3,6 · pendiente)`, con la pendiente en m/m medida con un paso de 1 u: llano 1; 10° (0,18) 1,63; 20° (0,36) 2,3; 35° (0,7, la máxima transitable) 3,5, lo que antes costaba una montaña llana. La clase de terreno no entra (antes multiplicaba: colina 1,8, montaña 3,5, cima 12) y el bosque tampoco, como ya pasaba. Es la capa `coste` del artefacto y el coste de las aristas del grafo, así que el motor no cambia. Semilla 42: ver §5.1 (región alpina).

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

> **En la reconstrucción el bosque es un campo continuo de densidad** (`generador/src/forestacionRegional.ts`, `densidadForestal`): `densidad = suave(t′) · aptitud`, con `t = potencial / 0,38` y el potencial de las masas, el detalle, la humedad y las riberas (fórmula de abajo, desde el 2026-10-31). **Forma de la densidad (decisión del autor del 2026-10-28, `PARAMETROS_FORESTALES`):** por encima de `codo` (0,43, donde la densidad llega a 0,4) la subida se estira (`t′ = codo + (t − codo) / estiramiento`, con `estiramiento` 2,4); por debajo, nada cambia. Y **claros**: un ruido propio (`forestacion-claros-1`, frecuencia 1/220 del generador, unas 18 u; dos octavas), que no mueve la tirada de las masas, multiplica la densidad por `1 − (1 − fondo) · suave((claros − umbral) / ancho)`, con `umbral` 0,66, `ancho` 0,06 y `fondo` 0,35: dentro de un claro, el bosque cerrado baja a 0,35. Y **bosquetes sueltos** (pedidos por el autor el mismo día para que las zonas peladas no queden sin madera): otro ruido propio (`forestacion-bosquetes-1`, frecuencia 1/240, dos octavas) da `bosquete = densidad · suave((bosquetes − umbral) / ancho) · aptitud`, con `densidad` 0,45, `umbral` 0,72 y `ancho` 0,05, y la densidad final es el mayor de los dos: dentro de un bosque no cambian nada. Medido con la semilla 42 sobre la tierra firme: la densidad de 0,7 o más pasa del 24,3 % al 11,6 %; la de 0,15 o más (hay bosque), del 47,5 % al 47,3 %; en las manchas grandes sale un claro cada unos 35 u (antes, uno cada 112); y la tierra a más de 40 u de todo bosque baja del 19,6 % al 3,5 % (a más de 60 u, del 11,7 % al 0,3 %). Que el bosque cambie también mueve los campamentos de mercenarios, que se colocan mirándolo. En el bosque cerrado (0,7 o más) no se funda ([`Fundacion_Y_Refundacion.md`](Fundacion_Y_Refundacion.md) §5).

> **Bosque en manchas (decisión del autor del 2026-10-31, como la propuesta de la mesa 3D; `PARAMETROS_FORESTALES.masas`, `.detalle` y `.pesos`).** El bosque era una alfombra: masas de ~360 u y la humedad del área con peso 0,62, así que regiones enteras quedaban cubiertas y otras peladas. Ahora son manchas densas y separadas por todo el mapa: `potencial = (masas − 0,5) · 3 + (detalle − 0,5) · 0,81 + (humedad − 0,5) · 0,3 + ribera · 0,25 − 0,13`, con las masas de un ruido de ~26 u (frecuencia 1/312 del generador, 3 octavas) y el detalle, que rompe los bordes, de uno de ~7,7 u (1/92, 2 octavas). La humedad ya no decide si hay bosque, solo cuánto: la estepa queda en torno al 15 % de la tierra con bosque y lo más húmedo en torno al 33 %. La forma de la densidad, los claros y los bosquetes siguen como arriba. Semilla 42 (hash `e5c637d4…`): hay bosque (0,15 o más) en el 23,3 % de la tierra firme (antes, el 43,9 %) y bosque cerrado (0,7 o más) en el 5,5 % (antes, el 10,8 %).

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

> **En la reconstrucción (2026-10-27, mecánicas 50 y 51): el mundo ya no tiene nodos de ganado.** El ganado sale de los rebaños de las aldeas agrícolas ([`Economia_Construccion.md`](Economia_Construccion.md) §14.0). Desde el 2026-10-30 el generador regional (`generador/src/recursos.ts` y `generador/scripts/generar-mundo.ts`) **ya no coloca ganadería**: antes la colocaba y la descartaba al escribir el artefacto para no mover la piedra. Quitarla movió la piedra (que se colocaba después) y, como la capa lejana de abajo se coloca al final y respeta la separación con la piedra, 27 yacimientos lejanos de cobre y hierro; el resto del artefacto (rasters, ríos, bosques, parcelas, campamentos, bandidos, alijos, navegación y el demás metal) sale idéntico (versión del generador `regional-17-costas-1+estrategico-2`; semilla 42: hash `4f9de7c0…`, 7.329 recursos; el hash anterior, desde el 2026-10-28 con la capa lejana y los bosques nuevos, era `43ccdaa2…` con 7.386). Desde el 2026-10-30 el artefacto gana además la capa `coste` (para el suavizado de rutas del servidor; el resto sale idéntico) y el hash de la semilla 42 pasa a `c6639fa6…`. Con el relieve a escala de pantalla y el grafo de 4 u (§5.1, el mismo día) pasa a `71697908…`. `REGENERACION_NODOS.livestock` se queda en el bloque pero ya no tiene a qué aplicarse. La tabla de arriba es la del TypeScript.

> **Capa de la tierra lejana (decisión del autor del 2026-10-28; `generador/src/recursos.ts`, `PARAMETROS_RECURSOS.capaLejana`).** El reparto por anillos del generador regional daba al cobre y al hierro 20 % foco, 80 % frontera y nada lejos, y a la piedra 60 / 40 / 0: medido con la semilla 42, el 87 % de las colinas y montañas está en la tierra lejana y solo tenía 1 cobre, 3 hierro y 17 piedra. Lo del foco y la frontera se queda como estaba (el mínimo garantizado, en los mismos sitios) y, después de colocar todos los tipos, se añade una capa solo en la tierra lejana: por cada tipo, `round(nodos del tipo en la frontera / candidatos de la frontera × densidadRelativa × candidatos lejanos)`, con `densidadRelativa` **cobre 0,25, hierro 0,25 y piedra 0,1** (el cobre y el hierro, a 1 desde el 2026-10-31, abajo), con las mismas reglas de terreno, calidad y separación que el resto. Sus ids son `<tipo>-l<n>` y su cantidad inicial es la del rango de su tipo × **1,5** (`factorCantidad`). Con la semilla 42 (hash `43ccdaa2…`, 9 campamentos): 139 de cobre, 206 de hierro y 777 de piedra lejanos; las colinas y montañas lejanas a más de 60 u de cualquier metal bajan del 66 % al 34 %.

> **Metales en la montaña lejana (decisiones del autor del 2026-10-31; `PARAMETROS_RECURSOS.capaLejana` y `pendienteMaxima`, `generador/src/mundoEstrategico.ts` `sitiosDeMetal`).** La montaña lejana (la región alpina sobre todo) se quedaba casi sin metal: la capa iba entera a los mejores sitios del mapa, y en la montaña la ladera pasaba de la pendiente máxima de un yacimiento. Tres cambios:
> - **La misma densidad que la frontera:** `densidadRelativa` del cobre y del hierro sube de 0,25 a **1**; la piedra sigue en 0,1. Los candidatos que se cuentan son los que valen para el tipo (calidad y pendiente).
> - **Reparto por zonas:** la capa se reparte en zonas cuadradas de **150 u** (`zona`), cada una en proporción a sus candidatos pesados por terreno (`pesoTerrenoMineral`: montaña y cima 2, colina 1, llano y costa 0,1; la piedra pesa 1 en todo). Se coloca siempre en la zona a la que más le falta, y dentro de ella el candidato de más peso que no esté bloqueado por la separación. Cuántos van no cambia: el peso solo decide dónde.
> - **Hasta la pendiente transitable:** una mina va en la ladera. El cobre y el hierro solo piden un sitio que se pueda pisar (hasta 0,7, la pendiente máxima transitable); la piedra sigue en 0,35 (`pendienteMaxima`). Los candidatos ya no excluyen la cima (la altura no cuenta, §5.2) y el mineral vale en colina, montaña o cima (en el llano y la costa, con `factorLlanoMineral`). Los sitios del estaño y el oro (`sitiosDeMetal`) son también colina, montaña o cima, transitables, sin otro límite de pendiente.
>
> Semilla 42 (hash `e5c637d4…`, 8 campamentos, con las costas de §5.1): 603 de cobre, 905 de hierro y 875 de piedra lejanos (antes, con el mundo `b93e8c18…`, 126, 189 y 742); en total 843 de cobre, 1.265 de hierro y 6.476 de piedra.

> **Estaño y oro: como mucho 2 vetas por región (decisión del autor del 2026-11-01, «vetas en varias sierras»; `PARAMETROS_METALES.maxVetasPorRegion`, `generador/src/metales.ts`).** El 70 % del estaño y el oro va en vetas en la tierra lejana (contrato 06; canon 1.9c), y el centro de cada veta va donde más sitios aptos de estaño hay alrededor. Sin tope, con la semilla 42 las 10 vetas caían en la sierra lejana mayor: las tres regiones altas del norte (Altiplano interior, Tierras altas secas y Piedemonte oriental), el 23 % de la tierra, tenían 50 de los 64 nodos de estaño y 18 de los 24 de oro. Ahora cada región del relieve regional (la de más peso en el centro, `regionEn`) admite como mucho 2 vetas; si con el tope no caben todas, las que faltan se ponen sin él. Semilla 42: las 10 vetas en 6 regiones (Altiplano interior, Tierras altas secas, Piedemonte oriental y Tierras del estrecho, 2 cada una; Gran llanura fluvial y Estepa abierta, 1), y el norte alto baja a 38 de 64 de estaño y 14 de 24 de oro.

> **La piedra, más separada y menos (decisión del autor del 2026-11-06; `PARAMETROS_RECURSOS.totalBase` y `separacionMismoTipo`, `generador/src/recursos.ts`).** Con las zonas de influencia al doble de radio (radio inicial 12 y techo de nivel 1 de 20 u desde el 2026-11-05, [`Economia_Construccion.md`](Economia_Construccion.md) §14.3) una plaza abarca cuatro veces más terreno, y junto a un campamento veía unas 7 piedras en sus 20 u (antes del cambio, 1,8 en 10 u). La piedra pasa a **una vez y media** de distancia: separación mínima entre dos piedras de 70 a **105** unidades del generador (de 5,8 a 8,75 u), y su total de partida, entre 2,25 (`totalBase.piedra` de 5.600 a **2.489**, escalado por área como siempre; la capa lejana, a 1/10 de la densidad de la frontera, baja sola). Los metales no cambian: mismos totales, separaciones y sitios. Semilla 42 (versión `regional-19-costas-2+estrategico-3+cauce-1`, hash `da9cc6de…`): **2.790 de piedra** (2.489 del foco y la frontera y 301 lejanas; antes 6.476, con 875 lejanas) y 4.986 recursos en total (antes 8.672); 843 de cobre, 1.265 de hierro, 64 de estaño y 24 de oro, como antes. Medido en sitios de tierra firme cada 10 u: en sus 20 u, una plaza del foco tiene de media 3,8 piedras (2 % de sitios sin ninguna) y una de la frontera 1,9 (16 % sin ninguna, lo mismo que antes del cambio de zonas con 10 u); en la zona inicial de 12 u no tiene piedra el 18 % de los sitios del foco y el 49 % de los de la frontera.

> **Nodos de estaño grandes (decisión del autor del 2026-11-10; `RANGO_CANTIDAD.estano` en `generador/src/recursos.ts` y `PARAMETROS_METALES.cantidadFoco` en `generador/src/metales.ts`).** La cantidad inicial de un yacimiento de estaño pasa de **50-150 a 300-600**, y el estaño del foco ya no lleva un tercio (`cantidadFoco` es ahora por metal: estaño 1, oro 1/3, como antes). Motivo: un nodo vacío se queda 10 minutos sin dar nada (`REGENERACION_NODOS`, [`Economia_Construccion.md`](Economia_Construccion.md) §5), y con nodos de 100 de media el pueblo de estaño sacaba de media 5,1 por minuto con su mina a 10 (2,6 el del foco) frente a los 7,2 del cobre; la escasez del estaño la da el número de nodos (64 frente a 843 de cobre), no su tamaño. Cobre, hierro, oro y piedra no cambian, y tampoco las posiciones: la cantidad sale de su propio generador derivado del id. Semilla 42 (versión `regional-20-costas-2+estrategico-3+cauce-1`, hash `c83ef5e2…`): los 64 de estaño entre 302 y 596 (media 466; el foco, 448); a 10 por minuto, de media por minuto: estaño 8,3, cobre 7,2, hierro 7,3, oro 7,4 y piedra 9,1 (antes, el estaño 5,1). Con los niveles del extractor (§14 de Economia_Construccion), a 20 y 30 por minuto: estaño 14,1 y 18,5; cobre 11,4 y 14,2.

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

> **En Rust** (paridad P8, 2026-10-27) no hay evento ni barrido: el nodo guarda desde cuándo está vacío y vuelve a su cantidad inicial en la primera extracción pasado el cooldown, que es cuando se emite `mapa.yacimiento_regenerado { nodo, recurso }` (Economia_Construccion §5).

### 10.2 Consultas espaciales

- Índice de celdas de **100×100**: los nodos se apuntan en la celda de su posición; los bosques, en **todas** las celdas que toca su caja (`floor((cx − r)/100) … floor((cx + r)/100)`, igual en y).
- `nodosEnRadio(c, r, opciones)`: nodos de las celdas de la caja `[c ± r]` con `distancia ≤ r` (inclusive). `nodosEnPoligono`: caja envolvente + `pointInPolygon` (par-impar). Ambos devuelven los nodos **en orden de generación** (se reordena tras recoger celdas) y después aplican `opciones`:
  `tipo`, `conStock` (stock > 0), `excluir` (ids) y `ordenarPorCercaniaA` (orden estable por distancia: a igual distancia, el de menor orden de generación). **El orden de generación es contrato**: sin él, dos nodos a igual distancia se desempatarían distinto y la simulación divergiría.
- `mejorPorFertilidad(candidatos)`: el de mayor fertilidad; empate → el primero (comparación estricta `>`, parte de −1). `rioMasCercano(p)`: el río con menor distancia al trazo; empate → el primero; `null` sin ríos. `dentroDelMapa(p)`: `0 ≤ x ≤ ancho ∧ 0 ≤ y ≤ alto` (inclusive).
- `hayBosqueEnRadio(c, r)`: algún bosque con `distancia(centro, c) < r + b.radio` (estricto: el criterio único de «el borde del bosque entra en el círculo»). `hayBosqueLibreEnRadio` añade que el bosque tenga Leñeras `< capacidadLeneras`.

### 10.3 Leñeras en un bosque

> **Nota (2026-10-13):** en la reconstrucción la capacidad por radio de bosque (`LENERA_POR_BOSQUE`, `capacidadLeneras`) se sustituye por **parcelas de explotación** (Doc 1.9c y 4.2; sin cuota por asentamiento desde el 2026-11-06). Lo que sigue describe el motor TypeScript.

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
  > Implementado en Rust (2026-10-20, fase 4B-1): el edificio militar (`militar = edificiosMilitares[huella mod 3]`, con `⌊x⌋` y `⌊y⌋` en unidades de mapa) se **deriva** en `Nucleo::tropas_del_campamento` desde la posición del artefacto; el artefacto no lo guarda.
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

> **En la reconstrucción los alijos los pone el artefacto** (C6 del plan del sistema 6; `generador/src/contenidoVivo.ts`): uno lejano junto a cada veta de estaño y oro (`alijo-<veta>`) y, por cada campamento de mercenarios, 3 a 20-40 u y 2 a 40-90 u (`alijo-<campamento>-<n>`), con ese campamento como el más cercano. **Cada alijo trae una sola cosa** (decisión del autor del 2026-10-28; antes, solo oro: 3 / 5 / 11): oro, madera, piedra o trigo, las cuatro con la misma probabilidad, con su propio generador derivado (`contenido-alijos-1`), así que el contenido no mueve las posiciones. Cuánto, por banda (cercano / medio / lejano): oro **15 / 25 / 55**; madera, piedra o trigo **5 / 10 / 15**. En el artefacto, `Alijo.recurso` y `Alijo.cantidad` (milésimas). El bloque `ALIJOS` de abajo dice lo mismo (sus `bandas`, con `oro` y `recurso`, y `contenido`); no lo lee nadie: el generador lleva las mismas cifras en `PARAMETROS_CONTENIDO`. El artefacto real de hoy (semilla 42, `43ccdaa2…`) trae 55: 9 de oro, 14 de madera, 20 de piedra y 12 de trigo.
>
> **Implementado en Rust (2026-10-25, sistema 6, tanda 6F).** Las reglas puras están en `tecnologia::alijos` (6A: `a_la_vista`, `abrir`) y el núcleo en `crates/nucleo/src/nucleo/alijos.rs`: los alijos del artefacto con una rejilla de 10 u (la de la economía) en la geografía del núcleo, que no es estado; cada héroe guarda los que abrió por índice del artefacto (`HeroeNucleo::alijos_abiertos`, foto `BAN36`). `AbrirAlijo { heroe, alijo }` (174; contrato `abrir_alijo` con el héroe de la cuenta y el id del artefacto): en este orden, `alijo.desconocido`, `alijo.no_puedes_buscar` (el héroe es de una Facción con alguna plaza), `alijo.ya_abierto` y `alijo.lejos` (su columna no está en el mapa o está a más de `radioPuerta` del alijo; dentro de un lugar no se abre). Lo que trae (oro, madera, piedra o trigo; desde el 2026-10-28), **en unidades enteras hacia abajo** (C3b), va al carro de la columna con `cargar_botin_en`: lo que no cabe **se pierde** y el alijo queda abierto igual, también con el carro lleno (decisión 10 del plan); en una columna con varios héroes cada uno abre el suyo y lo que trae va al carro común. El hecho es `alijo.abierto { heroe, alijo (id del artefacto), recurso, cantidad, cargado }` (hasta el 2026-10-28, `oro` en lugar de `recurso` y `cantidad`), solo para el héroe que lo abre (no lo oyen los demás de su columna). «A la vista» es la consulta `Nucleo::alijos_a_la_vista` (columna en el mapa a `VISION.jugadorSolo`, sin abrir, solo si puede buscar); su proyección en `HEROE` es de 6G. Sin tick: nada programa eventos. `Heroe.alijos_abiertos` viaja en `HEROE` y `ADMIN` con los ids del artefacto.
>
> **Implementado en Rust (2026-11-12, entrada 70): los objetos del alijo.** Quien lo abre recibe además la tabla `alijo` de `BOTIN_OBJETOS` ([`Objetos_Del_Heroe.md`](Objetos_Del_Heroe.md) §3.7): 3 crisoles, un plano de tier III (70 %) o IV (30 %) y una medalla grande de tropa, tirada con el rng del estado. Va a su mochila y, si no cabe, a su almacén de objetos; si tampoco, se pierde (D17 y D22), y lo cuenta el hecho `heroe.botin` (origen `alijo`), que también oye solo él. Prueba: `nucleo/tests/alijos.rs::quien_abre_un_alijo_recibe_su_tabla_de_objetos_en_la_mochila`.

## 13. Campamentos de bandidos (`engine/bandidos.ts`)

> **Implementado en Rust (2026-10-19, fase 4A-2), solo la parte de decisión:** `crates/combate::bandidos` (nivel al azar, plaza que recibe campamento, campamentos que tocan al anillo, punto del anillo con una validación que pone el mapa, acoso a una caravana, ataque de una columna, botín con rendimientos decrecientes y reaparición). El sitio (el bosque no reclamado más cercano, a ≥ 10 del borde de la zona) y los eventos de aparición son del núcleo. El punto del anillo se sortea **como el TypeScript, por ángulo y radio** (primero el ángulo, en grados enteros con la tabla de senos del trazado, `trazado::cos_sen_1000`; después el radio, uniforme entre `radioMin` y `radioMax`; enteros y reproducible; decisión del autor, 2026-10-23).

> **Implementado en Rust (2026-10-20, fase 4D): los bandidos en el espacio** (`nucleo/bandidos.rs`). Un campamento es una **entidad fija de la rejilla de encuentros** (`EstadoCombate.bandidos`, ids propios, foto `BAN12`). Nacen con la partida los del artefacto (`bandidos_iniciales`, junto a las vetas; **no reaparecen**) y la primera cita de cada anillo; las plazas reciben el suyo al fundarse; los de administración (`ColocarBandidos`) sirven para escenarios. Cada aparición, reaparición y renovación es un evento de prioridad 7 (ver `Reloj_Y_Eventos.md`). El acoso es el cruce exacto de la rejilla con radio `radioAtaqueCaravana` y se mira al vencer: una caravana parada, dentro de cualquier zona o en la protección de un campamento de mercenarios no es presa; una columna con caravanas adjuntas las defendía con su poder (una tirada por caravana, en orden de id) sin sufrir bajas; **desde el 2026-11-06 es una emboscada**: un combate con bajas para la columna, y si la columna o la escolta gana, el campamento cae ([`Combate.md`](Combate.md) §7.2). La ruina de una plaza se lleva su campamento sin reaparición. **Decisiones del autor del 2026-10-23:** el sitio de la plaza exige el centro del bosque a **≥ `margenZonaPlaza` del borde del disco** de toda zona (que contiene a su polígono; **5 u desde el 2026-11-05**, decisión del autor para que aparezcan más cerca; antes 10, escrito en el código); sin bosque que valga no hay tirada, y la plaza **vuelve a buscarlo en cuanto uno queda libre o válido** (se va un campamento de bandidos que lo ocupaba o una plaza cae en ruinas y su zona desaparece): la primera que espera, por id, en ese instante, y si lo encuentra, la siguiente, sin tick; la cita del anillo **se rehace al cambiar la demanda** (llega o se va un residente del campamento, o una Facción se queda sin plazas), y con la demanda cubierta no queda cita. // DUDA: (4) sin punto válido en el anillo se reintenta a los `reaparicionMinutos` y se gastan las tiradas de los intentos; (5) el acoso se tira **una vez por cruce** (regla desde el 2026-10-27, D4) (el TypeScript, cada tick mientras la caravana estuviera dentro del radio): una caravana que se salva no vuelve a ser acosada por ese campamento hasta que cambie de plan; y una caravana enganchada a una columna que ya está dentro del radio no se acosa hasta que la columna se mueva.
>
> **Implementado en Rust (2026-10-22, quinta ronda de Entrar con todo): el acoso con servidores de batalla.** Si la caravana va adjunta a una columna con algún héroe sano y el proceso tiene servidores de batalla, el cruce no tira: abre una batalla de caravana en Unity (los bandidos atacan, los héroes de la columna defienden; [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §4) y ahí acaba el cruce (la columna queda apartada; las demás caravanas adjuntas no se tiran). Sin héroes o sin servidores, la tirada de siempre. Un campamento no vuelve a acosar a una columna mientras se conserve su batalla de acoso: al volver al espacio, el redondeo de su posición la contaría como una entrada nueva en el radio.

> **Además, en la reconstrucción** (decidido el 2026-10-17: el generador regional es la referencia): el artefacto trae un campamento de bandidos de nivel 2-3 sin plaza dueña y un alijo lejano **por cada veta** de estaño y oro (Doc 1.9c), colocados por el generador, no por estos eventos. Lo de abajo describe solo los dos orígenes del motor TypeScript (por asentamiento y anillo de cada campamento de mercenarios).

**No hay ninguno al crear el mundo** (`campamentosBandidos = []`): aparecen por un evento programado al fin del plazo de cada plaza, y solo mientras existan asentamientos o campamentos de mercenarios que los atraigan. Los dos orígenes los evalúa `avanzarSpawnBandidos` cada tick (paso 8, consume el generador de la partida):

**Nivel** (`nivelAlAzar`): `t = r`; para los niveles 1, 2, 3 en ese orden, `t −= peso`; el primero que deja `t < 0`. Si ninguno, nivel 1. `poder` = `niveles[nivel].poder`.

**A. Uno por asentamiento, siempre.** Para cada asentamiento en orden: salta si ya tiene un campamento (`asentamientoId`) o si `instante < asentamiento.bandidosReaparecenEn`. Elige el **bosque no reclamado más cercano** a la posición del asentamiento
(bosque sin campamento ya encima — por `bosqueId` de **cualquier** campamento existente, incluidos los que se acaban de crear este tick — y con el **centro fuera de todo polígono de zona**; empate de distancia → el primero). Sin bosque, nada (y no gasta tirada). Si lo hay:
`{ id: "campamento-{asentamiento.id}", posicion: centro del bosque, bosqueId, asentamientoId, nivel }` (**una tirada** de nivel) y evento `bandidos.campamento_aparece`. Al destruirse, `agendarReaparicionBandidos` fija `asentamiento.bandidosReaparecenEn = instante + respawnMinutos`.

> **En Rust, desde el 2026-11-02 (decisión del autor: «10 de golpe cada 2 horas»): hasta `porPlaza` (10) por plaza, de golpe.** Al fundarse, la plaza recibe los diez a la vez, cada uno en el bosque no reclamado más cercano que quede (con su tirada de nivel y su `bandidos.campamento_aparece`). Al destruirse uno, si la plaza no tiene ya una oleada pendiente, se programa a `respawnMinutos` (**120**, 2 h); al vencer, repone **de golpe** todos los que falten entonces, aunque hayan caído varios entretanto (los que caen con la oleada pendiente los repone la misma). Sin bosque que valga, la plaza se queda con los que tenga y espera, sin gastar tirada, a que uno quede libre o válido. Antes: uno por plaza, y a las 6 h. Prueba: `combate::bandidos::una_plaza_recibe_de_golpe_los_que_le_faltan_hasta_diez_cuando_vence_su_oleada`.

**B. En el anillo de cada campamento de mercenarios, según la demanda.** Para cada uno en orden: salta si `instante < bandidosEn`. `tocan = clamp(ceil(residentesSinPlaza / residentesPorBandido), minimo, maximo)` en el TypeScript; **en Rust, desde el 2026-10-27 (decisión del autor: más bandidos y más cerca de los campamentos), `tocan = clamp(residentesSinPlaza × bandidosPorResidente, minimo, maximo)` = 4 por residente, **de 5 a 10** (desde el 2026-11-03; del 2026-10-28 al 2026-11-03, de 8 a 24; el 2026-10-27, 2 por residente, de 4 a 12), en un anillo de 12 a 25 u (antes 1 por cada 2 residentes, de 1 a 6, a 30-50 u)**. Desde el 2026-10-28 el generador ya no pone bandidos fijos alrededor de los campamentos de mercenarios (ponía 3 a 30-50 u): todos los del anillo los crea el motor; donde `residentesSinPlaza` cuenta a los residentes cuya Facción **no tiene asentamientos** o que no tienen Facción. **Mínimo de golpe (Rust, 2026-11-03, decisión del autor: «siempre haya como mínimo 5 bandidos en los anillos, hasta un máximo de 10»):** si el anillo tiene menos de `minimo`, la cita crea de una vez los que faltan hasta él (cada uno con su `puntoDelAnillo` y su tirada de nivel; si uno no encuentra sitio, se para ahí y se reintenta en la siguiente); con `minimo` o más, uno por cita, como antes (`combate::bandidos::cuantos_crear_en_el_anillo`). La primera cita de cada anillo vence al nacer el mundo, así que la partida empieza con 5 en cada uno.
Si ya hay `≥ tocan` campamentos suyos (`campamentoMercenariosId`), nada. Si no, `puntoDelAnillo`: hasta 20 intentos con `angulo = r·2π` y `radio = radioMin + r·(radioMax − radioMin)` (dos tiradas por intento), válido si está dentro del mapa, es transitable, no es agua, **no** cae en ninguna zona de influencia y no está en la
protección de ningún campamento de mercenarios. Sin punto: no se hace nada (ni se agenda), de modo que **se reintenta en el siguiente evento de aparición** y gasta tiradas cada vez. Con punto: `{ id: "bandidos-{merc.id}-{instante}", bosqueId: "", campamentoMercenariosId, nivel }` (una tirada de nivel) y
`bandidosEn = instante + reaparicionMinutos`.
**Su botín solo lo cobran los héroes sin plaza** (calibración 2, Cal-5, decisión del autor del 2026-10-24; en Rust, no en el TypeScript): los mismos `residentesSinPlaza` que piden estos campamentos, es decir, residentes de un campamento de mercenarios cuya Facción no tiene asentamientos o que no tienen Facción. Cualquier otro héroe que destruya uno gana solo experiencia, y esa destrucción no cuenta en sus rendimientos decrecientes ([`Combate.md`](Combate.md) §7.2). Los campamentos de los orígenes A (por asentamiento) y de las vetas pagan a todos.

**Acoso** (`avanzarAtaquesBandidos`, paso 9). Para cada caravana en orden, salvo las `disponible`, `preparando` y `aparcada`: toma el **primer** campamento (en el orden de la lista) a `≤ radioAtaqueCaravana`. No actúa si no hay ninguno, si la caravana está dentro de **cualquier** zona de influencia
(inmunidad, sin mirar de quién) o en la protección de un campamento de mercenarios. Si actúa gasta **una** tirada: `gana = poder_campamento · (1 + (2r − 1)·varianzaCombate) > defensa`, con `defensa` = `poderTotal(escuadras del ejército al que va adjunta, cohesión)` si va enganchada a una columna; si no,
`poderTotal(escolta sin héroe, cohesión)` si tiene escolta; si no, `MILITAR.defensaBaseCaravana`. Gana → la caravana **desaparece** (no hay quien reciba la carga), su escolta sin héroe vuelve a 0 al campamento de su héroe (`alCampamento`) y evento `bandidos.caravana_interceptada`. Pierde →
sigue su camino con evento `bandidos.caravana_escapa`. La escolta sin héroe sufre bajas (`aplicarBajas`) del 25 % si el bandido gana y del 5 % si no. *En Rust desde el 2026-11-06*, el ejército escolta también combate y sufre bajas, y si la escolta o el ejército ganan el campamento cae ([`Combate.md`](Combate.md) §7.2).

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
      "desde": 20,
      "hasta": 40,
      "cuantos": 3,
      "oro": 15,
      "recurso": 5
    },
    {
      "desde": 40,
      "hasta": 90,
      "cuantos": 2,
      "oro": 25,
      "recurso": 10
    },
    {
      "desde": 250,
      "hasta": 300,
      "cuantos": 1,
      "oro": 55,
      "recurso": 15
    }
  ],
  "contenido": ["oro", "madera", "piedra", "trigo"],
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
      "botinPorHeroe": 5,
      "monedasPorHeroe": 50,
      "peso": 0.5
    },
    "2": {
      "poder": 60,
      "unidades": 30,
      "botinPorHeroe": 7,
      "monedasPorHeroe": 75,
      "peso": 0.3
    },
    "3": {
      "poder": 120,
      "unidades": 60,
      "botinPorHeroe": 10,
      "monedasPorHeroe": 100,
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
    "radioMin": 12,
    "radioMax": 25,
    "bandidosPorResidente": 4,
    "minimo": 5,
    "maximo": 10,
    "reaparicionMinutos": 10
  },
  "radioAtaqueCaravana": 7,
  "porPlaza": 10,
  "margenZonaPlaza": 5,
  "respawnMinutos": 120
}
```
<!-- /parametros -->

> **2026-10-19 (fase 4A-2):** este bloque lleva ya los valores **calibrados** de `Calibracion_Distancias_Definicion` §4.3 (anillo de los campamentos de mercenarios 30-50 en vez de 150-250; `radioAtaqueCaravana` 7 en vez de 40), que el motor TypeScript no tenía; el crate `combate` lo lee tal cual.
>
> **2026-10-24 (calibración del sistema 4, M4):** `respawnMinutos` pasa de 60 a **360** (6 h): el campamento de bandidos de una plaza reaparece seis horas después de destruirse (decisión del autor tras el batch militar: con 60 min, 2.247 campamentos destruidos en 30 días). El anillo (`anillo.reaparicionMinutos`) no cambia.
>
> **2026-11-02 (decisión del autor):** `porPlaza` **10** (antes uno por plaza, sin parámetro) y `respawnMinutos` de 360 a **120** (2 h): la oleada repone de golpe los que falten a la plaza (§13 A).
>
> **Implementado en Rust (2026-11-08, tanda 69, T1):** el campamento destruido **ya no da oro**. `niveles.N.oroPorHeroe` (30 / 33 / 36) desaparece y lo sustituyen `botinPorHeroe` (5 / 7 / 10 **de cada uno** de madera, piedra y trigo) y `monedasPorHeroe` (50 / 75 / 100 monedas del héroe); los `rendimientos` (24 h, 8 completas, −15 % cada una, solo experiencia desde la 14.ª) se aplican a los dos. Detalle del cobro y de los hechos en [`Combate.md`](Combate.md) §7.2 (`combate::bandidos::botin_de_bandidos`). Los alijos (`ALIJOS`) siguen dando oro.

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
- **El campamento de bandidos de un asentamiento no bloquea el bosque que ocupa**; solo amenaza las caravanas que pasen a 7 o menos (`radioAtaqueCaravana`, bloque `CAMPAMENTOS_BANDIDOS`).
- **`margenBorde` se aplica al punto sorteado, antes de pegarlo a un bosque**: pegarse al borde de un bosque puede acercarlo al perímetro del mapa unas decenas de unidades por debajo del margen (solo se exige `dentroDelMapa`). La separación mínima, en cambio, se comprueba sobre el punto **ya pegado**.
- **Fuera del mapa todo está definido**: ruido, elevación, fertilidad y bioma se pueden evaluar en cualquier punto (los muestreos en anillo se salen). `Mapa.dentroDelMapa` es la única comprobación de límites y es inclusiva.
- **El agua infranqueable no está en `costeEnPunto`**: el coste 15 del agua no se usa; el grafo descarta cualquier arista con un punto no transitable.

## 17. Divergencias

No hay divergencias abiertas. Resueltas el 2026-10-12 al escribir este documento: se borró del canon la sección de chokepoints (1.5, sin renumerar) y la línea del spawn de jugadores; la regeneración del ganado se documenta como 3 frente a 10 minutos; se aceptó que un bosque admita 2 o 3 Leñeras (nunca 1) y se actualizaron los comentarios de calibración; la tabla de oro por nivel del canon 1.9 se corrigió a 30/33/36.

## Decisiones del autor del 2026-10-27 (paridad con TypeScript)

Confirmadas por el autor al repasar las dudas de la paridad (plan «Paridad con TypeScript: plan de tandas», D1-D16): describen lo que ya hace el motor y pasan a ser la regla.

- **D4, el acoso de los bandidos:** se cuenta **una vez por cruce**. Una caravana que ya está dentro del radio cuando aparece el campamento no es acosada hasta que salga y vuelva a entrar; la que se salva no vuelve a ser acosada por ese campamento hasta que cambie de plan.

## Catálogo de eventos

Los códigos de los hechos de dominio de este documento (4H T5, 2026-10-23; contrato 05 §8). La prueba `crates/nucleo/tests/catalogo.rs` lee el bloque de cada documento de `Docs/Mechanics` y exige que todo código que emite el núcleo esté en alguno, y que los que el núcleo aún no emite figuren como paridad pendiente en `Docs/Mecanicas a desarrollar.md`. Todos se emiten: `mapa.yacimiento_regenerado` desde la paridad P8 (2026-10-27), en el paso de la plaza que vuelve a extraer del nodo (Economia_Construccion §5).

<!-- eventos -->
```json
[
  "bandidos.campamento_aparece", "bandidos.campamento_disperso", "bandidos.campamento_vencido", "bandidos.caravana_escapa", "bandidos.caravana_interceptada", "mapa.yacimiento_regenerado"
]
```
<!-- /eventos -->
