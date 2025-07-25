// src/protocole.rs
// Définition du protocole de messagerie instantanée "SimpleChat Protocol" (SCP)

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Version du protocole
pub const PROTOCOL_VERSION: u8 = 1;

/// Taille maximale d'un message (64KB)
pub const MAX_MESSAGE_SIZE: usize = 65536;

/// Identifiant unique pour chaque client
pub type ClientId = String;

/// Identifiant unique pour chaque salon
pub type RoomId = String;

/// États possibles d'une session client
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// Client connecté mais pas encore authentifié
    Connected,
    /// Client authentifié avec un nom d'utilisateur
    Authenticated(String),
    /// Client a rejoint un salon
    InRoom(String, RoomId), // (username, room_id)
    /// Session fermée
    Closed,
}

/// Types de messages du protocole SCP
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] // Added PartialEq for testing
#[serde(tag = "type", content = "data")] // Allows distinguishing messages by a "type" field
pub enum Message {
    // --- Messages client vers serveur ---

    /// Connexion initiale avec nom d'utilisateur
    Connect { username: String },

    /// Rejoindre un salon
    JoinRoom { room_id: String },

    /// Quitter le salon actuel
    LeaveRoom,

    /// Envoyer un message dans le salon
    SendMessage { content: String },

    /// Message privé à un utilisateur
    PrivateMessage { target_user: String, content: String },

    /// Lister les salons disponibles
    ListRooms,

    /// Lister les utilisateurs dans le salon actuel
    ListUsers,

    /// Déconnexion propre
    Disconnect,

    // --- Messages serveur vers client ---

    /// Confirmation de connexion
    ConnectAck { client_id: String, message: String },

    /// Erreur lors de la connexion
    ConnectError { reason: String },

    /// Confirmation d'entrée dans un salon
    JoinRoomAck { room_id: String, users: Vec<String> },

    /// Erreur lors de l'entrée dans un salon
    JoinRoomError { reason: String },

    /// Notification qu'un utilisateur a rejoint le salon
    UserJoined { username: String, room_id: String },

    /// Notification qu'un utilisateur a quitté le salon
    UserLeft { username: String, room_id: String },

    /// Message reçu dans le salon
    RoomMessage {
        from: String,
        content: String,
        timestamp: DateTime<Utc>,
        room_id: String
    },

    /// Message privé reçu
    PrivateMessageReceived {
        from: String,
        content: String,
        timestamp: DateTime<Utc>
    },

    /// Liste des salons disponibles
    RoomList { rooms: HashMap<String, usize> }, // room_id -> nombre d'utilisateurs

    /// Liste des utilisateurs dans le salon
    UserList { users: Vec<String>, room_id: String },

    /// Erreur générale
    Error { code: ErrorCode, message: String },

    /// Ping pour maintenir la connexion
    Ping,

    /// Réponse au ping
    Pong,
}

/// Codes d'erreur du protocole
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] // Added PartialEq
pub enum ErrorCode {
    /// Nom d'utilisateur déjà pris
    UsernameAlreadyTaken,
    /// Salon inexistant
    RoomNotFound,
    /// Utilisateur non trouvé
    UserNotFound,
    /// Action non autorisée dans l'état actuel
    InvalidState,
    /// Format de message invalide
    InvalidFormat,
    /// Message trop volumineux
    MessageTooLarge,
    /// Limite de débit dépassée (non implémenté ici, mais bonne pratique)
    RateLimitExceeded,
    /// Erreur serveur interne
    InternalError,
}

/// Structure pour encapsuler un message avec des métadonnées
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] // Added PartialEq
pub struct ProtocolFrame {
    /// Version du protocole
    pub version: u8,
    /// Identifiant de session (optionnel, défini par le serveur à la connexion)
    pub session_id: Option<String>,
    /// Numéro de séquence pour l'ordre des messages
    pub sequence: u64,
    /// Le message lui-même
    pub message: Message,
    /// Timestamp d'envoi
    pub timestamp: DateTime<Utc>,
}

