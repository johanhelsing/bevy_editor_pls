use bevy::prelude::*;
use bevy::render::wireframe::{WireframePlugin, WireframeConfig};
use bevy::wgpu::{WgpuFeature, WgpuFeatures, WgpuOptions};

use bevy_inspector_egui::{Inspectable, WorldInspectorParams, WorldInspectorPlugin};
use bevy_mod_picking::{pick_labels::MESH_FOCUS, InteractablePickingPlugin, PickingPlugin, PickingPluginState};

use crate::{
    systems::EditorEvent,
    systems::{maintain_inspected_entities, send_editor_events},
    ui::{currently_inspected_system, menu_system},
};

/// See the [crate-level docs](index.html) for usage
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // bevy plugins
        app.insert_resource(WgpuOptions {
            features: WgpuFeatures {
                // The Wireframe requires NonFillPolygonMode feature
                features: vec![WgpuFeature::NonFillPolygonMode],
            },
            ..Default::default()
        });
        app.add_plugin(WireframePlugin);


        // bevy-inspector-egui
        app.insert_resource(WorldInspectorParams {
            enabled: false,
            ..Default::default()
        })
        .add_plugin(WorldInspectorPlugin);

        // bevy-mod-picking
        if !app.resources().contains::<PickingPluginState>() {
            app.add_plugin(PickingPlugin).add_plugin(InteractablePickingPlugin);
        };

        // resources
        app.init_resource::<EditorState>().add_event::<EditorEvent>();

        {
            let resources = app.resources_mut();
            let editor_settings = resources.get_or_insert_with(EditorSettings::default);
            let global_wireframe = match editor_settings.wireframe_mode {
                WireframeMode::None => false,
                WireframeMode::WithWireframeComponent => false,
                WireframeMode::All => true,
            };
            drop(editor_settings);
            // resources.get_mut::<WireframeConfig>().unwrap().global = global_wireframe;
            dbg!();
            resources.get_mut::<WireframeConfig>().unwrap().global = true;
        }

        // systems
        app.add_system(menu_system.system());

        app.add_system(currently_inspected_system.exclusive_system());
        app.add_system(send_editor_events.exclusive_system());

        app.add_system_to_stage(
            CoreStage::PostUpdate,
            maintain_inspected_entities.system().after(MESH_FOCUS),
        );
    }
}

#[derive(Default)]
pub struct EditorState {
    pub currently_inspected: Option<Entity>,
}

pub type ExclusiveAccessFn = Box<dyn Fn(&mut World, &mut Resources) + Send + Sync + 'static>;

#[derive(Inspectable, Debug, Copy, Clone)]
pub enum WireframeMode {
    None,
    WithWireframeComponent,
    All,
}

/// Configuration for for editor
pub struct EditorSettings {
    pub(crate) events_to_send: Vec<(String, ExclusiveAccessFn)>,
    pub(crate) state_transition_handlers: Vec<(String, ExclusiveAccessFn)>,
    /// controls whether clicking meshes with a [PickableBundle](bevy_mod_picking::PickableBundle) opens the inspector
    pub click_to_inspect: bool,
    pub wireframe_mode: WireframeMode,
}
impl Default for EditorSettings {
    fn default() -> Self {
        EditorSettings {
            events_to_send: Default::default(),
            state_transition_handlers: Default::default(),
            click_to_inspect: false,
            wireframe_mode: WireframeMode::None,
        }
    }
}
impl EditorSettings {
    /// Adds a event to the **Events** menu.
    /// When the menu item is clicked, the event provided by `get_event` will be sent.
    pub fn add_event<T, F>(&mut self, name: &'static str, get_event: F)
    where
        T: Resource,
        F: Fn() -> T + Send + Sync + 'static,
    {
        let f = Box::new(move |_: &mut World, resources: &mut Resources| {
            let mut events = resources
                .get_mut::<Events<T>>()
                .unwrap_or_else(|| panic!("no resource for Events<{}>", std::any::type_name::<T>()));
            events.send(get_event());
        });

        self.events_to_send.push((name.to_string(), f));
    }

    /// Adds an app to the **States** menu.
    /// When the menu item is clicked, the game will transition to that state.
    pub fn add_state<S: Resource + Clone>(&mut self, name: &'static str, state: S) {
        let f = Box::new(move |_: &mut World, resources: &mut Resources| {
            let mut events = resources.get_mut::<State<S>>().unwrap();
            if let Err(e) = events.set_next(state.clone()) {
                warn!("{}", e);
            }
        });

        self.state_transition_handlers.push((name.to_string(), f));
    }
}
