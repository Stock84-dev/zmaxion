use std::ops::{Deref, DerefMut};

use bevy_ecs::{
    prelude::*,
    system::{IsFunctionSystem, SystemParam},
    world::{EntityMut, EntityRef},
};
use zmaxion_utils::prelude::PrioMutexGuard;

use crate::{error::ZmaxionCoreError, prelude::*};

pub trait WorldExt {
    fn run_system<S, Params>(&mut self, system: S)
    where
        Params: SystemParam,
        S: IntoSystem<(), (), (IsFunctionSystem, Params, ())>;

    fn try_ref<C: Component>(&self, id: Entity) -> Result<&C, ZmaxionCoreError>;
    fn try_mut<C: Component>(&mut self, id: Entity) -> Result<Mut<C>, ZmaxionCoreError>;
}

impl WorldExt for World {
    fn run_system<S, Params>(&mut self, system: S)
    where
        Params: SystemParam,
        S: IntoSystem<(), (), (IsFunctionSystem, Params, ())>,
    {
        let mut system = IntoSystem::into_system(system);
        system.run((), self);
    }

    fn try_ref<C: Component>(&self, id: Entity) -> Result<&C, ZmaxionCoreError> {
        match self.get_entity(id) {
            None => Err(ZmaxionCoreError::MissingEntity(id)),
            Some(e) => match e.get::<C>() {
                None => Err(ZmaxionCoreError::MissingComponent {
                    component_name: std::any::type_name::<C>(),
                    entity_name: e.get::<Name>().map(|x| x.0.clone()),
                    id,
                }),
                Some(c) => Ok(c),
            },
        }
    }

    fn try_mut<'a, C: Component>(&'a mut self, id: Entity) -> Result<Mut<'a, C>, ZmaxionCoreError> {
        match self.get_entity_mut(id) {
            None => return Err(ZmaxionCoreError::MissingEntity(id)),
            _ => {}
        }
        // borrow checker problems, we need nll feature on stable
        let world = unsafe { &mut *(&*self as *const _ as *mut World) };
        let entity = self
            .get_entity_mut(id)
            .ok_or(ZmaxionCoreError::MissingEntity(id))?;
        unsafe { entity.get_unchecked_mut::<C>() }.ok_or_else(|| {
            ZmaxionCoreError::MissingComponent {
                component_name: std::any::type_name::<C>(),
                entity_name: world.get::<Name>(id).map(|x| x.0.clone()),
                id,
            }
        })
    }
}

impl<'a> WorldExt for PrioMutexGuard<'a, World> {
    fn run_system<S, Params>(&mut self, system: S)
    where
        Params: SystemParam,
        S: IntoSystem<(), (), (IsFunctionSystem, Params, ())>,
    {
        self.deref_mut().run_system(system)
    }

    fn try_ref<C: Component>(&self, id: Entity) -> Result<&C, ZmaxionCoreError> {
        self.deref().try_ref(id)
    }

    fn try_mut<C: Component>(&mut self, id: Entity) -> Result<Mut<C>, ZmaxionCoreError> {
        self.deref_mut().try_mut(id)
    }
}
