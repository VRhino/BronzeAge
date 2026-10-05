# Taberna e intel — decisiones y plan de ejecución

> **Estado (2026-10-05): diseño cerrado con el usuario e IMPLEMENTADO en el motor, la proyección al jugador y los bots; sin push.**
> Las **reglas de juego** viven en el canon: `Docs/Game/5` §5.12.10 (la intel), `Docs/Game/4` §4.2.1 (el edificio Taberna) y
> `Docs/Game/1` §1.9b (la taberna del campamento). Este documento guarda **las decisiones con sus alternativas descartadas**, la
> forma en el motor y lo que queda fuera. Enunciado original: `Docs/Mecanicas a desarrollar.md` §18 (cerrada y retirada).

## 1. De dónde viene

Con la niebla de guerra (Doc 5.12.7-8) saber qué pasa donde no ves vale algo, y ese valor es el **sink recurrente de oro** que la
calibración de la economía del oro asume (`Economia_Del_Oro_Definicion.md` §4.5): reclutar y comprar animales son compras de capital de
una vez; **la intel es una suscripción** —se paga, caduca, se vuelve a pagar— y es lo único que drena de verdad un tesoro que se acumula.

Antes de esto la taberna solo era un tipo del layout del campamento de mercenarios (ancla del trazado, `layoutCampamento.ts`), sin función.

## 2. Decisiones cerradas con el usuario (2026-10-05)

| # | Decisión | Alternativas descartadas |
|---|---|---|
| T1 | **Dos productos en la v1**: la **Mirada** (ojo prestado sobre un punto) y el **Informe de plaza** (foto con fecha de layout y defensa) | **Rumores** (celdas gruesas, sin dueño ni cifras): se deja para otra pasada; **Estado de Facción** (agregado de plazas, niveles y ejércitos): solapa con el score público (Doc 2.7) y con los Ladrones (Doc 2.10) |
| T2 | **Se compra en la taberna de plaza y en la de campamento** | Solo plaza (los héroes sin plaza se quedan sin acceso); solo campamento (obliga a viajar y castiga a las asentadas) |
| T3 | **Información exacta, congelada y con fecha**: la Mirada es en vivo mientras dura, el Informe se congela y envejece como la memoria de la niebla | Con ruido (rangos): más reglas que calibrar y mostrar; **falsa** a veces: rompe «lo que ves es lo que hay» y complica a los bots |
| T4 | **El Informe avisa, sin firma, a la Facción espiada; la Mirada no avisa a nadie** | Nunca se entera (espionaje silencioso sin contrapeso); aviso con identidad (anula el valor del intel de guerra) |
| T5 | **Mirada: radio 150, 2 h** (la vista de una columna) | 80 / 1 h (obliga a comprar varias); 300 / 6 h (vuelve irrelevante la exploración) |
| T6 | **Precio**: Mirada = base + distancia a los ojos propios; Informe = por nivel de la plaza; **cooldown por objetivo y cupo por taberna** | Coste creciente por compras recientes (castiga a quien de verdad prepara un asedio); precio fijo (no escala con el valor y permite espiar sin límite) |
| T7 | **La Mirada se comparte en vivo con quienes comparten visión; el Informe no**; no hay reventa | Nada se comparte (choca con la visión compartida ya hecha); todo se comparte (reduce el sink) |
| T8 | **Taberna de plaza: nivel 2 de asentamiento, 3 niveles internos** (cupo de Miradas 1 / 2 / 3; el 3 pide nivel 3), una por asentamiento | Nivel 1 sin niveles (al alcance de plazas sin tesoro); nivel 3 (los NPC rara vez llegan, Economía §1: el sink no se activaría) |

## 3. Decisiones de implementación (las tomé yo; se pueden revisar)

- **Quién compra en una plaza**: Rey, Embajador o Gobernador, presente en ella. Es una decisión mía por analogía con la compra a un Aeda
  (Rey o Gobernador, Doc 6.7), añadiendo al Embajador porque la intel de una plaza ajena es política exterior.
- **Quién compra en un campamento**: cualquier héroe **con Facción**. Un héroe sin Facción no tiene memoria donde guardar un informe ni
  aliados con quienes compartir una Mirada (`Heroe.plazasRecordadas` guarda interiores, no fichas ni informes); se deja fuera de la v1.
