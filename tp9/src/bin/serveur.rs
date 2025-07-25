use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
//use futures_util::{StreamExt, SinkExt};
use std::net::SocketAddr;
use futures_util::stream::StreamExt; // pour `.next()` et `.split()`
use futures_util::sink::SinkExt;     // pour `.send()`



#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:9001".parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).await.expect("Erreur bind serveur");

    println!("Serveur WebSocket en écoute sur {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(async move {
            let ws_stream = accept_async(stream)
                .await
                .expect("Erreur handshake WebSocket");
            println!("Nouvelle connexion de : {}", addr);

            let (mut write, mut read) = ws_stream.split();

            while let Some(msg) = read.next().await {
                let msg = msg.unwrap();
                println!("Reçu de {}: {}", addr, msg);

                // Répond avec un écho
                if write.send(msg).await.is_err() {
                    println!("Erreur en envoyant la réponse.");
                    break;
                }
            }

            println!("Connexion fermée avec {}", addr);
        });
    }
}
