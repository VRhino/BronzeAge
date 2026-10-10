# Combate con números: encuentros, asedio y conquista

## 1. Alcance y fuentes

Cubre cómo se **resuelve** un choque cuando el servidor lo calcula con números (sin Unity): columna contra columna, columna contra caravana, columna contra campamento de bandidos, ejército contra plaza (asedio), y lo que pasa cuando una plaza **cae** (conquista, saqueo, desalojo). El movimiento previo (llegar, perseguir, alcanzar a 15) está en [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md).

- **Módulos:** `src/engine/combate.ts` (resolución), `src/engine/ejercitos.ts` (`atacarColumna`, `interceptar`, `atacarCampamento`, `asediarPlaza`, `trasDerrota`, `cargarBotin`, validaciones, `enBatalla`), `src/engine/tropa.ts` (`poderEscuadron`, `defensaDe`, `alTopeDeBatalla`), `src/engine/chatarra.ts`, `src/engine/heroe.ts` (heridos), `src/session/comandos/interaccion.ts` (comando `atacar`).
- **Verificado contra:** `f1697af` (2026-10-10).
- **Canon:** Doc 5.2 (combate), 5.12.3–5.12.4 (encuentros y asedio), 5.12.9 (conquista), 5.15 (batalla, guarnición), 5.16 (héroe, bajas, heridos).
- **No es el combate de Unity.** Con servidores de batalla y al menos un héroe humano, `atacar` no resuelve nada aquí: abre una batalla de Unity (`session/batallas.ts`, ciclo de batalla del contrato) y el resultado se aplica cuando llega. Este documento es lo que corre **sin** ellos (bots contra bots, partidas sin servidor de batalla, y todo lo que Unity no cubre aún). Las consecuencias de ganar o perder (heridos, botín, conquista, desalojo) son las mismas por los dos caminos y están aquí.

## 2. La orden: `atacar`

Un comando, un objetivo: `ejercito` · `caravana` · `campamento` (de bandidos) · `asentamiento`.

1. El héroe debe estar **sano** (`exigirSano`) y en una columna.
2. **Protección de campamento de mercenarios** (`radioProteccion`): nadie inicia un combate a menos de ese radio de un campamento, ni desde allí ni contra lo que esté allí.
3. Si hay servidores de batalla (`ctx.batallasEnUnity`) y hay humano en alguno de los bandos → se abre batalla de Unity y termina aquí.
4. Si no, se resuelve según el objetivo (§4–§7), y el comando aplica las consecuencias.

Llegar a un sitio **no** ataca; atacar siempre se ordena (humano o bot).

## 3. Quién combate

### 3.1 Un ejército que ataca (`enBatalla`)
De la columna entran **las escuadras de los héroes sanos**, y de ellos solo los `tope` mejores (`BATALLA.capacidad.resto` para encuentros/caravanas/bandidos, `.asedio` para asedios; hoy 5 y 5): orden por **nivel de héroe descendente, luego poder de sus escuadras descendente, luego id**. El resto de escuadras se **aparta**: no combaten ni sufren bajas, y se devuelven intactas a su columna (`conApartadas`). Las de un héroe herido no entran (Doc 5.16.4).

### 3.2 Quién defiende una plaza (`defensaDe`)
- **La guarnición** (escuadras con `enGuarnicion` de los campamentos de los residentes) — sin héroe, siempre defiende, incluso si su dueño está herido.
- Los **residentes que defienden en persona** (`heroesQueDefienden`): residentes **dentro** de la plaza, **sanos** y **conectados** (`!fuera`); con tope `BATALLA.capacidad.asedio` (mismo orden que §3.1). Aportan las escuadras de su **loadout activo** que tengan en el campamento. El resto del campamento no defiende.
- Una escuadra aniquilada (`cantidad ≤ 0`) no cuenta.