- **Cupo del campamento**: 1 Mirada abierta por Facción y campamento (`INTEL.campamento.cupoMiradas`): su taberna no sube de nivel.
- **El cooldown de zona** de una Mirada empieza al **caducar** y se mide como «el centro a menos de un radio de otra Mirada reciente de la
  misma Facción». Se guarda `libreEn` en la propia Mirada y se purga al comprar: no hay tick de limpieza.
- **Un Informe pide conocer la plaza** (ficha en la memoria, o dentro de una Mirada propia abierta) para que no se pueda recorrer el mundo
  probando ids. El flujo natural es mirar una zona, ver la plaza y pedir su informe.
- **Qué entra en un Informe**: edificios que no estén en cola (tipo, sitio, nivel interno, estado), recintos con su avance, guarnición y
  héroes dentro. No el almacén, las colas, los cargos ni las políticas. La defensa es la misma que da la inspección a 40 (`defensaDePlaza`,
  compartida con `inspeccionarPlaza`).
- **Eventos**: solo el aviso al espiado (`asentamiento.informe_pedido`, atribuido a su plaza). **Comprar no emite evento**: un evento sin
  `asentamientoId` es público (Doc 5.12 / `eventosVisiblesParaJugador`) y publicaría quién compra intel; el resultado vuelve solo a quien compró.
- **La Taberna se coloca como el Palacio**: junto al centro, sin barrio (`CATEGORIA_POR_TIPO` no la tiene). Lo cazó un test: sin esa línea
  en `sitiosParaTipo` no encontraba sitio nunca y era imposible de construir.
- **Eras IV-V**: nada depende de ellas.

## 4. Forma en el motor

| Pieza | Dónde |
|---|---|
| Reglas puras: precio, cupo, cooldown, qué entra en un Informe | `engine/intel.ts` (`comprarMirada`, `validarInforme`, `levantarInforme`, `miradasActivasDe`) |
| Edificio, huella y catálogo | `EdificioTipo 'taberna'`, `EDIFICIO_CATALOGO.taberna`, `EDIFICIO_TAMANO`; `cupoMiradas` (`engine/asentamientoQuery.ts`) |
| Cifras | `INTEL` (`constants.ts`), todo PLACEHOLDER |
| Estado | `GameSessionState.miradasIntel?` (Miradas vigentes o enfriándose); `MemoriaFaccion.informes?` (último Informe por plaza). Opcionales: **sin migración** |
| Comandos | `comprarMirada {origen, centro}` y `comprarInformePlaza {origen, asentamientoId}` (`session/comandos/intel.ts`); `origen = {tipo:'asentamiento'\|'campamento', id}`. Código de error `intel.invalida` |
| Visión | La Mirada entra como ojo en `seVeAhora`, en la máscara `visibles` y en caravanas y campamentos avistados (`session/proyecciones/jugador.ts`), **solo en vivo, nunca en lo explorado ni en la memoria**, igual que los ojos aliados |
| Proyección | `miradasIntel`, `informesPlaza`, `tarifasIntel` en `ProyeccionJugador` |
| Bots | El Gobernador construye la taberna a nivel 2 con Mercado; el Rey o el Gobernador con oro de sobra compra el informe de la plaza ajena conocida más cercana cuya defensa no tenga fresca, y lo apunta en la pizarra (`bots/cerebro/gobierno.ts`) |

**Guarda importante:** `grabarLoVisto` reescribía la memoria de la Facción con `{ exploracion, asentamientos }`; ahora conserva el resto
(`...previa`), si no el primer tick borraba los informes. Hay un test (el informe sobrevive a un tick).

## 5. Fuera de esta pasada

- **Rumores** de las tabernas de campamento y **Estado de Facción**: sin diseñar más allá de lo descartado arriba.
- **Héroes sin Facción** comprando intel (necesitan memoria propia de informes).
- **Cliente de jugador (BronzeAgeClient)**: comprar, pintar las Miradas (círculo con cuenta atrás y zona enfriándose), mostrar los
  Informes con su fecha y avisar del `asentamiento.informe_pedido`. Contrato en `Docs/Coordinacion/` (01 §24 y 02).
- **Conquest/Unity**: nada. El contrato `src/contratos/v1/` no lleva la proyección del jugador; solo se tocó el modelo compartido.
- **Calibración** de todas las cifras de `INTEL` → `Docs/Mecanicas a balancear.md` §18.
- **Cotización antes de comprar**: el cliente calcula el precio con `tarifasIntel` y sus ojos; no hay comando de cotización.
- Los **diarios de batch anteriores no son comparables** si los bots construyen taberna y compran informes: es un sink nuevo en el mundo.
