import { describe, it, expect } from 'vitest';
import { crearRelieveRegional, evaluarRelieveRegional } from '../regional';

const limites = { ancho: 32000, alto: 24000 };
describe('montañas regionales 8', () => {
  it('genera cinco morfologías y ramales reproducibles sin depender del orden de muestreo', () => {
    const campo = crearRelieveRegional(42, limites);
    expect(new Set(campo.barreras.map(b => b.morfologia)).size).toBe(5);
    expect(campo.barreras.every(b => (b.ramales?.length ?? 0) > 0)).toBe(true);
    const copia = JSON.parse(JSON.stringify(campo));
    for (const b of campo.barreras) for (const r of b.ramales!) {
      for (const p of r.puntos) {
        expect(evaluarRelieveRegional(copia, p)).toBe(evaluarRelieveRegional(campo, p));
        const e = evaluarRelieveRegional(campo, p);
        for (const d of [{ x: .01, y: 0 }, { x: 0, y: .01 }])
          expect(Math.abs(evaluarRelieveRegional(campo, { x: p.x + d.x, y: p.y + d.y }) - e) * 1000).toBeLessThan(.1);
      }
    }
    expect(crearRelieveRegional(42, limites)).toEqual(campo);
    expect(crearRelieveRegional(7, limites).barreras).not.toEqual(campo.barreras);
  });
  it('ramifica el relieve y conserva exactamente los llanos fuera de su influencia', () => {
    const campo = crearRelieveRegional(42, limites);
    const sinRamales = { ...campo, barreras: campo.barreras.map(b => ({ ...b, ramales: [] })) };
    let aportes = 0;
    for (const b of campo.barreras) for (const r of b.ramales!) {
      const p = r.puntos[1]!;
      if (evaluarRelieveRegional(campo, p) - evaluarRelieveRegional(sinRamales, p) > .01) aportes++;
    }
    expect(aportes).toBeGreaterThan(5);
    const llano = { ...campo, barreras: [] };
    let muestras = 0;
    for (let x = 0; x <= limites.ancho; x += 1000) for (let y = 0; y <= limites.alto; y += 1000) {
      if (campo.barreras.some(b => x >= b.caja.minX - b.ancho * 4.5 && x <= b.caja.maxX + b.ancho * 4.5
        && y >= b.caja.minY - b.ancho * 4.5 && y <= b.caja.maxY + b.ancho * 4.5)) continue;
      expect(evaluarRelieveRegional(campo, {x,y})).toBe(evaluarRelieveRegional(llano, {x,y})); muestras++;
    }
    expect(muestras).toBeGreaterThan(100);
  });
});


it('mesetas con techo suave y crestas alargadas presentan secciones diferentes en el campo real', () => {
  const campo=crearRelieveRegional(42,limites);
  const base={...campo,barreras:[]};
  const altura=(b:typeof campo.barreras[number],c:NonNullable<typeof b.cumbres>[number],axial:number,lateral:number)=>{
    const eje=c.eje!;
    const p={x:c.centro.x+eje.x*axial-eje.y*lateral,y:c.centro.y+eje.y*axial+eje.x*lateral};
    const aislado={...campo,barreras:[{...b,tramos:[],ramales:[],pasos:[],cumbres:[c]}]};
    return (evaluarRelieveRegional(aislado,p)-evaluarRelieveRegional(base,p))*1000;
  };
  const meseta=campo.barreras.find(b=>b.morfologia==='meseta')!;
  const cima=meseta.cumbres![0]!;
  expect(Math.abs(altura(meseta,cima,0,0)-altura(meseta,cima,cima.radio*.15,0))).toBeLessThan(3);
  expect(altura(meseta,cima,0,0)-altura(meseta,cima,0,cima.radio*.7)).toBeGreaterThan(20);
  const larga=campo.barreras.find(b=>b.morfologia==='longitudinal'&&b.cumbres!.some(c=>c.longitud!/c.radio>1.8))!;
  const cresta=larga.cumbres!.find(c=>c.longitud!/c.radio>1.8)!;
  expect(altura(larga,cresta,cresta.radio*1.35,0)-altura(larga,cresta,0,cresta.radio*1.35)).toBeGreaterThan(20);
});


it('disecta solo Anatolia, conserva puertos y no cambia las otras cordilleras', () => {
  const campo=crearRelieveRegional(42,limites);
  const anterior={...campo,barreras:campo.barreras.map(b=>({...b,cumbres:b.cumbres?.map(c=>({...c,diseccion:undefined}))}))};
  expect(campo.barreras.filter(b=>b.cumbres?.some(c=>c.diseccion)).length).toBe(1);
  let cambios=0;
  const b=campo.barreras[1]!;
  for(const c of b.cumbres!) for(let i=0;i<16;i++){
    const a=i*Math.PI/8,p={x:c.centro.x+Math.cos(a)*c.radio*.5,y:c.centro.y+Math.sin(a)*c.radio*.5};
    if(Math.abs(evaluarRelieveRegional(campo,p)-evaluarRelieveRegional(anterior,p))>.003) cambios++;
    const e=evaluarRelieveRegional(campo,p);
    expect(Math.abs(evaluarRelieveRegional(campo,{x:p.x+.01,y:p.y})-e)*1000).toBeLessThan(.1);
  }
  expect(cambios).toBeGreaterThan(15);
  for(const paso of b.pasos)
    expect(Math.abs(evaluarRelieveRegional(campo,paso.centro)-evaluarRelieveRegional(anterior,paso.centro))*1000).toBeLessThan(1);
  for(let x=0;x<=limites.ancho;x+=1000) for(let y=0;y<=limites.alto;y+=1000){
    if(x>=b.caja.minX-b.ancho*4.5&&x<=b.caja.maxX+b.ancho*4.5&&y>=b.caja.minY-b.ancho*4.5&&y<=b.caja.maxY+b.ancho*4.5)continue;
    expect(evaluarRelieveRegional(campo,{x,y})).toBe(evaluarRelieveRegional(anterior,{x,y}));
  }
});