### 3.3 Poder de una escuadra
```
poderEscuadron = poderBase(tropa) × cantidad × (1 + nivel × bonusPoderPorNivelEscuadra)
```
`poderBase` sale del catálogo `TROPAS_RECLUTABLES`. El nivel de escuadra (1‥`nivelMaximoEscuadra`) sube con experiencia (`experienciaParaSubirEscuadra`); la tropa **prestada** por un campamento de mercenarios no gana experiencia.

## 4. La resolución (`resolverCombate`)

Es **un solo motor** para encuentro, asedio, campamento y caravana (con variantes en §5, §6, §7).

```
jitterA = 1 + (rng()·2 − 1) · varianzaCombate        // dos tiradas, A primero
jitterD = 1 + (rng()·2 − 1) · varianzaCombate
poderA  = Σ poder(atacantes)                          × jitterA
poderD  = Σ poder(defensores) × (1 + bonusCohesionPorEscuadronExtra·(nDef − 1)) × jitterD × multiplicadorDefensor × ventajaDefensor
ganador = poderA > poderD ? atacante : defensor       // empate exacto: gana el defensor
ratio   = min(poderA, poderD) / max(poderA, poderD, 1)
bajasGanador  = 0,05 + 0,15 · ratio
bajasPerdedor = 0,30 + 0,40 · (1 − ratio)
```
- La **cohesión** (`bonusCohesionPorEscuadronExtra`) solo la lleva el **defensor** y solo si hay más de una escuadra; el atacante nunca.
- `ventajaDefensor` (1,05) y el `multiplicadorDefensor` (muralla, §6) multiplican solo al defensor.
- Con `atacantes` o `defensores` vacíos lanza `CombateInvalidoError`.
- **Bajas permanentes** (`aplicarBajas`): cada escuadra pierde `round(cantidad × fracción)` (mínimo 0 de cantidad). La escuadra **persiste como identidad** con `cantidad: 0`. Además cada escuadra gana experiencia: `experienciaGanadaPorVictoria` si su bando ganó, `experienciaGanadaPorDerrota` si no.
- **Consume exactamente dos `rng()`** por combate, en ese orden. Las ramas sin combate (plaza sin defensores, protegida) **no consumen aleatoriedad**: una partida sin asedios hace las mismas llamadas al RNG que antes de existir los asedios.
- **Combate digno** (`esCombateDigno`): `min(poderA, poderD) ≥ NIVEL_FACCION.ratioCombateDigno × max(poderA, poderD)`. Solo un combate digno da experiencia de Facción.
- **Chatarra** (Doc 4.2.1): por cada soldado muerto, de cada bando, el metal de su equipo × `CHATARRA.fraccion`; se suma y la recoge quien gana.
- Evento `combate.resuelto` con poderes, tropas e informe de bajas por escuadra de cada bando.

### 4.1 Experiencia de Facción por combatir
`xpDeCombate = NIVEL_FACCION.xp.combate × (héroes DISTINTOS de las escuadras que participaron en ese bando)`, solo si el combate fue digno; se aplica a la Facción de cada bando. (Una caravana sin escolta no suma por su lado: su defensa es una cifra fija.)

## 5. Encuentro entre columnas (`atacarColumna` / `encuentroEntreEjercitos`)

Validaciones (`validarAtaqueAColumna`, compartidas con la apertura de batalla Unity): no la propia columna; **no a los tuyos ni a un aliado**; **misma clase** (personal contra personal, ejército contra ejército); atacante a ≤ `LOGISTICA.radioEncuentro` (15) con héroe sano; el defensor con algún héroe sano.

