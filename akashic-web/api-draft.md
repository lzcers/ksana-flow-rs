# 《幻世·人生回响》接口草案

## 1. 文档目标

本文档基于当前 `akashic-web` 前端页面规划与 `prd.md` 的交互时序，整理后端接口草案。

目标：

- 覆盖开局、叙事推进、结局归档、存档/读档、回廊、分享卡等核心能力
- 给出可联调的 `request/response JSON` 示例
- 区分 `P0` 核心闭环接口与 `P1` 增强玩法接口
- 为未来 `akashic-server + akashic-ecs` 对接预留统一数据结构

当前建议采用：

- `REST API`：用于创建、查询、存档、归档、分享等请求响应型操作
- `WebSocket`：用于流式叙事、世界新闻、NPC 介入、回合完成等实时推送

---

## 2. 基础约定

### 2.1 路径前缀

```text
/api
```

### 2.2 数据格式

- 请求与响应均使用 `application/json`
- 时间字段统一使用 `ISO 8601`
- ID 使用字符串，推荐 `uuid` 或可读前缀 ID

### 2.3 通用成功响应

```json
{
  "success": true,
  "data": {}
}
```

### 2.4 通用错误响应

```json
{
  "success": false,
  "error": {
    "code": "SESSION_NOT_FOUND",
    "message": "指定的游戏会话不存在"
  }
}
```

### 2.5 核心枚举建议

```json
{
  "phase": "FateWeaving",
  "gameState": "playing",
  "endingStatus": "none"
}
```

建议枚举：

- `gameState`: `lobby | creation | playing | ending | corridor`
- `phase`: `Idle | FateWeaving | NarratorScene | AwaitingProtagonist | FateConsequence | NarratorStory | Failed`
- `endingStatus`: `none | ready | archived`

---

## 3. 核心数据模型

### 3.1 Character

```json
{
  "name": "艾莉亚",
  "gender": "女",
  "age": 24,
  "appearance": "灰蓝眼瞳，佩着一枚旧怀表",
  "traits": {
    "courage": 60,
    "rationality": 70,
    "altruism": 45
  },
  "background": "失忆的流浪剑客"
}
```

### 3.2 World

```json
{
  "era": "蒸汽朋克",
  "coreConflict": "齿轮城的旧秩序正在崩塌，真相与忠诚互相撕扯",
  "specialRules": [
    "超自然力量存在",
    "死亡不是永久终点"
  ]
}
```

### 3.3 Choice

```json
{
  "id": "node-1-c1",
  "text": "推门进入酒馆，打听怀表的来历",
  "disabled": false,
  "costHints": {
    "intuition": 1,
    "obsession": 1
  }
}
```

### 3.4 StoryNode

```json
{
  "id": "node-1",
  "text": "雨水顺着屋檐滑落。你攥紧了怀中的黄铜怀表，那是唯一仍然忠于你的线索。",
  "image": "https://cdn.example.com/story/node-1-cover.jpg",
  "choices": [
    {
      "id": "node-1-c1",
      "text": "推门进入酒馆，打听怀表的来历",
      "disabled": false,
      "costHints": {
        "intuition": 1,
        "obsession": 1
      }
    },
    {
      "id": "node-1-c2",
      "text": "绕到后巷，从侧门潜入观察",
      "disabled": false,
      "costHints": {
        "intuition": 1,
        "obsession": 1
      }
    }
  ]
}
```

### 3.5 RuntimeStateView

该结构可直接参考 `akashic-ecs` 当前运行时状态视图。

```json
{
  "gameState": "playing",
  "phase": "AwaitingProtagonist",
  "turnIndex": 3,
  "activeTurnId": 3,
  "currentLocation": "铸铁之城 · 下环区",
  "currentScene": "昏黄灯光从酒馆门缝中渗出",
  "protagonistState": "警觉、疲惫，但仍保持克制",
  "npcsState": "酒馆老板对陌生来客保持戒备",
  "latestHistory": "你在雨夜来到下环区，决定调查怀表线索。",
  "latestBroadcastSummary": "局势仍未失控，但有人已经注意到你的到来。",
  "latestProtagonistAction": "推门进入酒馆，打听怀表的来历"
}
```

### 3.6 SessionResources

