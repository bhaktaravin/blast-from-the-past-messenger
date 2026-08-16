#[cfg(all(not(target_arch = "wasm32"), feature = "client"))]
pub mod audio;
#[cfg(target_arch = "wasm32")]
pub mod audio_web;
#[cfg(feature = "client")]
pub mod avatar_cache;
pub mod buddy_icon;
pub mod protocol;
