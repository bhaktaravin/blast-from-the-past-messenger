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
    pub id: Option<i64>,      // Message ID for read receipts
    pub read_count: Option<i32>, // Number of people who read this message
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatus {
    pub username: String,
    pub away: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToClient {
    Welcome { message: String },
    AuthOk { username: String },
    AuthError { message: String },
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
}


