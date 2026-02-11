use self_update::cargo_crate_version;

pub async fn check_for_updates() -> Result<Option<String>, String> {
    // Run the blocking check in a spawn_blocking context
    let result = tokio::task::spawn_blocking(|| {
        self_update::backends::github::ReleaseList::configure()
            .repo_owner("ravinathannur")
            .repo_name("chatmessagediscordclone")
            .build()
            .map_err(|e| format!("Failed to build release list: {}", e))?
            .fetch()
            .map_err(|e| format!("Failed to fetch releases: {}", e))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?;

    match result {
        Ok(releases) => {
            if let Some(latest) = releases.first() {
                let current_version = cargo_crate_version!();
                if should_update(current_version, &latest.version) {
                    return Ok(Some(latest.version.clone()));
                }
            }
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

fn should_update(current: &str, latest: &str) -> bool {
    // Simple version comparison: "0.1.0" -> [0, 1, 0]
    let current_parts: Vec<u32> = current
        .trim_start_matches('v')
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let latest_parts: Vec<u32> = latest
        .trim_start_matches('v')
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    for (curr, new) in current_parts.iter().zip(latest_parts.iter()) {
        if new > curr {
            return true;
        } else if new < curr {
            return false;
        }
    }
    latest_parts.len() > current_parts.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(should_update("0.1.0", "0.1.1"));
        assert!(should_update("0.1.0", "0.2.0"));
        assert!(!should_update("0.2.0", "0.1.0"));
        assert!(!should_update("0.1.0", "0.1.0"));
    }
}
