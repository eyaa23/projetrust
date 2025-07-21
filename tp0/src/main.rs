use std::io;
fn main() {

    let nom = "Kevin";
    let age:u32 = 30;  // u32 = entier non signé sur 32 bits ( valeurs positives)
    let age_papa = 70; // rust comprend que c'est un entier par défaut i32 
    let temperature:f32 = 32.5 ; 
    println!("Hello, world! {}",nom);
    println!("J'ai {} ans.", age ); 
    println!("Papa il a  {} ans.", age_papa ); 
    println!("Il fait {} aujourd'hui", temperature);
    // Il faut utiliser  les snake_case ( par convention de RUST )
    // ne jamais commencer par chiffre, pas d'espaces ni tirets 

    // i32    32   signé   -2xxx  à 2xxxxxxx
    // u32     32   non signé       0 à 4 xxxxxxx
    // i64     64     signé         très grand intervalle
    // u8    8     non signé      à à 255 


    // 2. les fonctions  : 
       // fn définit une fonction 
       // &str est de type de chaine de caractères ( référence)
       // on cree une fonction addtion() qui retourne une somme et on l'appelle depuis le main
      let resultat = addition(12,3);
       println!("La somme est : {}", resultat);
       say_hello("Loggi Hello");



       // Les conditions les boucles 

         let nombre = 16;
          if nombre %2 == 0 {
            println!("Pair");
          } else {
             println!("Impair");
          }

      // une boucle 
         for i in 1..=10{
            println!(" i vaut {}", i);
         }    

         // A noter que  1..5
         //  ..  intervalle exculsif ( fin exclue ) : 1,2,3,4
         // ..=  intervalle inclusif ( fin incluse ) : 1,2,3,4,5



         // Exemple de tableau : itérer sur un tableau 

         let  voitures = ["jeep", "renault", "bmw"];
         for voiture in voitures {
            println!("Voiture : {}", voiture);
         }

         //    for ( index, valeur) in  collection.iter().enumerate(){
         //  on peut utiliser index et valeur ici }

         // je reprends l'exemple de voiture 
         for (i,voiture) in voitures.iter().enumerate(){
            println!("Index {} : {}", i, voiture);
         }
         // iter(): crée un itérateur sur la collection sans le consommer
         // enumerate: transforme l'itérateur en une séquence de index,valeur 

         // Exemple de vecteur 

         let noms = vec![String::from("Kevin"), String::from("Nourdine")];
         for(i,nom) in noms.iter().enumerate(){
            println!("Nom {} :{}", i, nom);
         }

         // Usage de enumerate dans un cas réel : Afficher un Menu avec numéro et choix

         let options = ["Afficher solde","Retrait","Liste comptes","Quitter"];

         println!("Menu:");
         for ( i,option) in options.iter().enumerate(){
            // afficher chaque option et on commence par 1 
            println!("{}.{}", i+1, option); 
         }

         println!("Veuillez saisir un numéro de votre choix:");

         let mut choix = String::new();
         io::stdin().read_line(&mut choix).expect("Attention erreur de lecture");
         
         let choix:usize = match choix.trim().parse(){
            Ok(num) => num,
            Err(_)=> {
                println!("Veuillez saisir un numero valide");
                return;
            }
         };

         if choix == 0 || choix > options.len(){
            println!(" choix hors système !! limite système ");
         } else {
            println!("Vous avez sélectionné : {}", options[choix-1]);
            // ici on peut exécuter une action selon choix dans options 
         }


}

   fn addition(n1:i32, n2:i32) -> i32{   //  -> i32 retourne un entier 
       n1+n2
   }

   fn say_hello( nom :&str){
    println!("Bonjour, {}", nom);
   }

