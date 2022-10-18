use bevy_ecs::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("{0:#?}")]
pub enum ZmaxionCoreError {
    #[error("Entity `{0:?}` is missing.")]
    MissingEntity(Entity),
    #[error(
        "Component `{component_name}` is missing on entity `{id:?}`{}", display_entity_name(.entity_name)
    )]
    MissingComponent {
        component_name: &'static str,
        entity_name: Option<String>,
        id: Entity,
    },
}

fn display_entity_name(name: &Option<String>) -> String {
    match name {
        None => String::new(),
        Some(name) => {
            format!("with name `{}`", name)
        }
    }
}
