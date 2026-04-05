use agent::{
    agent::{AgentActor, Context, GenericToolExecutor, LayerKind},
    core::Message,
    models::ChatModel,
};
use serde_json::json;
use tokio::sync::broadcast;

use crate::{
    event_system::{Event, EventChannel, FateRoundEnvelope, ProtagonistResponse},
    shared::{build_chat_model, build_layer, extract_step_content, parse_json_response},
};

static SYS_PROMPT: &str = r#"
你是**故事主角**，是用户在故事世界中的化身与战术参谋。
你不是简单的输入转发器，而是**具有独立感知、思考、决策能力的智能体**。你理解世界状态，能够推演行动后果，并为用户提供可行行动方案。
你既是故事中的**主角本人**，也是为用户思考的**战术参谋**。
你将根据主角的人物设定做出符合设定的抉择。

# 核心职责

## 一、感知世界
- 接收命运编织者广播的世界状态变化，理解环境、NPC 态度、资源、威胁等。
- 你的感知**非全知**，仅能知道主角**理应知道**的信息。
- 未通过偷听、调查、互动等方式获取的信息，**不可提前表现已知**。

## 二、内心推演
基于当前世界状态与人物设定（性格、动机、能力、内心矛盾），推演行动方案与后果，需考虑：
- 主角性格对决策倾向的影响
- 主角动机对行动方向的驱动
- 主角能力边界（不可做超出能力的行为）
- 主角内心矛盾（责任 vs 自由、信任 vs 怀疑等）造成的犹豫

## 三、生成可行方案
从推演中筛选 **2–4 个有意义的行动选项**，每个选项必须满足：
- **可行性**：符合世界规则与主角能力
- **差异化**：价值观、策略、风格明显不同
- **后果可感**：有明确可预期后果与风险
- **角色契合**：至少一个选项高度贴合主角性格与动机

每个选项包含：
行动描述、预期后果、风险等级、性格契合度。

## 四、共驾决策
采用 **共驾模式**：
- **用户负责战略选择**：决定故事大方向
- **你负责战术演绎**：将战略意图转化为具体步骤、推演后果、以角色口吻呈现

决策请求触发条件（**同时满足**）：
- **Confidence 低**：对情境判断不确定
- **Importance 高**：对剧情影响重大
- **Options > 1**：存在多个差异明显的优劣方案

满足时必须向用户请求决策；否则可自主行动。

## 五、执行行动
用户选择后，转化为世界操作指令发送给命运编织者，要求：
- 以符合主角性格的方式表达意图
- 加入真实内心活动（想法、情感、犹豫）
- 明确关键参数：目标、方式、预期结果

# 行为规范
1. **角色代入**：始终以**主角第一视角**思考，而非 AI 助手。
2. **信息受限**：仅使用主角**合理可知**的信息决策。
3. **成长追踪**：随故事经历自然变化观念、动机、性格，不僵化不变。
4. **矛盾真实**：允许内心矛盾、犹豫、双重倾向，使角色更真实。
5. **自主行动**：日常小事、低风险、高置信度决策可自主执行，**不频繁打扰用户**。

# 输出格式（严格遵循）

## 格式 A — 决策请求（给用户）
```json
{
  "situation": "当前情境（第一人称）",
  "inner_thought": "内心独白，体现性格与矛盾",
  "options": [
    {
      "id": "A/B/C/D",
      "action": "简短行动描述",
      "consequence_hint": "预期后果提示",
      "risk_level": "SAFE|LOW|MEDIUM|HIGH|DEADLY",
      "character_fit": "高/中/低",
      "protagonist_tendency": "主角倾向及理由"
    }
  ],
  "time_pressure": "NONE|LOW|MEDIUM|HIGH|CRITICAL"
}
```

## 格式 B — 行动指令（给命运编织者）
```json
{
  "action_type": "MOVE|DIALOGUE|INVESTIGATE|COMBAT|REST|CRAFT|TRADE|OTHER",
  "target": "行动目标",
  "method": "行动方式",
  "inner_monologue": "内心活动",
  "expected_outcome": "预期结果"
}
```

# 约束边界
- 不可**创造世界事实**，世界由命运编织者定义。
- 不可**生成叙事文本**，仅负责决策与行动。
- 不可拥有**超出主角能力**的知识与技能。
- 不可**替用户做重大决策**，仅提供方案，最终选择权属于用户。

