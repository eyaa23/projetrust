use std::fs::{self, OpenOptions};
use std::io::{self, Write, Read};
use chrono::Utc;

struct Fichier {
    nom: String,
}

impl Fichier {
    fn lire(&self) {
        match fs::read_to_string(&self.nom) {
            Ok(contenu) => println!("Contenu de {}:\n{}", self.nom, contenu),
            Err(_) => println!("Erreur : fichier introuvable ou illisible."),
        }
    }

    fn ecrire(&self, texte: &str) {
        let mut fichier = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.nom)
            .expect("Erreur à l'ouverture du fichier");

        writeln!(fichier, "{} - {}", Utc::now().format("%d/%m/%Y %H:%M:%S"), texte)
            .expect("Erreur à l'écriture");
        println!("Écriture réussie !");
    }

    fn modifier(&self, nouveau_texte: &str) {
        fs::write(&self.nom, nouveau_texte).expect("Erreur à la modification");
        println!("Fichier modifié.");
    }

    fn supprimer(&self) {
        fs::remove_file(&self.nom).expect("Erreur à la suppression");
        println!("Fichier supprimé.");
    }
}

fn main() {
    println!("Bienvenue dans le gestionnaire de fichiers !");
    
    let mut nom_fichier = String::new();
    println!("Entrez le nom du fichier à gérer :");
    io::stdin().read_line(&mut nom_fichier).expect("Erreur de lecture");
    let mut nom_fichier = nom_fichier.trim().to_string();

    // ✅ Ajoute l'extension automatiquement si manquante
    if !nom_fichier.ends_with(".txt") {
        nom_fichier.push_str(".txt");
    }

    let mon_fichier = Fichier { nom: nom_fichier };

    loop {
        println!("\n--- MENU ---");
        println!("1. Lire le fichier");
        println!("2. Écrire dans le fichier");
        println!("3. Modifier le fichier");
        println!("4. Supprimer le fichier");
        println!("5. Quitter");

        let mut choix = String::new();
        io::stdin().read_line(&mut choix).expect("Erreur de lecture");
        let choix = choix.trim();

        match choix {
            "1" => mon_fichier.lire(),
            "2" => {
                println!("Texte à écrire :");
                let mut texte = String::new();
                io::stdin().read_line(&mut texte).expect("Erreur");
                mon_fichier.ecrire(texte.trim());
            },
            "3" => {
                println!("Nouveau contenu :");
                let mut texte = String::new();
                io::stdin().read_line(&mut texte).expect("Erreur");
                mon_fichier.modifier(texte.trim());
            },
            "4" => {
                mon_fichier.supprimer();
                break;
            },
            "5" => break,
            _ => println!("Choix invalide."),
        }
    }

    println!("Merci d'avoir utilisé le programme.");
}
