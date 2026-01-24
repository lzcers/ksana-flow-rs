use flow::{FlowEvent, NodeInputs, SendableAny};
use serde_json::{Map, Value, json};

pub fn try_downcast_to_value(any: &dyn SendableAny) -> Option<Value> {
    let erased = any.as_any();
    if let Some(v) = erased.downcast_ref::<String>() {
        return Some(Value::String(v.clone()));
    }
    if let Some(v) = erased.downcast_ref::<Value>() {
        return Some(v.clone());
    }
    if let Some(v) = erased.downcast_ref::<bool>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<i8>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<i16>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<i32>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<i64>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<isize>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<u8>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<u16>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<u32>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<u64>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<usize>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<f32>() {
        return Some(json!(v));
    }
    if let Some(v) = erased.downcast_ref::<f64>() {
        return Some(json!(v));
    }
    if let Some(()) = erased.downcast_ref::<()>() {
        return Some(json!(null));
    }
    if let Some(inner_box) = erased.downcast_ref::<Box<dyn SendableAny>>() {
        let inner: &dyn SendableAny = &**inner_box;
        if let Some(v) = try_downcast_to_value(inner) {
            return Some(v);
        }
    }

    None
}

pub fn sendable_any_to_value_lossy(any: &dyn SendableAny) -> Value {
    try_downcast_to_value(any).unwrap_or_else(|| {
        json!({
            "$unserializable": true,
            "type": any.type_name()
        })
    })
}

pub fn node_inputs_to_value_lossy(inputs: &NodeInputs) -> Value {
    let mut map = Map::new();
    for (node_id, any) in &inputs.inputs {
        let inner: &dyn SendableAny = &**any;
        map.insert(node_id.clone(), sendable_any_to_value_lossy(inner));
    }
    Value::Object(map)
}

pub fn flow_event_to_value_lossy(event: &FlowEvent) -> Value {
    match event {
        FlowEvent::NodeStarted(node_id) => json!({ "NodeStarted": node_id }),
        FlowEvent::NodeCompleted(node_id) => json!({ "NodeCompleted": node_id }),
        FlowEvent::NodeError(node_id, msg) => json!({ "NodeError": [node_id, msg] }),
        FlowEvent::NodeInMessage(node_id, inputs) => {
            json!({ "NodeInMessage": [node_id, node_inputs_to_value_lossy(inputs)] })
        }
        FlowEvent::NodeOutMessage(node_id, any) => {
            let inner: &dyn SendableAny = &**any;
            json!({ "NodeOutMessage": [node_id, sendable_any_to_value_lossy(inner)] })
        }
        FlowEvent::NodeStreamStarted(node_id) => json!({ "NodeStreamStarted": node_id }),
        FlowEvent::NodeStreamNextMessage(node_id, any) => {
            let inner: &dyn SendableAny = &**any;
            json!({ "NodeStreamNextMessage": [node_id, sendable_any_to_value_lossy(inner)] })
        }
        FlowEvent::FlowPaused => Value::String("FlowPaused".to_string()),
        FlowEvent::FlowResumed => Value::String("FlowResumed".to_string()),
        FlowEvent::FlowStopped => Value::String("FlowStopped".to_string()),
        FlowEvent::FlowFinished => Value::String("FlowFinished".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct NotJson {
        _x: i32,
    }

    #[test]
    fn downcast_value_supports_recursive_box() {
        let inner: Box<dyn SendableAny> = Box::new("hello".to_string());
        let wrapped: Box<dyn SendableAny> = Box::new(inner);
        let v = sendable_any_to_value_lossy(&*wrapped);
        assert_eq!(v, Value::String("hello".to_string()));
    }

    #[test]
    fn downcast_value_has_fallback() {
        let v = sendable_any_to_value_lossy(&NotJson { _x: 1 });
        assert!(v.is_object());
    }

    #[test]
    fn flow_event_serializes_to_frontend_shape() {
        let e = FlowEvent::NodeOutMessage("n1".to_string(), Box::new(123i32));
        let v = flow_event_to_value_lossy(&e);
        assert_eq!(v["NodeOutMessage"][0], Value::String("n1".to_string()));
        assert_eq!(v["NodeOutMessage"][1], json!(123));
    }
}
