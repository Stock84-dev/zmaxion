use crate::{
    bundle::Bundle,
    entity::Entity,
    parallel_executor::{system::SystemId, SystemContainer},
    world::World,
};

pub struct SpawnEntity(Vec<Entity>);
pub struct GetOrSpawnEntity(Vec<Entity>);
pub struct SpawnBundle(Vec<Box<dyn DynWorldCommand>>);
pub struct Insert(Vec<Box<dyn DynWorldCommand>>);
pub struct InsertBundle(Vec<Box<dyn DynWorldCommand>>);
pub struct RemoveBundle(Entity, fn(&mut World));
pub struct Remove(Entity, fn(&mut World));
pub struct Despawn(Entity);
pub struct SpawnBatch(Vec<Box<dyn DynWorldCommand>>);
pub struct InsertOrSpawnBatch(Vec<Box<dyn DynWorldCommand>>);
pub struct InitResource(Vec<fn(&mut World)>);
pub struct RemoveResource(Vec<fn(&mut World)>);

impl SpawnEntity {
    pub fn push(&mut self, entity: Entity) {
        self.0.push(entity);
    }
}

impl GetOrSpawnEntity {
    pub fn push(&mut self, entity: Entity) {
        self.0.push(entity);
    }
}

impl SpawnBundle {
    pub fn push<T: Bundle>(&mut self, bundle: T) {

        //        self.0.push(|world: &mut World| world.insert);
    }
}

pub enum WorldCommand {
    Spawn(Entity),
    GetOrSpawn(Entity),
    Dyn(Box<dyn DynWorldCommand>),
    // goes straight into scheduler
    //    RunSystem(Arc<SystemContainer>),
}

pub trait DynWorldCommand {
    fn apply(self, world: &mut World);
}

impl<F> DynWorldCommand for F
where
    F: FnOnce(&mut World) + Send + Sync + 'static,
{
    fn apply(self, world: &mut World) {
        self(world);
    }
}

pub struct RemoveSystem(SystemId);
pub struct AddSystem(SystemId, SystemContainer);
