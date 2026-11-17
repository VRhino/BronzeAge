//! El runner contra el servidor real (7D): llegan seis bots escalonados (un grupo de cinco amigos en el mismo campamento y uno más), piensan a su ritmo con la pizarra de su Facción y,
//! tras parar y volver a arrancar con el mismo registro, siguen los mismos héroes sin cuentas nuevas.

use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use bronce_age_bots_brain::{llegadas::Perfil, runner::Registro, runner::Config, Cerebro, Contexto, Fallo, Proceso};

mod comun;

/// Un cerebro que solo apunta: cuántas veces piensa cada uno y en qué campamento reside.
#[derive(Clone, Default)]
struct Apunta(Arc<AtomicUsize>);

impl Cerebro for Apunta {
    type Memoria = u32;
    type Pizarra = HashSet<String>;
    async fn pensar(&self, ctx: &mut Contexto<'_, Self>) -> Result<(), Fallo> {
        *ctx.memoria += 1;
        let yo = ctx.yo.to_string();
        ctx.con_pizarra(|p| p.insert(yo));
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

fn config(dir: &str, registro: std::path::PathBuf) -> Config {
    Config {
        servidor: dir.into(),
        codigo_bots: "bots".into(),
        registro,
        total: 6,
        // Todos en ≈ 6 s de mundo (el servidor va a ×1).
        dias_llegada: 6.0 / 86_400.0,
        semilla: 3,
        cada_ms: 500,
        siempre: true,
        lugares: Default::default(),
    }
}

async fn hasta(que: &str, mut cond: impl FnMut() -> bool) {
    for _ in 0..600 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no pasó: {que}");
}

#[tokio::test(flavor = "multi_thread")]
async fn llegan_piensan_y_vuelven_con_los_mismos_heroes() {
    let datos = tempfile::tempdir().unwrap();
    let Some((_servidor, dir)) = comun::servidor(datos.path()).await else {
        eprintln!("sin binario del servidor o sin artefacto de mundo: se omite");
        return;
    };
    let ruta = datos.path().join("registro.json");
    let leer = || Registro::leer(&ruta).unwrap();

    let pensado = Arc::new(AtomicUsize::new(0));
    let p = Proceso::arrancar(config(&dir, ruta.clone()), Apunta(pensado.clone())).await.expect("arrancar");
    hasta("llegan los seis", || leer().is_some_and(|r| r.bots.len() == 6)).await;
    hasta("piensan todos", || pensado.load(Ordering::Relaxed) >= 18).await;
    p.parar().await.expect("parar");

    let r = leer().unwrap();
    assert_eq!(r.llegadas_hechas, 2, "un grupo de amigos y uno más");
    let lider = &r.bots[0].heroe_id;
    assert!(r.bots[..5].iter().all(|b| b.perfil == Perfil::Amigos { lider: lider.clone() }), "{:?}", r.bots);
    assert_ne!(r.bots[5].perfil, Perfil::Amigos { lider: lider.clone() });
    let heroes: HashSet<_> = r.bots.iter().map(|b| b.heroe_id.clone()).collect();
    assert_eq!(heroes.len(), 6);

    // Otra vez con el mismo registro: los mismos héroes vuelven a pensar y nadie más llega.
    let antes = pensado.load(Ordering::Relaxed);
    let p = Proceso::arrancar(config(&dir, ruta.clone()), Apunta(pensado.clone())).await.expect("rearrancar");
    hasta("vuelven a pensar", || pensado.load(Ordering::Relaxed) >= antes + 12).await;
    p.parar().await.expect("parar otra vez");
    let r2 = leer().unwrap();
    assert_eq!((r2.bots.len(), r2.llegadas_hechas, &r2.serie), (6, 2, &r.serie));
    assert_eq!(r2.bots.iter().map(|b| b.heroe_id.clone()).collect::<HashSet<_>>(), heroes);
}
