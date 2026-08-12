use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    Canvas, ClipOp, Color, Data, FilterMode, FontStyle, Image, MipmapMode, Paint, RRect, Rect,
    SamplingOptions,
};
use windows::ApplicationModel::AppDisplayInfo;
use windows::Foundation::Size;
use windows::Storage::Streams::DataReader;
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds, UserNotification};
use windows::core::HRESULT;

use crate::ui::compact::{CompactOverlayState, CompactSize};
use crate::utils::font::{DrawTextCachedParams, FontManager};

const DISPLAY_DURATION: Duration = Duration::from_secs(5);
const ENTER_DURATION: Duration = Duration::from_millis(220);
const FADE_DURATION: Duration = Duration::from_millis(280);
const DETAIL_LINE_GAP: f32 = 21.0;
const MAX_ICON_BYTES: u64 = 2 * 1024 * 1024;
const DATABASE_CHECK_INTERVAL: Duration = Duration::from_millis(100);
const DATABASE_CHANGE_DEBOUNCE: Duration = Duration::from_millis(150);
const DATABASE_SETTLE_INTERVAL: Duration = Duration::from_millis(300);
const FALLBACK_POLL_INTERVAL: Duration = Duration::from_secs(30);
const RETRY_INTERVAL: Duration = Duration::from_secs(5);

pub(super) struct NotificationPayload {
    notification_id: u32,
    app_name: String,
    app_user_model_id: Option<String>,
    title: String,
    detail: String,
    icon: Option<NotificationIconData>,
}

struct NotificationIconData {
    bytes: Vec<u8>,
    visible_bounds: Option<IconBounds>,
}

struct NotificationIcon {
    image: Image,
    visible_bounds: Option<IconBounds>,
}

#[derive(Clone, Copy)]
struct IconBounds {
    left: u32,
    top: u32,
    width: u32,
    height: u32,
}

enum NotificationPollResult {
    Latest(Option<u32>),
    Failed(HRESULT),
}

#[derive(Default)]
struct NotificationDatabaseWatch {
    path: Option<PathBuf>,
    modified: Option<SystemTime>,
    check_after: Option<Instant>,
    query_after: Option<Instant>,
    settle_until: Option<Instant>,
}

impl NotificationDatabaseWatch {
    fn start(&mut self, now: Instant) {
        self.path = notification_database_path();
        self.modified = self.read_modified();
        self.check_after = Some(now + DATABASE_CHECK_INTERVAL);
        self.query_after = None;
        self.settle_until = None;
    }

    fn update(&mut self, now: Instant) -> bool {
        if self
            .check_after
            .is_some_and(|check_after| now < check_after)
        {
            return false;
        }
        self.check_after = Some(now + DATABASE_CHECK_INTERVAL);
        let modified = self.read_modified();
        let changed = modified != self.modified;
        self.modified = modified;

        if let Some(settle_until) = self.settle_until {
            if changed {
                self.settle_until = Some(now + DATABASE_SETTLE_INTERVAL);
            } else if now >= settle_until {
                self.settle_until = None;
            }
            self.query_after = None;
            return false;
        }
        if changed {
            self.query_after = Some(now + DATABASE_CHANGE_DEBOUNCE);
        }
        let due = self
            .query_after
            .is_some_and(|query_after| now >= query_after);
        if due {
            self.query_after = None;
        }
        due
    }

    fn settle_after_query(&mut self, now: Instant) {
        self.modified = self.read_modified();
        self.check_after = Some(now + DATABASE_CHECK_INTERVAL);
        self.query_after = None;
        self.settle_until = Some(now + DATABASE_SETTLE_INTERVAL);
    }

    fn read_modified(&self) -> Option<SystemTime> {
        self.path
            .as_ref()
            .and_then(|path| std::fs::metadata(path).ok())
            .and_then(|metadata| metadata.modified().ok())
    }
}

#[derive(Default)]
pub(super) struct NotificationMonitor {
    listener: Option<UserNotificationListener>,
    latest_notification_id: Arc<Mutex<Option<u32>>>,
    access_receiver: Option<Receiver<bool>>,
    poll_receiver: Option<Receiver<NotificationPollResult>>,
    access_attempted: bool,
    retry_after: Option<Instant>,
    history_poll_after: Option<Instant>,
    fallback_poll_after: Option<Instant>,
    database_watch: NotificationDatabaseWatch,
    last_polled_notification_id: Option<u32>,
    poll_initialized: bool,
}

