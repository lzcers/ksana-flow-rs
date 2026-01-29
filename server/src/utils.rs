use flow::{FlowEvent, Input};
use serde_json::{Map, Value, json};

pub fn node_inputs_to_value_lossy(inputs: &Input) -> Value {
    let mut map = Map::new();
    for (node_id, payload) in inputs.get_values() {
        map.insert(node_id.clone(), payload.clone());
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
        FlowEvent::NodeOutMessage(node_id, payload) => {
            json!({ "NodeOutMessage": [node_id, payload] })
        }
        FlowEvent::NodeStreamStarted(node_id) => json!({ "NodeStreamStarted": node_id }),
        FlowEvent::NodeStreamNextMessage(node_id, payload) => {
            json!({ "NodeStreamNextMessage": [node_id, payload] })
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

    #[test]
    fn flow_event_serializes_to_frontend_shape() {
        let e = FlowEvent::NodeOutMessage("n1".to_string(), json!(123));
        let v = flow_event_to_value_lossy(&e);
        assert_eq!(v["NodeOutMessage"][0], Value::String("n1".to_string()));
        assert_eq!(v["NodeOutMessage"][1], json!(123));
    }
}
