// src/bin/client.rs
// Client de messagerie utilisant le protocole SCP

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, stdin};
use tokio::sync::mpsc;
use std::io::{self, BufReader, BufRead};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

// Import elements from the `protocole` module
use tp8::protocole::{
    PROTOCOL_VERSION, MAX_MESSAGE_SIZE, Message, ProtocolFrame, ErrorCode,
    ClientId, RoomId, SessionState
};

/// Client local state
struct ClientLocalState {
    id: Option<ClientId>,
    username: Option<String>,
    current_room: Option<RoomId>,
    session_state: SessionState,
}

impl ClientLocalState {
    fn new() -> Self {
        Self {
            id: None,
            username: None,
            current_room: None,
            session_state: SessionState::Connected,
        }
    }

    fn update_state(&mut self, new_state: SessionState) {
        self.session_state = new_state;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("👋 === CLIENT DE MESSAGERIE (SCP v{}) ===", PROTOCOL_VERSION);

    let addr = "127.0.0.1:9999";
    println!("Tentative de connexion au serveur sur {}", addr);

    let stream = TcpStream::connect(addr).await?;
    println!("✅ Connecté au serveur sur {}", addr);

    // Split stream into read and write halves for concurrent operations
    let (mut reader, mut writer) = stream.into_split();

    // Channel for internal client messages (e.g., from command input to sender task)
    let (tx_commands, mut rx_commands) = mpsc::unbounded_channel::<ClientCommand>();

    // Shared state for the client (Arc<RwLock<...>>)
    let client_state = Arc::new(RwLock::new(ClientLocalState::new()));
    let client_state_for_sender = Arc::clone(&client_state);

    // --- Sender Task ---
    // Reads commands from `rx_commands` and sends them over the network
    let send_task = tokio::spawn(async move {
        while let Some(command) = rx_commands.recv().await {
            let current_client_state = client_state_for_sender.read().await;

            let frame = match process_client_command(command, &current_client_state) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Client command error: {}", e);
                    continue;
                }
            };

            if let Ok(data) = frame.serialize() {
                let length = data.len() as u32;

                if writer.write_all(&length.to_be_bytes()).await.is_err() {
                    eprintln!("❌ Error writing message length to server. Connection lost.");
                    break;
                }
                if writer.write_all(&data).await.is_err() {
                    eprintln!("❌ Error writing message data to server. Connection lost.");
                    break;
                }
            } else {
                eprintln!("❌ Error serializing message to send.");
            }
        }
        println!("⚙️ Send task finished.");
    });

    // --- Reader Task ---
    // Reads incoming messages from the network and prints them
    let client_state_for_reader = Arc::clone(&client_state);
    let receive_task = tokio::spawn(async move {
        let mut buffer = vec![0u8; MAX_MESSAGE_SIZE];

        loop {
            let mut length_buf = [0u8; 4];
            match reader.read_exact(&mut length_buf).await {
                Ok(0) => {
                    println!("🔌 Server closed the connection.");
                    break;
                },
                Ok(_) => {
                    let length = u32::from_be_bytes(length_buf) as usize;

                    if length > MAX_MESSAGE_SIZE {
                        eprintln!("❌ Received message too large ({} bytes). Max is {} bytes.", length, MAX_MESSAGE_SIZE);
                        // Attempt to consume the rest of the malformed message to avoid misalignment
                        let _ = reader.read_exact(&mut buffer[0..MAX_MESSAGE_SIZE]).await; // Read up to max
                        continue; // Skip to next message
                    }
                    if length == 0 { // Empty message after length, skip
                        continue;
                    }

                    buffer.resize(length, 0); // Resize buffer to exact message length
                    match reader.read_exact(&mut buffer).await {
                        Ok(_) => {
                            match ProtocolFrame::deserialize(&buffer) {
                                Ok(frame) => {
                                    handle_server_message(frame, &client_state_for_reader).await;
                                }
                                Err(e) => {
                                    eprintln!("❌ Deserialization error from server: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Error reading message data from server: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error reading message length from server: {}", e);
                    break;
                }
            }
        }
        println!("⚙️ Receive task finished.");
    });

    // --- Input Loop ---
    // Reads user input from console and sends commands to `tx_commands`
    let stdin = stdin();
    let mut reader = BufReader::new(stdin).lines();

    println!("Enter your commands:");
    println!("  /connect <username>");
    println!("  /join <room_id>");
    println!("  /leave");
    println!("  /msg <message>");
    println!("  /priv <username> <message>");
    println!("  /rooms");
    println!("  /users");
    println!("  /quit");
    println!("  /ping");
    println!("------------------------------------");

    loop {
        print!("> ");
        io::stdout().flush().await?; // Ensure prompt is displayed

        let line = match reader.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => { // EOF, stdin closed
                println!("EOF received from stdin. Quitting...");
                break;
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let command = parts[0];

        let cmd = match command {
            "/connect" => {
                if parts.len() < 2 {
                    println!("Usage: /connect <username>");
                    continue;
                }
                ClientCommand::Connect(parts[1].to_string())
            }
            "/join" => {
                if parts.len() < 2 {
                    println!("Usage: /join <room_id>");
                    continue;
                }
                ClientCommand::JoinRoom(parts[1].to_string())
            }
            "/leave" => ClientCommand::LeaveRoom,
            "/msg" => {
                if parts.len() < 2 {
                    println!("Usage: /msg <message>");
                    continue;
                }
                ClientCommand::SendMessage(parts[1].to_string())
            }
            "/priv" => {
                let sub_parts: Vec<&str> = parts[1..].join(" ").splitn(2, ' ').collect();
                if sub_parts.len() < 2 {
                    println!("Usage: /priv <username> <message>");
                    continue;
                }
                ClientCommand::PrivateMessage(sub_parts[0].to_string(), sub_parts[1].to_string())
            }
            "/rooms" => ClientCommand::ListRooms,
            "/users" => ClientCommand::ListUsers,
            "/ping" => ClientCommand::Ping,
            "/quit" => {
                println!("Quitting...");
                tx_commands.send(ClientCommand::Disconnect)?; // Send disconnect message to server
                break; // Exit input loop
            }
            _ => {
                println!("Unknown command: {}", command);
                continue;
            }
        };

        if tx_commands.send(cmd).is_err() {
            eprintln!("Error sending command to sender task. Server connection might be closed.");
            break;
        }
    }

    // Await tasks to ensure they complete cleanup or are aborted
    let _ = send_task.await;
    let _ = receive_task.await;

    println!("Client disconnected. Goodbye!");
    Ok(())
}

/// Internal commands for the client
enum ClientCommand {
    Connect(String),
    JoinRoom(String),
    LeaveRoom,
    SendMessage(String),
    PrivateMessage(String, String),
    ListRooms,
    ListUsers,
    Disconnect,
    Ping,
}

/// Processes a client command and converts it into a ProtocolFrame
fn process_client_command(
    command: ClientCommand,
    client_state: &ClientLocalState,
) -> Result<ProtocolFrame, String> {
    let message = match command {
        ClientCommand::Connect(username) => Message::Connect { username },
        ClientCommand::JoinRoom(room_id) => Message::JoinRoom { room_id },
        ClientCommand::LeaveRoom => Message::LeaveRoom,
        ClientCommand::SendMessage(content) => Message::SendMessage { content },
        ClientCommand::PrivateMessage(target_user, content) => Message::PrivateMessage { target_user, content },
        ClientCommand::ListRooms => Message::ListRooms,
        ClientCommand::ListUsers => Message::ListUsers,
        ClientCommand::Disconnect => Message::Disconnect,
        ClientCommand::Ping => Message::Ping,
    };

    let session_id = client_state.id.clone();
    let sequence = 0; // Client doesn't track sequence numbers for outgoing requests in this simple example

    Ok(ProtocolFrame::new(message, session_id, sequence))
}

/// Handles incoming messages from the server
async fn handle_server_message(frame: ProtocolFrame, client_state: &Arc<RwLock<ClientLocalState>>) {
    let mut state = client_state.write().await;

    match frame.message {
        Message::ConnectAck { client_id, message } => {
            state.id = Some(client_id.clone());
            state.username = Some(message.split("Bienvenue, ").last().unwrap_or("unknown").trim_end_matches('!').to_string());
            state.update_state(SessionState::Authenticated(state.username.clone().unwrap_or_default()));
            println!("\n[SERVER] {}", message);
            println!("Your Client ID: {}", client_id);
            println!("You are now authenticated as: {}", state.username.as_ref().unwrap_or(&"N/A".to_string()));
        }
        Message::ConnectError { reason } => {
            println!("\n[SERVER ERROR] Connection failed: {}", reason);
            state.update_state(SessionState::Closed); // Consider session closed on connection error
        }
        Message::JoinRoomAck { room_id, users } => {
            state.current_room = Some(room_id.clone());
            if let Some(username) = &state.username {
                state.update_state(SessionState::InRoom(username.clone(), room_id.clone()));
            }
            println!("\n[SERVER] Joined room: #{}", room_id);
            println!("Users in #{}: {}", room_id, users.join(", "));
        }
        Message::JoinRoomError { reason } => {
            println!("\n[SERVER ERROR] Failed to join room: {}", reason);
        }
        Message::UserJoined { username, room_id } => {
            println!("\n[ROOM #{}] {} has joined.", room_id, username);
        }
        Message::UserLeft { username, room_id } => {
            println!("\n[ROOM #{}] {} has left.", room_id, username);
        }
        Message::RoomMessage { from, content, timestamp, room_id } => {
            println!("\n[#{}] <{}> {}: {}", room_id, timestamp.format("%H:%M:%S"), from, content);
        }
        Message::PrivateMessageReceived { from, content, timestamp } => {
            println!("\n[PRIVATE from {}] <{}>: {}", from, timestamp.format("%H:%M:%S"), content);
        }
        Message::RoomList { rooms } => {
            println!("\n[SERVER] Available Rooms:");
            if rooms.is_empty() {
                println!("  No rooms available.");
            } else {
                for (room_id, user_count) in rooms {
                    println!("  - #{} ({} users)", room_id, user_count);
                }
            }
        }
        Message::UserList { users, room_id } => {
            println!("\n[SERVER] Users in #{}:", room_id);
            if users.is_empty() {
                println!("  No users in this room.");
            } else {
                for user in users {
                    println!("  - {}", user);
                }
            }
        }
        Message::Error { code, message } => {
            println!("\n[SERVER ERROR] Code: {:?}, Message: {}", code, message);
        }
        Message::Pong => {
            println!("\n[SERVER] Pong!");
        }
        // Client should not receive these message types directly as responses
        _ => {
            eprintln!("\n[SERVER] Received unexpected message type: {:?}", frame.message);
        }
    }
    print!("> ");
    let _ = io::stdout().flush().await; // Re-display prompt after server message
}