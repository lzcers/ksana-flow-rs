use bevy_ecs::resource::Resource;
use serde::{Deserialize, Serialize};

/// 世界 Agent 单轮完整输出
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorldSnapshot {
    pub encounter: Encounter,
    pub narration_metadata: NarrationMetadata,
    pub world_state: WorldState,
}

// ─── Encounter ────────────────────────────────────────

/// 呈现给主角和故事演绎 Agent 的当前情境
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Encounter {
    pub scene_title: String,
    pub time_context: String,
    pub location: EncounterLocation,
    /// 场景整体氛围与感官细节
    pub description: String,
    /// 刚刚发生或正在发生的核心事件
    pub current_event: String,
    /// 本段情境中主角可感知的新线索或信息
    pub new_info: Vec<String>,
    /// 情境中蕴藏的冲突、压力或两难
    pub inner_conflict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EncounterLocation {
    pub name: String,
    pub exits: Vec<String>,
    /// 该地点的当前特殊状态
    pub status: String,
}

// ─── Narration Metadata ───────────────────────────────

/// 传递给故事演绎 Agent 的写作约束
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NarrationMetadata {
    /// 必须出现在最终叙事中的关键信息（伏笔、情绪真相、逻辑事实）
    pub hard_anchors: Vec<String>,
    /// 风格、节奏、镜头等软性建议
    /// 如 "急促"、"舒缓"、"紧绷中带着一丝哀伤"
    pub pace: String,
    /// 如 "潮湿的阴冷感"、"古老金属的锈味与星光交织"
    pub atmosphere: String,
    /// 镜头聚焦建议，如 "聚焦主角被符文割破的指尖"
    pub focal_point: String,
}

// ─── World State ──────────────────────────────────────

/// 完整世界快照，供下一轮世界 Agent 读取
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorldState {
    pub round: u32,
    pub time: WorldTime,
    pub location: WorldLocation,
    pub protagonist: ProtagonistState,
    pub npcs: Vec<NpcState>,
    pub items_of_interest: Vec<ItemState>,
    pub events_in_progress: Vec<OngoingEvent>,
    pub unsolved_plot_threads: Vec<String>,
    /// 当前叙事节奏的简评与下一轮建议
    pub pacing_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorldTime {
    /// 故事内绝对时间，如 "第三日 凌晨两点一刻"
    pub absolute: String,
    /// 关键时间压力的描述，如 "距离穹顶自毁仅剩一炷香"
    pub relative: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorldLocation {
    pub current: String,
    pub exits: Vec<String>,
    /// 此处此刻的特殊状态与环境变化
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProtagonistState {
    pub name: String,
    /// 身心状态的文学化描述，例如 "灵能过度消耗，面色苍白，但精神高度集中"
    pub condition: String,
    /// 主角已确切知晓的剧情秘密
    pub known_secrets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NpcState {
    pub id: String,
    pub name: String,
    pub location: String,
    /// 当前情绪与心理状态的描述
    pub mood: String,
    /// 对主角的态度，使用自然语言，如 "既信赖又隐隐藏着愧疚"
    pub attitude: String,
    /// 此刻最直接的意图或行动倾向
    pub current_goal: String,
    /// 该 NPC 保守的秘密或关键信息
    pub secrets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ItemState {
    pub id: String,
    pub name: String,
    /// 物品当前所在地或持有者
    pub location: String,
    /// 状态或可用性，如 "已被激活，光芒渐弱"
    pub status: String,
    /// 被主角察觉的程度，如 "刚瞥见"、"未察觉"
    pub awareness: String,
    /// 此物与主线伏笔的关联
    pub plot_relevance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OngoingEvent {
    pub id: String,
    pub name: String,
    /// 当前态势的叙事描述，如 "闸门已被撞开一半，脚步声近在咫尺"
    pub status_description: String,
    /// 可能导致事态升级的触发条件
    pub escalation_trigger: String,
}

use std::fmt::Write;

impl WorldSnapshot {
    /// 将世界状态快照转换为可追加的流水账文本。
    /// 仅使用 `world_state` 字段；`encounter` 与 `narration_metadata` 分别供给其他 Agent 使用。
    pub fn to_ledger(&self) -> String {
        let ws = &self.world_state;
        let mut out = String::new();

        // ── 标题 ──
        writeln!(out, "【世界状态｜第{}轮】", ws.round).unwrap();

        // ── 时间 ──
        write!(out, "时间：{}", ws.time.absolute).unwrap();
        if let Some(ref rel) = ws.time.relative {
            write!(out, "，{}", rel).unwrap();
        }
        writeln!(out).unwrap();

        // ── 地点 ──
        write!(out, "地点：{}。", ws.location.current).unwrap();
        if !ws.location.exits.is_empty() {
            write!(out, "出口有").unwrap();
            let exits_desc: Vec<String> = ws.location.exits.iter().map(|e| e.clone()).collect();
            write!(out, "{}。", exits_desc.join("、")).unwrap();
        }
        writeln!(out).unwrap();
        writeln!(out, "环境：{}", ws.location.status).unwrap();

        // ── 主角 ──
        writeln!(out, "主角：{}。", ws.protagonist.name).unwrap();
        writeln!(out, "状态：{}", ws.protagonist.condition).unwrap();
        if !ws.protagonist.known_secrets.is_empty() {
            writeln!(out, "已知秘密：{}", ws.protagonist.known_secrets.join("；")).unwrap();
        }

        // ── NPC ──
        if !ws.npcs.is_empty() {
            writeln!(out, "NPC：").unwrap();
            for (i, npc) in ws.npcs.iter().enumerate() {
                write!(out, "{}. {}（位置：{}）", i + 1, npc.name, npc.location).unwrap();
                write!(out, "——情绪：{}", npc.mood).unwrap();
                write!(out, " 态度：{}", npc.attitude).unwrap();
                write!(out, " 当前意图：{}", npc.current_goal).unwrap();
                if !npc.secrets.is_empty() {
                    write!(out, " 秘密：{}", npc.secrets.join("；")).unwrap();
                }
                writeln!(out).unwrap();
            }
        }

        // ── 关键物品 ──
        if !ws.items_of_interest.is_empty() {
            writeln!(out, "关键物品：").unwrap();
            for item in &ws.items_of_interest {
                writeln!(
                    out,
                    "- {}（{}，状态：{}，主角察觉：{}，剧情关联：{}）",
                    item.name, item.location, item.status, item.awareness, item.plot_relevance
                )
                .unwrap();
            }
        }

        // ── 进行中的事件 ──
        if !ws.events_in_progress.is_empty() {
            writeln!(out, "进行中的事件：").unwrap();
            for (i, ev) in ws.events_in_progress.iter().enumerate() {
                writeln!(
                    out,
                    "{}. {}：{}，触发升级条件：{}",
                    i + 1,
                    ev.name,
                    ev.status_description,
                    ev.escalation_trigger
                )
                .unwrap();
            }
        }

        // ── 未解伏笔 ──
        if !ws.unsolved_plot_threads.is_empty() {
            writeln!(out, "未解伏笔：").unwrap();
            for thread in &ws.unsolved_plot_threads {
                writeln!(out, "- {}", thread).unwrap();
            }
        }

        // ── 叙事节奏 ──
        writeln!(out, "叙事节奏：{}", ws.pacing_note).unwrap();

        out
    }

    /// 生成给故事 Agent 的可读创作提示。
    /// `protagonist_action` 是主角上一轮的实际行动，可能为空。
    pub fn to_story_prompt(&self, protagonist_action: Option<&str>) -> String {
        let enc = &self.encounter;
        let meta = &self.narration_metadata;
        let mut out = String::new();

        // ── 基础情境 ──
        writeln!(out, "【本轮创作任务】").unwrap();
        writeln!(out, "场景：{}", enc.scene_title).unwrap();
        writeln!(out, "时间：{}", enc.time_context).unwrap();
        writeln!(out, "地点：{}", enc.location.name).unwrap();
        if !enc.location.exits.is_empty() {
            writeln!(out, "可用出口：{}", enc.location.exits.join("、")).unwrap();
        }
        writeln!(out, "地点状态：{}", enc.location.status).unwrap();
        writeln!(out, "环境细节：{}", enc.description).unwrap();

        // ── 主角行动 ──
        if !protagonist_action.is_some() {
            writeln!(out, "主角刚刚的行动：{}", protagonist_action.unwrap()).unwrap();
        }

        // ── 当前事件 ──
        writeln!(out, "正在发生的事情：{}", enc.current_event).unwrap();

        // ── 新信息与线索 ──
        if !enc.new_info.is_empty() {
            writeln!(out, "主角注意到的新线索：").unwrap();
            for info in &enc.new_info {
                writeln!(out, "- {}", info).unwrap();
            }
        }

        // ── 内在冲突 ──
        writeln!(out, "当前困境与内心冲突：{}", enc.inner_conflict).unwrap();

        // ── 硬性写作要求（必须体现） ──
        if !meta.hard_anchors.is_empty() {
            writeln!(out, "硬性写作要求（必须在故事中明确呈现）：").unwrap();
            for anchor in &meta.hard_anchors {
                writeln!(out, "- {}", anchor).unwrap();
            }
        }

        // ── 风格建议 ──
        writeln!(
            out,
            "风格参考：节奏——{}，氛围——{}",
            meta.pace, meta.atmosphere
        )
        .unwrap();
        if !meta.focal_point.is_empty() {
            writeln!(out, "镜头建议：{}", meta.focal_point).unwrap();
        }

        writeln!(out, "请根据以上信息续写故事，保持与前面段落的连贯性。").unwrap();

        out
    }

    /// 生成给主角 Agent 的决策提示文本。
    pub fn to_protagonist_prompt(&self) -> String {
        let enc = &self.encounter;
        let ps = &self.world_state.protagonist;
        let mut out = String::new();

        writeln!(out, "场景：{}", enc.scene_title).unwrap();
        writeln!(out, "时间：{}", enc.time_context).unwrap();
        writeln!(out, "地点：{}", enc.location.name).unwrap();
        if !enc.location.exits.is_empty() {
            writeln!(out, "可用出口：{}", enc.location.exits.join("、")).unwrap();
        }
        writeln!(out, "地点状态：{}", enc.location.status).unwrap();
        writeln!(out, "环境细节：{}", enc.description).unwrap();
        writeln!(out, "正在发生的事情：{}", enc.current_event).unwrap();

        if !enc.new_info.is_empty() {
            writeln!(out, "新线索：").unwrap();
            for info in &enc.new_info {
                writeln!(out, "- {}", info).unwrap();
            }
        }

        writeln!(out, "当前困境与内心冲突：{}", enc.inner_conflict).unwrap();
        writeln!(out, "主角身心状态：{}", ps.condition).unwrap();

        if !ps.known_secrets.is_empty() {
            writeln!(out, "主角已知秘密：{}", ps.known_secrets.join("；")).unwrap();
        }

        out
    }
}
