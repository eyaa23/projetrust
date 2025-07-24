# TP1 & TP2

Ce dépôt contient deux projets en Rust réalisés dans le cadre de TP :

- **TP1 : Gestionnaire de Comptes Bancaires**
- **TP2 : Gestionnaire de Fichiers avec Date et Opérations**

---

## TP1 : Gestionnaire de Comptes Bancaires

Ce programme permet de gérer plusieurs comptes bancaires via une interface en ligne de commande. Il supporte les opérations suivantes :

- Liste des comptes existants
- Affichage du solde d’un compte sélectionné
- Dépôt d’argent sur un compte
- Retrait d’argent d’un compte (avec vérification de solde)
- Renommage d’un compte
- Quitter le programme

### Fonctionnalités techniques

- Structure `CompteBancaire` avec champs `nom` et `solde`
- Implémentation des méthodes pour encapsuler la logique bancaire (`afficher_solde`, `deposer`, `retirer`, `renommer`)
- Utilisation d’une boucle `loop` et d’un `match` pour le menu principal
- Gestion des entrées utilisateur avec validation et traitement des erreurs
- Utilisation de la fonction `clone` pour renommer sans emprunt mutable partout

---

## TP2 : Gestionnaire de Fichiers

Ce programme offre un menu pour gérer un fichier texte dont le nom est saisi par l'utilisateur, avec les opérations suivantes :

- Lire le contenu du fichier
- Écrire dans le fichier (ajout en fin de fichier)
- Modifier le contenu du fichier (écrasement)
- Supprimer définitivement le fichier
- Quitter le programme

### Particularités

- Ajout automatique de l’extension `.txt` si l'utilisateur ne la fournit pas
- Ajout d’un horodatage lors de l’écriture dans le fichier, grâce à la crate `chrono`
- Encapsulation de la logique dans une structure `Fichier` avec méthodes (`lire`, `ecrire`, `modifier`, `supprimer`)
- Gestion des erreurs de lecture et écriture avec des messages utilisateur clairs
- Utilisation de `loop` et `match` pour le menu utilisateur

## Instructions d’exécution

1. Cloner le dépôt
2. Compiler avec `cargo build`
3. Lancer le programme avec `cargo run`
4. Suivre les instructions à l’écran


## Remarques

Ces projets respectent les principes de possession (`ownership`) et d’emprunt (`borrowing`) en Rust, utilisent des structures et implémentations (`impl`), ainsi que des boucles et correspondances (`loop`, `match`) pour une interface utilisateur claire et robuste.