impl NotificationMonitor {
    pub(super) fn update(&mut self, enabled: bool) -> Option<NotificationPayload> {
        if !enabled {
            self.stop();
            self.access_attempted = false;
            self.retry_after = None;
            return None;
        }

        self.finish_access_request();
        if self.listener.is_none()
            && self.access_receiver.is_none()
            && !self.access_attempted
            && self
                .retry_after
                .is_none_or(|retry_after| Instant::now() >= retry_after)
        {
            self.access_attempted = true;
            self.request_access();
        }
        self.poll_notifications();
        self.take_payload()
    }

    fn request_access(&mut self) {
        let Ok(listener) = UserNotificationListener::Current() else {
            log::warn!("Notification listener is unavailable");
            self.schedule_retry();
            return;
        };
        match listener.GetAccessStatus() {
            Ok(UserNotificationListenerAccessStatus::Allowed) => self.start_monitor(listener),
            Ok(UserNotificationListenerAccessStatus::Unspecified) => {
                let Ok(operation) = listener.RequestAccessAsync() else {
                    log::warn!("Notification access request could not be started");
                    self.schedule_retry();
                    return;
                };
                let (sender, receiver) = mpsc::sync_channel(1);
                tokio::task::spawn_blocking(move || {
                    let granted = matches!(
                        operation.join(),
                        Ok(UserNotificationListenerAccessStatus::Allowed)
                    );
                    let _ = sender.send(granted);
                });
                self.access_receiver = Some(receiver);
            }
            Ok(status) => log::warn!("Notification access was not granted: {:?}", status),
            Err(error) => {
                log::warn!("Notification access status is unavailable: {:?}", error);
                self.schedule_retry();
            }
        }
    }

    fn finish_access_request(&mut self) {
        let result = self
            .access_receiver
            .as_ref()
            .map(|receiver| receiver.try_recv());
        match result {
            Some(Ok(true)) => {
                self.access_receiver = None;
                let Ok(listener) = UserNotificationListener::Current() else {
                    log::warn!("Notification listener is unavailable after access was granted");
                    self.schedule_retry();
                    return;
                };
                self.start_monitor(listener);
            }
            Some(Ok(false)) => {
                self.access_receiver = None;
                log::warn!("Notification access was not granted");
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.access_receiver = None;
                log::warn!("Notification access request ended unexpectedly");
                self.schedule_retry();
            }
            Some(Err(mpsc::TryRecvError::Empty)) | None => {}
        }
    }

    fn start_monitor(&mut self, listener: UserNotificationListener) {
        let now = Instant::now();
        self.listener = Some(listener);
        self.retry_after = None;
        self.history_poll_after = Some(now);
        self.fallback_poll_after = Some(now + FALLBACK_POLL_INTERVAL);
        self.database_watch.start(now);
        self.last_polled_notification_id = None;
        self.poll_initialized = false;
    }

