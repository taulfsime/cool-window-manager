use anyhow::Result;
use regex::RegexBuilder;
use strsim::levenshtein;

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub name: String,
    pub pid: i32,
    pub bundle_id: Option<String>,
    pub titles: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum MatchType {
    Exact,
    Prefix,
    Regex { pattern: String },
    Fuzzy { distance: usize },
    TitleExact { title: String },
    TitlePrefix { title: String },
    TitleRegex { title: String, pattern: String },
    TitleFuzzy { title: String, distance: usize },
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
            MatchType::Regex { pattern } => {
                format!("\"{}\" (regex: /{}/)", self.app.name, pattern)
            }
            MatchType::Fuzzy { distance } => {
                format!("\"{}\" (fuzzy, distance={})", self.app.name, distance)
            }
            MatchType::TitleExact { title } => {
                format!("\"{}\" (title exact: \"{}\")", self.app.name, title)
            }
            MatchType::TitlePrefix { title } => {
                format!("\"{}\" (title prefix: \"{}\")", self.app.name, title)
            }
            MatchType::TitleRegex { title, pattern } => {
                format!(
                    "\"{}\" (title regex: /{}/ matched \"{}\")",
                    self.app.name, pattern, title
                )
            }
            MatchType::TitleFuzzy { title, distance } => {
                format!(
                    "\"{}\" (title fuzzy: \"{}\", distance={})",
                    self.app.name, title, distance
                )
            }
        }
    }
}

/// Parse a query to detect if it's a regex pattern
/// Returns Some((pattern, case_insensitive)) if the query is /pattern/ or /pattern/i
fn parse_regex_pattern(query: &str) -> Option<(String, bool)> {
    if !query.starts_with('/') {
        return None;
    }

    if query.ends_with("/i") && query.len() > 3 {
        Some((query[1..query.len() - 2].to_string(), true))
    } else if query.ends_with('/') && query.len() > 2 {
        Some((query[1..query.len() - 1].to_string(), false))
    } else {
        None
    }
}

/// Find an app by regex pattern matching against name and title
fn find_app_by_regex(
    pattern: &str,
    case_insensitive: bool,
    apps: &[AppInfo],
) -> Option<MatchResult> {
    let regex = RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()
        .ok()?;

    // 1. try name match first
    for app in apps {
        if regex.is_match(&app.name) {
            return Some(MatchResult {
                app: app.clone(),
                match_type: MatchType::Regex {
                    pattern: pattern.to_string(),
                },
            });
        }
    }

    // 2. try title match
    for app in apps {
        for title in &app.titles {
            if regex.is_match(title) {
                return Some(MatchResult {
                    app: app.clone(),
                    match_type: MatchType::TitleRegex {
                        title: title.clone(),
                        pattern: pattern.to_string(),
                    },
                });
            }
        }
    }

    None
}

/// Find an app by name or window title using fuzzy matching
/// Priority: name exact > name prefix > regex > name fuzzy > title exact > title prefix > title regex > title fuzzy
pub fn find_app(query: &str, apps: &[AppInfo], fuzzy_threshold: usize) -> Option<MatchResult> {
    // check for regex pattern first (e.g., /Safari.*/ or /chrome/i)
    if let Some((pattern, case_insensitive)) = parse_regex_pattern(query) {
        return find_app_by_regex(&pattern, case_insensitive, apps);
    }

    let query_lower = query.to_lowercase();

    // 1. exact name match (case-insensitive)
    if let Some(app) = apps.iter().find(|a| a.name.to_lowercase() == query_lower) {
        return Some(MatchResult {
            app: app.clone(),
            match_type: MatchType::Exact,
        });
    }

    // 2. prefix name match (case-insensitive), take first alphabetically
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

    // 3. fuzzy name match (Levenshtein distance), take best match within threshold
    let mut fuzzy_matches: Vec<_> = apps
        .iter()
        .map(|a| {
            let distance = levenshtein(&query_lower, &a.name.to_lowercase());
            (a, distance)
        })
        .filter(|(_, distance)| *distance <= fuzzy_threshold)
        .collect();

    fuzzy_matches.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| a.0.name.to_lowercase().cmp(&b.0.name.to_lowercase()))
    });

    if let Some((app, distance)) = fuzzy_matches.first() {
        return Some(MatchResult {
            app: (*app).clone(),
            match_type: MatchType::Fuzzy {
                distance: *distance,
            },
        });
    }

    // 4. exact title match (case-insensitive)
    for app in apps {
        for title in &app.titles {
            if title.to_lowercase() == query_lower {
                return Some(MatchResult {
                    app: app.clone(),
                    match_type: MatchType::TitleExact {
                        title: title.clone(),
                    },
                });
            }
        }
    }

    // 5. prefix title match (case-insensitive)
    let mut title_prefix_matches: Vec<_> = apps
        .iter()
        .flat_map(|a| a.titles.iter().map(move |t| (a, t)))
        .filter(|(_, t)| t.to_lowercase().starts_with(&query_lower))
        .collect();

    title_prefix_matches.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

    if let Some((app, title)) = title_prefix_matches.first() {
        return Some(MatchResult {
            app: (*app).clone(),
            match_type: MatchType::TitlePrefix {
                title: (*title).clone(),
            },
        });
    }

    // 6. fuzzy title match (Levenshtein distance)
    let mut title_fuzzy_matches: Vec<_> = apps
        .iter()
        .flat_map(|a| a.titles.iter().map(move |t| (a, t)))
        .map(|(a, t)| {
            let distance = levenshtein(&query_lower, &t.to_lowercase());
            (a, t, distance)
        })
        .filter(|(_, _, distance)| *distance <= fuzzy_threshold)
        .collect();

    title_fuzzy_matches.sort_by(|a, b| {
        a.2.cmp(&b.2)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });

    if let Some((app, title, distance)) = title_fuzzy_matches.first() {
        return Some(MatchResult {
            app: (*app).clone(),
            match_type: MatchType::TitleFuzzy {
                title: (*title).clone(),
                distance: *distance,
            },
        });
    }

    None
}

