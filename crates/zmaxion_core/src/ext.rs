use bevy_ecs::prelude::*;

use crate::resources::GlobalEntity;

pub trait EntityExt {
    fn get<'a, C: Component>(&self, query: &'a Query<'_, '_, &'static C>) -> &'a C {
        self.expect::<C, _, _>(query.get(self.id()))
    }

    fn get_mut<'s, C: Component>(
        &self,
        query: &'s mut Query<'_, '_, &'static mut C>,
    ) -> Mut<'s, C> {
        self.expect::<C, _, _>(query.get_mut(self.id()))
    }

    fn id(&self) -> Entity;

    fn expect<C, R, E>(&self, result: Result<R, E>) -> R;
}

impl EntityExt for GlobalEntity {
    fn id(&self) -> Entity {
        self.0
    }

    fn expect<C, R, E>(&self, result: Result<R, E>) -> R {
        match result {
            Ok(c) => c,
            Err(_) => panic!(
                "Entity `{:?}`(global) doesn't have `{}` component.",
                self.0,
                std::any::type_name::<C>()
            ),
        }
    }
}

impl EntityExt for Entity {
    fn id(&self) -> Entity {
        *self
    }

    fn expect<C, R, E>(&self, result: Result<R, E>) -> R {
        match result {
            Ok(c) => c,
            Err(_) => panic!(
                "Entity `{:?}` doesn't have `{}` component.",
                self,
                std::any::type_name::<C>()
            ),
        }
    }
}