```json
{
  "obsessionPoints": 3,
  "intuitionPoints": 5,
  "daysLeft": 24,
  "worldNews": null
}
```

### 3.7 EndingData

```json
{
  "biography": "艾莉亚的《此生回响录》：在蒸汽朋克的时代，面对齿轮城的旧秩序正在崩塌，真相与忠诚互相撕扯的残酷现实，你走出了一条独特的道路。",
  "turningPoints": [
    {
      "cause": "面对未知的星辰",
      "effect": "选择毫不犹豫地踏上旅程"
    },
    {
      "cause": "遭遇遗迹守卫的考验",
      "effect": "以智慧化解了危机"
    },
    {
      "cause": "面临最终的抉择",
      "effect": "选择了守护真相，即便代价沉重"
    }
  ],
  "legacy": "你的选择为世界留下了一丝希望的火种。",
  "cgs": [
    "https://cdn.example.com/endings/ending-1-cg-1.jpg",
    "https://cdn.example.com/endings/ending-1-cg-2.jpg",
    "https://cdn.example.com/endings/ending-1-cg-3.jpg"
  ]
}
```

---

## 4. P0 核心接口

## 4.1 创建新局

`POST /api/game-sessions`

作用：

- 创建一局新的游戏会话
- 写入角色设定与世界观设定
- 初始化模拟层状态
- 返回首个剧情节点与当前会话快照

request:

```json
{
  "character": {
    "name": "艾莉亚",
    "gender": "女",
    "age": 24,
    "appearance": "灰蓝眼瞳，佩着一枚旧怀表",
    "traits": {
      "courage": 60,
      "rationality": 70,
      "altruism": 45
    },
    "background": "失忆的流浪剑客"
  },
  "world": {
    "era": "蒸汽朋克",
    "coreConflict": "齿轮城的旧秩序正在崩塌，真相与忠诚互相撕扯",
    "specialRules": [
      "超自然力量存在"
    ]
  },
  "seed": null
}
```

response:

```json
{
  "success": true,
  "data": {
    "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
    "createdAt": "2026-05-03T10:00:00Z",
    "character": {
      "name": "艾莉亚",
      "gender": "女",
      "age": 24,
      "appearance": "灰蓝眼瞳，佩着一枚旧怀表",
      "traits": {
        "courage": 60,
        "rationality": 70,
        "altruism": 45
      },
      "background": "失忆的流浪剑客"
    },
    "world": {
      "era": "蒸汽朋克",
      "coreConflict": "齿轮城的旧秩序正在崩塌，真相与忠诚互相撕扯",
      "specialRules": [
        "超自然力量存在"
      ]
    },
    "resources": {
      "obsessionPoints": 3,
      "intuitionPoints": 5,
      "daysLeft": 30,
      "worldNews": null
    },
    "currentNode": {
      "id": "node-1",
      "text": "雨水顺着屋檐滑落。你攥紧了怀中的黄铜怀表，那是唯一仍然忠于你的线索。",
      "image": "https://cdn.example.com/story/node-1-cover.jpg",
      "choices": [
        {
          "id": "node-1-c1",
          "text": "推门进入酒馆，打听怀表的来历",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        },
        {
          "id": "node-1-c2",
          "text": "绕到后巷，从侧门潜入观察",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        }
      ]
    },
    "stateView": {
      "gameState": "playing",
      "phase": "AwaitingProtagonist",
      "turnIndex": 1,
      "activeTurnId": 1,
      "currentLocation": "铸铁之城 · 下环区",
      "currentScene": "雨夜，酒馆门口，怀表线索浮现",
      "protagonistState": "初入局势，神经紧绷",
      "npcsState": "附近的陌生人对你保持观察",
      "latestHistory": "故事开始。",
      "latestBroadcastSummary": "命运尚未真正偏转。",
      "latestProtagonistAction": ""
    }
  }
}
```

## 4.2 获取当前会话快照

`GET /api/game-sessions/:sessionId`

作用：

- 用于页面刷新恢复
- 用于从回廊返回主舞台
- 用于轮询查询当前局面

response:

