use std::io;

#[derive(Clone)]
struct CompteBancaire {
    nom: String,
    solde: f64,
}

impl CompteBancaire {
    fn afficher_solde(&self) {
        println!("{} a un solde de {:.2} €", self.nom, self.solde);
    }

    fn retirer(&mut self, montant: f64) {
        if montant > 0.0 && montant <= self.solde {
            self.solde -= montant;
            println!("{} a retiré {:.2} €. Nouveau solde : {:.2} €", self.nom, montant, self.solde);
        } else {
            println!("Montant invalide ou solde insuffisant.");
        }
    }

    fn deposer(&mut self, montant: f64) {
        if montant > 0.0 {
            self.solde += montant;
            println!("{} a déposé {:.2} €. Nouveau solde : {:.2} €", self.nom, montant, self.solde);
        } else {
            println!("Le dépôt doit être positif !");
        }
    }

    fn renommer(&self, nouveau_nom: &str) -> CompteBancaire {
        CompteBancaire {
            nom: nouveau_nom.to_string(),
            solde: self.solde,
        }
    }
}

fn main() {
    let mut comptes = vec![
        CompteBancaire { nom: "Kevin".to_string(), solde: 500.0 },
        CompteBancaire { nom: "Nourdine".to_string(), solde: 1000.0 },
        CompteBancaire { nom: "Fatou".to_string(), solde: 750.0 },
    ];

    loop {
        println!("\n--- MENU ---");
        println!("1 - Liste des comptes");
        println!("2 - Afficher solde d’un compte");
        println!("3 - Dépôt");
        println!("4 - Retrait");
        println!("5 - Renommer un compte");
        println!("6 - Quitter");

        println!("Entrez le numéro de votre choix :");

        let mut choix = String::new();
        io::stdin().read_line(&mut choix).expect("Erreur de lecture");
        let choix: u32 = match choix.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Choix invalide.");
                continue;
            }
        };

        match choix {
            1 => {
                println!("Liste des comptes :");
                for (i, compte) in comptes.iter().enumerate() {
                    println!("{}. {}", i + 1, compte.nom);
                }
            },
            2 => {
                let index = choisir_compte(&comptes);
                if let Some(i) = index {
                    comptes[i].afficher_solde();
                }
            },
            3 => {
                let index = choisir_compte(&comptes);
                if let Some(i) = index {
                    println!("Montant à déposer : ");
                    if let Some(montant) = lire_f64() {
                        comptes[i].deposer(montant);
                    }
                }
            },
            4 => {
                let index = choisir_compte(&comptes);
                if let Some(i) = index {
                    println!("Montant à retirer : ");
                    if let Some(montant) = lire_f64() {
                        comptes[i].retirer(montant);
                    }
                }
            },
            5 => {
                let index = choisir_compte(&comptes);
                if let Some(i) = index {
                    println!("Nouveau nom pour le compte : ");
                    let mut nouveau_nom = String::new();
                    io::stdin().read_line(&mut nouveau_nom).expect("Erreur");
                    let nouveau_nom = nouveau_nom.trim();
                    comptes[i] = comptes[i].renommer(nouveau_nom);
                    println!("Compte renommé avec succès.");
                }
            },
            6 => {
                println!("Au revoir !");
                break;
            },
            _ => println!("Option invalide."),
        }
    }
}

// Fonction utilitaire pour choisir un compte
fn choisir_compte(comptes: &[CompteBancaire]) -> Option<usize> {
    println!("Sélectionnez un compte :");
    for (i, compte) in comptes.iter().enumerate() {
        println!("{} - {}", i + 1, compte.nom);
    }

    let mut choix = String::new();
    io::stdin().read_line(&mut choix).expect("Erreur");
    match choix.trim().parse::<usize>() {
        Ok(num) if num >= 1 && num <= comptes.len() => Some(num - 1),
        _ => {
            println!("Choix invalide.");
            None
        }
    }
}

// Lire un nombre flottant (montant)
fn lire_f64() -> Option<f64> {
    let mut entree = String::new();
    io::stdin().read_line(&mut entree).expect("Erreur");
    match entree.trim().parse::<f64>() {
        Ok(val) => Some(val),
        Err(_) => {
            println!("Entrée invalide.");
            None
        }
    }
}
