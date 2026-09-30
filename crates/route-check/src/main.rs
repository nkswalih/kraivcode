use jcode_base::provider_catalog::{
    openai_compatible_profile_is_configured, openai_compatible_profile_static_models,
    openai_compatible_profiles, resolve_openai_compatible_profile,
};

fn main() {
    println!("== configured openai-compatible profiles (what the daemon advertises) ==");
    for profile in openai_compatible_profiles().iter().copied() {
        let resolved = resolve_openai_compatible_profile(profile);
        let configured = openai_compatible_profile_is_configured(profile);
        let statics = openai_compatible_profile_static_models(profile);
        println!(
            "{:>30}  id={:<14} configured={:<5} requires_key={:<5} base={:<34} static_count={}",
            resolved.display_name,
            resolved.id,
            configured,
            resolved.requires_api_key,
            resolved.api_base,
            statics.len()
        );
        if !configured {
            continue;
        }
        match live_cache_for(&resolved) {
            Some((models, stale)) => {
                println!(
                    "       -> LIVE catalog ({} models, stale={})",
                    models.len(),
                    stale
                );
                for m in models.iter().take(8) {
                    println!("           - {}", m);
                }
                if models.len() > 8 {
                    println!("           ... +{} more", models.len() - 8);
                }
            }
            None => {
                println!("       -> static fallback (no matching live cache):");
                for m in statics.iter().take(12) {
                    println!("           - {}", m);
                }
                if statics.len() > 12 {
                    println!("           ... +{} more", statics.len() - 12);
                }
            }
        }
    }
}

fn live_cache_for(
    resolved: &jcode_base::provider_catalog::ResolvedOpenAiCompatibleProfile,
) -> Option<(Vec<String>, bool)> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    let path = std::path::Path::new(&home)
        .join(".jcode")
        .join("cache")
        .join(format!("{}_models.json", resolved.id));
    let text = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let cached_at = json.get("cached_at").and_then(|v| v.as_u64()).unwrap_or(0);
    let source = json
        .get("source_api_base")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let stale = now.saturating_sub(cached_at) >= 15 * 60;
    let _ = source;
    if source.is_empty() {
        return None;
    }
    let models = json
        .get("models")?
        .as_array()?
        .iter()
        .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if models.is_empty() {
        return None;
    }
    Some((models, stale))
}
