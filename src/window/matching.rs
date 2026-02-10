use anyhow::Result;
use strsim::levenshtein;

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub name: String,
    pub pid: i32,
    pub bundle_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MatchType {
    Exact,
    Prefix,
    Fuzzy { distance: usize },
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub app: AppInfo,
    pub match_type: MatchType,
}

impl MatchResult {
    pub fn describe(&self) -> String {
        match &self.match_type {
            MatchType::Exact => format!("\"{}\" (exact match)", self.app.name),
            MatchType::Prefix => format!("\"{}\" (prefix match)", self.app.name),
            MatchType::Fuzzy { distance } => {
                format!("\"{}\" (fuzzy, distance={})", self.app.name, distance)
            }
        }
    }
}

/// Find an app by name using fuzzy matching
/// Priority: exact match > prefix match > fuzzy match (within threshold)
pub fn find_app(query: &str, apps: &[AppInfo], fuzzy_threshold: usize) -> Option<MatchResult> {
    let query_lower = query.to_lowercase();

    // 1. exact match (case-insensitive)
    if let Some(app) = apps.iter().find(|a| a.name.to_lowercase() == query_lower) {
        return Some(MatchResult {
            app: app.clone(),
            match_type: MatchType::Exact,
        });
    }

    // 2. prefix match (case-insensitive), take first alphabetically
    let mut prefix_matches: Vec<_> = apps
        .iter()
        .filter(|a| a.name.to_lowercase().starts_with(&query_lower))
        .collect();

    prefix_matches.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    if let Some(app) = prefix_matches.first() {
        return Some(MatchResult {
            app: (*app).clone(),
            match_type: MatchType::Prefix,
        });
    }

    // 3. fuzzy match (Levenshtein distance), take best match within threshold
    let mut fuzzy_matches: Vec<_> = apps
        .iter()
        .map(|a| {
            let distance = levenshtein(&query_lower, &a.name.to_lowercase());
            (a, distance)
        })
        .filter(|(_, distance)| *distance <= fuzzy_threshold)
        .collect();

    // sort by distance, then alphabetically
    fuzzy_matches.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| a.0.name.to_lowercase().cmp(&b.0.name.to_lowercase()))
    });

    if let Some((app, distance)) = fuzzy_matches.first() {
        return Some(MatchResult {
            app: (*app).clone(),
            match_type: MatchType::Fuzzy { distance: *distance },
        });
    }

    None
}

/// Get list of running applications
#[cfg(target_os = "macos")]
pub fn get_running_apps() -> Result<Vec<AppInfo>> {
    use cocoa::base::nil;
    use objc::runtime::Object;

    let mut apps = Vec::new();

    unsafe {
        let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
        let running_apps: *mut Object = msg_send![workspace, runningApplications];
        let count: usize = msg_send![running_apps, count];

        for i in 0..count {
            let app: *mut Object = msg_send![running_apps, objectAtIndex: i];

            let name_ns: *mut Object = msg_send![app, localizedName];
            if name_ns == nil {
                continue;
            }

            let name_ptr: *const i8 = msg_send![name_ns, UTF8String];
            if name_ptr.is_null() {
                continue;
            }
            let name = std::ffi::CStr::from_ptr(name_ptr)
                .to_string_lossy()
                .into_owned();

            let pid: i32 = msg_send![app, processIdentifier];

            let bundle_id_ns: *mut Object = msg_send![app, bundleIdentifier];
            let bundle_id = if bundle_id_ns != nil {
                let bundle_ptr: *const i8 = msg_send![bundle_id_ns, UTF8String];
                if !bundle_ptr.is_null() {
                    Some(
                        std::ffi::CStr::from_ptr(bundle_ptr)
                            .to_string_lossy()
                            .into_owned(),
                    )
                } else {
                    None
                }
            } else {
                None
            };

            // skip background apps (those without a name or with pid -1)
            if !name.is_empty() && pid > 0 {
                apps.push(AppInfo {
                    name,
                    pid,
                    bundle_id,
                });
            }
        }
    }

    // sort alphabetically
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(apps)
}

#[cfg(not(target_os = "macos"))]
pub fn get_running_apps() -> Result<Vec<AppInfo>> {
    Err(anyhow!("Getting running apps is only supported on macOS"))
}



#[cfg(test)]
mod tests {
    use super::*;

    fn test_apps() -> Vec<AppInfo> {
        vec![
            AppInfo {
                name: "Slack".to_string(),
                pid: 1,
                bundle_id: None,
            },
            AppInfo {
                name: "Safari".to_string(),
                pid: 2,
                bundle_id: None,
            },
            AppInfo {
                name: "Google Chrome".to_string(),
                pid: 3,
                bundle_id: None,
            },
            AppInfo {
                name: "Terminal".to_string(),
                pid: 4,
                bundle_id: None,
            },
        ]
    }

    #[test]
    fn test_exact_match() {
        let apps = test_apps();
        let result = find_app("Slack", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Slack");
        assert!(matches!(result.match_type, MatchType::Exact));
    }

    #[test]
    fn test_exact_match_case_insensitive() {
        let apps = test_apps();
        let result = find_app("slack", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Slack");
        assert!(matches!(result.match_type, MatchType::Exact));
    }

    #[test]
    fn test_prefix_match() {
        let apps = test_apps();
        let result = find_app("Goo", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Google Chrome");
        assert!(matches!(result.match_type, MatchType::Prefix));
    }

    #[test]
    fn test_fuzzy_match() {
        let apps = test_apps();
        let result = find_app("Slakc", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Slack");
        assert!(matches!(result.match_type, MatchType::Fuzzy { distance: _ }));
    }

    #[test]
    fn test_fuzzy_match_beyond_threshold() {
        let apps = test_apps();
        let result = find_app("XXXXX", &apps, 2);
        assert!(result.is_none());
    }
}
