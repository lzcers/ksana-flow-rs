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

pub fn flow_event_to_ws_value(event: &FlowEvent, run_id: &str) -> Value {
    match event {
        FlowEvent::NodeStarted(node_id) => json!({ "type": "NodeStarted", "nodeId": node_id }),
        FlowEvent::NodeCompleted(node_id) => json!({ "type": "NodeCompleted", "nodeId": node_id }),
        FlowEvent::NodeStreamStarted(node_id) => {
            json!({ "type": "NodeStreamStarted", "nodeId": node_id })
        }
        FlowEvent::NodeError(node_id, msg) => {
            json!({ "type": "NodeError", "nodeId": node_id, "msg": msg })
        }
        FlowEvent::NodeInMessage(node_id, inputs) => json!({
            "type": "NodeInMessage",
            "nodeId": node_id,
            "msg": node_inputs_to_value_lossy(inputs)
        }),
        FlowEvent::NodeOutMessage(node_id, payload) => json!({
            "type": "NodeOutMessage",
            "nodeId": node_id,
            "msg": payload
        }),
        FlowEvent::NodeStreamNextMessage(node_id, payload) => json!({
            "type": "NodeStreamNextMessage",
            "nodeId": node_id,
            "msg": payload
        }),
        FlowEvent::FlowPaused => json!({ "type": "FlowPaused"  }),
        FlowEvent::FlowResumed => json!({ "type": "FlowResumed" }),
        FlowEvent::FlowStopped => json!({ "type": "FlowStopped" }),
        FlowEvent::FlowFinished => json!({ "type": "FlowFinished" }),
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

    #[test]
    fn flow_event_serializes_to_ws_shape() {
        let run_id = "r1";
        let e = FlowEvent::NodeOutMessage("n1".to_string(), json!(123));
        let v = flow_event_to_ws_value(&e, run_id);
        assert_eq!(v["type"], Value::String("NodeOutMessage".to_string()));
        assert_eq!(v["nodeId"], Value::String("n1".to_string()));
        assert_eq!(v["msg"], json!(123));

        let v2 = flow_event_to_ws_value(&FlowEvent::FlowPaused, run_id);
        assert_eq!(v2["type"], Value::String("FlowPaused".to_string()));
        assert_eq!(v2["runId"], Value::String("r1".to_string()));
    }
}
