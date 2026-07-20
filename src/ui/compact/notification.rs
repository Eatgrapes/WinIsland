use std::borrow::Cow;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    Canvas, ClipOp, Color, Data, FilterMode, FontStyle, Image, MipmapMode, Paint, RRect, Rect,
    SamplingOptions,
};
use windows::ApplicationModel::AppDisplayInfo;
use windows::Foundation::{Size, TypedEventHandler};
use windows::Storage::Streams::DataReader;
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::{
    KnownNotificationBindings, UserNotification, UserNotificationChangedEventArgs,
    UserNotificationChangedKind,
};

use crate::ui::compact::{CompactOverlayState, CompactSize};
use crate::utils::font::{DrawTextCachedParams, FontManager};

const DISPLAY_DURATION: Duration = Duration::from_secs(5);
const ENTER_DURATION: Duration = Duration::from_millis(220);
const FADE_DURATION: Duration = Duration::from_millis(280);
const DETAIL_LINE_GAP: f32 = 21.0;
const MAX_ICON_BYTES: u64 = 2 * 1024 * 1024;
const RETRY_INTERVAL: Duration = Duration::from_secs(5);

pub(super) struct NotificationPayload {
    app_name: String,
    title: String,
    detail: String,
    icon_bytes: Option<Vec<u8>>,
}

#[derive(Default)]
pub(super) struct NotificationMonitor {
    listener: Option<UserNotificationListener>,
    handler: Option<TypedEventHandler<UserNotificationListener, UserNotificationChangedEventArgs>>,
    registration: Option<i64>,
    latest_notification_id: Arc<Mutex<Option<u32>>>,
    access_receiver: Option<Receiver<bool>>,
    access_attempted: bool,
    retry_after: Option<Instant>,
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
        self.take_payload()
    }

    fn request_access(&mut self) {
        let Ok(listener) = UserNotificationListener::Current() else {
            log::warn!("Notification listener is unavailable");
            self.schedule_retry();
            return;
        };
        match listener.GetAccessStatus() {
            Ok(UserNotificationListenerAccessStatus::Allowed) => self.start_listener(listener),
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
                self.start_listener(listener);
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

    fn start_listener(&mut self, listener: UserNotificationListener) {
        let latest_notification_id = self.latest_notification_id.clone();
        let handler =
            TypedEventHandler::<UserNotificationListener, UserNotificationChangedEventArgs>::new(
                move |_sender, args| {
                    let args = args.ok()?;
                    if args.ChangeKind()? == UserNotificationChangedKind::Added {
                        *latest_notification_id
                            .lock()
                            .unwrap_or_else(|error| error.into_inner()) =
                            Some(args.UserNotificationId()?);
                    }
                    Ok(())
                },
            );
        match listener.NotificationChanged(&handler) {
            Ok(registration) => {
                self.listener = Some(listener);
                self.handler = Some(handler);
                self.registration = Some(registration);
                self.retry_after = None;
            }
            Err(error) => {
                log::warn!(
                    "Notification listener could not register for events: {:?}",
                    error
                );
                self.schedule_retry();
            }
        }
    }

    fn stop(&mut self) {
        if let (Some(listener), Some(registration)) = (&self.listener, self.registration.take()) {
            let _ = listener.RemoveNotificationChanged(registration);
        }
        self.handler = None;
        self.listener = None;
        self.access_receiver = None;
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
    let (app_name, icon_bytes) = notification
        .AppInfo()
        .ok()
        .and_then(|app| {
            let display = app.DisplayInfo().ok()?;
            let name = display
                .DisplayName()
                .map(|name| name.to_string())
                .unwrap_or_default();
            Some((name, read_app_icon(&display)))
        })
        .unwrap_or_default();

    if title.is_empty() {
        title = app_name.clone();
    }
    (!title.is_empty()).then_some(NotificationPayload {
        app_name,
        title,
        detail,
        icon_bytes,
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
        if !text.is_empty() {
            lines.push(text);
        }
    }
    (
        lines.first().cloned().unwrap_or_default(),
        lines.into_iter().skip(1).collect::<Vec<_>>().join(" "),
    )
}

fn read_app_icon(display: &AppDisplayInfo) -> Option<Vec<u8>> {
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
    Some(bytes)
}

#[derive(Default)]
pub(super) struct NotificationIndicator {
    app_name: String,
    title: String,
    detail: String,
    icon: Option<Image>,
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
            app_name,
            title,
            detail,
            icon_bytes,
        } = notification;
        self.app_name = if title.trim().eq_ignore_ascii_case(app_name.trim()) {
            String::new()
        } else {
            app_name
        };
        self.title = title;
        self.detail = detail;
        self.icon = icon_bytes.and_then(|bytes| Image::from_encoded(Data::new_copy(&bytes)));
        self.display_started = Some(Instant::now());
        self.display_until = Some(Instant::now() + DISPLAY_DURATION);
        received_notification
    }

    pub(super) fn clear(&mut self) {
        self.pending = None;
        self.clear_display();
    }

    fn clear_display(&mut self) {
        self.app_name = String::new();
        self.title = String::new();
        self.detail = String::new();
        self.display_started = None;
        self.display_until = None;
        self.icon = None;
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

fn draw_notification_icon(canvas: &Canvas, icon: &Image, rect: Rect, scale: f32, alpha: u8) {
    let size = 42.0 * scale;
    let icon_rect = Rect::from_xywh(
        rect.left() + 18.0 * scale,
        rect.center_y() - size / 2.0,
        size,
        size,
    );
    let image_width = icon.width() as f32;
    let image_height = icon.height() as f32;
    if image_width <= 0.0 || image_height <= 0.0 {
        return;
    }
    let source = if image_width > image_height {
        let width = image_height;
        Rect::from_xywh((image_width - width) / 2.0, 0.0, width, image_height)
    } else {
        let height = image_width;
        Rect::from_xywh(0.0, (image_height - height) / 2.0, image_width, height)
    };
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
        icon,
        Some((&source, SrcRectConstraint::Fast)),
        icon_rect,
        SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear),
        &paint,
    );
    canvas.restore();
}
