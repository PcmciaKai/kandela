//! Lists the Hue bridges found on the local network.

#[tokio::main]
async fn main() {
    let bridges = kandela_core::discovery::discover().await;
    if bridges.is_empty() {
        println!("No bridges found.");
    }
    for b in bridges {
        println!("{} ({}) at {} [{}]", b.name, b.model, b.address, b.id);
    }
}
