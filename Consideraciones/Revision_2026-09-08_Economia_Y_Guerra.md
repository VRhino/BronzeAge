# Revisión de sesión — 2026-09-08: economía del oro + guerra/conquista

Consolidación de todo lo trabajado en la sesión, para revisar de una pasada antes de repartir a canon.
Los detalles viven en los tres docs de definición; esto es el índice y el estado.

---

## 1. Estado por pieza

| Pieza | Estado | Doc de detalle |
|---|---|---|
| **Impuestos: recaudación de oro por población** | ✅ implementado (commit `d0510e1`) | `Economia_Del_Oro_Definicion.md` §4.1 |
| Política "Presión Fiscal" (Tesorero) | ✅ implementado | `Economia_Del_Oro_Definicion.md` §5 |
| Reclutamiento cuesta oro por escalón (salvo milicia) | ✅ implementado | ídem §4.2 |
| Buey de caravana en oro (30 madera → 12 oro) | ✅ implementado | ídem §4.3 |
| `MANTENIMIENTO.nivelParaOro` 3 → 2 | ✅ implementado | ídem §4.4 |
| Caravana #0 gratis del Mercado | ❌ probada y descartada (amplificaba la fuente) | ídem §4.3 |
| **Fix: colapso por bosque saturado** (`evaluarViabilidadFundacion` exige bosque LIBRE) | ✅ implementado | ídem §10 |
| Instrumentación de batch (`BATCH_RUINAS_DIAG`, `pctSueloOcupado`) | ✅ implementado | ídem §10 |
| **Calibración (Paso 6) del bloque de oro** | 🔶 en curso — iter 2: oroMedio ×2.2 base y trepando | ídem §10 |
| **Ocupación post-conquista** (saqueo, ventana, oro reducido) | ✅ implementado — pasos 3-9 (commits `26662b4`, `c2f21f4`); 🔶 calibración (paso 10) en curso | `Ocupacion_Post_Conquista_Definicion.md` |
| **Guarnición-ejército** (el conquistador se vuelve guarnición) | ✅ implementado (Paso 4, `26662b4`) | ídem §2.2 |
| `guarnecer` (defender plaza propia marchando) | ✅ implementado **2026-09-09** (follow-up separado del plan de ocupación) | `Ocupacion_Post_Conquista_Definicion.md` §2.3/§11 |
| **Reclutamiento desatado de residencia** (reponer/mover fuera de casa; nuevo escuadrón solo en casa) | ✅ implementado (Paso 1, commit `f844f7a`) | ídem §1 Ronda 2, §2.3b |
| **Comando `cambiarResidencia`** | ✅ implementado (Paso 2, commit `f844f7a`) | ídem §2.3c |

Leyenda: ✅ en código y commit · 🔶 en código, calibrando · ❌ descartado tras medir · 📐 diseño cerrado, sin
código.

---

## 2. Lo implementado (commit `d0510e1`) — canon YA escrito

