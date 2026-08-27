export function renderPestanaInformacion(): string {
  return `
    <div class="information-panel-view">
      <span class="faction-kicker">Referencia del mundo</span>
      <h2>Información</h2>
      <dl class="glossary-list">
        <div><dt>Facción</dt><dd>Grupo político al que pertenece un jugador y que comparte ciudadanía y territorio.</dd></div>
        <div><dt>Asentamiento</dt><dd>Centro desde el que una facción organiza población, edificios y expansión.</dd></div>
        <div><dt>Zona de influencia</dt><dd>Área alrededor de un asentamiento que representa su alcance territorial.</dd></div>
        <div><dt>Bosque</dt><dd>Zona natural que aporta acceso a madera cuando queda dentro del territorio.</dd></div>
        <div><dt>Nodo de recurso</dt><dd>Yacimiento visible del mapa, como piedra, cobre, estaño u oro.</dd></div>
      </dl>
    </div>
  `;
}
