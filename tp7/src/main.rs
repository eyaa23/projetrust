use std::collections::HashMap;
use std::net::UdpSocket;

fn main() -> std::io::Result<()> {
    // Associer un socket UDP à une adresse locale
    let socket = UdpSocket::bind("127.0.0.1:8053")?;
    println!("Serveur DNS démarré sur 127.0.0.1:8053");

    // Base de données DNS simulée
    let dns_records: HashMap<&str, &str> = HashMap::from([
        ("esgi.fr", "192.168.1.42"),
        ("yahoo.com", "93.184.216.34"),
        ("google.com", "8.8.8.8"),
    ]);

    let mut buffer = [0u8; 1024];

    loop {
        // Réception de la requête
        let (taille, src) = socket.recv_from(&mut buffer)?;
        let requete = String::from_utf8_lossy(&buffer[..taille]).to_string();
        println!("Requête de {}: {}", src, requete);

        // Traitement : résolution DNS
        let reponse = match dns_records.get(requete.trim()) {
            Some(ip) => ip.to_string(),
            None => "Domaine inconnu".to_string(),
        };

        // Envoi de la réponse
        socket.send_to(reponse.as_bytes(), &src)?;
    }
}
