use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde_json::Value;

/// ReduceNode - 批量聚合节点
///
/// 对输入数组进行批量聚合操作，支持多种内置聚合器
///
/// # 示例
/// ```
/// use nodes::reduce_node::ReduceNode;
/// let reduce_node = ReduceNode::sum();
/// // 输入: [1, 2, 3, 4, 5]
/// // 输出: 15
/// ```
pub struct ReduceNode {
    /// 初始值
    initial: Value,
    /// 聚合函数
    reducer: Reducer,
}

/// 聚合函数类型
pub enum Reducer {
    /// 求和（数值）
    Sum,
    /// 字符串连接
    Concat { separator: String },
    /// 对象/数组合并
    Merge { deep: bool },
    /// 计数
    Count,
    /// 最大值
    Max,
    /// 最小值
    Min,
    /// 自定义聚合函数
    Custom(Box<dyn Fn(&Value, &Value) -> Value + Send + Sync>),
}

impl std::fmt::Debug for Reducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reducer::Sum => write!(f, "Sum"),
            Reducer::Concat { separator } => write!(f, "Concat(sep: {:?})", separator),
            Reducer::Merge { deep } => write!(f, "Merge(deep: {})", deep),
            Reducer::Count => write!(f, "Count"),
            Reducer::Max => write!(f, "Max"),
            Reducer::Min => write!(f, "Min"),
            Reducer::Custom(_) => write!(f, "Custom"),
        }
    }
}

impl ReduceNode {
    /// 创建自定义 ReduceNode
    pub fn new(initial: Value, reducer: Reducer) -> Self {
        Self { initial, reducer }
    }

    /// 创建求和聚合器
    pub fn sum() -> Self {
        Self::new(Value::Number(0.into()), Reducer::Sum)
    }

    /// 创建字符串连接聚合器
    pub fn concat(separator: impl Into<String>) -> Self {
        Self::new(
            Value::String("".to_string()),
            Reducer::Concat {
                separator: separator.into(),
            },
        )
    }

    /// 创建对象/数组合并聚合器
    pub fn merge(deep: bool) -> Self {
        Self::new(
            if deep {
                Value::Object(serde_json::Map::new())
            } else {
                Value::Array(vec![])
            },
            Reducer::Merge { deep },
        )
    }

    /// 创建计数聚合器
    pub fn count() -> Self {
        Self::new(Value::Number(0.into()), Reducer::Count)
    }

    /// 创建最大值聚合器
    pub fn max() -> Self {
        Self::new(Value::Null, Reducer::Max)
    }

    /// 创建最小值聚合器
    pub fn min() -> Self {
        Self::new(Value::Null, Reducer::Min)
    }

    /// 创建自定义聚合器
    pub fn custom<F>(initial: Value, f: F) -> Self
    where
        F: Fn(&Value, &Value) -> Value + Send + Sync + 'static,
    {
        Self::new(initial, Reducer::Custom(Box::new(f)))
    }

