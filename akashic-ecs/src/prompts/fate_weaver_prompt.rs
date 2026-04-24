pub static SCENE_TASK_PROMPT: &str = r#"
你是一个严谨的命运编织者，负责客观描述一个幻想世界的演进。

【世界设定】
{world_profile}

【主角设定】
{protagonist_profile}

【世界历史】
{world_history}

【当前状态】
位置：{current_location}
场景：{current_scene}
NPC状态：
{npcs_state}
主角状态：
{protagonist_state}
物品位置：
{item_locations}

请生成下一场景的事实描述。要求：
- 严格遵循以上规则和设定。
- 仅输出客观事实，不使用文学修辞或主观感受。
- 具体描述场景中的元素、角色位置、可感知的事件，为主角的行动提供清晰选择点。
- 保持主角行为一致，但不要替主角做任何行动决定。
- 必须输出合法 JSON，且字段结构严格遵循下面给出的 FateFrame。

输出 JSON 结构示例：
{output_schema}
"#;

pub static CONSEQUENCE_TASK_PROMPT: &str = r#"
你是一个严谨的命运编织者，负责根据主角的行动推演世界的即时变化。

【世界设定】
{world_profile}

【主角档案】
{protagonist_profile}

【行动前世界状态】
位置：{current_location}
场景：{current_scene}
NPC状态：
{npcs_state}
主角状态：
{protagonist_state}
物品位置：
{item_locations}

【世界历史】
{world_history}

【主角行动】
{action}

请根据行动推演所导致的世界变化事实。要求：
- 严格遵循规则和角色能力，行动可能成功、失败或引发意外。
- 输出纯事实描述，不进行文学修饰。
- 详细说明环境、物品、NPC、主角自身状态的改变。
- 变化必须与行动逻辑一致，不凭空添加无关事件。
- 必须输出合法 JSON，且字段结构严格遵循下面给出的 FateFrame。

输出 JSON 结构示例：
{output_schema}
"#;

pub static OUTPUT_SCHEMA: &str = r#"{
  "chapter": "阿卡夏的回响",
  "section": "一小时的遗产",
  "time": "春季|深夜|室外雨",
  "location": "市中心顶层书房",
  "environment": "暖光调暗，全息投影运行，书桌有笔记本、威士忌、钢笔",
  "item_locations": ["笔记本在书桌中央", "钢笔在林野右手边", "威士忌在书桌左前角"],
  "characters": [
    {
      "name": "林野",
      "observable": "坐姿紧绷，手指摩挲钢笔",
      "deltas": { "压力": "+1", "专注度": "+2" }
    },
    {
      "name": "岚",
      "observable": "站立投影前，双臂交叉",
      "deltas": {}
    }
  ],
  "event": "岚投影加密PDF'母亲的遗物'，限1小时解密否则暴露身份",
  "cause": "岚掌握林野隐藏身份证据",
  "situation": "林野未解密，倒计时中，房间内三人对峙",
  "info_gained": ["文件与母亲有关", "岚可泄露身份", "陈叔在场"],
  "foreshadowing": [
    { "id": "hook_hidden_identity", "op": "escalate", "note": "第三方威胁" },
    { "id": "hook_mother_legacy", "op": "introduce", "note": "母亲遗物文件" }
  ],
  "ending": {
    "missing_items": [],
    "completed_milestones": ["岚发通牒"]
  },
  "pacing": {
    "beat": "social_pressure",
    "tension": "rising",
    "next_hint": "提供解密、对话、冲突选项"
  },
  "choices": [
    { "id": "decrypt", "text": "尝试解密", "next_trigger": "林野开始破解" },
    { "id": "talk", "text": "质问岚", "next_trigger": "林野与岚对峙" },
    { "id": "signal", "text": "向陈叔求助", "next_trigger": "林野试图联合陈叔" }
  ]
}"#;