    fn poll_notifications(&mut self) {
        let result = self
            .poll_receiver
            .as_ref()
            .map(|receiver| receiver.try_recv());
        match result {
            Some(Ok(NotificationPollResult::Latest(notification_id))) => {
                self.poll_receiver = None;
                self.finish_history_poll(Instant::now());
                self.update_polled_notification(notification_id);
            }
            Some(Ok(NotificationPollResult::Failed(error))) => {
                self.poll_receiver = None;
                self.retry_history_poll(Instant::now());
                log::warn!("Notification history could not be read: {:?}", error);
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.poll_receiver = None;
                self.retry_history_poll(Instant::now());
                log::warn!("Notification history request ended unexpectedly");
            }
            Some(Err(mpsc::TryRecvError::Empty)) => return,
            None => {}
        }

        let now = Instant::now();
        let database_due = self.database_watch.update(now);
        let history_due = self
            .history_poll_after
            .is_some_and(|poll_after| now >= poll_after);
        let fallback_due = self
            .fallback_poll_after
            .is_some_and(|poll_after| now >= poll_after);
        if self.poll_receiver.is_some() || (!history_due && !database_due && !fallback_due) {
            return;
        }
        let Some(listener) = self.listener.as_ref() else {
            return;
        };
        let Ok(operation) = listener.GetNotificationsAsync(NotificationKinds::Toast) else {
            self.retry_history_poll(now);
            log::warn!("Notification history request could not be started");
            return;
        };
        self.history_poll_after = None;
        self.fallback_poll_after = None;
        let (sender, receiver) = mpsc::sync_channel(1);
        tokio::task::spawn_blocking(move || {
            let result = operation.join().map(|notifications| {
                let mut latest_notification_id: Option<u32> = None;
                if let Ok(count) = notifications.Size() {
                    for index in 0..count {
                        if let Ok(notification) = notifications.GetAt(index)
                            && let Ok(notification_id) = notification.Id()
                        {
                            latest_notification_id = Some(
                                latest_notification_id
                                    .map_or(notification_id, |latest| latest.max(notification_id)),
                            );
                        }
                    }
                }
                latest_notification_id
            });
            let result = match result {
                Ok(notification_id) => NotificationPollResult::Latest(notification_id),
                Err(error) => NotificationPollResult::Failed(error.code()),
            };
            let _ = sender.send(result);
        });
        self.poll_receiver = Some(receiver);
    }

    fn finish_history_poll(&mut self, now: Instant) {
        self.history_poll_after = None;
        self.fallback_poll_after = Some(now + FALLBACK_POLL_INTERVAL);
        self.database_watch.settle_after_query(now);
    }

    fn retry_history_poll(&mut self, now: Instant) {
        self.history_poll_after = Some(now + RETRY_INTERVAL);
        self.fallback_poll_after = None;
        self.database_watch.settle_after_query(now);
    }

    fn update_polled_notification(&mut self, notification_id: Option<u32>) {
        if !self.poll_initialized {
            self.last_polled_notification_id = notification_id;
            self.poll_initialized = true;
            return;
        }
        let Some(notification_id) = notification_id else {
            return;
        };
        if self
            .last_polled_notification_id
            .is_none_or(|latest| notification_id > latest)
        {
            self.last_polled_notification_id = Some(notification_id);
            *self
                .latest_notification_id
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = Some(notification_id);
        }
    }

    fn stop(&mut self) {
        self.listener = None;
        self.access_receiver = None;
        self.poll_receiver = None;
        self.history_poll_after = None;
        self.fallback_poll_after = None;
        self.database_watch = NotificationDatabaseWatch::default();
        self.last_polled_notification_id = None;
        self.poll_initialized = false;
        *self
            .latest_notification_id
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
    }

    fn schedule_retry(&mut self) {
        self.access_attempted = false;
        self.retry_after = Some(Instant::now() + RETRY_INTERVAL);
    }

    fn take_payload(&self) -> Option<NotificationPayload> {
        let notification_id = self
            .latest_notification_id
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()?;
        self.listener
            .as_ref()
            .and_then(|listener| read_notification(listener, notification_id))
    }

    pub(super) fn remove_notification(&self, notification_id: u32) {
        if let Some(listener) = &self.listener
            && let Err(error) = listener.RemoveNotification(notification_id)
        {
            log::debug!("Notification could not be removed: {error:?}");
        }
    }
}

fn notification_database_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|path| {
        path.join("Microsoft")
            .join("Windows")
            .join("Notifications")
            .join("wpndatabase.db")
    })
}

impl Drop for NotificationMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

fn read_notification(
    listener: &UserNotificationListener,
    notification_id: u32,
) -> Option<NotificationPayload> {
    let notification = listener.GetNotification(notification_id).ok()?;
    let (mut title, detail) = read_notification_text(&notification);
    let (app_name, app_user_model_id, icon) = notification
        .AppInfo()
        .ok()
        .and_then(|app| {
            let display = app.DisplayInfo().ok()?;
            let name = display
                .DisplayName()
                .map(|name| name.to_string())
                .unwrap_or_default();
            let app_user_model_id = app
                .AppUserModelId()
                .ok()
                .map(|app_user_model_id| app_user_model_id.to_string())
                .filter(|app_user_model_id| !app_user_model_id.is_empty());
            Some((name, app_user_model_id, read_app_icon(&display)))
        })
        .unwrap_or_default();

    if title.is_empty() {
        title = app_name.clone();
    }
    (!title.is_empty()).then_some(NotificationPayload {
        notification_id,
        app_name,
        app_user_model_id,
        title,
        detail,
        icon,
    })
}