```json
{
  "success": true,
  "data": {
    "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
    "status": "active",
    "character": {
      "name": "艾莉亚",
      "gender": "女",
      "age": 24,
      "appearance": "灰蓝眼瞳，佩着一枚旧怀表",
      "traits": {
        "courage": 60,
        "rationality": 70,
        "altruism": 45
      },
      "background": "失忆的流浪剑客"
    },
    "world": {
      "era": "蒸汽朋克",
      "coreConflict": "齿轮城的旧秩序正在崩塌，真相与忠诚互相撕扯",
      "specialRules": [
        "超自然力量存在"
      ]
    },
    "resources": {
      "obsessionPoints": 2,
      "intuitionPoints": 4,
      "daysLeft": 24,
      "worldNews": "远方城邦燃起烽火，旧秩序摇摇欲坠。"
    },
    "currentNode": {
      "id": "node-3",
      "text": "你选择了进入酒馆。随着你的决定，命运的齿轮再次转动。",
      "image": "https://cdn.example.com/story/node-3-cover.jpg",
      "choices": [
        {
          "id": "node-3-c1",
          "text": "继续深入探索未知",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        },
        {
          "id": "node-3-c2",
          "text": "寻找盟友的帮助",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        }
      ]
    },
    "stateView": {
      "gameState": "playing",
      "phase": "AwaitingProtagonist",
      "turnIndex": 3,
      "activeTurnId": 3,
      "currentLocation": "铸铁之城 · 黑雾酒馆",
      "currentScene": "屋内的低语声正在蔓延",
      "protagonistState": "察觉到被跟踪后的警惕",
      "npcsState": "老板、信使和巡逻人都开始关注你",
      "latestHistory": "你进入酒馆并得到了第一条有效情报。",
      "latestBroadcastSummary": "隐藏势力开始试探你的底线。",
      "latestProtagonistAction": "推门进入酒馆，打听怀表的来历"
    },
    "endingStatus": "none"
  }
}
```

## 4.3 提交玩家选择

`POST /api/game-sessions/:sessionId/choices`

作用：

- 提交玩家意图 ID
- 扣减执念资源
- 驱动模拟层推进
- 创建下一回合的叙事任务

说明：

- 若系统采用 WebSocket 流式叙事，接口可先返回 `accepted`
- 若系统暂未接入流式，接口也可直接同步返回完整 `currentNode`

request:

```json
{
  "choiceId": "node-3-c2",
  "useObsession": true
}
```

response:

```json
{
  "success": true,
  "data": {
    "accepted": true,
    "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
    "turnId": 4,
    "resourceDelta": {
      "obsessionPoints": -1,
      "intuitionPoints": 0
    },
    "resources": {
      "obsessionPoints": 1,
      "intuitionPoints": 4,
      "daysLeft": 18,
      "worldNews": null
    },
    "stateView": {
      "gameState": "playing",
      "phase": "FateWeaving",
      "turnIndex": 4,
      "activeTurnId": 4,
      "currentLocation": "铸铁之城 · 黑雾酒馆",
      "currentScene": "你决定寻求盟友的帮助",
      "protagonistState": "执念被点燃，判断更激进",
      "npcsState": "潜在盟友开始对你产生兴趣",
      "latestHistory": "你选择向他人求助，并强行扭动命运。",
      "latestBroadcastSummary": "局势开始偏离温和解法。",
      "latestProtagonistAction": "寻找盟友的帮助"
    }
  }
}
```

可选同步版 response：

```json
{
  "success": true,
  "data": {
    "accepted": true,
    "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
    "turnId": 4,
    "resourceDelta": {
      "obsessionPoints": -1,
      "intuitionPoints": 0
    },
    "resources": {
      "obsessionPoints": 1,
      "intuitionPoints": 4,
      "daysLeft": 18,
      "worldNews": "市场物价剧烈波动，暗流涌动。"
    },
    "currentNode": {
      "id": "node-4",
      "text": "你选择了寻找盟友。随着你的决定，命运的齿轮再次转动。",
      "image": "https://cdn.example.com/story/node-4-cover.jpg",
      "choices": [
        {
          "id": "node-4-c1",
          "text": "相信来者并公开线索",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        },
        {
          "id": "node-4-c2",
          "text": "试探对方的真实立场",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        }
      ]
    },
    "endingStatus": "none"
  }
}
```

## 4.4 获取结局报告

`GET /api/game-sessions/:sessionId/ending`

作用：

