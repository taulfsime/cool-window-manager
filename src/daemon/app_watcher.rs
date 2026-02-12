use anyhow::Result;

use crate::config::AppRule;

/// matched rule info passed to callback
pub struct MatchedRule {
    pub action: String,
    pub delay_ms: Option<u64>,
    pub app_name: String,
}

mod macos {
    use super::*;
    use std::ffi::c_void;
    use std::sync::Mutex;

    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSRunningApplication, NSWorkspace};
    use objc2_foundation::{NSNotification, NSString};

    type Callback = Box<dyn Fn(MatchedRule, i32) + Send + 'static>;

    // wrapper to make raw pointers Send
    struct SendPtr(*mut c_void);
    unsafe impl Send for SendPtr {}

    static APP_RULES: Mutex<Option<Vec<AppRule>>> = Mutex::new(None);
    static CALLBACK: Mutex<Option<Callback>> = Mutex::new(None);
    static OBSERVER_PTR: Mutex<SendPtr> = Mutex::new(SendPtr(std::ptr::null_mut()));
    static BLOCK_PTR: Mutex<SendPtr> = Mutex::new(SendPtr(std::ptr::null_mut()));

    fn handle_app_launch(notification: &NSNotification) {
        unsafe {
            // get userInfo from notification
            let user_info = notification.userInfo();

            let Some(user_info) = user_info else {
                return;
            };

            // get the NSRunningApplication from userInfo
            let app_key = NSString::from_str("NSWorkspaceApplicationKey");
            let app_obj = user_info.objectForKey(&app_key);

            let Some(app_obj) = app_obj else {
                return;
            };

            // cast to NSRunningApplication
            let app: &NSRunningApplication =
                &*(&*app_obj as *const AnyObject as *const NSRunningApplication);

            let app_name = match app.localizedName() {
                Some(name) => name.to_string(),
                None => return,
            };

            let pid = app.processIdentifier();

            if pid <= 0 {
                return;
            }

            // check if this app matches any rules
            let matched_rule = {
                let rules_guard = APP_RULES.lock().ok();
                let rules: Option<&Vec<AppRule>> = rules_guard.as_ref().and_then(|g| g.as_ref());

                if let Some(rules) = rules {
                    let app_name_lower = app_name.to_lowercase();

                    let mut found: Option<MatchedRule> = None;
                    for rule in rules {
                        let rule_app_lower = rule.app.to_lowercase();

                        // match by exact name (case-insensitive) or prefix
                        if app_name_lower == rule_app_lower
                            || app_name_lower.starts_with(&rule_app_lower)
                        {
                            found = Some(MatchedRule {
                                action: rule.action.clone(),
                                delay_ms: rule.delay_ms,
                                app_name: app_name.clone(),
                            });
                            break;
                        }
                    }
                    found
                } else {
                    None
                }
            };

            // call the callback outside of the rules lock
            if let Some(rule) = matched_rule {
                if let Ok(callback_guard) = CALLBACK.lock() {
                    if let Some(ref callback) = *callback_guard {
                        callback(rule, pid);
                    }
                }
            }
        }
    }

    pub fn start_watching(
        rules: Vec<AppRule>,
        callback: impl Fn(MatchedRule, i32) + Send + 'static,
    ) -> Result<()> {
        // store rules and callback
        {
            let mut rules_guard = APP_RULES
                .lock()
                .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
            *rules_guard = Some(rules);
        }
        {
            let mut callback_guard = CALLBACK
                .lock()
                .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
            *callback_guard = Some(Box::new(callback));
        }

        // get the workspace notification center
        let workspace = NSWorkspace::sharedWorkspace();
        let nc: Retained<AnyObject> = unsafe { msg_send![&workspace, notificationCenter] };

        // create the notification name
        let notification_name = NSString::from_str("NSWorkspaceDidLaunchApplicationNotification");

        // create a block to handle the notification
        // use &AnyObject which encodes as '@' for Objective-C objects
        let block = block2::RcBlock::new(|notification: &AnyObject| {
            // cast AnyObject to NSNotification
            let notification: &NSNotification =
                unsafe { &*(notification as *const AnyObject as *const NSNotification) };
            handle_app_launch(notification);
        });

        // add observer
        // use Option<&AnyObject> for nullable object parameters (encodes as '@')
        let nil_object: Option<&AnyObject> = None;
        let observer: Retained<AnyObject> = unsafe {
            msg_send![
                &*nc,
                addObserverForName: &*notification_name,
                object: nil_object,
                queue: nil_object,
                usingBlock: &*block
            ]
        };

        // store observer and block as raw pointers (we need to retain them)
        let observer_ptr = Retained::into_raw(observer) as *mut c_void;
        let block_ptr = Box::into_raw(Box::new(block)) as *mut c_void;

        {
            let mut ptr_guard = OBSERVER_PTR
                .lock()
                .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
            *ptr_guard = SendPtr(observer_ptr);
        }
        {
            let mut ptr_guard = BLOCK_PTR
                .lock()
                .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
            *ptr_guard = SendPtr(block_ptr);
        }

        Ok(())
    }

    pub fn stop_watching() {
        // remove observer
        let observer_ptr = {
            let mut ptr_guard = OBSERVER_PTR.lock().ok();
            ptr_guard
                .as_mut()
                .map(|g| std::mem::replace(&mut g.0, std::ptr::null_mut()))
                .unwrap_or(std::ptr::null_mut())
        };

        if !observer_ptr.is_null() {
            unsafe {
                let workspace = NSWorkspace::sharedWorkspace();
                let nc: Retained<AnyObject> = msg_send![&workspace, notificationCenter];

                // reconstruct the Retained to get the reference
                let observer = Retained::from_raw(observer_ptr as *mut AnyObject);
                if let Some(observer) = observer {
                    let _: () = msg_send![&*nc, removeObserver: &*observer];
                    // observer is dropped here, releasing it
                }
            }
        }

        // drop the block
        let block_ptr = {
            let mut ptr_guard = BLOCK_PTR.lock().ok();
            ptr_guard
                .as_mut()
                .map(|g| std::mem::replace(&mut g.0, std::ptr::null_mut()))
                .unwrap_or(std::ptr::null_mut())
        };

        if !block_ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(block_ptr as *mut block2::RcBlock<dyn Fn(&AnyObject)>);
            }
        }

        // clear state
        if let Ok(mut rules_guard) = APP_RULES.lock() {
            *rules_guard = None;
        }
        if let Ok(mut callback_guard) = CALLBACK.lock() {
            *callback_guard = None;
        }
    }
}

pub use macos::{start_watching, stop_watching};