/// Get window titles for an application using Accessibility API
fn get_window_titles(pid: i32) -> Vec<String> {
    use core_foundation::base::{CFTypeRef, TCFType};
    use core_foundation::string::CFString;

    type AXUIElementRef = *mut std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: core_foundation::string::CFStringRef,
            value: *mut CFTypeRef,
        ) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFArrayGetCount(array: core_foundation::array::CFArrayRef) -> isize;
        fn CFArrayGetValueAtIndex(
            array: core_foundation::array::CFArrayRef,
            index: isize,
        ) -> *const std::ffi::c_void;
        fn CFGetTypeID(cf: CFTypeRef) -> usize;
        fn CFStringGetTypeID() -> usize;
    }

    const K_AX_ERROR_SUCCESS: i32 = 0;

    let mut titles = Vec::new();

    unsafe {
        let app_element = AXUIElementCreateApplication(pid);
        if app_element.is_null() {
            return titles;
        }

        let windows_attr = CFString::new("AXWindows");
        let mut windows_value: CFTypeRef = std::ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(
            app_element,
            windows_attr.as_concrete_TypeRef(),
            &mut windows_value,
        );

        if result != K_AX_ERROR_SUCCESS || windows_value.is_null() {
            core_foundation::base::CFRelease(app_element as CFTypeRef);
            return titles;
        }

        let count = CFArrayGetCount(windows_value as _);

        for i in 0..count {
            let window = CFArrayGetValueAtIndex(windows_value as _, i) as AXUIElementRef;
            if window.is_null() {
                continue;
            }

            let title_attr = CFString::new("AXTitle");
            let mut title_value: CFTypeRef = std::ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                window,
                title_attr.as_concrete_TypeRef(),
                &mut title_value,
            );

            if result == K_AX_ERROR_SUCCESS && !title_value.is_null() {
                // verify it's a CFString before converting
                if CFGetTypeID(title_value) == CFStringGetTypeID() {
                    let cf_string: core_foundation::string::CFString =
                        core_foundation::string::CFString::wrap_under_get_rule(
                            title_value as core_foundation::string::CFStringRef,
                        );
                    let title = cf_string.to_string();
                    if !title.is_empty() {
                        titles.push(title);
                    }
                }
                core_foundation::base::CFRelease(title_value);
            }
        }

        core_foundation::base::CFRelease(windows_value);
        core_foundation::base::CFRelease(app_element as CFTypeRef);
    }

    titles
}