- 结局页读取完整《此生回响录》
- 回廊查看某一局的结局详情

response:

```json
{
  "success": true,
  "data": {
    "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
    "endingStatus": "ready",
    "ending": {
      "biography": "艾莉亚的《此生回响录》：在蒸汽朋克的时代，面对齿轮城的旧秩序正在崩塌，真相与忠诚互相撕扯的残酷现实，你走出了一条独特的道路。",
      "turningPoints": [
        {
          "cause": "面对未知的星辰",
          "effect": "选择毫不犹豫地踏上旅程"
        },
        {
          "cause": "遭遇遗迹守卫的考验",
          "effect": "以智慧化解了危机"
        },
        {
          "cause": "面临最终的抉择",
          "effect": "选择了守护真相，即便代价沉重"
        }
      ],
      "legacy": "你的选择为世界留下了一丝希望的火种。",
      "cgs": [
        "https://cdn.example.com/endings/ending-1-cg-1.jpg",
        "https://cdn.example.com/endings/ending-1-cg-2.jpg",
        "https://cdn.example.com/endings/ending-1-cg-3.jpg"
      ]
    }
  }
}
```

## 4.5 创建存档

`POST /api/saves`

作用：

- 将当前会话保存为可继续的档位
- 用于主舞台的“存档”与“存档/分享”

request:

```json
{
  "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
  "title": "黑雾酒馆前夜",
  "autoGenerateShareCard": false
}
```

response:

```json
{
  "success": true,
  "data": {
    "saveId": "save_01JVB34HG4T3B2QY2S4B8J6D1N",
    "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
    "title": "黑雾酒馆前夜",
    "summary": "你刚进入黑雾酒馆，怀表线索开始显现，三方势力正在靠近。",
    "coverImage": "https://cdn.example.com/saves/save-1-cover.jpg",
    "turnIndex": 3,
    "savedAt": "2026-05-03T10:30:00Z"
  }
}
```

## 4.6 获取存档列表

`GET /api/saves`

作用：

- 回廊列出进行中的旅程
- 读档弹窗展示可恢复档位

response:

```json
{
  "success": true,
  "data": {
    "items": [
      {
        "saveId": "save_01JVB34HG4T3B2QY2S4B8J6D1N",
        "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
        "title": "黑雾酒馆前夜",
        "characterName": "艾莉亚",
        "background": "失忆的流浪剑客",
        "era": "蒸汽朋克",
        "turnIndex": 3,
        "summary": "你刚进入黑雾酒馆，怀表线索开始显现。",
        "coverImage": "https://cdn.example.com/saves/save-1-cover.jpg",
        "savedAt": "2026-05-03T10:30:00Z"
      }
    ]
  }
}
```

## 4.7 读档

`POST /api/saves/:saveId/load`

作用：

- 载入指定存档
- 返回恢复后的当前局面

response:

```json
{
  "success": true,
  "data": {
    "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
    "loadedFromSaveId": "save_01JVB34HG4T3B2QY2S4B8J6D1N",
    "status": "active",
    "resources": {
      "obsessionPoints": 2,
      "intuitionPoints": 4,
      "daysLeft": 24,
      "worldNews": null
    },
    "currentNode": {
      "id": "node-3",
      "text": "你选择了进入酒馆。随着你的决定，命运的齿轮再次转动。",
      "image": "https://cdn.example.com/story/node-3-cover.jpg",
      "choices": [
        {
          "id": "node-3-c1",
          "text": "继续深入探索未知",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        },
        {
          "id": "node-3-c2",
          "text": "寻找盟友的帮助",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        }
      ]
    },
    "stateView": {
      "gameState": "playing",
      "phase": "AwaitingProtagonist",
      "turnIndex": 3,
      "activeTurnId": 3,
      "currentLocation": "铸铁之城 · 黑雾酒馆",
      "currentScene": "屋内的低语声正在蔓延",
      "protagonistState": "察觉到被跟踪后的警惕",
      "npcsState": "老板、信使和巡逻人都开始关注你",
      "latestHistory": "你进入酒馆并得到了第一条有效情报。",
      "latestBroadcastSummary": "隐藏势力开始试探你的底线。",
      "latestProtagonistAction": "推门进入酒馆，打听怀表的来历"
    }
  }
}
```

---