    /// 应用聚合函数
    fn apply_reducer(&self, acc: &Value, item: &Value) -> Result<Value, String> {
        match &self.reducer {
            Reducer::Sum => {
                match (acc.as_f64(), item.as_f64()) {
                    (Some(a), Some(b)) => {
                        // 使用 from_f64 处理浮点数
                        match serde_json::Number::from_f64(a + b) {
                            Some(n) => Ok(Value::Number(n)),
                            None => Err("Invalid numeric result".to_string()),
                        }
                    }
                    _ => Err("Sum requires numeric values".to_string()),
                }
            }
            Reducer::Concat { separator } => {
                let a = acc.as_str().unwrap_or("");
                let b = item.as_str().unwrap_or("");
                if a.is_empty() {
                    Ok(Value::String(b.to_string()))
                } else {
                    Ok(Value::String(format!("{}{}{}", a, separator, b)))
                }
            }
            Reducer::Merge { deep } => {
                let mut result = acc.clone();
                match (deep, &mut result, item) {
                    (true, Value::Object(acc_map), Value::Object(item_map)) => {
                        // 深度合并
                        for (key, value) in item_map {
                            if acc_map.contains_key(key) {
                                let existing = acc_map.get(key).unwrap().clone();
                                let merged = self.deep_merge(&existing, value)?;
                                acc_map.insert(key.clone(), merged);
                            } else {
                                acc_map.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    (_, Value::Array(acc_arr), Value::Array(item_arr)) => {
                        // 数组合并
                        acc_arr.extend(item_arr.clone());
                    }
                    (_, Value::Object(acc_map), Value::Object(item_map)) => {
                        // 浅层合并
                        for (key, value) in item_map {
                            acc_map.insert(key.clone(), value.clone());
                        }
                    }
                    _ => {
                        return Err("Merge requires objects or arrays".to_string());
                    }
                }
                Ok(result)
            }
            Reducer::Count => {
                let current = acc.as_u64().unwrap_or(0);
                Ok(Value::Number((current + 1).into()))
            }
            Reducer::Max => {
                match (acc.as_f64(), item.as_f64()) {
                    (Some(a), Some(b)) if !a.is_nan() && !b.is_nan() => {
                        if acc.is_null() || b > a {
                            Ok(item.clone())
                        } else {
                            Ok(acc.clone())
                        }
                    }
                    (None, Some(_)) => Ok(item.clone()),
                    (Some(_), None) => Ok(acc.clone()),
                    _ => Ok(Value::Null),
                }
            }
            Reducer::Min => {
                match (acc.as_f64(), item.as_f64()) {
                    (Some(a), Some(b)) if !a.is_nan() && !b.is_nan() => {
                        if acc.is_null() || b < a {
                            Ok(item.clone())
                        } else {
                            Ok(acc.clone())
                        }
                    }
                    (None, Some(_)) => Ok(item.clone()),
                    (Some(_), None) => Ok(acc.clone()),
                    _ => Ok(Value::Null),
                }
            }
            Reducer::Custom(f) => Ok(f(acc, item)),
        }
    }

    /// 深度合并两个值
    fn deep_merge(
        &self,
        a: &Value,
        b: &Value,
    ) -> Result<Value, String> {
        match (a, b) {
            (Value::Object(a_map), Value::Object(b_map)) => {
                let mut result = a_map.clone();
                for (key, value) in b_map {
                    if result.contains_key(key) {
                        let existing = result.get(key).unwrap().clone();
                        let merged = self.deep_merge(&existing, value)?;
                        result.insert(key.clone(), merged);
                    } else {
                        result.insert(key.clone(), value.clone());
                    }
                }
                Ok(Value::Object(result))
            }
            (Value::Array(a_arr), Value::Array(b_arr)) => {
                let mut result = a_arr.clone();
                result.extend(b_arr.clone());
                Ok(Value::Array(result))
            }
            // 对于其他类型，b 覆盖 a
            (_, b) => Ok(b.clone()),
        }
    }
}

#[async_trait]
impl Node for ReduceNode {
    const TRIGGER_STRATEGY: flow::TriggerStrategy = flow::TriggerStrategy::AllUpstreamReady;

    async fn run(
        &mut self,
        _ctx: &Context,
        input: &Input,
    ) -> Result<Output, String> {
        // 从输入中提取数组
        let items: Vec<Value> = match input.get_any_as::<Vec<Value>>() {
            Some(items) => items,
            None => {
                // 尝试获取单个值作为单元素数组
                match input.get_any() {
                    Some(value) => vec![value.clone()],
                    None => return Err("Reduce requires array input".to_string()),
                }
            }
        };

        // 如果输入为空，返回初始值
        if items.is_empty() {
            return Ok(self.initial.clone().into());
        }

        // 批量聚合
        let mut accumulator = self.initial.clone();
        for item in items {
            accumulator = self.apply_reducer(&accumulator, &item)?;
        }

        Ok(accumulator.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reduce_node_sum() {
        let reduce = ReduceNode::sum();
        assert_eq!(reduce.initial, Value::Number(0.into()));
    }

    #[test]
    fn test_reduce_node_concat() {
        let reduce = ReduceNode::concat(", ");
        assert_eq!(reduce.initial, Value::String("".to_string()));
    }

    #[test]
    fn test_reduce_node_count() {
        let reduce = ReduceNode::count();
        assert_eq!(reduce.initial, Value::Number(0.into()));
    }

    #[test]
    fn test_reduce_node_custom() {
        let reduce = ReduceNode::custom(
            Value::Number(1.into()),
            |acc, item| {
                match (acc.as_i64(), item.as_i64()) {
                    (Some(a), Some(b)) => Value::Number((a * b).into()),
                    _ => Value::Null,
                }
            }
        );
        assert_eq!(reduce.initial, Value::Number(1.into()));
    }
}
