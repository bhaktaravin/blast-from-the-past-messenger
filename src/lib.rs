pub mod db;
pub mod network;
pub mod protocol;
pub mod update;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub from: String,
    pub body: String,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChatTarget {
    Lobby,
    Direct(String),
}
