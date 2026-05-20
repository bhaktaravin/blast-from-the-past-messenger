//! Maps protocol messages to UI/network command enums (shared by native and web).

use crate::{
    ChatMessage, ChatTarget, HistoryTarget, NetToUi, UiToNet,
};
use chatmessagediscordclone::protocol::{ClientToServer, ServerToClient};

pub fn server_to_ui(event: ServerToClient) -> Option<NetToUi> {
    Some(match event {
        ServerToClient::Welcome { message } => {
            NetToUi::Chat {
                from: "Server".to_string(),
                body: message,
            }
        }
        ServerToClient::AuthOk { username } => NetToUi::AuthOk { username },
        ServerToClient::AuthError { message } => NetToUi::AuthError(message),
        ServerToClient::Presence { users } => NetToUi::Presence(users),
        ServerToClient::Chat { from, body } => NetToUi::Chat { from, body },
        ServerToClient::DirectMessage { from, body } => NetToUi::DirectMessage { from, body },
        ServerToClient::Threads { users } => NetToUi::Threads(users),
        ServerToClient::History { target, messages } => NetToUi::History {
            target: history_target_to_chat(target),
            messages: map_message_records(messages),
        },
        ServerToClient::SearchResults {
            target,
            query,
            messages,
        } => NetToUi::SearchResults {
            target: history_target_to_chat(target),
            query,
            messages: map_message_records(messages),
        },
        ServerToClient::System { message } => NetToUi::System(message),
        ServerToClient::AddFriendResult {
            _username,
            success,
            message,
        } => NetToUi::AddFriendResult {
            _username,
            success,
            message,
        },
        ServerToClient::FriendRequest { from } => NetToUi::FriendRequest { from },
        ServerToClient::FriendRequestResult { username, accepted } => {
            NetToUi::FriendRequestResult { username, accepted }
        }
        ServerToClient::KeyExchange { from, public_key } => {
            NetToUi::KeyExchange { from, public_key }
        }
        ServerToClient::EncryptedDirectMessage { from, encrypted_body } => {
            NetToUi::EncryptedDirectMessage { from, encrypted_body }
        }
        ServerToClient::ChatRoomCreated { room_id, name } => {
            NetToUi::ChatRoomCreated { room_id, name }
        }
        ServerToClient::ChatRoomList { rooms } => NetToUi::ChatRoomList {
            rooms: rooms
                .into_iter()
                .map(|r| (r.id, r.name, r.member_count))
                .collect(),
        },
        ServerToClient::RoomMessage {
            room_id,
            from,
            body,
            message_id,
        } => NetToUi::RoomMessage {
            room_id,
            from,
            body,
            message_id,
        },
        ServerToClient::UserJoinedRoom { room_id, username } => {
            NetToUi::UserJoinedRoom { room_id, username }
        }
        ServerToClient::UserLeftRoom { room_id, username } => {
            NetToUi::UserLeftRoom { room_id, username }
        }
        ServerToClient::RoomMembers { room_id, members } => {
            NetToUi::RoomMembers { room_id, members }
        }
        ServerToClient::UserTyping { room_id, username } => {
            NetToUi::UserTyping { room_id, username }
        }
        ServerToClient::UserStoppedTyping { room_id, username } => {
            NetToUi::UserStoppedTyping { room_id, username }
        }
        ServerToClient::ReadReceipt { message_id, read_by } => {
            NetToUi::ReadReceipt { message_id, read_by }
        }
        ServerToClient::MessageEdited {
            message_id,
            new_body,
            edited_by: _,
        } => NetToUi::MessageEdited { message_id, new_body },
        ServerToClient::MessageDeleted {
            message_id,
            deleted_by: _,
        } => NetToUi::MessageDeleted { message_id },
        ServerToClient::MessageReaction {
            message_id,
            emoji,
            from,
        } => NetToUi::MessageReaction {
            message_id,
            emoji,
            from,
        },
        ServerToClient::Nudged { from } => NetToUi::Nudged { from },
        ServerToClient::Winked { from, emoji } => NetToUi::Winked { from, emoji },
        ServerToClient::ProfileData {
            username,
            bio,
            status,
            joined,
            avatar_url,
        } => NetToUi::ProfileData {
            username,
            bio,
            status,
            joined,
            avatar_url,
        },
        ServerToClient::IncomingVideoCall { from, room_url } => {
            NetToUi::IncomingVideoCall { from, room_url }
        }
    })
}

