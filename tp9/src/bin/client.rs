use tokio_tungstenite::connect_async;
use url::Url;
use futures_util::{SinkExt, StreamExt};
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    let url = Url::parse("ws://127.0.0.1:9001").unwrap();
    let (mut ws_stream, _) = connect_async(url).await.expect("Connexion échouée");

    println!("Connecté au serveur WebSocket. Tape un message :");

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "exit" {
            break;
        }

        ws_stream.send(input.into()).await.unwrap();

        if let Some(msg) = ws_stream.next().await {
            let msg = msg.unwrap();
            println!("Réponse du serveur : {}", msg);
        }
    }
}
