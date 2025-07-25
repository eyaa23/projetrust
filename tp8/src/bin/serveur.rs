// src/bin/serveur.rs
// Serveur de messagerie utilisant le protocole SCP

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

// Import elements from the `protocole` module, which is now in our crate
use tp8::protocole::{
    PROTOCOL_VERSION, MAX_MESSAGE_SIZE, Message, ProtocolFrame, ErrorCode,
    ClientId, RoomId, Room, SessionState
};

/// Structure representing a connected client
#[derive(Debug, Clone)]
struct Client {
    id: ClientId,
    username: Option<String>,
    current_room: Option<RoomId>,
    session_state: SessionState,
    sequence_number: u64, // Sequence number for messages sent by this client
}

impl Client {
    fn new(id: ClientId) -> Self {
        Self {
            id,
            username: None,
            current_room: None,
            session_state: SessionState::Connected,
            sequence_number: 0,
        }
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence_number += 1;
        self.sequence_number
    }
}

/// Global server state
struct ServerState {
    clients: HashMap<ClientId, Client>,
    rooms: HashMap<RoomId, Room>,
    username_to_client: HashMap<String, ClientId>, // To find a client by username
    client_senders: HashMap<ClientId, tokio::sync::mpsc::UnboundedSender<ProtocolFrame>>, // To send messages to specific clients
}

impl ServerState {
    fn new() -> Self {
        let mut state = Self {
            clients: HashMap::new(),
            rooms: HashMap::new(),
            username_to_client: HashMap::new(),
            client_senders: HashMap::new(),
        };

        // Create some default rooms
        state.rooms.insert("general".to_string(), Room::new("general".to_string(), "Salon Général".to_string()));
        state.rooms.insert("tech".to_string(), Room::new("tech".to_string(), "Discussions Tech".to_string()));
        state.rooms.insert("random".to_string(), Room::new("random".to_string(), "Discussions Libres".to_string()));

        state
    }

    fn add_client(&mut self, client_id: ClientId, sender: tokio::sync::mpsc::UnboundedSender<ProtocolFrame>) {
        self.clients.insert(client_id.clone(), Client::new(client_id.clone()));
        self.client_senders.insert(client_id, sender);
    }

    fn remove_client(&mut self, client_id: &ClientId) {
        if let Some(client) = self.clients.get(client_id) {
            // Remove from username -> client map if the user was authenticated
            if let Some(username) = &client.username {
                self.username_to_client.remove(username);
            }

            // Remove from the current room if the user was in one
            if let Some(room_id) = &client.current_room {
                if let Some(room) = self.rooms.get_mut(room_id) {
                    if room.remove_user(client_id).is_some() {
                        // Notify other room members that the user left
                        let notification = Message::UserLeft {
                            username: client.username.clone().unwrap_or_else(|| "un client anonyme".to_string()),
                            room_id: room_id.clone(),
                        };
                        let frame = ProtocolFrame::new(notification, None, 0); // Sequence 0 for notifications
                        self.broadcast_to_room(room_id, frame, Some(client_id));
                    }
                }
            }
        }

        // Remove the client and its sender
        self.clients.remove(client_id);
        self.client_senders.remove(client_id);
    }

    fn authenticate_client(&mut self, client_id: &ClientId, username: String) -> Result<(), String> {
        // Check if username is already taken
        if self.username_to_client.contains_key(&username) {
            return Err("Nom d'utilisateur déjà pris".to_string());
        }

        if let Some(client) = self.clients.get_mut(client_id) {
            // Ensure the client is in "Connected" state
            if !matches!(client.session_state, SessionState::Connected) {
                return Err(format!("Action non autorisée. Client déjà dans l'état: {:?}", client.session_state));
            }
            client.username = Some(username.clone());
            client.session_state = SessionState::Authenticated(username.clone());
            self.username_to_client.insert(username, client_id.clone());
            Ok(())
        } else {
            Err("Client non trouvé".to_string())
        }
    }