fn read_notification_text(notification: &UserNotification) -> (String, String) {
    let Some(binding_name) = KnownNotificationBindings::ToastGeneric().ok() else {
        return (String::new(), String::new());
    };
    let Some(binding) = notification
        .Notification()
        .ok()
        .and_then(|notification| notification.Visual().ok())
        .and_then(|visual| visual.GetBinding(&binding_name).ok())
    else {
        return (String::new(), String::new());
    };
    let Some(text_elements) = binding.GetTextElements().ok() else {
        return (String::new(), String::new());
    };
    let mut lines = Vec::new();
    for index in 0..text_elements.Size().unwrap_or(0) {
        let Some(text) = text_elements
            .GetAt(index)
            .ok()
            .and_then(|element| element.Text().ok())
            .map(|text| text.to_string())
        else {
            continue;
        };
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !text.is_empty() {
            lines.push(text);
        }
    }
    (
        lines.first().cloned().unwrap_or_default(),
        lines.into_iter().skip(1).collect::<Vec<_>>().join(" "),
    )
}

fn read_app_icon(display: &AppDisplayInfo) -> Option<NotificationIconData> {
    let logo = display
        .GetLogo(Size {
            Width: 64.0,
            Height: 64.0,
        })
        .ok()?;
    let stream = logo.OpenReadAsync().ok()?.join().ok()?;
    let size = stream.Size().ok()?;
    if size == 0 || size > MAX_ICON_BYTES {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size as u32).ok()?.join().ok()?;
    let mut bytes = vec![0; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    Some(NotificationIconData {
        visible_bounds: visible_icon_bounds(&bytes),
        bytes,
    })
}

fn visible_icon_bounds(bytes: &[u8]) -> Option<IconBounds> {
    const ALPHA_THRESHOLD: u8 = 8;

    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (image_width, image_height) = image.dimensions();
    let mut bounds: Option<IconBounds> = None;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] < ALPHA_THRESHOLD {
            continue;
        }
        bounds = Some(match bounds {
            Some(bounds) => {
                let right = bounds.left + bounds.width - 1;
                let bottom = bounds.top + bounds.height - 1;
                let left = bounds.left.min(x);
                let top = bounds.top.min(y);
                IconBounds {
                    left,
                    top,
                    width: right.max(x) - left + 1,
                    height: bottom.max(y) - top + 1,
                }
            }
            None => IconBounds {
                left: x,
                top: y,
                width: 1,
                height: 1,
            },
        });
    }
    bounds.filter(|bounds| {
        bounds.width > 0
            && bounds.height > 0
            && bounds.left + bounds.width <= image_width
            && bounds.top + bounds.height <= image_height
    })
}

#[derive(Default)]
pub(super) struct NotificationIndicator {
    notification_id: Option<u32>,
    app_name: String,
    app_user_model_id: Option<String>,
    title: String,
    detail: String,
    icon: Option<NotificationIcon>,
    pending: Option<NotificationPayload>,
    display_started: Option<Instant>,
    display_until: Option<Instant>,
}

impl NotificationIndicator {
    pub(super) fn update(
        &mut self,
        notification: Option<NotificationPayload>,
        state: CompactOverlayState,
    ) -> bool {
        if self
            .display_until
            .is_some_and(|until| until + FADE_DURATION <= Instant::now())
        {
            self.clear_display();
        }
        let received_notification = notification.is_some();
        if let Some(notification) = notification {
            self.pending = Some(notification);
        }
        if !matches!(state, CompactOverlayState::Present) {
            self.clear_display();
            if matches!(state, CompactOverlayState::Discard) {
                self.pending = None;
            }
            return received_notification;
        }
        let Some(notification) = self.pending.take() else {
            return received_notification;
        };

        let NotificationPayload {
            notification_id,
            app_name,
            app_user_model_id,
            title,
            detail,
            icon,
        } = notification;
        self.app_name = if title.trim().eq_ignore_ascii_case(app_name.trim()) {
            String::new()
        } else {
            app_name
        };
        self.title = title;
        self.detail = detail;
        self.notification_id = Some(notification_id);
        self.app_user_model_id = app_user_model_id;
        self.icon = icon.and_then(|icon| {
            Image::from_encoded(Data::new_copy(&icon.bytes)).map(|image| NotificationIcon {
                image,
                visible_bounds: icon.visible_bounds,
            })
        });
        self.display_started = Some(Instant::now());
        self.display_until = Some(Instant::now() + DISPLAY_DURATION);
        received_notification
    }