/// Get list of running applications
pub fn get_running_apps() -> Result<Vec<AppInfo>> {
    use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace};
    use std::collections::HashMap;

    let mut apps = Vec::new();

    let workspace = NSWorkspace::sharedWorkspace();
    let running_apps = workspace.runningApplications();

    for app in running_apps.iter() {
        // only include regular apps (those that appear in Dock and have UI)
        // skip accessory apps and prohibited (background-only) apps
        if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
            continue;
        }

        let name = match app.localizedName() {
            Some(name) => name.to_string(),
            None => continue,
        };

        let pid = app.processIdentifier();

        let bundle_id = app.bundleIdentifier().map(|s| s.to_string());

        if !name.is_empty() && pid > 0 {
            let titles = get_window_titles(pid);
            apps.push(AppInfo {
                name,
                pid,
                bundle_id,
                titles,
            });
        }
    }

    // sort alphabetically
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // handle duplicate names by appending instance number
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for app in &apps {
        *name_counts.entry(app.name.clone()).or_insert(0) += 1;
    }

    let mut name_indices: HashMap<String, usize> = HashMap::new();
    for app in &mut apps {
        if name_counts.get(&app.name).copied().unwrap_or(0) > 1 {
            let idx = name_indices.entry(app.name.clone()).or_insert(0);
            *idx += 1;
            app.name = format!("{} ({})", app.name, idx);
        }
    }

    Ok(apps)
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
                titles: vec!["general - Slack".to_string()],
            },
            AppInfo {
                name: "Safari".to_string(),
                pid: 2,
                bundle_id: None,
                titles: vec!["GitHub - taulfsime/cool-window-mng".to_string()],
            },
            AppInfo {
                name: "Google Chrome".to_string(),
                pid: 3,
                bundle_id: None,
                titles: vec!["New Tab".to_string(), "Google Search".to_string()],
            },
            AppInfo {
                name: "Terminal".to_string(),
                pid: 4,
                bundle_id: None,
                titles: vec!["zsh - ~/Projects".to_string()],
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
        assert!(matches!(
            result.match_type,
            MatchType::Fuzzy { distance: _ }
        ));
    }

    #[test]
    fn test_fuzzy_match_beyond_threshold() {
        let apps = test_apps();
        let result = find_app("XXXXX", &apps, 2);
        assert!(result.is_none());
    }

    #[test]
    fn test_title_exact_match() {
        let apps = test_apps();
        let result = find_app("New Tab", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Google Chrome");
        assert!(matches!(result.match_type, MatchType::TitleExact { .. }));
    }

    #[test]
    fn test_title_prefix_match() {
        let apps = test_apps();
        let result = find_app("GitHub - taulfsime", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Safari");
        assert!(matches!(result.match_type, MatchType::TitlePrefix { .. }));
    }

    #[test]
    fn test_title_fuzzy_match() {
        let apps = test_apps();
        // "Nwe Tab" is 1 edit away from "New Tab"
        let result = find_app("Nwe Tab", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Google Chrome");
        assert!(matches!(result.match_type, MatchType::TitleFuzzy { .. }));
    }

    #[test]
    fn test_name_match_takes_priority_over_title() {
        let apps = test_apps();
        // "Slack" matches app name exactly, even though "general - Slack" contains it
        let result = find_app("Slack", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Slack");
        assert!(matches!(result.match_type, MatchType::Exact));
    }

    // ========================================================================
    // Additional matching tests
    // ========================================================================

    #[test]
    fn test_empty_query_matches_first_app() {
        let apps = test_apps();
        let result = find_app("", &apps, 2);
        // empty query matches first app as prefix (empty string is prefix of everything)
        assert!(result.is_some());
    }

    #[test]
    fn test_no_match_empty_apps() {
        let apps: Vec<AppInfo> = vec![];
        let result = find_app("Safari", &apps, 2);
        assert!(result.is_none());
    }

    #[test]
    fn test_prefix_match_case_insensitive() {
        let apps = test_apps();
        let result = find_app("goo", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Google Chrome");
        assert!(matches!(result.match_type, MatchType::Prefix));
    }

    #[test]
    fn test_fuzzy_match_threshold_zero() {
        let apps = test_apps();
        // with threshold 0, only exact matches should work
        let result = find_app("Slakc", &apps, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_match_threshold_one() {
        let apps = test_apps();
        // "Slakc" is 2 edits from "Slack", so threshold 1 should not match
        let result = find_app("Slakc", &apps, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_match_at_threshold() {
        let apps = test_apps();
        // "Slak" is 1 edit from "Slack"
        let result = find_app("Slak", &apps, 1).unwrap();
        assert_eq!(result.app.name, "Slack");
    }

    #[test]
    fn test_match_result_describe_exact() {
        let result = MatchResult {
            app: AppInfo {
                name: "Safari".to_string(),
                pid: 1,
                bundle_id: None,
                titles: vec![],
            },
            match_type: MatchType::Exact,
        };
        let desc = result.describe();
        assert!(desc.contains("Safari"));
        assert!(desc.contains("exact"));
    }

    #[test]
    fn test_match_result_describe_prefix() {
        let result = MatchResult {
            app: AppInfo {
                name: "Safari".to_string(),
                pid: 1,
                bundle_id: None,
                titles: vec![],
            },
            match_type: MatchType::Prefix,
        };
        let desc = result.describe();
        assert!(desc.contains("Safari"));
        assert!(desc.contains("prefix"));
    }

    #[test]
    fn test_match_result_describe_fuzzy() {
        let result = MatchResult {
            app: AppInfo {
                name: "Safari".to_string(),
                pid: 1,
                bundle_id: None,
                titles: vec![],
            },
            match_type: MatchType::Fuzzy { distance: 2 },
        };
        let desc = result.describe();
        assert!(desc.contains("Safari"));
        assert!(desc.contains("fuzzy"));
        assert!(desc.contains("2"));
    }

    #[test]
    fn test_match_result_describe_title_exact() {
        let result = MatchResult {
            app: AppInfo {
                name: "Chrome".to_string(),
                pid: 1,
                bundle_id: None,
                titles: vec![],
            },
            match_type: MatchType::TitleExact {
                title: "New Tab".to_string(),
            },
        };
        let desc = result.describe();
        assert!(desc.contains("Chrome"));
        assert!(desc.contains("title exact"));
        assert!(desc.contains("New Tab"));
    }

    #[test]
    fn test_match_result_describe_title_prefix() {
        let result = MatchResult {
            app: AppInfo {
                name: "Safari".to_string(),
                pid: 1,
                bundle_id: None,
                titles: vec![],
            },
            match_type: MatchType::TitlePrefix {
                title: "GitHub - taulfsime".to_string(),
            },
        };
        let desc = result.describe();
        assert!(desc.contains("Safari"));
        assert!(desc.contains("title prefix"));
    }

    #[test]
    fn test_match_result_describe_title_fuzzy() {
        let result = MatchResult {
            app: AppInfo {
                name: "Chrome".to_string(),
                pid: 1,
                bundle_id: None,
                titles: vec![],
            },
            match_type: MatchType::TitleFuzzy {
                title: "New Tab".to_string(),
                distance: 1,
            },
        };
        let desc = result.describe();
        assert!(desc.contains("Chrome"));
        assert!(desc.contains("title fuzzy"));
        assert!(desc.contains("1"));
    }

    #[test]
    fn test_app_info_clone() {
        let app = AppInfo {
            name: "Safari".to_string(),
            pid: 123,
            bundle_id: Some("com.apple.Safari".to_string()),
            titles: vec!["Title 1".to_string(), "Title 2".to_string()],
        };
        let cloned = app.clone();

        assert_eq!(cloned.name, app.name);
        assert_eq!(cloned.pid, app.pid);
        assert_eq!(cloned.bundle_id, app.bundle_id);
        assert_eq!(cloned.titles, app.titles);
    }

    #[test]
    fn test_match_type_clone() {
        let mt = MatchType::Fuzzy { distance: 5 };
        let cloned = mt.clone();

        assert!(matches!(cloned, MatchType::Fuzzy { distance: 5 }));
    }

    #[test]
    fn test_match_result_clone() {
        let result = MatchResult {
            app: AppInfo {
                name: "Test".to_string(),
                pid: 1,
                bundle_id: None,
                titles: vec![],
            },
            match_type: MatchType::Exact,
        };
        let cloned = result.clone();

        assert_eq!(cloned.app.name, result.app.name);
    }

    #[test]
    fn test_multiple_title_matches_first_wins() {
        let apps = vec![AppInfo {
            name: "Chrome".to_string(),
            pid: 1,
            bundle_id: None,
            titles: vec![
                "Tab One".to_string(),
                "Tab Two".to_string(),
                "Tab Three".to_string(),
            ],
        }];

        let result = find_app("Tab One", &apps, 2).unwrap();
        assert!(matches!(result.match_type, MatchType::TitleExact { .. }));
    }

    #[test]
    fn test_app_with_no_titles() {
        let apps = vec![AppInfo {
            name: "Finder".to_string(),
            pid: 1,
            bundle_id: None,
            titles: vec![],
        }];

        // should still match by name
        let result = find_app("Finder", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Finder");
        assert!(matches!(result.match_type, MatchType::Exact));
    }

    #[test]
    fn test_levenshtein_distance_used() {
        // verify we're using levenshtein distance correctly
        // "kitten" -> "sitting" has distance 3
        let distance = levenshtein("kitten", "sitting");
        assert_eq!(distance, 3);
    }

    // ========================================================================
    // Regex matching tests
    // ========================================================================

    #[test]
    fn test_parse_regex_pattern_valid() {
        let result = parse_regex_pattern("/Safari.*/");
        assert_eq!(result, Some(("Safari.*".to_string(), false)));
    }

    #[test]
    fn test_parse_regex_pattern_case_insensitive() {
        let result = parse_regex_pattern("/chrome/i");
        assert_eq!(result, Some(("chrome".to_string(), true)));
    }

    #[test]
    fn test_parse_regex_pattern_not_regex() {
        assert_eq!(parse_regex_pattern("Safari"), None);
        assert_eq!(parse_regex_pattern("Saf/ari"), None);
    }

    #[test]
    fn test_parse_regex_pattern_incomplete() {
        // missing closing slash
        assert_eq!(parse_regex_pattern("/Safari"), None);
        // too short
        assert_eq!(parse_regex_pattern("//"), None);
        assert_eq!(parse_regex_pattern("/i"), None);
    }

    #[test]
    fn test_regex_name_match_case_sensitive() {
        let apps = test_apps();
        let result = find_app("/^Slack$/", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Slack");
        assert!(matches!(result.match_type, MatchType::Regex { .. }));
    }

    #[test]
    fn test_regex_name_match_case_sensitive_no_match() {
        let apps = test_apps();
        // lowercase "slack" should not match "Slack" without /i flag
        let result = find_app("/^slack$/", &apps, 2);
        assert!(result.is_none());
    }

    #[test]
    fn test_regex_name_match_case_insensitive() {
        let apps = test_apps();
        let result = find_app("/^slack$/i", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Slack");
        assert!(matches!(result.match_type, MatchType::Regex { .. }));
    }

    #[test]
    fn test_regex_name_match_partial() {
        let apps = test_apps();
        // should match "Google Chrome" with partial regex
        let result = find_app("/Google/", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Google Chrome");
        assert!(matches!(result.match_type, MatchType::Regex { .. }));
    }

    #[test]
    fn test_regex_title_match() {
        let apps = test_apps();
        // match window title containing "GitHub"
        let result = find_app("/GitHub.*cool-window/", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Safari");
        assert!(matches!(result.match_type, MatchType::TitleRegex { .. }));
    }

    #[test]
    fn test_regex_title_match_case_insensitive() {
        let apps = test_apps();
        let result = find_app("/github/i", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Safari");
        assert!(matches!(result.match_type, MatchType::TitleRegex { .. }));
    }

    #[test]
    fn test_regex_invalid_pattern_returns_none() {
        let apps = test_apps();
        // invalid regex (unclosed group)
        let result = find_app("/Safari(/", &apps, 2);
        assert!(result.is_none());
    }

    #[test]
    fn test_regex_name_takes_priority_over_title() {
        let apps = test_apps();
        // "Slack" appears in both app name and title "general - Slack"
        // name match should take priority
        let result = find_app("/Slack/", &apps, 2).unwrap();
        assert_eq!(result.app.name, "Slack");
        assert!(matches!(result.match_type, MatchType::Regex { .. }));
    }

    #[test]
    fn test_regex_alternation() {
        let apps = test_apps();
        // match either Safari or Chrome
        let result = find_app("/Safari|Chrome/", &apps, 2).unwrap();
        // should match first alphabetically (Google Chrome comes before Safari)
        assert!(
            result.app.name == "Google Chrome" || result.app.name == "Safari",
            "expected Safari or Chrome, got {}",
            result.app.name
        );
    }

    #[test]
    fn test_regex_no_match() {
        let apps = test_apps();
        let result = find_app("/NonExistentApp/", &apps, 2);
        assert!(result.is_none());
    }

    #[test]
    fn test_match_result_describe_regex() {
        let result = MatchResult {
            app: AppInfo {
                name: "Safari".to_string(),
                pid: 1,
                bundle_id: None,
                titles: vec![],
            },
            match_type: MatchType::Regex {
                pattern: "Saf.*".to_string(),
            },
        };
        let desc = result.describe();
        assert!(desc.contains("Safari"));
        assert!(desc.contains("regex"));
        assert!(desc.contains("Saf.*"));
    }

    #[test]
    fn test_match_result_describe_title_regex() {
        let result = MatchResult {
            app: AppInfo {
                name: "Chrome".to_string(),
                pid: 1,
                bundle_id: None,
                titles: vec![],
            },
            match_type: MatchType::TitleRegex {
                title: "GitHub - PR #123".to_string(),
                pattern: "PR #\\d+".to_string(),
            },
        };
        let desc = result.describe();
        assert!(desc.contains("Chrome"));
        assert!(desc.contains("title regex"));
        assert!(desc.contains("PR #\\d+"));
        assert!(desc.contains("GitHub - PR #123"));
    }
}
