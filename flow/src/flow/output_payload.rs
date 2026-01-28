use std::{any::Any, sync::Arc};

use crate::StreamSubscriptionFn;
use core::fmt;

pub trait SharedAny: Any + Send + Sync {}
impl<T: Any + Send + Sync> SharedAny for T {}

pub trait CloneAny: Any + Send {
    fn clone_box(&self) -> Box<dyn CloneAny>;
    fn as_any(&self) -> &dyn Any;
    fn type_name(&self) -> &'static str;
}

impl<T: Any + Send + Clone> CloneAny for T {
    fn clone_box(&self) -> Box<dyn CloneAny> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

pub enum OutputPayload {
    Shared {
        value: Arc<dyn SharedAny>,
        type_name: &'static str,
    },
    Cloned(Box<dyn CloneAny>),
    Stream(StreamSubscriptionFn),
}

impl OutputPayload {
    pub fn shared<T: Any + Send + Sync>(value: T) -> Self {
        Self::Shared {
            value: Arc::new(value),
            type_name: std::any::type_name::<T>(),
        }
    }

    pub fn cloned<T: Any + Send + Clone>(value: T) -> Self {
        Self::Cloned(Box::new(value))
    }

    pub fn stream(subscribe: StreamSubscriptionFn) -> Self {
        Self::Stream(subscribe)
    }

    pub fn as_any(&self) -> Option<&dyn Any> {
        match self {
            Self::Shared { value, .. } => Some(value.as_ref()),
            Self::Cloned(v) => Some(v.as_any()),
            Self::Stream(_) => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Shared { type_name, .. } => type_name,
            Self::Cloned(v) => v.type_name(),
            Self::Stream(_) => "Stream",
        }
    }
}

impl Clone for OutputPayload {
    fn clone(&self) -> Self {
        match self {
            Self::Shared { value, type_name } => Self::Shared {
                value: value.clone(),
                type_name,
            },
            Self::Cloned(v) => Self::Cloned(v.clone_box()),
            Self::Stream(_) => {
                panic!("Stream OutputPayload cannot be cloned.");
            }
        }
    }
}

impl fmt::Debug for OutputPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shared { type_name, .. } => f.debug_tuple("Shared").field(type_name).finish(),
            Self::Cloned(v) => f.debug_tuple("Cloned").field(&v.type_name()).finish(),
            Self::Stream(_) => f.debug_tuple("Stream").finish(),
        }
    }
}