    fn join_room(&mut self, client_id: &ClientId, room_id: &str) -> Result<Vec<String>, String> {
        let client = self.clients.get_mut(client_id).ok_or("Client non trouvé")?;
        let username = client.username.clone().ok_or("Client non authentifié")?;

        // Check if the room exists
        if !self.rooms.contains_key(room_id) {
            return Err("Salon inexistant".to_string());
        }

        // Leave current room if applicable
        if let Some(old_room_id) = client.current_room.take() { // `take` removes the value and leaves `None`
            if let Some(old_room) = self.rooms.get_mut(&old_room_id) {
                old_room.remove_user(client_id);
                // Notify old room members
                let notification = Message::UserLeft {
                    username: username.clone(),
                    room_id: old_room_id.clone(),
                };
                let frame = ProtocolFrame::new(notification, None, 0);
                self.broadcast_to_room(&old_room_id, frame, Some(client_id));
                println!("🚪 {} a quitté le salon {}", username, old_room_id);
            }
        }

        // Join the new room
        client.current_room = Some(room_id.to_string());
        client.session_state = SessionState::InRoom(username.clone(), room_id.to_string());

        let room = self.rooms.get_mut(room_id).unwrap(); // We know the room exists
        room.add_user(client_id.clone(), username);

        Ok(room.get_usernames())
    }

    fn leave_room(&mut self, client_id: &ClientId) -> Result<(), String> {
        let client = self.clients.get_mut(client_id).ok_or("Client non trouvé")?;
        let username = client.username.clone().ok_or("Client non authentifié")?;

        if let Some(room_id) = client.current_room.take() {
            client.session_state = SessionState::Authenticated(username.clone());

            // Remove from room
            if let Some(room) = self.rooms.get_mut(&room_id) {
                room.remove_user(client_id);
            }

            // Notify other room users
            let notification = Message::UserLeft {
                username: username.clone(),
                room_id: room_id.clone(),
            };
            let frame = ProtocolFrame::new(notification, None, 0);
            self.broadcast_to_room(&room_id, frame, Some(client_id));

            println!("🚪 {} a quitté le salon {}", username, room_id);
            Ok(())
        } else {
            Err("Vous n'êtes pas dans un salon".to_string())
        }
    }

    // Helper function to send a message to a specific client
    async fn send_message_to_client(&self, client_id: &ClientId, message: Message) {
        if let Some(sender) = self.client_senders.get(client_id) {
            let frame = ProtocolFrame::new(message, Some(client_id.clone()), 0); // Sequence 0 for server messages
            if sender.send(frame).is_err() {
                eprintln!("Error: Could not send message to channel for client {}. Perhaps disconnected.", client_id);
            }
        } else {
            eprintln!("Warning: Sender not found for client {}", client_id);
        }
    }

    fn broadcast_to_room(&self, room_id: &str, message_frame: ProtocolFrame, exclude_client: Option<&ClientId>) {
        if let Some(room) = self.rooms.get(room_id) {
            for client_id in room.users.keys() {
                if let Some(exclude) = exclude_client {
                    if client_id == exclude {
                        continue;
                    }
                }

                if let Some(sender) = self.client_senders.get(client_id) {
                    let _ = sender.send(message_frame.clone()); // Send a copy of the frame for this example
                }
            }
        }
    }

    fn send_private_message(&self, from_username: &str, to_username: &str, content: &str) -> Result<(), String> {
        let to_client_id = self.username_to_client.get(to_username)
            .ok_or("Utilisateur destinataire non trouvé")?;

        if let Some(sender) = self.client_senders.get(to_client_id) {
            let message = Message::PrivateMessageReceived {
                from: from_username.to_string(),
                content: content.to_string(),
                timestamp: Utc::now(),
            };
            let frame = ProtocolFrame::new(message, Some(to_client_id.clone()), 0); // Sequence 0 for server messages
            let _ = sender.send(frame).map_err(|e| format!("Error sending private message to channel: {}", e))?;
            Ok(())
        } else {
            Err("Unable to send message: Sender not found".to_string())
        }
    }
}

/// Main server handler
struct ChatServer {
    state: Arc<RwLock<ServerState>>,
}

