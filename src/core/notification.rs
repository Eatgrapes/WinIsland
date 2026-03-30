use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, Duration};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
};
use windows::UI::Notifications::Management::{
    UserNotificationListener,
    UserNotificationListenerAccessStatus,
    NotificationKinds,
};
use windows::Foundation::TypedEventHandler;

#[derive(Clone, Debug)]
pub struct NotificationInfo {
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub timestamp: Instant,
}

impl Default for NotificationInfo {
    fn default() -> Self {
        Self {
            app_name: String::new(),
            title: String::new(),
            body: String::new(),
            timestamp: Instant::now(),
        }
    }
}

pub struct NotificationListener {
    current: Arc<Mutex<Option<NotificationInfo>>>,
    active: Arc<AtomicBool>,
    has_permission: Arc<Mutex<bool>>,
    excluded_apps: Arc<Mutex<Vec<String>>>,
}

impl NotificationListener {
    pub fn new() -> Self {
        Self {
            current: Arc::new(Mutex::new(None)),
            active: Arc::new(AtomicBool::new(true)),
            has_permission: Arc::new(Mutex::new(false)),
            excluded_apps: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn set_excluded_apps(&self, apps: Vec<String>) {
        *self.excluded_apps.lock().unwrap() = apps;
    }

    pub fn get_permission_status(&self) -> bool {
        *self.has_permission.lock().unwrap()
    }

    pub fn get_current(&self) -> Option<NotificationInfo> {
        self.current.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        *self.current.lock().unwrap() = None;
    }

    pub fn request_access(&self) -> bool {
        let listener = match UserNotificationListener::new() {
            Ok(l) => l,
            Err(_) => return false,
        };

        let result = match listener.RequestAccessAsync() {
            Ok(op) => match op.get() {
                Ok(status) => status,
                Err(_) => return false,
            },
            Err(_) => return false,
        };

        let has_access = result == UserNotificationListenerAccessStatus::Allowed;
        *self.has_permission.lock().unwrap() = has_access;
        has_access
    }

    fn parse_notification(notification: &windows::UI::Notifications::Notification) -> Option<NotificationInfo> {
        let visual = notification.Visual()?;
        let bindings = visual.Bindings()?;
        
        let mut title = String::new();
        let mut body = String::new();
        
        if let Ok(count) = bindings.Size() {
            for i in 0..count {
                if let Ok(binding) = bindings.GetAt(i) {
                    if let Ok(template) = binding.Template() {
                        let template_name = template.to_string();
                        if template_name.contains("ToastText") || template_name.contains("ToastGeneric") {
                            if let Ok(texts) = binding.GetTextElements() {
                                if let Ok(text_count) = texts.Size() {
                                    for j in 0..text_count {
                                        if let Ok(text_elem) = texts.GetAt(j) {
                                            if let Ok(text) = text_elem.Text() {
                                                let text_str = text.to_string();
                                                if j == 0 {
                                                    title = text_str;
                                                } else if j == 1 {
                                                    body = text_str;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if title.is_empty() && body.is_empty() {
            return None;
        }

        let app_name = if let Ok(app_info) = notification.AppInfo() {
            if let Ok(display_name) = app_info.DisplayInfo() {
                if let Ok(name) = display_name.DisplayName() {
                    name.to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        Some(NotificationInfo {
            app_name,
            title,
            body,
            timestamp: Instant::now(),
        })
    }

    pub fn init(&self) {
        let current_clone = self.current.clone();
        let active_clone = self.active.clone();
        let has_permission_clone = self.has_permission.clone();
        let excluded_apps_clone = self.excluded_apps.clone();

        std::thread::spawn(move || {
            let listener = match UserNotificationListener::new() {
                Ok(l) => l,
                Err(_) => return,
            };

            let access_result = match listener.RequestAccessAsync() {
                Ok(op) => match op.get() {
                    Ok(status) => status,
                    Err(_) => {
                        *has_permission_clone.lock().unwrap() = false;
                        return;
                    }
                },
                Err(_) => {
                    *has_permission_clone.lock().unwrap() = false;
                    return;
                }
            };

            let has_access = access_result == UserNotificationListenerAccessStatus::Allowed;
            *has_permission_clone.lock().unwrap() = has_access;

            if !has_access {
                return;
            }

            let current_for_handler = current_clone.clone();
            let excluded_for_handler = excluded_apps_clone.clone();
            let handler = TypedEventHandler::new(move |_, args: &Option<windows::UI::Notifications::NotificationChangedEventArgs>| {
                if let Some(event_args) = args {
                    if let Ok(notification) = event_args.Notification() {
                        if let Some(info) = Self::parse_notification(&notification) {
                            let excluded = excluded_for_handler.lock().unwrap();
                            if !excluded.iter().any(|e| info.app_name.contains(e) || e.contains(&info.app_name)) {
                                *current_for_handler.lock().unwrap() = Some(info);
                            }
                        }
                    }
                }
                Ok(())
            });

            let _ = listener.NotificationChanged(&handler);

            while active_clone.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(500));
            }
        });
    }
}

impl Drop for NotificationListener {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
    }
}
