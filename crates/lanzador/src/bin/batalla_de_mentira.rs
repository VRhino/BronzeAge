//! `bronzeage-batalla-de-mentira`: el servidor de batalla de mentira ([`lanzador::mentira`]) como ejecutable aparte.

#[tokio::main]
async fn main() {
    lanzador::mentira::correr().await;
}