pub fn ui_to_server(cmd: UiToNet) -> Option<ClientToServer> {
    match cmd {
        UiToNet::SendChat { body } => Some(ClientToServer::Chat { body }),
        UiToNet::SendDirect { to, body } => Some(ClientToServer::DirectMessage { to, body }),
        UiToNet::ExchangeKey { to, public_key } => {
            Some(ClientToServer::ExchangeKey { to, public_key })
        }
        UiToNet::SendEncryptedDirect { to, encrypted_body } => {
            Some(ClientToServer::EncryptedDirectMessage { to, encrypted_body })
        }
        UiToNet::FetchHistory { target } => Some(ClientToServer::FetchHistory {
            target: chat_target_to_history(target),
        }),
        UiToNet::FetchThreads => Some(ClientToServer::FetchThreads),
        UiToNet::Search { target, query } => Some(ClientToServer::Search {
            target: chat_target_to_history(target),
            query,
        }),
        UiToNet::SetAway { away } => Some(ClientToServer::SetAway { away }),
        UiToNet::Block { username } => Some(ClientToServer::Block { username }),
        UiToNet::Unblock { username } => Some(ClientToServer::Unblock { username }),
        UiToNet::Mute { username } => Some(ClientToServer::Mute { username }),
        UiToNet::Unmute { username } => Some(ClientToServer::Unmute { username }),
        UiToNet::Report { username, reason } => Some(ClientToServer::Report { username, reason }),
        UiToNet::AddFriend { username } => Some(ClientToServer::AddFriend { username }),
        UiToNet::AcceptFriendRequest { username } => {
            Some(ClientToServer::AcceptFriendRequest { username })
        }
        UiToNet::DeclineFriendRequest { username } => {
            Some(ClientToServer::DeclineFriendRequest { username })
        }
        UiToNet::CreateChatRoom { name } => Some(ClientToServer::CreateChatRoom { name }),
        UiToNet::JoinChatRoom { room_id } => Some(ClientToServer::JoinChatRoom { room_id }),
        UiToNet::LeaveChatRoom { room_id } => Some(ClientToServer::LeaveChatRoom { room_id }),
        UiToNet::SendRoomMessage { room_id, body } => {
            Some(ClientToServer::SendRoomMessage { room_id, body })
        }
        UiToNet::FetchChatRooms => Some(ClientToServer::FetchChatRooms),
        UiToNet::FetchRoomMembers { room_id } => {
            Some(ClientToServer::FetchRoomMembers { room_id })
        }
        UiToNet::StartTyping { room_id } => Some(ClientToServer::StartTyping { room_id }),
        UiToNet::StopTyping { room_id } => Some(ClientToServer::StopTyping { room_id }),
        UiToNet::MarkMessageAsRead { message_id } => {
            Some(ClientToServer::MarkMessageAsRead { message_id })
        }
        UiToNet::EditMessage { message_id, new_body } => {
            Some(ClientToServer::EditMessage { message_id, new_body })
        }
        UiToNet::DeleteMessage { message_id } => {
            Some(ClientToServer::DeleteMessage { message_id })
        }
        UiToNet::ReactToMessage { message_id, emoji } => {
            Some(ClientToServer::ReactToMessage { message_id, emoji })
        }
        UiToNet::Nudge { to } => Some(ClientToServer::Nudge { to }),
        UiToNet::Wink { to, emoji } => Some(ClientToServer::Wink { to, emoji }),
        UiToNet::SetStatus { status } => Some(ClientToServer::SetStatus { status }),
        UiToNet::SetBio { bio } => Some(ClientToServer::SetBio { bio }),
        UiToNet::FetchProfile { username } => Some(ClientToServer::FetchProfile { username }),
        UiToNet::ReplyToMessage { reply_to_id, body } => {
            Some(ClientToServer::ReplyToMessage { reply_to_id, body })
        }
        UiToNet::ReplyToDirect { to, reply_to_id, body } => {
            Some(ClientToServer::ReplyToDirect {
                to,
                reply_to_id,
                body,
            })
        }
        UiToNet::SetAvatar { avatar_data } => Some(ClientToServer::SetAvatar { avatar_data }),
        UiToNet::StartVideoCall { to } => Some(ClientToServer::StartVideoCall { to }),
        UiToNet::Connect { .. } | UiToNet::Disconnect => None,
    }
}

fn history_target_to_chat(target: HistoryTarget) -> ChatTarget {
    match target {
        HistoryTarget::Lobby => ChatTarget::Lobby,
        HistoryTarget::Direct { username } => ChatTarget::Direct(username),
        HistoryTarget::Room { room_id } => ChatTarget::Room(room_id),
    }
}

fn chat_target_to_history(target: ChatTarget) -> HistoryTarget {
    match target {
        ChatTarget::Lobby => HistoryTarget::Lobby,
        ChatTarget::Direct(username) => HistoryTarget::Direct { username },
        ChatTarget::Room(room_id) => HistoryTarget::Room { room_id },
    }
}

fn map_message_records(
    messages: Vec<chatmessagediscordclone::protocol::MessageRecord>,
) -> Vec<ChatMessage> {
    messages
        .into_iter()
        .map(|record| ChatMessage {
            from: record.from,
            body: record.body,
            at: record.at,
            id: record.id,
            read_count: record.read_count,
        })
        .collect()
}
