use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HistoryTarget {
    Lobby,
    Direct { username: String },
    Room { room_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub from: String,
    pub body: String,
    pub at: String,
    pub id: Option<i64>,
    pub read_count: Option<i32>,
    /// ID of the message being replied to
    pub reply_to_id: Option<i64>,
    /// Snippet of the replied-to message body
    pub reply_to_body: Option<String>,
    pub reply_to_from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatus {
    pub username: String,
    pub away: Option<String>,
    /// Custom status emoji + text e.g. "🎮 Playing Halo"
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRoom {
    pub id: String,
    pub name: String,
    pub member_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientToServer {
    Register { username: String, password: String },
    Login { username: String, password: String },
    /// E2E key exchange: send our public key to a peer via the server
    ExchangeKey { to: String, public_key: String },
    /// E2E encrypted DM — body is base64(nonce + ciphertext), opaque to server
    EncryptedDirectMessage { to: String, encrypted_body: String },
    SetAway { away: Option<String> },
    Chat { body: String },
    DirectMessage { to: String, body: String },
    FetchThreads,
    FetchHistory { target: HistoryTarget },
    Search { target: HistoryTarget, query: String },
    Block { username: String },
    Unblock { username: String },
    Mute { username: String },
    Unmute { username: String },
    Report { username: String, reason: String },
    AddFriend { username: String },
    AcceptFriendRequest { username: String },
    DeclineFriendRequest { username: String },
    CreateChatRoom { name: String },
    JoinChatRoom { room_id: String },
    LeaveChatRoom { room_id: String },
    SendRoomMessage { room_id: String, body: String },
    FetchChatRooms,
    FetchRoomMembers { room_id: String },
    StartTyping { room_id: String },
    StopTyping { room_id: String },
    MarkMessageAsRead { message_id: i64 },
    /// Edit a previously sent message
    EditMessage { message_id: i64, new_body: String },
    /// Delete a previously sent message
    DeleteMessage { message_id: i64 },
    /// React to a message with an emoji
    ReactToMessage { message_id: i64, emoji: String },
    /// Nudge a user (screen shake)
    Nudge { to: String },
    /// Update custom status (shown in buddy list)
    SetStatus { status: Option<String> },
    /// Update profile bio
    SetBio { bio: String },
    /// Fetch a user's profile
    FetchProfile { username: String },
    /// Reply to a specific message
    ReplyToMessage { reply_to_id: i64, body: String },
    ReplyToDirect { to: String, reply_to_id: i64, body: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToClient {
    Welcome { message: String },
    AuthOk { username: String },
    AuthError { message: String },
    /// Relay a peer's public key to us for E2E key exchange
    KeyExchange { from: String, public_key: String },
    /// Relay an encrypted DM — body is opaque to the server
    EncryptedDirectMessage { from: String, encrypted_body: String },
    Presence { users: Vec<UserStatus> },
    Chat { from: String, body: String },
    DirectMessage { from: String, body: String },
    Threads { users: Vec<String> },
    History { target: HistoryTarget, messages: Vec<MessageRecord> },
    SearchResults {
        target: HistoryTarget,
        query: String,
        messages: Vec<MessageRecord>,
    },
    System { message: String },
    AddFriendResult { _username: String, success: bool, message: String },
    FriendRequest { from: String },
    FriendRequestResult { username: String, accepted: bool },
    ChatRoomCreated { room_id: String, name: String },
    ChatRoomList { rooms: Vec<ChatRoom> },
    RoomMessage { room_id: String, from: String, body: String, message_id: i64 },
    UserJoinedRoom { room_id: String, username: String },
    UserLeftRoom { room_id: String, username: String },
    RoomMembers { room_id: String, members: Vec<String> },
    UserTyping { room_id: String, username: String },
    UserStoppedTyping { room_id: String, username: String },
    ReadReceipt { message_id: i64, read_by: String },
    /// Broadcast an edited message to relevant peers
    MessageEdited { message_id: i64, new_body: String, edited_by: String },
    /// Broadcast a deleted message to relevant peers
    MessageDeleted { message_id: i64, deleted_by: String },
    /// A reaction was added to a message
    MessageReaction { message_id: i64, emoji: String, from: String },
    /// Someone nudged you
    Nudged { from: String },
    /// A user's profile data
    ProfileData { username: String, bio: String, status: Option<String>, joined: String },
}