impl ChatServer {
    fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ServerState::new())),
        }
    }

    async fn handle_client(&self, stream: TcpStream, client_id: ClientId) {
        println!("📱 Nouveau client connecté: {}", client_id);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Add the client to the server state
        {
            let mut state = self.state.write().await;
            state.add_client(client_id.clone(), tx);
        }

        // Clone the TcpStream for separate reading and writing.
        // `stream.try_clone()` returns a `Result`, so we use `.expect()` to unwrap it.
        let mut read_stream = stream.try_clone().expect("Failed to clone TcpStream for reading");
        let mut write_stream = stream; // The original `stream` is now moved into `write_stream`

        // Task to send messages to the client
        // This task takes ownership of `write_stream`
        let send_task = tokio::spawn(async move {
            while let Some(frame) = rx.recv().await {
                if let Ok(data) = frame.serialize() {
                    let length = data.len() as u32;

                    // Send length then data
                    // Check if writing fails (e.g., client disconnected)
                    if write_stream.write_all(&length.to_be_bytes()).await.is_err() {
                        eprintln!("❌ Error writing length to client {}. Connection might be closed.", client_id);
                        break;
                    }
                    if write_stream.write_all(&data).await.is_err() {
                        eprintln!("❌ Error writing data to client {}. Connection might be closed.", client_id);
                        break;
                    }
                } else {
                    eprintln!("❌ Error serializing frame to send to client {}.", client_id);
                }
            }
            println!("⚙️ Send task for client {} finished.", client_id);
        });

        // Main message reception loop
        // This loop uses `read_stream`
        let mut buffer = vec![0u8; 4096]; // Initial buffer size, will be resized if necessary

        loop {
            // Read message length (first 4 bytes)
            let mut length_buf = [0u8; 4];
            match read_stream.read_exact(&mut length_buf).await {
                Ok(0) => { // Connection closed by client (0 bytes read)
                    println!("🔌 Client {} disconnected (0 bytes read).", client_id);
                    break;
                },
                Ok(_) => {
                    let length = u32::from_be_bytes(length_buf) as usize;

                    if length > MAX_MESSAGE_SIZE {
                        eprintln!("❌ Message too large from client {}: {} bytes. Disconnecting.", client_id, length);
                        // Try to send an error to the client before closing the connection
                        let error_msg = Message::Error {
                            code: ErrorCode::MessageTooLarge,
                            message: format!("Message too large ({} bytes), max is {} bytes.", length, MAX_MESSAGE_SIZE),
                        };
                        let state_guard = self.state.read().await; // Read access to send error
                        state_guard.send_message_to_client(&client_id, error_msg).await;
                        break; // Break loop to disconnect client
                    }

                    // Resize buffer for full message and read it
                    buffer.resize(length, 0);
                    match read_stream.read_exact(&mut buffer).await {
                        Ok(_) => {
                            match ProtocolFrame::deserialize(&buffer) {
                                Ok(frame) => {
                                    if let Err(e) = self.process_message(frame, &client_id).await {
                                        eprintln!("❌ Error processing message from client {}: {}", client_id, e);
                                        // Send an internal error to the client
                                        let error_msg = Message::Error {
                                            code: ErrorCode::InternalError,
                                            message: format!("Processing error: {}", e),
                                        };
                                        let state_guard = self.state.read().await;
                                        state_guard.send_message_to_client(&client_id, error_msg).await;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ Deserialization error from client {}: {}. Disconnecting.", client_id, e);
                                    let error_msg = Message::Error {
                                        code: ErrorCode::InvalidFormat,
                                        message: format!("Invalid message format: {}", e),
                                    };
                                    let state_guard = self.state.read().await;
                                    state_guard.send_message_to_client(&client_id, error_msg).await;
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Error reading data from client {}: {}", client_id, e);
                            break; // Break loop on read error
                        }
                    }
                }
                Err(e) => {
                    // This error usually means the connection was lost
                    eprintln!("❌ Error reading message length from client {}: {}", client_id, e);
                    break;
                }
            }
        }

        // Cleanup on disconnection
        send_task.abort(); // Abort send task if it hasn't finished yet
        {
            let mut state = self.state.write().await;
            state.remove_client(&client_id);
            // The "Client disconnected" message is now handled within remove_client for notifications
        }
        println!("🔌 Client connection {} closed.", client_id);
    }

    async fn process_message(&self, frame: ProtocolFrame, client_id: &ClientId) -> Result<(), String> {
        // Validate the frame (version, size)
        frame.validate()?;

        // Access client state for state validation
        let client_state_guard = self.state.read().await;
        let current_client = client_state_guard.clients.get(client_id)
            .ok_or("Client not found in server state (internal error)")?;

        // Precondition checks for received message state
        match &frame.message {
            Message::Connect { .. } => {
                // Connect message is allowed only if the client is not already authenticated
                if !matches!(current_client.session_state, SessionState::Connected) {
                    let error_msg = format!("Already connected or authenticated. Current state: {:?}", current_client.session_state);
                    let response = Message::Error { code: ErrorCode::InvalidState, message: error_msg.clone() };
                    client_state_guard.send_message_to_client(client_id, response).await;
                    return Err(error_msg);
                }
            },
            _ => {
                // All other messages require authentication (except Ping which is handled below)
                if frame.message.requires_auth() && !matches!(current_client.session_state, SessionState::Authenticated(_) | SessionState::InRoom(_, _)) {
                    let error_msg = format!("Authentication required for this action. Current state: {:?}", current_client.session_state);
                    let response = Message::Error { code: ErrorCode::InvalidState, message: error_msg.clone() };
                    client_state_guard.send_message_to_client(client_id, response).await;
                    return Err(error_msg);
                }

                // Check if the message requires being in a room
                if frame.message.requires_room() && !matches!(current_client.session_state, SessionState::InRoom(_, _)) {
                    let error_msg = format!("Requires being in a room. Current state: {:?}", current_client.session_state);
                    let response = Message::Error { code: ErrorCode::InvalidState, message: error_msg.clone() };
                    client_state_guard.send_message_to_client(client_id, response).await;
                    return Err(error_msg);
                }
            }
        }

        // Release the read RwLock before operations that require a write RwLock
        drop(client_state_guard);

        // Message processing
        match frame.message {
            Message::Connect { username } => {
                self.handle_connect(client_id, username).await
            }
            Message::JoinRoom { room_id } => {
                self.handle_join_room(client_id, room_id).await
            }
            Message::LeaveRoom => {
                self.handle_leave_room(client_id).await
            }
            Message::SendMessage { content } => {
                self.handle_send_message(client_id, content).await
            }
            Message::PrivateMessage { target_user, content } => {
                self.handle_private_message(client_id, target_user, content).await
            }
            Message::ListRooms => {
                self.handle_list_rooms(client_id).await
            }
            Message::ListUsers => {
                self.handle_list_users(client_id).await
            }
            Message::Disconnect => {
                // Client requests explicit disconnection.
                // `handle_client` will manage connection closing and cleanup.
                println!("👋 Client {} sent DISCONNECT.", client_id);
                Ok(())
            }
            Message::Ping => {
                self.handle_ping(client_id).await
            }
            // Server-to-client messages should never be received here;
            // if so, it's a client protocol error.
            _ => {
                let error_msg = format!("Unexpected message type received from client: {:?}", frame.message);
                let response = Message::Error { code: ErrorCode::InvalidFormat, message: error_msg.clone() };
                let state_guard = self.state.read().await;
                state_guard.send_message_to_client(client_id, response).await;
                Err(error_msg)
            }
        }
    }

    async fn handle_connect(&self, client_id: &ClientId, username: String) -> Result<(), String> {
        let mut state = self.state.write().await;

        match state.authenticate_client(client_id, username.clone()) {
            Ok(()) => {
                let response = Message::ConnectAck {
                    client_id: client_id.clone(),
                    message: format!("Bienvenue, {} !", username),
                };
                state.send_message_to_client(client_id, response).await;
                println!("✅ Utilisateur {} authentifié ({})", username, client_id);
                Ok(())
            }
            Err(e) => {
                let response = Message::ConnectError { reason: e.clone() };
                state.send_message_to_client(client_id, response).await;
                Err(e)
            }
        }
    }

    async fn handle_join_room(&self, client_id: &ClientId, room_id: String) -> Result<(), String> {
        let mut state = self.state.write().await;

        match state.join_room(client_id, &room_id) {
            Ok(users_in_room) => {
                let response = Message::JoinRoomAck {
                    room_id: room_id.clone(),
                    users: users_in_room.clone(),
                };
                state.send_message_to_client(client_id, response).await;

                // Notify other users in the room that someone joined
                if let Some(client) = state.clients.get(client_id) {
                    if let Some(username) = &client.username {
                        let notification = Message::UserJoined {
                            username: username.clone(),
                            room_id: room_id.clone(),
                        };
                        let frame = ProtocolFrame::new(notification, None, 0); // Sequence 0 for notifications
                        state.broadcast_to_room(&room_id, frame, Some(client_id)); // Exclude the client who just joined
                        println!("🚪 {} a rejoint le salon {}", username, room_id);
                    }
                }
                Ok(())
            }
            Err(e) => {
                let response = Message::JoinRoomError { reason: e.clone() };
                state.send_message_to_client(client_id, response).await;
                Err(e)
            }
        }
    }

    async fn handle_leave_room(&self, client_id: &ClientId) -> Result<(), String> {
        let mut state = self.state.write().await;

        match state.leave_room(client_id) {
            Ok(_) => {
                // No specific success message for LeaveRoom, the client knows it left
                // A generic message could be sent if desired
                Ok(())
            },
            Err(e) => {
                let response = Message::Error {
                    code: ErrorCode::InvalidState,
                    message: e.clone(),
                };
                state.send_message_to_client(client_id, response).await;
                Err(e)
            }
        }
    }

    async fn handle_send_message(&self, client_id: &ClientId, content: String) -> Result<(), String> {
        let state = self.state.read().await;

        let client = state.clients.get(client_id).ok_or("Client not found")?;
        let username = client.username.as_ref().ok_or("Client not authenticated")?;
        let room_id = client.current_room.as_ref().ok_or("Client not in a room")?;

        let message = Message::RoomMessage {
            from: username.clone(),
            content: content.clone(),
            timestamp: Utc::now(),
            room_id: room_id.clone(),
        };

        let frame = ProtocolFrame::new(message, None, 0); // Sequence 0 for room messages
        state.broadcast_to_room(room_id, frame, None); // Broadcast to all members of the room

        println!("💬 [{}] {}: {}", room_id, username, content);
        Ok(())
    }

    async fn handle_private_message(&self, client_id: &ClientId, target_user: String, content: String) -> Result<(), String> {
        let state = self.state.read().await;

        let client = state.clients.get(client_id).ok_or("Client not found")?;
        let username = client.username.as_ref().ok_or("Client not authenticated")?;

        // Check that the target user is not the sender
        if username == &target_user {
            let error_msg = "You cannot send a private message to yourself.".to_string();
            let response = Message::Error { code: ErrorCode::InvalidState, message: error_msg.clone() };
            state.send_message_to_client(client_id, response).await;
            return Err(error_msg);
        }

        match state.send_private_message(username, &target_user, &content) {
            Ok(_) => {
                println!("📩 {} -> {} (privé): {}", username, target_user, content);
                Ok(())
            },
            Err(e) => {
                let response = Message::Error {
                    code: ErrorCode::UserNotFound, // Or other appropriate code
                    message: e.clone(),
                };
                state.send_message_to_client(client_id, response).await;
                Err(e)
            }
        }
    }

    async fn handle_list_rooms(&self, client_id: &ClientId) -> Result<(), String> {
        let state = self.state.read().await;

        let rooms: HashMap<String, usize> = state.rooms.iter()
            .map(|(id, room)| (id.clone(), room.user_count()))
            .collect();

        let response = Message::RoomList { rooms };
        state.send_message_to_client(client_id, response).await;

        Ok(())
    }

    async fn handle_list_users(&self, client_id: &ClientId) -> Result<(), String> {
        let state = self.state.read().await;

        let client = state.clients.get(client_id).ok_or("Client not found")?;
        let room_id = client.current_room.as_ref().ok_or("Client not in a room")?;

        if let Some(room) = state.rooms.get(room_id) {
            let response = Message::UserList {
                users: room.get_usernames(),
                room_id: room_id.clone(),
            };
            state.send_message_to_client(client_id, response).await;
        } else {
            // Should not happen if client.current_room is Some
            let response = Message::Error {
                code: ErrorCode::InternalError,
                message: "Room not found for user list.".to_string(),
            };
            state.send_message_to_client(client_id, response).await;
        }

        Ok(())
    }

    async fn handle_ping(&self, client_id: &ClientId) -> Result<(), String> {
        let state = self.state.read().await;
        let response = Message::Pong;
        state.send_message_to_client(client_id, response).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 === MESSAGING SERVER (SCP v{}) ===", PROTOCOL_VERSION);

    let server = ChatServer::new();
    let listener = TcpListener::bind("127.0.0.1:9999").await?;

    println!("📡 Server listening on 127.0.0.1:9999");
    println!("💡 Available rooms: general, tech, random");

    while let Ok((stream, addr)) = listener.accept().await {
        let client_id = Uuid::new_v4().to_string();
        println!("🔗 New connection: {} ({})", addr, client_id);

        let server_clone = ChatServer { // Clone the Arc reference to the server state
            state: Arc::clone(&server.state),
        };

        tokio::spawn(async move {
            server_clone.handle_client(stream, client_id).await;
        });
    }

    Ok(())
}