//serveur de journalisation

use tokio::net::{TcpListener, TcpStream}; //gérer les connexions réseau asynchrones (serveur/client TCP)
use tokio::io::{AsyncBufReadExt, BufReader}; //lire les messages du client de façon asynchrone, ligne par ligne
use std::sync::Arc; //partager les données entre plusieurs tâches (threads)
use tokio::sync::Mutex; //protéger les accès concurrents au fichier de log
use std::fs::OpenOptions; //ouvrir/créer un fichier avec des options (ici, en mode ajout)
use std::io::Write; //écrire manuellement dans le fichier
use chrono::Utc; //obtenir la date et l'heure actuelles


//Structure pour gérer le fichier de logs partagé
struct LogManager {
    log_file: Arc<Mutex<std::fs::File>>, 
}
//initialisation du gestionnaire de logs
impl LogManager {
    fn new() -> Result<Self, std::io::Error> {
        //Créer le dossier logs s'il n'existe pas
        std::fs::create_dir_all("logs")?;
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/server.log")?;    //ouvrir le fichier de logs en mode append

            
        Ok(LogManager {
            log_file: Arc::new(Mutex::new(file)),
        })
    }
    //ecrire le message dans le fichier log
    async fn write_log(&self, message: &str) -> Result<(), std::io::Error> {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ"); //ajout du timestamp
        let log_entry = format!("[{}] {}\n", timestamp, message); //formate le log
        
        let mut file = self.log_file.lock().await; //attend le verou
        file.write_all(log_entry.as_bytes())?;
        file.flush()?;
        
        println!("Log écrit: [{}] {}", timestamp, message); //affichage terminal
        Ok(())
    }
}

//fonction pour gérer chaque client connecté
async fn handle_client(mut socket: TcpStream, log_manager: Arc<LogManager>, client_id: u32) {
    println!("Client {} connecté", client_id);
    
    let reader = BufReader::new(&mut socket); 
    let mut lines = reader.lines();
    
    //écrire un log de connexion
    if let Err(e) = log_manager.write_log(&format!("Client {} connecté", client_id)).await {
        eprintln!("Erreur lors de l'écriture du log de connexion: {}", e);
    }
    
    // Lire les messages du client ligne par ligne
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        
        // Si le client envoie "quit", on ferme la connexion
        if line.trim().eq_ignore_ascii_case("quit") {
            break;
        }
        
        // Écrire le message dans le fichier de logs
        let log_message = format!("Client {}: {}", client_id, line.trim());
        if let Err(e) = log_manager.write_log(&log_message).await {
            eprintln!("Erreur lors de l'écriture du log: {}", e);
            break;
        }
    }
    
    // Log de déconnexion
    if let Err(e) = log_manager.write_log(&format!("Client {} déconnecté", client_id)).await {
        eprintln!("Erreur lors de l'écriture du log de déconnexion: {}", e);
    }
    
    println!("Client {} déconnecté", client_id);
}

//main
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _debut = std::time::Instant::now();
    println!("=== SERVEUR DE JOURNALISATION ===");
    println!("Démarrage du serveur de journalisation asynchrone...");
    
    //Initialiser le gestionnaire de logs
    let log_manager = Arc::new(LogManager::new()?);
    
    // Créer le listener TCP sur le port 8080
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!(" Serveur en écoute sur 127.0.0.1:8080");
    
    // Log du démarrage du serveur
    log_manager.write_log("Serveur de journalisation démarré").await?;
    
    let mut client_counter = 0u32;
    let mut tasks = Vec::new();
    
    println!(" En attente de connexions clients... (Ctrl+C pour arrêter)");
    println!(" Pour tester: ouvrez un autre terminal et tapez 'cargo run --bin client'");
    
    // Boucle principale pour accepter les connexions
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                client_counter += 1;
                println!(" Nouvelle connexion de {} - Client ID: {}", addr, client_counter);
                
                // Cloner les références pour la tâche
                let log_manager_clone = Arc::clone(&log_manager);
                let current_client_id = client_counter;
                
                // Lancer une tâche asynchrone pour chaque client
                let task = tokio::spawn(async move {
                    handle_client(socket, log_manager_clone, current_client_id).await;
                });
                
                tasks.push(task);
                
                // Nettoyer les tâches terminées (optionnel, pour éviter l'accumulation)
                tasks.retain(|task| !task.is_finished());
                
            }
            Err(e) => {
                eprintln!(" Erreur lors de l'acceptation de connexion: {}", e);
            }
        }
    }
}