## 5. P1 增强接口

## 5.1 命运窥镜

`POST /api/game-sessions/:sessionId/intuition-preview`

作用：

- 消耗 1 点直觉值
- 返回某个选项的模糊未来片段

request:

```json
{
  "choiceId": "node-3-c2"
}
```

response:

```json
{
  "success": true,
  "data": {
    "choiceId": "node-3-c2",
    "previewText": "未来的模糊片段在你眼前闪现：若选择这一条路，某个被遗忘的人会比你更早抵达真相。",
    "resourceDelta": {
      "intuitionPoints": -1
    },
    "resources": {
      "obsessionPoints": 2,
      "intuitionPoints": 3,
      "daysLeft": 24,
      "worldNews": null
    }
  }
}
```

## 5.2 历史记录

`GET /api/game-sessions/:sessionId/history`

作用：

- 用于“历史记录回看”
- 支持展示最近叙事文本与事件摘要

response:

```json
{
  "success": true,
  "data": {
    "items": [
      {
        "type": "narration",
        "turnIndex": 1,
        "text": "雨水顺着屋檐滑落。你攥紧了怀中的黄铜怀表。",
        "createdAt": "2026-05-03T10:00:03Z"
      },
      {
        "type": "player_choice",
        "turnIndex": 1,
        "text": "推门进入酒馆，打听怀表的来历",
        "createdAt": "2026-05-03T10:01:12Z"
      },
      {
        "type": "world_news",
        "turnIndex": 2,
        "text": "远方城邦燃起烽火，旧秩序摇摇欲坠。",
        "createdAt": "2026-05-03T10:02:01Z"
      }
    ]
  }
}
```

## 5.3 回廊归档列表

`GET /api/archives`

作用：

- 返回已完成结局的馆藏列表

response:

```json
{
  "success": true,
  "data": {
    "items": [
      {
        "archiveId": "arch_01JVB9E9J84D4Y2J3P8M5B7P1C",
        "title": "灰烬港的夜巡人",
        "tag": "已归档",
        "era": "末日废土",
        "summary": "你在塌陷的灯塔下守住最后的航标，让一座濒死聚落撑过风暴夜。",
        "coverImage": "https://cdn.example.com/archives/archive-1-cover.jpg",
        "createdAt": "2026-04-28T08:00:00Z"
      },
      {
        "archiveId": "arch_01JVB9FA6P4V5P9N2G1Q0T7B3X",
        "title": "最新回响",
        "tag": "本次命运",
        "era": "蒸汽朋克",
        "summary": "你的选择为世界留下了一丝希望的火种。",
        "coverImage": "https://cdn.example.com/endings/ending-1-cg-1.jpg",
        "createdAt": "2026-05-03T11:30:00Z"
      }
    ]
  }
}
```

## 5.4 回廊归档详情

`GET /api/archives/:archiveId`

作用：

- 返回某一份《此生回响录》的完整内容

response:

```json
{
  "success": true,
  "data": {
    "archiveId": "arch_01JVB9FA6P4V5P9N2G1Q0T7B3X",
    "title": "最新回响",
    "era": "蒸汽朋克",
    "ending": {
      "biography": "艾莉亚的《此生回响录》：在蒸汽朋克的时代，面对齿轮城的旧秩序正在崩塌，真相与忠诚互相撕扯的残酷现实，你走出了一条独特的道路。",
      "turningPoints": [
        {
          "cause": "面对未知的星辰",
          "effect": "选择毫不犹豫地踏上旅程"
        },
        {
          "cause": "遭遇遗迹守卫的考验",
          "effect": "以智慧化解了危机"
        },
        {
          "cause": "面临最终的抉择",
          "effect": "选择了守护真相，即便代价沉重"
        }
      ],
      "legacy": "你的选择为世界留下了一丝希望的火种。",
      "cgs": [
        "https://cdn.example.com/endings/ending-1-cg-1.jpg",
        "https://cdn.example.com/endings/ending-1-cg-2.jpg",
        "https://cdn.example.com/endings/ending-1-cg-3.jpg"
      ]
    }
  }
}
```

## 5.5 生成存档分享卡

`POST /api/share/save-card`

作用：

- 针对某个存档生成分享卡图

request:

