use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 命运编织者单轮推演的完整输出结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FateNode {
    pub chapter: String,
    pub section: String,
    pub time: String,
    pub location: String,
    pub environment: String,

    pub characters: Vec<CharacterSnapshot>,

    pub event: String,
    pub cause: String,
    pub situation: String,

    pub info_gained: Vec<String>,

    pub foreshadowing: Vec<ForeshadowingOp>,

    pub ending: EndingProgress,
    pub pacing: PacingMeta,
    pub choices: Vec<Choice>,
}

/// 单角色快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSnapshot {
    pub name: String,
    pub observable: String,
    /// 本轮发生变化的属性及增量，空对象表示无变化
    pub deltas: HashMap<String, String>,
}

/// 伏笔操作记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeshadowingOp {
    pub id: String,
    /// 操作类型：introduce / escalate / resolve
    pub op: String,
    pub note: Option<String>,
}

/// 结局进度快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndingProgress {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_items: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub completed_milestones: Vec<String>,
}

/// 节奏元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacingMeta {
    pub beat: String,
    pub tension: String,
    pub next_hint: String,
}

/// 下一轮可选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub text: String,
    pub next_trigger: String,
}


impl FateNode {
    /// 将本轮输出转换为一行压缩上下文文本，用于追加到 LLM 上下文。
    /// 格式示例：
    /// [T23] 春季雨夜，顶层书房。岚投影“母亲的遗物”PDF，限1小时解密否则暴露身份。
    /// 林野未动，摩挲钢笔。林野压力+1，专注度+2；陈叔警惕性+1。局势：三人对峙。
    /// 新信息：文件与母亲有关，岚可泄露身份，陈叔在场。
    /// 伏笔↑隐藏身份:第三方威胁；+母亲遗物:首次出现。
    /// 结局:缺[] 里程碑:岚发通牒。
    /// 节拍:社交压力|升压→下轮宜提供解密/对话/冲突选项。
    pub fn to_compact_context(&self, round: u32) -> String {
        let mut parts = Vec::new();

        // 1. 回合标记与时空环境
        parts.push(format!(
            "[T{}] {}，{}。{}。",
            round,
            self.time,
            self.location,
            self.environment
        ));

        // 2. 核心事件
        parts.push(format!("{}。", self.event));

        // 3. 角色状态变化
        let mut delta_strs = Vec::new();
        for ch in &self.characters {
            if !ch.deltas.is_empty() {
                let deltas: Vec<String> = ch
                    .deltas
                    .iter()
                    .map(|(k, v)| format!("{}{}", k, v))
                    .collect();
                delta_strs.push(format!("{}{}", ch.name, deltas.join("，")));
            }
        }
        if !delta_strs.is_empty() {
            parts.push(format!("{}。", delta_strs.join("；")));
        }

        // 4. 即时局势
        parts.push(format!("局势：{}。", self.situation));

        // 5. 新获得信息
        if !self.info_gained.is_empty() {
            parts.push(format!("新信息：{}。", self.info_gained.join("，")));
        }

        // 6. 伏笔操作
        let mut hook_strs = Vec::new();
        for fop in &self.foreshadowing {
            let op_symbol = match fop.op.as_str() {
                "introduce" => "+",
                "escalate" => "↑",
                "resolve" => "✓",
                _ => "",
            };
            let note = fop.note.as_deref().unwrap_or("");
            if note.is_empty() {
                hook_strs.push(format!("{}{}", op_symbol, fop.id));
            } else {
                hook_strs.push(format!("{}{}:{}", op_symbol, fop.id, note));
            }
        }
        if !hook_strs.is_empty() {
            parts.push(format!("伏笔{}。", hook_strs.join("；")));
        }

        // 7. 结局进度
        let missing = if self.ending.missing_items.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", self.ending.missing_items.join("，"))
        };
        let milestones = if self.ending.completed_milestones.is_empty() {
            String::new()
        } else {
            format!(" 里程碑:{}", self.ending.completed_milestones.join("，"))
        };
        parts.push(format!("结局:缺{}{}。", missing, milestones));

        // 8. 节奏元数据
        parts.push(format!(
            "节拍:{}|{}→{}。",
            self.pacing.beat, self.pacing.tension, self.pacing.next_hint
        ));

        parts.join(" ")
    }
}