    pub(super) fn clear(&mut self) {
        self.pending = None;
        self.clear_display();
    }

    pub(super) fn activate(&mut self) -> Option<u32> {
        let app_user_model_id = self.app_user_model_id.as_deref()?;
        if crate::utils::win32::activate_application(app_user_model_id) {
            self.take_notification_id()
        } else {
            None
        }
    }

    pub(super) fn dismiss(&mut self) -> Option<u32> {
        self.take_notification_id()
    }

    fn clear_display(&mut self) {
        self.notification_id = None;
        self.app_name = String::new();
        self.app_user_model_id = None;
        self.title = String::new();
        self.detail = String::new();
        self.display_started = None;
        self.display_until = None;
        self.icon = None;
    }

    fn take_notification_id(&mut self) -> Option<u32> {
        let notification_id = self.notification_id.take();
        self.clear_display();
        notification_id
    }

    pub(super) fn is_visible(&self) -> bool {
        self.display_until
            .is_some_and(|until| until + FADE_DURATION > Instant::now())
    }

    pub(super) fn target_size(&self, base_width: f32, base_height: f32, scale: f32) -> CompactSize {
        CompactSize {
            width: base_width.max(330.0) * scale,
            height: base_height.max(82.0) * scale,
        }
    }

    pub(super) fn draw(&self, canvas: &Canvas, rect: Rect, scale: f32, alpha: f32) {
        let (opacity, offset_y) = self.presentation();
        let alpha = (alpha * opacity * 255.0).round().clamp(0.0, 255.0) as u8;
        if alpha == 0 {
            return;
        }

        canvas.save();
        canvas.translate((0.0, offset_y * scale));

        let has_icon = self.icon.is_some();
        let content_left = if has_icon {
            rect.left() + 72.0 * scale
        } else {
            rect.left() + 20.0 * scale
        };
        let content_width = rect.right() - 18.0 * scale - content_left;
        if content_width <= 0.0 {
            canvas.restore();
            return;
        }

        if let Some(icon) = &self.icon {
            draw_notification_icon(canvas, icon, rect, scale, alpha);
        }

        let mut app_paint = Paint::default();
        app_paint.set_anti_alias(true);
        app_paint.set_color(Color::from_argb((alpha as f32 * 0.65) as u8, 255, 255, 255));
        let mut title_paint = Paint::default();
        title_paint.set_anti_alias(true);
        title_paint.set_color(Color::from_argb(alpha, 255, 255, 255));
        let mut detail_paint = Paint::default();
        detail_paint.set_anti_alias(true);
        detail_paint.set_color(Color::from_argb((alpha as f32 * 0.72) as u8, 255, 255, 255));

        let top = rect.top();
        if !self.app_name.is_empty() {
            draw_notification_text(
                DrawTextCachedParams {
                    canvas,
                    text: &self.app_name,
                    x: content_left,
                    y: top + 22.0 * scale,
                    size: 11.0 * scale,
                    bold: false,
                    paint: &app_paint,
                },
                content_width,
            );
        }
        let title_y = if self.app_name.is_empty() && self.detail.is_empty() {
            top + (rect.height() + 13.0 * scale) / 2.0
        } else if self.app_name.is_empty() {
            top + 34.0 * scale
        } else {
            top + 43.0 * scale
        };
        draw_notification_text(
            DrawTextCachedParams {
                canvas,
                text: &self.title,
                x: content_left,
                y: title_y,
                size: 13.0 * scale,
                bold: true,
                paint: &title_paint,
            },
            content_width,
        );
        if !self.detail.is_empty() {
            draw_notification_text(
                DrawTextCachedParams {
                    canvas,
                    text: &self.detail,
                    x: content_left,
                    y: title_y + DETAIL_LINE_GAP * scale,
                    size: 11.0 * scale,
                    bold: false,
                    paint: &detail_paint,
                },
                content_width,
            );
        }
        canvas.restore();
    }