```json
{
  "saveId": "save_01JVB34HG4T3B2QY2S4B8J6D1N",
  "style": "default"
}
```

response:

```json
{
  "success": true,
  "data": {
    "shareCardId": "share_01JVBAT0ZB0M6N5A6Q8D3J1V9F",
    "imageUrl": "https://cdn.example.com/share/save-card-1.jpg",
    "expiresAt": "2026-05-10T10:30:00Z"
  }
}
```

## 5.6 生成结局分享卡

`POST /api/share/ending-card`

作用：

- 根据结局档案生成社交分享图

request:

```json
{
  "archiveId": "arch_01JVB9FA6P4V5P9N2G1Q0T7B3X",
  "includeCgs": true,
  "style": "default"
}
```

response:

```json
{
  "success": true,
  "data": {
    "shareCardId": "share_01JVBB1V0S0Q0G6H9Y6P1M4K2R",
    "imageUrl": "https://cdn.example.com/share/ending-card-1.jpg",
    "expiresAt": "2026-05-10T11:30:00Z"
  }
}
```

---

## 6. WebSocket 协议草案

## 6.1 连接

`GET /api/game-sessions/:sessionId/stream`

建议查询参数：

```text
?token=xxx
```

## 6.2 通用事件格式

```json
{
  "event": "narration_chunk",
  "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
  "turnId": 4,
  "payload": {}
}
```

## 6.3 事件类型

### a. `turn_started`

```json
{
  "event": "turn_started",
  "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
  "turnId": 4,
  "payload": {
    "phase": "FateWeaving",
    "turnIndex": 4
  }
}
```

### b. `narration_chunk`

用于逐字或分段输出叙事文本。

```json
{
  "event": "narration_chunk",
  "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
  "turnId": 4,
  "payload": {
    "chunk": "你选择了寻找盟友。",
    "done": false
  }
}
```

### c. `world_news`

```json
{
  "event": "world_news",
  "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
  "turnId": 4,
  "payload": {
    "text": "市场物价剧烈波动，暗流涌动。",
    "ttlMs": 5000
  }
}
```

### d. `npc_intervention`

```json
{
  "event": "npc_intervention",
  "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
  "turnId": 4,
  "payload": {
    "npcId": "npc_letter_sender",
    "npcName": "无名信使",
    "text": "我看到了你当时的决断，这让我对你刮目相看。"
  }
}
```

### e. `turn_resolved`

当回合叙事完成时发送，前端据此更新当前节点与资源。

```json
{
  "event": "turn_resolved",
  "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
  "turnId": 4,
  "payload": {
    "resources": {
      "obsessionPoints": 1,
      "intuitionPoints": 4,
      "daysLeft": 18,
      "worldNews": null
    },
    "currentNode": {
      "id": "node-4",
      "text": "你选择了寻找盟友。随着你的决定，命运的齿轮再次转动。",
      "image": "https://cdn.example.com/story/node-4-cover.jpg",
      "choices": [
        {
          "id": "node-4-c1",
          "text": "相信来者并公开线索",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        },
        {
          "id": "node-4-c2",
          "text": "试探对方的真实立场",
          "disabled": false,
          "costHints": {
            "intuition": 1,
            "obsession": 1
          }
        }
      ]
    },
    "stateView": {
      "gameState": "playing",
      "phase": "AwaitingProtagonist",
      "turnIndex": 4,
      "activeTurnId": 4,
      "currentLocation": "铸铁之城 · 黑雾酒馆后室",
      "currentScene": "盟友出现，但意图尚未完全显露",
      "protagonistState": "表面镇定，实际高度戒备",
      "npcsState": "潜在盟友与监视者同时接近",
      "latestHistory": "你选择求助后，局势进一步复杂化。",
      "latestBroadcastSummary": "新的联盟机会出现，但代价未知。",
      "latestProtagonistAction": "寻找盟友的帮助"
    },
    "endingStatus": "none"
  }
}
```

### f. `ending_ready`

```json
{
  "event": "ending_ready",
  "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
  "turnId": 6,
  "payload": {
    "endingStatus": "ready",
    "archiveId": "arch_01JVB9FA6P4V5P9N2G1Q0T7B3X"
  }
}
```

### g. `error`