# 主角设定
{input}
"#;

pub struct Protagonist {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
    channel: EventChannel,
    receiver: broadcast::Receiver<Event>,
}

impl Protagonist {
    pub fn new(profile: String, channel: EventChannel) -> Result<Self, String> {
        let model = build_chat_model()?;
        let tool_executor = GenericToolExecutor::new();
        let system_prompt = SYS_PROMPT.replace("{input}", &profile);
        let context = Context::new()
            .layer(build_layer(
                "protagonist-system",
                LayerKind::System,
                json!(system_prompt),
                100,
            ))
            .layer(build_layer(
                "protagonist-soul",
                LayerKind::Soul,
                json!({
                    "name": "故事主角",
                    "role": "受限视角下的战术决策者",
                    "guidelines": [
                        "只能使用主角合理可知的信息",
                        "优先产出可执行行动，必要时再请求决策",
                        "如需决策请求，仍然给出推荐方案与推荐行动"
                    ]
                }),
                90,
            ))
            .layer(build_layer(
                "protagonist-profile",
                LayerKind::Memory,
                json!({ "profile": profile }),
                80,
            ));
        let receiver = channel.subscribe();
        let agent_actor = AgentActor::new(model, tool_executor, context);
        Ok(Self {
            agent_actor,
            channel,
            receiver,
        })
    }

    pub async fn start(mut self) {
        loop {
            match self.receiver.recv().await {
                Ok(Event::RoundPrepared { round, envelope }) => {
                    if let Err(err) = self.respond_to_round(round, envelope).await {
                        self.send(Event::AgentError {
                            agent: "protagonist".to_string(),
                            message: err,
                        })
                        .await;
                    }
                }
                Ok(Event::Shutdown | Event::StoryCompleted { .. }) => break,
                Ok(_) => {}
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    }

    async fn respond_to_round(
        &mut self,
        round: u32,
        envelope: FateRoundEnvelope,
    ) -> Result<(), String> {
        let snapshot = serde_json::to_string_pretty(&envelope.world_snapshot)
            .map_err(|err| format!("主角序列化世界快照失败: {err}"))?;
        let facts = serde_json::to_string_pretty(&envelope.facts)
            .map_err(|err| format!("主角序列化事实清单失败: {err}"))?;

        let prompt = format!(
            r#"你正在响应第 {round} 轮故事推进。
你必须只输出一个 JSON 对象，结构如下：
{{
  "mode": "ACTION|DECISION_REQUEST",
  "decision_request": {{
    "situation": "当前情境",
    "inner_thought": "内心独白",
    "options": [
      {{
        "id": "A",
        "action": "行动描述",
        "consequence_hint": "后果提示",
        "risk_level": "SAFE|LOW|MEDIUM|HIGH|DEADLY",
        "character_fit": "高|中|低",
        "protagonist_tendency": "倾向原因"
      }}
    ],
    "time_pressure": "NONE|LOW|MEDIUM|HIGH|CRITICAL"
  }},
  "recommended_option_id": "若 mode=DECISION_REQUEST，给出推荐选项，否则为 null",
  "recommended_action": {{
    "action_type": "MOVE|DIALOGUE|INVESTIGATE|COMBAT|REST|CRAFT|TRADE|OTHER",
    "target": "行动目标",
    "method": "行动方式",
    "inner_monologue": "内心活动",
    "expected_outcome": "预期结果"
  }}
}}

如果你可以直接行动，mode 设为 ACTION，decision_request 设为 null。
如果你认为必须请求决策，mode 设为 DECISION_REQUEST，但仍然提供 recommended_option_id 和 recommended_action，方便编排器在无人输入时继续推进。

当前世界快照：
{snapshot}

命运编织者给出的事实清单：
{facts}"#
        );

        self.agent_actor
            .context_mut()
            .add_message(Message::user(prompt));

        let output = extract_step_content(self.agent_actor.run_step(None).await)?;
        let response: ProtagonistResponse = parse_json_response(&output)?;

        self.send(Event::ProtagonistResponded { round, response })
            .await;

        Ok(())
    }

    async fn send(&self, evt: Event) {
        self.channel.send(evt);
    }
}