    fn presentation(&self) -> (f32, f32) {
        let Some(started) = self.display_started else {
            return (0.0, 0.0);
        };
        let Some(until) = self.display_until else {
            return (0.0, 0.0);
        };
        let now = Instant::now();
        let enter = ease_out_cubic(
            (now.saturating_duration_since(started).as_secs_f32() / ENTER_DURATION.as_secs_f32())
                .clamp(0.0, 1.0),
        );
        let exit_elapsed = now.saturating_duration_since(until);
        let exit = if exit_elapsed.is_zero() {
            1.0
        } else {
            (1.0 - exit_elapsed.as_secs_f32() / FADE_DURATION.as_secs_f32()).clamp(0.0, 1.0)
        };
        (enter * exit, (1.0 - enter) * 7.0)
    }
}

fn ease_out_cubic(value: f32) -> f32 {
    1.0 - (1.0 - value).powi(3)
}

fn draw_notification_text(params: DrawTextCachedParams<'_>, max_width: f32) {
    let text = truncate_notification_text(params.text, params.size, params.bold, max_width);
    FontManager::global().draw_text_cached(DrawTextCachedParams {
        text: &text,
        ..params
    });
}

fn truncate_notification_text<'a>(
    text: &'a str,
    size: f32,
    bold: bool,
    max_width: f32,
) -> Cow<'a, str> {
    let font_manager = FontManager::global();
    let style = if bold {
        FontStyle::bold()
    } else {
        FontStyle::normal()
    };
    if font_manager.measure_text_cached(text, size, style) <= max_width {
        return Cow::Borrowed(text);
    }

    const ELLIPSIS: &str = "…";
    let ellipsis_width = font_manager.measure_text_cached(ELLIPSIS, size, style);
    if ellipsis_width >= max_width {
        return Cow::Borrowed(ELLIPSIS);
    }

    let mut truncated = String::new();
    let mut width = 0.0;
    for character in text.chars() {
        let character_width = font_manager.measure_text_cached(&character.to_string(), size, style);
        if width + character_width + ellipsis_width > max_width {
            break;
        }
        width += character_width;
        truncated.push(character);
    }
    truncated.push_str(ELLIPSIS);
    Cow::Owned(truncated)
}

fn draw_notification_icon(
    canvas: &Canvas,
    icon: &NotificationIcon,
    rect: Rect,
    scale: f32,
    alpha: u8,
) {
    let size = 42.0 * scale;
    let icon_rect = Rect::from_xywh(
        rect.left() + 18.0 * scale,
        rect.center_y() - size / 2.0,
        size,
        size,
    );
    let image_width = icon.image.width() as f32;
    let image_height = icon.image.height() as f32;
    if image_width <= 0.0 || image_height <= 0.0 {
        return;
    }
    let source = icon
        .visible_bounds
        .filter(|bounds| {
            bounds.left < icon.image.width() as u32
                && bounds.top < icon.image.height() as u32
                && bounds.width > 0
                && bounds.height > 0
        })
        .map(|bounds| {
            Rect::from_xywh(
                bounds.left as f32,
                bounds.top as f32,
                bounds.width.min(icon.image.width() as u32 - bounds.left) as f32,
                bounds.height.min(icon.image.height() as u32 - bounds.top) as f32,
            )
        })
        .unwrap_or_else(|| Rect::from_xywh(0.0, 0.0, image_width, image_height));
    let scale = (icon_rect.width() / source.width()).min(icon_rect.height() / source.height());
    let destination = Rect::from_xywh(
        icon_rect.center_x() - source.width() * scale / 2.0,
        icon_rect.center_y() - source.height() * scale / 2.0,
        source.width() * scale,
        source.height() * scale,
    );
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_alpha_f(alpha as f32 / 255.0);
    canvas.save();
    canvas.clip_rrect(
        RRect::new_rect_xy(icon_rect, 11.0 * scale, 11.0 * scale),
        ClipOp::Intersect,
        true,
    );
    canvas.draw_image_rect_with_sampling_options(
        &icon.image,
        Some((&source, SrcRectConstraint::Fast)),
        destination,
        SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear),
        &paint,
    );
    canvas.restore();
}
