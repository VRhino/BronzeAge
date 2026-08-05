import type { Faccion, RelacionPolitica } from '../domain/types';

/**
 * La Liga (Doc 0) NO se guarda como entidad propia — es la RED de Facciones conectadas por vasallaje o
 * alianza. Se deriva por componentes conexas del grafo de relaciones activas, solo para mostrarla en UI.
 */
export interface LigaInfo {
  miembrosFaccionIds: string[];
  tieneVasallaje: boolean;
  /** Gran Rey (Doc 0/2.7): Rey de la Facción señora de más alto nivel dentro de una Liga por vasallaje. */
  granReyFaccionId?: string;
}

export function computeLigas(relaciones: RelacionPolitica[], facciones: Faccion[]): LigaInfo[] {
  const activas = relaciones.filter((r) => r.estado === 'activa');
  const padre = new Map<string, string>(facciones.map((f) => [f.id, f.id]));

  function encontrar(id: string): string {
    let raiz = id;
    while (padre.get(raiz) !== raiz) raiz = padre.get(raiz)!;
    return raiz;
  }
  function unir(a: string, b: string): void {
    const ra = encontrar(a);
    const rb = encontrar(b);
    if (ra !== rb) padre.set(ra, rb);
  }
  for (const r of activas) unir(r.faccionAId, r.faccionBId);

  const grupos = new Map<string, string[]>();
  for (const f of facciones) {
    const raiz = encontrar(f.id);
    grupos.set(raiz, [...(grupos.get(raiz) ?? []), f.id]);
  }

  return [...grupos.values()]
    .filter((miembros) => miembros.length > 1)
    .map((miembrosFaccionIds) => {
      const vasallajesDeLaLiga = activas.filter(
        (r) => r.tipo === 'vasallaje' && miembrosFaccionIds.includes(r.faccionAId) && miembrosFaccionIds.includes(r.faccionBId)
      );
      const señoras = new Set(vasallajesDeLaLiga.map((r) => r.faccionAId));
      const vasallas = new Set(vasallajesDeLaLiga.map((r) => r.faccionBId));
      // Gran Rey: señora que nunca es vasalla de otra dentro de esta misma Liga.
      const cima = [...señoras].find((id) => !vasallas.has(id));
      return {
        miembrosFaccionIds,
        tieneVasallaje: vasallajesDeLaLiga.length > 0,
        granReyFaccionId: cima,
      };
    });
}
