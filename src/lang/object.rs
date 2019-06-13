// Dependencies

use std::any::Any;
use super::typename::Typename;

// Structs

pub struct Object {
    typename: String,
    object: Box<dyn Any>
}

// Implementations

impl Object {
    pub fn new<Type: Any + Typename>(object: Type) -> Self {
        let object = Box::new(object);
        let object = Box::<dyn Any>::from(object);
        Object{typename: Type::typename(), object}
    }

    pub fn typename(&self) -> &String {
        &self.typename
    }

    pub fn is<Type: Any + Typename>(&self) -> bool {
        self.object.is::<Type>()
    }

    pub fn downcast_ref<Type: Any + Typename>(&self) -> Option<&Type> {
        self.object.downcast_ref::<Type>()
    }

    pub fn downcast_mut<Type: Any + Typename>(&mut self) -> Option<&mut Type> {
        self.object.downcast_mut::<Type>()
    }
}