```json
{
  "event": "error",
  "sessionId": "sess_01JVB2X3A7N0W8S2G0J8P4QK1D",
  "turnId": 4,
  "payload": {
    "code": "INSUFFICIENT_OBSESSION",
    "message": "执念已经耗尽，无法继续强行扭动命运。"
  }
}
```

## 6.4 一次完整回合的流式示例

```json
{"event":"turn_started","sessionId":"sess_xxx","turnId":4,"payload":{"phase":"FateWeaving","turnIndex":4}}
{"event":"narration_chunk","sessionId":"sess_xxx","turnId":4,"payload":{"chunk":"你选择了寻找盟友。","done":false}}
{"event":"narration_chunk","sessionId":"sess_xxx","turnId":4,"payload":{"chunk":"随着你的决定，命运的齿轮再次转动。","done":false}}
{"event":"world_news","sessionId":"sess_xxx","turnId":4,"payload":{"text":"市场物价剧烈波动，暗流涌动。","ttlMs":5000}}
{"event":"npc_intervention","sessionId":"sess_xxx","turnId":4,"payload":{"npcId":"npc_letter_sender","npcName":"无名信使","text":"我看到了你当时的决断，这让我对你刮目相看。"}}
{"event":"turn_resolved","sessionId":"sess_xxx","turnId":4,"payload":{"resources":{"obsessionPoints":1,"intuitionPoints":4,"daysLeft":18,"worldNews":null},"currentNode":{"id":"node-4","text":"你选择了寻找盟友。随着你的决定，命运的齿轮再次转动。","image":"https://cdn.example.com/story/node-4-cover.jpg","choices":[{"id":"node-4-c1","text":"相信来者并公开线索","disabled":false,"costHints":{"intuition":1,"obsession":1}},{"id":"node-4-c2","text":"试探对方的真实立场","disabled":false,"costHints":{"intuition":1,"obsession":1}}]},"stateView":{"gameState":"playing","phase":"AwaitingProtagonist","turnIndex":4,"activeTurnId":4,"currentLocation":"铸铁之城 · 黑雾酒馆后室","currentScene":"盟友出现，但意图尚未完全显露","protagonistState":"表面镇定，实际高度戒备","npcsState":"潜在盟友与监视者同时接近","latestHistory":"你选择求助后，局势进一步复杂化。","latestBroadcastSummary":"新的联盟机会出现，但代价未知。","latestProtagonistAction":"寻找盟友的帮助"},"endingStatus":"none"}}
```

---

## 7. 前后端字段映射建议

前端当前 Zustand 字段可尽量与接口保持一致：

- `character` <-> `Character`
- `world` <-> `World`
- `storyNodes[currentNodeId]` <-> `currentNode`
- `obsessionPoints` / `intuitionPoints` <-> `resources`
- `endingData` <-> `ending`
- `worldNews` <-> `resources.worldNews` 或 WebSocket `world_news`

建议前端逐步从“保存完整 `storyNodes[]`”转向“保存 `currentNode + history[]`”：

- 主舞台只需要当前节点和最近历史
- 历史记录页通过 `/history` 单独拉取
- 这样更贴近后端真实状态机，不必每回合回传整个节点数组

---

## 8. 推荐落地顺序

第一批：

1. `POST /api/game-sessions`
2. `GET /api/game-sessions/:sessionId`
3. `POST /api/game-sessions/:sessionId/choices`
4. `GET /api/game-sessions/:sessionId/ending`
5. `POST /api/saves`
6. `GET /api/saves`
7. `POST /api/saves/:saveId/load`
8. `WS /api/game-sessions/:sessionId/stream`

第二批：

1. `POST /api/game-sessions/:sessionId/intuition-preview`
2. `GET /api/game-sessions/:sessionId/history`
3. `GET /api/archives`
4. `GET /api/archives/:archiveId`
5. `POST /api/share/save-card`
6. `POST /api/share/ending-card`

---

## 9. 后续建议

- 若 `akashic-server` 作为 HTTP/WS 外层，建议优先把 `akashic-ecs` 的 `RuntimeStateView` 暴露成统一 DTO
- 叙事流建议统一走 WebSocket，避免长轮询与同步等待
- 图像生成建议异步任务化，接口先返回任务状态或占位图，避免主流程被阻塞
- 分享卡可先同步拼图，后续再升级为异步渲染服务
