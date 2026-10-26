# Mercenarios por contrato

> **Diseño acordado el 2026-10-26 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Ninguna cifra de este documento es vigente: las tarifas, los plazos y los topes son placeholders que se calibran con bots (fase 7). No hay bloque `parametros:` todavía: se crea al implementarlo. Contexto: `Docs/Mecanicas a desarrollar.md` entrada 48; relación con los campamentos en [`Campamentos_Mercenarios.md`](../Mechanics/Campamentos_Mercenarios.md).

## 1. Qué es

Un héroe **bot** que reside en un campamento de mercenarios se ofrece, y un héroe con rango de una Facción lo **contrata como miembro de esa Facción** (no como héroe personal de otro héroe) para un trabajo concreto (**contrato temporal**) o para formar parte de su ejército (**contrato permanente**). El bot sigue **fuera del motor** y juega por el contrato 05 como cualquier jugador (`Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md`): el motor guarda el contrato, retiene el pago y aplica reglas duras; no sabe si quien cumple es un bot.

Descartado: alquilar escuadras sin héroe; que un humano se ofrezca como mercenario; el mercenario como héroe personal de otro.

## 2. Quién ofrece y quién contrata

| Rol | Puede |
|---|---|
| Héroe bot residente en un campamento | `OfrecerseComoMercenario` (solo esta orden la restringe: `heroe.controlador = bot`, que ya existe). Entra en la lista de ese campamento |
| Rey, Embajador | contratar cualquier trabajo de la tabla (§3) para **todas** las plazas de su Facción, y contratos permanentes |
| Gobernador de una plaza | contratar solo trabajos que afecten a **su plaza, sus pueblos, sus avanzadas o sus caravanas**; nunca permanentes ni ofensivos |

**Se contrata en persona.** El héroe con rango viaja con su columna hasta el campamento llevando el pago en su carro, entra, elige de la lista de ese campamento y paga con el oro del carro (el mismo mecanismo que comprar de visita, `Comercio_Caravanas.md` §9.2). No hay contratación a distancia. El oro pasa al almacén personal del mercenario, que lo gasta en el campamento (reclutar, comprar): es un sumidero de oro, no una fuente.

## 3. Tabla de trabajos (`TRABAJOS_MERCENARIOS`, borrador de partida)

Es una tabla de datos que el motor lee: añadir un trabajo es añadir una fila y su regla de objetivo.

| Trabajo | Tipo | Quién lo asigna | Objetivo | Orden directa para atacar |
|---|---|---|---|---|
| Defender una plaza | temporal | Gobernador de esa plaza; Rey/Embajador | la plaza | no hace falta para defenderse |
| Defender un pueblo extractor o aldea | temporal | Gobernador de la madre; Rey/Embajador | el pueblo | no hace falta para defenderse |
| Defender una avanzada | temporal | Gobernador de la madre; Rey/Embajador | la avanzada | no hace falta para defenderse |
| Escoltar una caravana de la plaza | temporal | Gobernador del origen; Rey/Embajador | la caravana | no hace falta para defenderse |
| Escoltar el carro de acarreo de un pueblo | temporal | Gobernador de la madre; Rey/Embajador | el carro | no hace falta para defenderse |
| Despejar un campamento de bandidos | temporal | Gobernador (los de su zona); Rey/Embajador | el campamento | sí, siempre |
| Acompañar a un ejército en campaña | temporal | solo Rey/Embajador | el ejército | sí, para cada ataque |
| Ser parte del ejército de la Facción | permanente | solo Rey/Embajador | la Facción | sí, para cada ataque |

Los trabajos de defensa y escolta no necesitan guerra declarada ni orden para responder a un ataque; los ofensivos sí.

## 4. Temporal y permanente

| | Temporal | Permanente |
|---|---|---|
| Pago | **por adelantado**, retenido al firmar; no se devuelve (ni al despedir ni al acabar) | **pago único** al firmar; tampoco se devuelve |
| Qué es el mercenario | miembro contratado; sigue residiendo en su campamento | **ciudadano con marca «contratado»**, residente en la plaza que elige el contratante al firmar (cambia después solo con otra orden de mando) |
| Duración | uno de tres plazos fijos de la tabla (1, 3 y 7 días de mundo, placeholders) | hasta despido |
| Órdenes | Rey y Embajador; el Gobernador, dentro de su alcance | Rey y Embajador; el Gobernador, solo dentro de su plaza |
| Reclutar | no | sí, de la plaza donde reside, como cualquier residente (libre; las reservas dinámicas del mantenimiento y la reserva del Tesorero protegen lo esencial) |