1. Cada bando pasa por `enBatalla`. El atacante es el «atacante» del motor y el otro el «defensor»; **sin muralla** (multiplicador 1) y, por tanto, el defensor en campo abierto conserva la cohesión y la ventaja de 1,05 (el motor no distingue).
2. **El atacante solo gana si el defensor se queda sin un soldado en pie** (`gano = el atacante conserva soldados y el defensor ninguno`). En cualquier otro resultado —incluido que `resolverCombate` diera la victoria de poder al atacante pero el defensor siga con hombres— **el perdedor es el atacante**. Un test lo fija así («un ataque que no tumba al defensor es una derrota del atacante»). Como el perdedor de poder pierde entre el 30 % y el 70 % de cada escuadra, aniquilar a un defensor con escuadras grandes es raro: **en la práctica atacar una columna casi siempre cuesta heridos y media carga al atacante**. Ver [`Divergencias.md`](Divergencias.md).
3. **Secuela de la derrota** (`trasDerrota`): el perdedor pierde `MOVIMIENTO.fraccionRobada` (la mitad) de **cada recurso de su carro**; los héroes del perdedor (`vencidos` = **todos** sus participantes, hayan combatido o no) quedan **heridos**. Si era personal, además suelta su presa.
4. **Botín**: lo robado + la chatarra de las bajas va al carro del ganador (`cargarBotin`) hasta `capacidadCargaDe(ganador, caravanas)`; **lo que no cabe se pierde**.
5. Eventos: `combate.resuelto`, `combate.encuentro`; se narran a los **dos** hogares.

## 6. Asedio de una plaza (`asediarPlaza` → `asediarConEjercito`)

Validaciones (`validarAsedio`): atacante **ejército** (una columna personal no abre un asedio); plaza de **otra Facción**; a ≤ 15 con héroe sano; plaza **no protegida** tras su conquista.

Orden (`asediarConEjercito`):
1. **Plaza protegida** (`estaProtegida`: `ahora < protegidaHasta`): rebota, `combate.asedio_resistido`, sin combate ni RNG.
2. **Sin defensores** (ninguna escuadra con soldados en `defensaDe`) → **cae sin combate**, sin RNG: `combate.asedio_conquista` y XP de conquista para el atacante. (Sin atacantes con soldados → «sin nadie con quien combatir», no cae.)
3. En otro caso, `resolverCombate(atacantes, defensores, rng, multiplicadorMuralla)` con
   `multiplicadorMuralla = 1 + (bonoDefensaPorNivel[nivel] − 1) × integridad / nº puertas`, usando solo el **último recinto** de la plaza (el más exterior) — ver [`Murallas.md`](Murallas.md) §9. `integridad` ∈ [0,1]: un anillo a medio cerrar casi no defiende.
4. **Reputación**: si atacante y defensor están **aliados** (se atacó sin romper la alianza), la Facción atacante recibe `REPUTACION.penalizacionAtacarAliado` (la más severa).
5. **XP de Facción**: combate (a ambos, si digno) y, si conquista, `NIVEL_FACCION.xp.conquista` al atacante.
6. Resultado: `conquistado = (ganador == atacante)`. El ejército **sigue fuera y acampa**; conquistar no lo convierte en guarnición (Doc 5.15.5).

Consecuencias que aplica `asediarPlaza`:
- **Heridos**: si hubo combate real (`tropaDefensora` no vacía), quedan heridos los **defensores en persona** si la plaza cae, o **todos los participantes del ejército** si resiste. Sin combate, nadie.
- **Chatarra**: al carro del ejército si conquista (limitada por su capacidad de carga); al almacén de la plaza si resiste.
- **Si cae**: `aplicarConquista` (§8) + `desalojarResidentes` (§9) + `registrarDerrota` (§10).
- Eventos: se atribuyen siempre al hogar del atacante, y a la plaza **solo si resistió** (si cae es del atacante y contárselo a ella lo enseñaría dos veces). Hueco conocido: al vencido no le llega su derrota, porque pierde la plaza por la que la vería.

> **`iniciarAsedio`** (asentamiento contra asentamiento, exige General) sigue en `combate.ts` con su misma lógica pero **no lo llama ningún comando en producción**: solo sus tests. Es un camino heredado de antes de los ejércitos; ver [`Divergencias.md`](Divergencias.md).

## 7. Otros objetivos

