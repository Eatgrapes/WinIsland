use crate::plugin::types::WidgetDrawFnV1;

pub struct PluginWidget {
    pub id: u64,
    pub span_cols: u32,
    pub span_rows: u32,
    pub title: String,
    pub body: String,
    pub on_draw: Option<WidgetDrawFnV1>,
    pub callback_data: usize,
}

pub struct WidgetManager {
    plugin_widgets: Vec<PluginWidget>,
}

impl WidgetManager {
    pub fn new() -> Self {
        Self {
            plugin_widgets: Vec::new(),
        }
    }

    pub fn upsert_widget(&mut self, widget: PluginWidget) {
        if let Some(existing) = self
            .plugin_widgets
            .iter_mut()
            .find(|existing| existing.id == widget.id)
        {
            *existing = widget;
        } else {
            self.plugin_widgets.push(widget);
        }
    }

    pub fn remove_widget(&mut self, id: u64) -> bool {
        let original_len = self.plugin_widgets.len();
        self.plugin_widgets.retain(|widget| widget.id != id);
        self.plugin_widgets.len() != original_len
    }

    pub fn widgets(&self) -> &[PluginWidget] {
        &self.plugin_widgets
    }
}