**Tarifa.** La fija el motor por fórmula (nivel y tropa del mercenario, con los precios de reclutar del campamento, `Heroe_Y_Tropa.md` §12 y `Campamentos_Mercenarios.md` §5.2) y **el plazo multiplica el coste**: más tiempo, más oro. El bot no fija precio.

**El contrato es de la Facción** (o de la plaza, si lo firmó un Gobernador), no de la persona: si quien lo firmó pierde el cargo o se va, lo manda quien ocupe el cargo ahora.

## 5. Cómo funciona

1. **Oferta.** `OfrecerseComoMercenario` pone al bot en la lista del campamento (ámbito `CAMPAMENTO`, el tablón de la taberna) con su tropa y su historial de contratos firmados, terminados y despedidos.
2. **Firma.** Un héroe con rango, dentro del campamento, firma un trabajo de la tabla; el motor comprueba rol, alcance, plazo y que el carro lleve el oro, y descuenta el pago.
3. **Temporal.** Mientras dura, cuenta como de la Facción para ir en sus convocatorias y ejércitos, entrar por su puerta y combatir en su bando. No es ciudadano, no tiene cargos ni residencia en sus plazas. Al acabar el plazo o ser despedido, la afiliación cae en el mismo instante, como la tropa prestada (`Campamentos_Mercenarios.md` §6).
4. **Permanente.** Al firmar, el motor ejecuta por dentro lo que hoy son `SolicitarIngreso` aceptada y `CambiarResidencia`, sin el cooldown de residencia, y marca la ciudadanía como «contratada». La marca impide: ser Rey, Embajador o Gobernador; heredar el trono (la sucesión y el caso «Facción sin nadie» de Diplomacia §3.3 y 5A-8 lo saltan); dejar la Facción por su cuenta (`dejarFaccion` se rechaza). Solo sale por **`DespedirMercenario`**, orden nueva (hoy no existe expulsar a un ciudadano).
5. **Órdenes de mando.** `OrdenarMercenario { mercenario, orden }`, solo de quien tenga alcance (§2 y §4), queda guardada en el estado del mercenario y llega a su bot por el ámbito `HEROE`. El bot decide cómo cumplirla con sus órdenes de siempre.
6. **Reglas duras** (matriz de autorización de `Proyecciones_Y_Autorizacion.md` §5.1): un mercenario **no declara guerra**, **no ataca sin una orden vigente** y **nunca ataca a su propia Facción**, como cualquier héroe.

## 6. Qué toca en el motor (plan, sin hacer)

- Estado nuevo: contratos (por Facción o plaza), ofertas por campamento, marca de ciudadanía contratada; foto nueva (hoy `BAN40`).
- Órdenes nuevas tras la 178, con rechazos en `07_Catalogo_de_Rechazos.md` y filas en la matriz §5.1: ofrecerse, contratar, ordenar, despedir (y retirar la oferta).
- Hechos `mercenarios.contratado`, `.despedido`, `.terminado`, `.ordenado` (nombres provisionales), en el bloque `<!-- eventos -->` de este documento cuando exista.
- Política: una guarda en `dejarFaccion`, en `asignarRey`/sucesión y en los cargos locales para la marca.
- Proyección: contratos en `FACCION` (un Gobernador ve solo los de su plaza); la oferta en `CAMPAMENTO`; la orden de mando en `HEROE` del mercenario.
- Cliente (BronzeAgeClient): pestaña «Mercenarios» de la Facción con la lista (trabajo, objetivo, plazo, ubicación, tropa), «Contratar» desde el tablón de la taberna, «Asignar trabajo» con la tabla, «Ordenar» y «Despedir».
- Bots (BotsBrain): perfil «mercenario» que se ofrece, viaja al trabajo y espera órdenes.

## 7. Cantidad limitada y calibración

La cantidad de mercenarios es **limitada** y se prueba cuando corran los bots: tope de contratos por Facción, tope de permanentes por Facción y número de bots que el runner mantiene en los campamentos. Hay que medir además: el oro que los mercenarios devuelven a circulación, lo que un permanente gasta de su plaza al reclutar y el coste por orden de cada bot más (techo medido ≈ 65 órdenes/s con ≈ 450 vistas, T6 de 4H).

## 8. Casos límite a recordar

- Sin verificación del pago: el motor no comprueba que el temporal cumpla; el historial de la taberna es la reputación del bot.
- Una Facción solo de mercenarios: la marca impide que el trono pase a uno; el caso queda a decidir al implementar.
- Un campamento no tiene puerta ni vetos: contratar no depende de relaciones.