`Docs/Game/3` §3.1 (usos del oro), §3.13.2 (buey en oro, sin caravana #0);
`Docs/Game/4` §4.1 (recaudación por clase), §4.4 (Presión Fiscal), §4.5 (`nivelParaOro: 2`);
`Docs/Game/5` §5.8 (oro de reclutamiento).

Falta cerrar en canon (tras la campaña de calibración, Paso 6): las cifras finales de todos los placeholder
(`IMPUESTOS`, `RECLUTAMIENTO_ORO_POR_ESCALON`, buey, factores de política) y el diario de la campaña.

**El fix del bosque saturado** (`evaluarViabilidadFundacion` / `Mapa.hayBosqueLibreEnRadio`) va a canon en
`Docs/Game/4` §4.2 (auto-construcción / Leñeras) y §1 (worldgen, criterio de fundación) — pendiente.

---

## 3. Mapa de canon — ✅ REPARTIDO (2026-09-08)

Todo lo de esta sección está **implementado y en canon**. Destino final entre paréntesis en cada apartado;
detalle del reparto en `Ocupacion_Post_Conquista_Definicion.md` §10.

### 3.1 Ocupación post-conquista → **`Docs/Game/5` §5.12.9 nuevo** + bullets de §5.4

- El ejército conquistador se convierte en la guarnición (`absorberColumna`). **Nunca queda a 0.**
- Saqueo: −25% pesants+artesanos (nobleza intacta); ~25% de edificios `activo` → `en_cola` dañados,
  reconstrucción al 50% de coste/tiempo (Centro Urbano + 1 Granja + 1 Leñera exentos); murallas −30% de
  `avance`; `medidorMantenimiento` a 100.
- Ventana de ocupación (`Asentamiento.ocupacionHasta`, ~90 min): inmune a nuevo asedio, recaudación ×0.5,
  crecimiento ×0.5, mantenimiento no degrada. Al vencer: la guarnición se queda, el resto vuelve a normal.
- Constante nueva `OCUPACION` (todo placeholder).

### 3.2 Guarnición-ejército → `Docs/Game/5` §5.12.4 + §5.12.9 (`guarnecer` general implementado 2026-09-09, ver §11)

"La guarnición es lo único que defiende" se matiza: **la guarnición puede ser una fuerza posada por un
ejército** (`guarnecer` — marchar a una plaza propia y volcar los escuadrones en su guarnición), no solo
tropa de residentes. Cierra el hueco "proteger una plaza propia marchando a defenderla" (Doc 2, hoy no
funciona). Un escuadrón de no-residente en una guarnición defiende, come trigo y su dueño lo repone y
re-moviliza.

### 3.3 Reclutamiento desatado de residencia → **`Docs/Game/5` §5.8** (bloque nuevo) y **`Docs/Game/2` §2.5**

| Acción | Requisito |
|---|---|
| Reclutar escuadrón NUEVO / cambiar roster | Residir en el asentamiento (sin cambios) |
| Reponer un escuadrón que ya tienes ahí (guarnición o columna) | Misma Facción + presente + el asentamiento lo permite (`politicaDeAcceso` ≠ `cerrado`, sin veto) |
| Mover escuadrones propios | Donde sea que estén — `movilizarEjercito` deja de exigir residir si tienes escuadrones vivos propios en la plaza |

Supera la regla de Doc 2.5 (2026-09-06) "reclutar únicamente en tu residencia". `Escuadron` pasa a ser por
jugador **y por asentamiento** (puedes tener el mismo `tropaId` en dos plazas). El Liderazgo sigue topando
solo lo que sacas a campaña.

### 3.4 Comando `cambiarResidencia` → **`Docs/Game/2` §2.5**

Atómico: deja la residencia actual (libera vivienda, vacía cargos locales viejos, los escuadrones posados se
quedan como guarnición de no-residente) + toma la nueva (`comprarCasa`). Precondiciones: misma Facción, hueco
de vivienda, permiso. Cooldown (`CIUDADANIA.cooldownCambioResidenciaDias`) reservado, sin implementar (necesita
jugador situado). Cierra la limitación conocida de Doc 2.5 ("no hay comando dejar residencia").

Es el prerrequisito de que un JUGADOR consolide una conquista (recluta nuevo ahí, cargos, recaudación 100%).
El NPC no lo necesita — sostiene por la guarnición-ejército.

### 3.5 Preguntas abiertas que esto RESUELVE

- `Preguntas_Abiertas.md` §1: "cómo se conquista un asentamiento enemigo exactamente" → guarnición del
  conquistador + saqueo + ventana de ocupación.
- `Movimiento_Ejercitos_Definicion.md` punto abierto #2 (cupo de nivel al conquistar) — sigue sin re-evaluarse;
  anotar que la ocupación no lo toca.
- `Preguntas_Abiertas.md` §1: "cómo se declara una guerra" → **sigue abierta**. Hoy cualquier no-aliado es
  atacable; no hay estado de guerra. Fuera del alcance de esta sesión.

---

## 4. Cómo encaja todo — la guerra como sink de oro

`Economia_Del_Oro_Definicion.md` §5: el oro debe obligar a elegir entre ejército / flota / intel. La guerra
ya es un sink parcial (reponer bajas de combate cuesta oro; reclutamientos NPC −17% al meter el coste). Lo
que el diseño de conquista añade:

- **Conquistar cuesta oro y no lo devuelve pronto:** el asedio deja bajas (→ oro para reponer), la guarnición
  de ocupación se repone de la plaza conquistada (→ su mano de obra y su almacén), y la recaudación de esa
  plaza está a la mitad durante la ventana.
- **El que gana igual acumula** (poca baja del ganador) — sigue siendo el hueco: la guerra drena al perdedor,
  no al sistema. Palancas pendientes: subir `RECLUTAMIENTO_ORO_POR_ESCALON`; y la Taberna/intel como sink
  recurrente para el que no combate.

---

## 5. Calibración del bloque de oro — dónde está (Paso 6)

Medición iter 2 (40 facciones, 1500 ticks), tras `IMPUESTOS` ÷2 + sin caravana #0 + fix de bosque:

| | Base | Iter 2 |
|---|---|---|
| `oroMedio` @1500 | 244 | **532** (×2.2, 112→259→532, sin mesetear) ⚠ |
| `vivos` | 47 | 61 |
| ruinas (eventos) | 146 | 76 (−48%) |
| muertes por madera | ~124 | 21 (−77%) |
| `pctSueloOcupado` | — | 25% (mapa lejos de lleno) |
| militar NPC (`campamentosDestruidos`) | 147 | 146 (PvE intacto) |

**Pendiente #1: el `oroMedio` sigue trepando.** Candidatos: otro recorte de `IMPUESTOS`; subir el oro de
reclutamiento; implementar la ocupación (baja la recaudación de las plazas conquistadas y drena mano de obra
al reponer guarniciones — puede ayudar de forma no trivial); Taberna/intel.

---

## 6. Puntos abiertos consolidados

| # | Abierto | Dónde |
|---|---|---|
| 1 | `oroMedio` no mesetea — falta calibración / sink recurrente | Economía §10 |
| 2 | Todas las cifras `OCUPACION.*`, `CIUDADANIA.cooldownCambioResidenciaDias` | Ocupación §8 |
| 3 | `cambiarResidencia`: ¿cooldown solo, o también coste? | Ocupación §8 |
| 4 | Ficha `Taberna_Intel_Definicion.md` — sin diseñar (sink recurrente) | Economía §4.5 |
| 5 | "Cómo se declara una guerra" / estado de guerra activa | Preguntas_Abiertas §1 |
| 6 | Revuelta de un asentamiento ocupado con felicidad baja | Ocupación §8 |
| 7 | Rotación de guarniciones del NPC (reforzar frontera con `guarnecer`) | Ocupación §8 |
| 8 | 21 muertes de madera residuales (bosque saturado DESPUÉS de fundar) | Economía §10 |
| 9 | El escuadrón mermado paga Liderazgo completo | Movimiento_Ejercitos §7 |
| 10 | Mercenarios, sueldos de tropa en oro (segundo drenaje militar) | Economía §8 |

---

## 7. Orden sugerido de trabajo a partir de aquí

1. **Cerrar la calibración conjunta oro + ocupación** — la ocupación ya está en código (pasos 3-9). Correr el
   batch con `BATCH_OCUPACION_DIAG=1`, ver si el ping-pong baja (distribución "veces conquistada → nº plazas")
   y qué le hace a `oroMedio`; iterar `OCUPACION.*` / `IMPUESTOS` / reclutamiento. Escribir el diario.
2. **Repartir a canon** — mapa en §3 de este doc y `Ocupacion §10`. Doc 5.4/5.12.4 tienen párrafos que ahora
   CONTRADICEN el código (guarnición a 0, "la ciudad no se toca", "el ejército no entra"): hay que reescribirlos.
3. **Diseñar la Taberna/intel** — el sink recurrente que le falta al bloque de oro.
