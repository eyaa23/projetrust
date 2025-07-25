use std::io::{self, Write};
use std::net::UdpSocket;

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0")?; // Port aléatoire local
    socket.connect("127.0.0.1:8053")?;

    loop {
        print!(" Entrez un nom de domaine (ou 'quit') : ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("quit") {
            break;
        }

        socket.send(input.as_bytes())?;

        let mut buffer = [0u8; 1024];
        let taille = socket.recv(&mut buffer)?;
        let reponse = String::from_utf8_lossy(&buffer[..taille]);

        println!(" Réponse du serveur : {}", reponse);
    }

    Ok(())
}