### 7.1 Interceptar una caravana (`interceptar` → `interceptarCaravanaConEjercito`)
- Rechaza una caravana **adjunta** (se ataca al ejército) y una en **refugio** (dentro de la zona de influencia de otra Facción).
- Defensa = `poderTotal(escolta, cohesión)` si la caravana tiene **escolta sin héroe**; si no, `MILITAR.defensaBaseCaravana`.
- `gana = poder(atacante) × jitter > defensa` (**una** tirada de `rng`). Si gana: bajas del atacante 5 %, la escolta 25 % y **la caravana se destruye**; el atacante carga `umbralCapturaCaravana` (50 %) de cada recurso en su carro, limitado por el hueco libre (el resto se pierde); la escolta de una caravana capturada queda a 0 y vuelve a su campamento. Si pierde: bajas del atacante 25 %, escolta 5 %, la caravana se zafa y **los héroes del atacante quedan heridos**.
- Interceptar no da experiencia de Facción.

### 7.2 Atacar un campamento de bandidos (`atacarCampamento` → `atacarCampamentoConColumna`)
- La columna debe llevar soldados; compara `poder × jitter` con el `poder` fijo del campamento (**una** tirada). Gana → bajas 5 %, el campamento cae (su botín de oro por héroe lo reparte el comando con `botinDeBandidos`); pierde → bajas 25 %, héroes heridos y se pierde la mitad del carro (nadie se la queda).
- XP de Facción: `NIVEL_FACCION.xp.bandidos / nivel de Facción` por campamento destruido (0 desde `nivelSinXpBandidos`), **sin exigir combate digno**.

## 8. Conquista: lo que le pasa a la plaza (`aplicarConquista`)

Pura y determinista (no usa RNG). La ciudad cambia de dueño entera:
- **Cargos**: todos los cargos locales quedan vacíos. **Fundadores y casas compradas**: vacíos.
- **Población**: pesants, artesanos y nobleza pierden `OCUPACION.fraccionSaqueoPoblacion` (`floor` de lo que queda).
- **Edificios**: de los `activo`, ordenados por id, exentos el Centro Urbano y la **primera Granja y la primera Leñera** activas, los primeros `ceil(fraccionEdificiosDanados × n)` pasan a `en_cola` marcados `danado` (se reconstruyen por `fraccionCosteReconstruccion`). Un Almacén o Granero dañado deja de aportar capacidad y lo guardado que ya no cabe se pierde.
- **Recintos**: cada recinto completo pierde `floor(fraccionDanoMuralla × celdas)` de `avance` (mínimo −1): no cae, deja de dar el multiplicador pleno hasta repararse por obra normal.
- **La obra de ascenso en curso se pierde** sin devolución.
- **Las decisiones del gobierno derrotado caen**: políticas activas, reserva manual del Tesorero, pausa de auto-construcción y recetas paradas.
- `medidorMantenimiento = 100`; `protegidaHasta = ahora + proteccionMinutos` (inmune a otro asedio un día); `ocupacionHasta = ahora + duracionMinutos` (recaudación y crecimiento al `factorRecaudacion` / `factorCrecimiento`, mantenimiento congelado).
- **Queda sin guarnición**: nadie la guarnece por haberla ganado; si los conquistadores quieren defenderla, tienen que pasar a residir en ella.

## 9. Desalojo (`desalojarResidentes`)

Nadie se queda dentro de una plaza enemiga:
- Quien estaba **dentro**: si tiene una columna aparcada a la puerta, vuelve a ella; si no, nace una **columna personal** estacionada en la plaza (`${plaza}-salida-${héroe}-${instante}`), con el carro vacío y las escuadras con las que **defendió y sobrevivieron**; su `faccionId` es la de la plaza si residía, `''` si era visitante. Un residente herido no defendió, así que sale solo.
- **Todos los residentes**: el resto de su campamento (guarnición incluida) queda a 0 y pasa con ellos al **asentamiento más cercano de su Facción** (empate por id), donde pasan a residir (se añaden a sus `casasCompradas`). **Sin ninguno**, pasan al **campamento de mercenarios más cercano** a la plaza perdida (`acogerEnCampamentoMasCercano`), con sus escuadras a 0 pero suyas, con su nivel y experiencia. Lo que llevaban fuera (columna, escolta) no se toca.
- El traslado **no mira el cupo de viviendas** del destino (`ponytail:` anotado; si hace falta tope, ahí).

