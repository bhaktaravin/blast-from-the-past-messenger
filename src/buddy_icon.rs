//! Buddy icon / avatar display helpers (emoji, presets, URLs).

/// Classic AIM-style preset icons: stored as `icon:name` in avatar_url.
pub const BUDDY_ICON_PRESETS: &[(&str, &str)] = &[
    ("smiley", "😀"),
    ("cool", "😎"),
    ("robot", "🤖"),
    ("alien", "👾"),
    ("game", "🎮"),
    ("music", "🎵"),
    ("star", "⭐"),
    ("heart", "💖"),
    ("flower", "🌸"),
    ("cat", "🐱"),
    ("dog", "🐶"),
    ("bear", "🐻"),
    ("ghost", "👻"),
    ("skull", "💀"),
    ("rocket", "🚀"),
    ("diamond", "💎"),
    ("fire", "🔥"),
    ("lightning", "⚡"),
    ("rainbow", "🌈"),
    ("pizza", "🍕"),
    ("coffee", "☕"),
    ("phone", "📞"),
    ("mail", "📧"),
    ("soccer", "⚽"),
    ("guitar", "🎸"),
    ("crown", "👑"),
    ("wizard", "🧙"),
    ("ninja", "🥷"),
    ("clown", "🤡"),
    ("party", "🎉"),
];

pub fn preset_emoji(name: &str) -> Option<&'static str> {
    BUDDY_ICON_PRESETS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, e)| *e)
}

/// How to draw a buddy's avatar circle.
pub enum BuddyIconDisplay<'a> {
    Emoji(&'a str),
    Url(&'a str),
    Initials,
}

pub fn resolve_buddy_icon<'a>(avatar_url: &'a Option<String>, _username: &'a str) -> BuddyIconDisplay<'a> {
    match avatar_url.as_deref() {
        None | Some("") => BuddyIconDisplay::Initials,
        Some(s) if s.starts_with("icon:") => {
            let name = s.trim_start_matches("icon:");
            preset_emoji(name)
                .map(BuddyIconDisplay::Emoji)
                .unwrap_or(BuddyIconDisplay::Initials)
        }
        Some(s) if s.chars().count() <= 4 && !s.starts_with("http") && !s.starts_with("data:") && s.len() < 80 => {
            BuddyIconDisplay::Emoji(s)
        }
        Some(s)
            if s.starts_with("http://")
                || s.starts_with("https://")
                || s.starts_with("data:image")
                || is_raw_base64_image(s) =>
        {
            BuddyIconDisplay::Url(s)
        }
        Some(_) => BuddyIconDisplay::Initials,
    }
}

pub fn get_initials(username: &str) -> String {
    username
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn is_raw_base64_image(s: &str) -> bool {
    s.len() > 80
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=' || b == b'\n' || b == b'\r'
        })
}

pub fn username_to_color(username: &str) -> (u8, u8, u8) {
    let hash = username.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
    (
        (50 + (hash % 150)) as u8,
        (50 + ((hash >> 8) % 150)) as u8,
        (50 + ((hash >> 16) % 150)) as u8,
    )
}
