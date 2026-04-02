use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::collections::HashSet;
use windows::UI::Notifications::Management::UserNotificationListener;
use windows::UI::Notifications::Management::UserNotificationListenerAccessStatus;
use windows::UI::Notifications::NotificationKinds;

fn get_log_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("WinIsland");
    std::fs::create_dir_all(&path).ok();
    path.push("notification.log");
    path
}

fn log(msg: &str) {
    let path = get_log_path();
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut file) => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
        }
        Err(e) => {
            eprintln!("[Notification] Failed to open log file: {:?}", e);
        }
    }
}

#[derive(Clone, Debug)]
pub struct NotificationInfo {
    pub id: u32,
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub timestamp: u64,
}

pub struct NotificationListener {
    current: Arc<Mutex<Option<NotificationInfo>>>,
    excluded_apps: Arc<Mutex<Vec<String>>>,
    processed_ids: Arc<Mutex<HashSet<u32>>>,
}

impl NotificationListener {
    pub fn new() -> Self {
        Self {
            current: Arc::new(Mutex::new(None)),
            excluded_apps: Arc::new(Mutex::new(Vec::new())),
            processed_ids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn set_excluded_apps(&self, apps: Vec<String>) {
        *self.excluded_apps.lock().unwrap() = apps;
    }

    pub fn init(&self) {
        let current = self.current.clone();
        let excluded_apps = self.excluded_apps.clone();
        let processed_ids = self.processed_ids.clone();
        
        std::thread::spawn(move || {
            log("Starting notification init...");
            
            let listener = match UserNotificationListener::Current() {
                Ok(l) => {
                    log("Got listener");
                    l
                }
                Err(e) => {
                    log(&format!("Failed to get listener: {:?}", e));
                    return;
                }
            };
            
            log("Requesting access...");
            let access = match listener.RequestAccessAsync() {
                Ok(async_op) => {
                    log("Got async op");
                    match async_op.get() {
                        Ok(status) => {
                            log(&format!("Access status: {:?}", status));
                            status
                        }
                        Err(e) => {
                            log(&format!("Failed to get access: {:?}", e));
                            return;
                        }
                    }
                }
                Err(e) => {
                    log(&format!("Failed to request access: {:?}", e));
                    return;
                }
            };
            
            if access != UserNotificationListenerAccessStatus::Allowed {
                log("Access denied or not allowed");
                return;
            }
            
            log("Access granted! Starting polling mode...");
            
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                
                match listener.GetNotificationsAsync(NotificationKinds::Toast) {
                    Ok(async_op) => {
                        match async_op.get() {
                            Ok(notifications) => {
                                let count = notifications.Size().unwrap_or(0);
                                
                                {
                                    let mut processed = processed_ids.lock().unwrap();
                                    let current_ids: HashSet<u32> = (0..count)
                                        .filter_map(|i| {
                                            notifications.GetAt(i).ok().and_then(|n| n.Id().ok())
                                        })
                                        .collect();
                                    
                                    let removed_ids: Vec<u32> = processed
                                        .difference(&current_ids)
                                        .copied()
                                        .collect();
                                    
                                    for id in removed_ids {
                                        processed.remove(&id);
                                    }
                                }
                                
                                for i in 0..count {
                                    if let Ok(notif) = notifications.GetAt(i) {
                                        if let Ok(notif_id) = notif.Id() {
                                            let already_processed = {
                                                let processed = processed_ids.lock().unwrap();
                                                processed.contains(&notif_id)
                                            };
                                            
                                            if already_processed {
                                                continue;
                                            }
                                            
                                            if let Ok(app_notif) = notif.Notification() {
                                                let mut notif_info = NotificationInfo {
                                                    id: notif_id,
                                                    app_name: String::new(),
                                                    title: String::new(),
                                                    body: String::new(),
                                                    timestamp: std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap_or_default()
                                                        .as_secs(),
                                                };
                                                
                                                if let Ok(app_info) = notif.AppInfo() {
                                                    if let Ok(display_info) = app_info.DisplayInfo() {
                                                        if let Ok(display_name) = display_info.DisplayName() {
                                                            let name = display_name.to_string();
                                                            if !name.is_empty() {
                                                                notif_info.app_name = name;
                                                            }
                                                        }
                                                    }
                                                    if notif_info.app_name.is_empty() {
                                                        if let Ok(app_user_model_id) = app_info.AppUserModelId() {
                                                            let aumid = app_user_model_id.to_string();
                                                            if let Some(last_part) = aumid.split('!').last() {
                                                                notif_info.app_name = last_part.to_string();
                                                            } else if !aumid.is_empty() {
                                                                notif_info.app_name = aumid;
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                                if notif_info.app_name.is_empty() {
                                                    notif_info.app_name = "Unknown App".to_string();
                                                }
                                                
                                                if let Ok(visual) = app_notif.Visual() {
                                                    if let Ok(bindings) = visual.Bindings() {
                                                        let binding_count = bindings.Size().unwrap_or(0);
                                                        for j in 0..binding_count {
                                                            if let Ok(binding) = bindings.GetAt(j) {
                                                                if let Ok(text_elements) = binding.GetTextElements() {
                                                                    let text_count = text_elements.Size().unwrap_or(0);
                                                                    for k in 0..text_count {
                                                                        if let Ok(text_elem) = text_elements.GetAt(k) {
                                                                            if let Ok(text) = text_elem.Text() {
                                                                                let text_str = text.to_string();
                                                                                if k == 0 && notif_info.title.is_empty() {
                                                                                    notif_info.title = text_str;
                                                                                } else if k == 1 && notif_info.body.is_empty() {
                                                                                    notif_info.body = text_str;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                                log(&format!("New notification: id={}, title='{}', body='{}'", 
                                                    notif_id, notif_info.title, notif_info.body));
                                                
                                                if !notif_info.title.is_empty() || !notif_info.body.is_empty() {
                                                    let excluded = excluded_apps.lock().unwrap();
                                                    if !excluded.contains(&notif_info.app_name) {
                                                        let mut current = current.lock().unwrap();
                                                        *current = Some(notif_info);
                                                    }
                                                }
                                                
                                                {
                                                    let mut processed = processed_ids.lock().unwrap();
                                                    processed.insert(notif_id);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log(&format!("GetNotificationsAsync get failed: {:?}", e));
                            }
                        }
                    }
                    Err(e) => {
                        log(&format!("GetNotificationsAsync failed: {:?}", e));
                    }
                }
            }
        });
    }

    pub fn get_current(&self) -> Option<NotificationInfo> {
        self.current.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        *self.current.lock().unwrap() = None;
    }
    
    pub fn exclude_app(&self, app_name: String) {
        let mut excluded = self.excluded_apps.lock().unwrap();
        if !excluded.contains(&app_name) {
            excluded.push(app_name);
        }
    }
}

impl Default for NotificationListener {
    fn default() -> Self {
        Self::new()
    }
}
