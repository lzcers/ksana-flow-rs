use crate::flow::reactive_stream::StreamSubscriptionFn;
use std::any::Any;

pub trait SendableAny: Any + Send {
    fn clone_box(&self) -> Box<dyn SendableAny>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn type_name(&self) -> &'static str;
    fn is_stream(&self) -> bool {
        false
    }
    fn into_stream_subscriber(
        self: Box<Self>,
    ) -> Result<StreamSubscriptionFn, Box<dyn SendableAny>>;
}

// 为实现了 Clone 的 SendableAny 提供默认实现
// 所有节点的输出应该是 SendableAny + Clone 的
impl<T: Any + Send + Clone> SendableAny for T {
    fn clone_box(&self) -> Box<dyn SendableAny> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
    fn into_stream_subscriber(
        self: Box<Self>,
    ) -> Result<StreamSubscriptionFn, Box<dyn SendableAny>> {
        Err(self)
    }
}

impl Clone for Box<dyn SendableAny> {
    fn clone(&self) -> Self {
        self.as_ref().clone_box()
    }
}
