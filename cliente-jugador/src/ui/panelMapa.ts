export function renderPanelMapa(): string {
  return `
    <div class="card-panel">
      <div style="display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-glass); padding-bottom: 10px;">
        <h2 id="titulo-mapa" class="panel-title" style="border: none; padding: 0; margin: 0;">Mapa del Mundo (Evaluación T2a)</h2>
      </div>
      <div class="canvas-wrapper">
        <canvas id="mapa" width="900" height="900"></canvas>
      </div>
      <p id="estado" style="font-size: 0.85rem; color: var(--text-secondary);">Cargando mapa y datos...</p>
    </div>
  `;
}