impl ProtocolFrame {
    /// Créer une nouvelle trame
    pub fn new(message: Message, session_id: Option<String>, sequence: u64) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            session_id,
            sequence,
            message,
            timestamp: Utc::now(),
        }
    }

    /// Sérialiser la trame en JSON
    pub fn serialize(&self) -> Result<Vec<u8>, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(json.into_bytes())
    }

    /// Désérialiser une trame depuis JSON
    pub fn deserialize(data: &[u8]) -> Result<Self, serde_json::Error> {
        let json = String::from_utf8_lossy(data);
        serde_json::from_str(&json)
    }

    /// Valider la trame (côté serveur principalement)
    pub fn validate(&self) -> Result<(), String> {
        if self.version != PROTOCOL_VERSION {
            return Err(format!("Version de protocole non supportée: {}", self.version));
        }

        // Vérifier la taille du message sérialisé (utile avant l'envoi aussi)
        // Note: Cette validation est pour la taille du message *après* sérialisation JSON.
        // Si le contenu brut (par ex. un très long string) dépasse MAX_MESSAGE_SIZE avant même la sérialisation,
        // cette vérification ne le détectera pas à ce stade. Elle est surtout pour les messages entrants.
        let serialized_len = self.serialize()
            .map_err(|e| format!("Erreur de sérialisation interne pour validation de taille: {}", e))?
            .len();

        if serialized_len > MAX_MESSAGE_SIZE {
            return Err(format!("Message trop volumineux: {} bytes (max: {})", serialized_len, MAX_MESSAGE_SIZE));
        }

        Ok(())
    }
}

/// Utilitaires pour le protocole
impl Message {
    /// Vérifier si un message nécessite une authentification
    pub fn requires_auth(&self) -> bool {
        matches!(self,
            Message::JoinRoom { .. } |
            Message::LeaveRoom |
            Message::SendMessage { .. } |
            Message::PrivateMessage { .. } |
            Message::ListRooms |
            Message::ListUsers |
            Message::Disconnect // Disconnect should be from an authenticated client
        )
    }

    /// Vérifier si un message nécessite d'être dans un salon
    pub fn requires_room(&self) -> bool {
        matches!(self,
            Message::SendMessage { .. } |
            Message::ListUsers
        )
    }
}

/// Structure pour représenter l'état d'un salon
#[derive(Debug, Clone, PartialEq)] // Added PartialEq
pub struct Room {
    pub id: RoomId,
    pub name: String,
    pub users: HashMap<ClientId, String>, // client_id -> username
    pub created_at: DateTime<Utc>,
}

impl Room {
    pub fn new(id: RoomId, name: String) -> Self {
        Self {
            id,
            name,
            users: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_user(&mut self, client_id: ClientId, username: String) {
        self.users.insert(client_id, username);
    }

    pub fn remove_user(&mut self, client_id: &ClientId) -> Option<String> {
        self.users.remove(client_id)
    }

    pub fn get_usernames(&self) -> Vec<String> {
        self.users.values().cloned().collect()
    }

    pub fn user_count(&self) -> usize {
        self.users.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_frame_serialization() {
        let message = Message::Connect { username: "test_user".to_string() };
        let frame = ProtocolFrame::new(message, Some("session123".to_string()), 1);

        let serialized = frame.serialize().unwrap();
        let deserialized = ProtocolFrame::deserialize(&serialized).unwrap();

        // Use PartialEq derived on the structs/enums
        assert_eq!(frame, deserialized);
    }

    #[test]
    fn test_message_validation() {
        let message = Message::Connect { username: "test".to_string() };
        assert!(!message.requires_auth());
        assert!(!message.requires_room());

        let message = Message::SendMessage { content: "hello".to_string() };
        assert!(message.requires_auth());
        assert!(message.requires_room());
    }

    #[test]
    fn test_protocol_frame_validation_max_size() {
        // Create a message that is intentionally too large after serialization
        let long_content = "a".repeat(MAX_MESSAGE_SIZE / 2); // Half of max size
        let message = Message::SendMessage { content: long_content };
        let frame = ProtocolFrame::new(message, Some("session_id".to_string()), 1);

        // This message should be fine as it's below MAX_MESSAGE_SIZE even after JSON overhead
        let result = frame.validate();
        assert!(result.is_ok(), "Should be OK for a message within size limits: {:?}", result);

        // Now, let's create a message that will exceed the limit
        let super_long_content = "b".repeat(MAX_MESSAGE_SIZE + 100); // Definitely too large
        let large_message = Message::SendMessage { content: super_long_content };
        let large_frame = ProtocolFrame::new(large_message, Some("session_id_2".to_string()), 2);

        let result_large = large_frame.validate();
        assert!(result_large.is_err());
        assert!(result_large.unwrap_err().contains("Message trop volumineux"));
    }
}