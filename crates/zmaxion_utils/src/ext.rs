use std::any::TypeId;

pub trait TypeName {
    fn type_name() -> &'static str;
}

impl<T: ?Sized> TypeName for T {
    fn type_name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub trait HasTypeId {
    fn id() -> TypeId;
}

impl<T: ?Sized + 'static> HasTypeId for T {
    fn id() -> TypeId {
        std::any::TypeId::of::<Self>()
    }
}
