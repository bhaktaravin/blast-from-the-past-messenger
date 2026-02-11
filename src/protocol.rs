use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HistoryTarget {
    Lobby,
    Direct { username: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub from: String,
    pub body: String,
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatus {
    pub username: String,
    pub away: Option<String>,
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
    AddFriend { username: String, nickname: Option<String> },
    FriendRequest { to: String },
    RespondToFriendRequest { from: String, accepted: bool },
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
    FriendAdded { username: String },
    FriendRequest { from: String },
    FriendRequestResponse { from: String, accepted: bool },
}


