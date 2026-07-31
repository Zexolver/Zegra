//! Resolves a "mod site" id — built-in (Nexus Mods, CurseForge) or one of the
//! user's own custom sites — to its URL and display title, and validates
//! custom sites before they're saved.

use crate::settings::CustomModSite;

/// (id, url, title) for the sites Zegra ships with. Nexus Mods and
/// CurseForge get special treatment elsewhere (Nexus Mods' `nxm://` link
/// handling), everything else a user adds is a plain custom site.
pub const BUILT_IN_SITES: &[(&str, &str, &str)] = &[
    ("nexusmods", "https://www.nexusmods.com", "Nexus Mods"),
    ("curseforge", "https://www.curseforge.com", "CurseForge"),
];

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ModSiteError {
    #[error("unknown mod site: {0}")]
    UnknownSite(String),
    #[error("mod site name cannot be empty")]
    EmptyName,
    #[error("mod site URL must start with http:// or https://: {0}")]
    InvalidUrl(String),
    #[error("a mod site with id \"{0}\" already exists")]
    DuplicateId(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSite {
    pub url: String,
    pub title: String,
}

/// Looks up `site_id` among the built-in sites first, then the user's custom
/// sites.
pub fn resolve_site(
    site_id: &str,
    custom_sites: &[CustomModSite],
) -> Result<ResolvedSite, ModSiteError> {
    if let Some((_, url, title)) = BUILT_IN_SITES.iter().find(|(id, _, _)| *id == site_id) {
        return Ok(ResolvedSite {
            url: url.to_string(),
            title: title.to_string(),
        });
    }
    custom_sites
        .iter()
        .find(|s| s.id == site_id)
        .map(|s| ResolvedSite {
            url: s.url.clone(),
            title: s.name.clone(),
        })
        .ok_or_else(|| ModSiteError::UnknownSite(site_id.to_string()))
}

/// Validates a single custom mod site against the sites that would already
/// exist alongside it: non-empty name, an `http://`/`https://` URL only
/// (blocking `file://`, `javascript:`, `nxm://`, or anything else that isn't
/// just "open this website"), and no id collision with a built-in site or
/// another custom one.
fn validate_one(site: &CustomModSite, existing: &[CustomModSite]) -> Result<(), ModSiteError> {
    if site.name.trim().is_empty() {
        return Err(ModSiteError::EmptyName);
    }
    let parsed = url::Url::parse(&site.url)
        .map_err(|_| ModSiteError::InvalidUrl(site.url.clone()))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(ModSiteError::InvalidUrl(site.url.clone()));
    }
    if BUILT_IN_SITES.iter().any(|(id, _, _)| *id == site.id) {
        return Err(ModSiteError::DuplicateId(site.id.clone()));
    }
    if existing.iter().any(|s| s.id == site.id) {
        return Err(ModSiteError::DuplicateId(site.id.clone()));
    }
    Ok(())
}

/// Validates an entire custom-sites list (as saved from Settings), checking
/// each entry against every one before it so duplicate ids within the list
/// itself are caught too.
pub fn validate_all(sites: &[CustomModSite]) -> Result<(), ModSiteError> {
    for (i, site) in sites.iter().enumerate() {
        validate_one(site, &sites[..i])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(id: &str, name: &str, url: &str) -> CustomModSite {
        CustomModSite {
            id: id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
        }
    }

    #[test]
    fn resolves_built_in_sites() {
        let resolved = resolve_site("nexusmods", &[]).unwrap();
        assert_eq!(resolved.url, "https://www.nexusmods.com");
        assert_eq!(resolved.title, "Nexus Mods");
    }

    #[test]
    fn resolves_custom_sites() {
        let custom = vec![site("moddb", "ModDB", "https://www.moddb.com")];
        let resolved = resolve_site("moddb", &custom).unwrap();
        assert_eq!(resolved.url, "https://www.moddb.com");
        assert_eq!(resolved.title, "ModDB");
    }

    #[test]
    fn built_in_sites_take_priority_over_a_same_id_custom_site() {
        let custom = vec![site("nexusmods", "Fake Nexus", "https://evil.example")];
        let resolved = resolve_site("nexusmods", &custom).unwrap();
        assert_eq!(resolved.url, "https://www.nexusmods.com");
    }

    #[test]
    fn unknown_site_errors() {
        let err = resolve_site("not-a-real-site", &[]).unwrap_err();
        assert_eq!(err, ModSiteError::UnknownSite("not-a-real-site".to_string()));
    }

    #[test]
    fn validates_a_good_custom_site() {
        let sites = vec![site("gamebanana", "GameBanana", "https://gamebanana.com")];
        assert_eq!(validate_all(&sites), Ok(()));
    }

    #[test]
    fn rejects_empty_name() {
        let sites = vec![site("x", "  ", "https://example.com")];
        assert_eq!(validate_all(&sites), Err(ModSiteError::EmptyName));
    }

    #[test]
    fn rejects_non_http_schemes() {
        for url in ["file:///etc/passwd", "javascript:alert(1)", "nxm://game/mods/1/files/2", "ftp://example.com"] {
            let sites = vec![site("x", "Bad", url)];
            assert_eq!(
                validate_all(&sites),
                Err(ModSiteError::InvalidUrl(url.to_string())),
                "expected {url} to be rejected"
            );
        }
    }

    #[test]
    fn rejects_malformed_url() {
        let sites = vec![site("x", "Bad", "not a url")];
        assert!(matches!(validate_all(&sites), Err(ModSiteError::InvalidUrl(_))));
    }

    #[test]
    fn rejects_id_colliding_with_a_built_in_site() {
        let sites = vec![site("curseforge", "My Site", "https://example.com")];
        assert_eq!(
            validate_all(&sites),
            Err(ModSiteError::DuplicateId("curseforge".to_string()))
        );
    }

    #[test]
    fn rejects_duplicate_ids_within_the_custom_list_itself() {
        let sites = vec![
            site("moddb", "ModDB", "https://moddb.com"),
            site("moddb", "ModDB Again", "https://moddb.com/other"),
        ];
        assert_eq!(
            validate_all(&sites),
            Err(ModSiteError::DuplicateId("moddb".to_string()))
        );
    }
}