## 10. Derrota de Facción (`registrarDerrota`)
Si tras caer la plaza la Facción perdedora **no conserva ningún asentamiento**, queda anotada `derrotadaPor = ganadora` (o `null` si la última colapsó sin que nadie la derrotara). Qué hace después con sus héroes lo decide quien los gobierna.

## 11. Heridos

`herir(heroes, ids, ahora)`: `heridoHasta = ahora + HEROE.heridoMinutos`; a quien ya lo está no se le alarga. `heridosEn(heroes, ahora)` da el conjunto vigente. Un herido **no persigue, no se le persigue, no entra en batallas** y, si todos los héroes de una columna lo están, esa columna es intocable. Sustituye a la antigua Tregua.

## 12. Parámetros

<!-- parametros:MILITAR -->
```json
{
  "racionPorSoldadoPorMinuto": 0.15,
  "regeneracionMoralPorMinuto": 5,
  "degradacionMoralSinRacion": 20,
  "desercionFraccionPorMinutoSinMoral": 0.05,
  "bonusPoderPorNivelEscuadra": 0.01,
  "nivelMaximoEscuadra": 10,
  "experienciaParaSubirEscuadra": [
    5,
    10,
    20,
    40,
    40,
    40,
    40,
    40,
    40
  ],
  "experienciaGanadaPorVictoria": 1,
  "experienciaGanadaPorDerrota": 0.5,
  "ventajaDefensor": 1.05,
  "bonusCohesionPorEscuadronExtra": 0.1,
  "varianzaCombate": 0.15,
  "umbralCapturaCaravana": 0.5,
  "defensaBaseCaravana": 15
}
```
<!-- /parametros -->

<!-- parametros:HEROE -->
```json
{
  "bronceInicial": 500,
  "topeAtributo": 100,
  "heridoMinutos": 2
}
```
<!-- /parametros -->

<!-- parametros:CHATARRA -->
```json
{
  "fraccion": 0.5
}
```
<!-- /parametros -->

<!-- parametros:NIVEL_FACCION -->
```json
```
<!-- /parametros -->

## 13. Invariantes

- Un combate con números consume **exactamente** las tiradas indicadas (2 en `resolverCombate`, 1 en caravana y campamento, 0 en las ramas sin combate): lo vigila el guardián de determinismo.
- Las bajas son permanentes y una escuadra nunca desaparece: queda a `cantidad: 0` hasta que se rellene.
- Una columna con todos sus héroes heridos no se puede atacar ni perseguir.
- Una plaza conquistada es inatacable durante `proteccionMinutos`.
- Tras `desalojarResidentes` ningún héroe queda con `ubicacion` dentro de la plaza conquistada.

## 14. Casos límite y decisiones que parecen raras

- **Todos los participantes del perdedor quedan heridos**, hayan entrado en la batalla o no (el tope de 5 héroes por bando aparta a los demás, pero la derrota es de la columna).
- **El carro se pierde por mitades aunque el héroe que lo llevaba estuviera herido y no combatiera**: el carro es de la columna.
- **Un encuentro en campo no da ventaja de defensor «de verdad»**: el motor da cohesión y 1,05 al segundo bando; por eso el orden atacante/defensor importa y el comando lo fija (quien ordena es el atacante).
- **El multiplicador de muralla usa solo el último recinto** de la lista, no el mejor: los recintos interiores son «historia en piedra» (ver `Murallas.md`).
- **El mismo asedio puede pasar por Unity o por números**; las consecuencias deben ser idénticas por los dos caminos (`aplicarConquista`, `desalojarResidentes`, `registrarDerrota` son compartidos).

## 15. Divergencias

Ver [`Divergencias.md`](Divergencias.md): el atacante de una columna pierde casi siempre, y `iniciarAsedio` sin llamadores.
