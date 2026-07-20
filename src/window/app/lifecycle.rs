use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::WindowId;

use super::App;

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.on_resumed(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self
            .settings
            .as_ref()
            .and_then(|settings| settings.window_id())
            == Some(id)
        {
            if let Some(settings) = self.settings.as_mut() {
                settings.handle_window_event(event_loop, event);
            }
            if self
                .settings
                .as_ref()
                .is_some_and(|settings| settings.close_requested())
            {
                self.close_settings();
            }
            return;
        }
        self.on_window_event(event_loop, id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.on_about_to_wait(event_loop);
        if let Some(settings_deadline) = self
            .settings
            .as_mut()
            .and_then(|settings| settings.update())
        {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                self.next_frame_deadline.min(settings_deadline),
            ));
        }
    }
}
