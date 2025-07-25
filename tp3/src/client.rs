//test client


use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CLIENT DE TEST ===");
    println!(" Connexion au serveur de logs...");
    
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("Connecté au serveur !");
    
    println!("Tapez vos messages (tapez 'quit' pour quitter) :");
    
    loop {
        print!("> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let message = input.trim();
        
        if message.is_empty() {
            continue;
        }
        
        // Envoyer le message au serveur
        stream.write_all(format!("{}\n", message).as_bytes()).await?;
        
        if message.eq_ignore_ascii_case("quit") {
            break;
        }
        
        println!("Message envoyé: {}", message);
    }
    
    println!("Déconnexion...");
    Ok(())
}