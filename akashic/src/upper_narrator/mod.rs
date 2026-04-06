pub mod types;
#[allow(unused_imports)]
pub use types::*;

use agent::{
    agent::{AgentActor, AgentActorEvent, Context, GenericToolExecutor, LayerKind},
    core::Message,
    models::ChatModel,
};
use serde_json::json;
use tokio::sync::{broadcast, mpsc};

use crate::{
    event_system::{Event, EventChannel, SystemEvent},
    fate_weaver::FateWeaverEvent,
    shared::{build_chat_model, build_layer, extract_step_content},
};

static SYS_PORMPT: &str = r#"
你是上层叙事者，故事的文学讲述者。你是连接模拟世界与用户感知的桥梁——命运编织者产出的是干瘪的事实记录，而你的工作是将这些事实转化为丰满的、引人入胜的叙事文本。你不创造事实，只描述事实；不改变世界，只呈现世界。你是一位技艺精湛的小说家，懂得如何选择最佳的叙事角度、节奏和风格来讲述每一个故事瞬间。

核心职责

一、文学化叙事渲染

你接收命运编织者发来的事实清单，将其转化为具有文学性的叙事段落。转化时你必须遵循：

• Show Don't Tell：通过具体的行动、对话和细节来展现信息，而非直接告知用户。不要写“他很紧张”，写“他的手指不自觉地敲击着桌面，目光在门口和窗户之间来回游移”。
• 感官写作：调动多种感官进行描写——视觉、听觉、触觉、嗅觉。五感俱全的场景描写能让读者更加沉浸。
• 对话驱动：善用对话来展现角色性格和推动情节。每个角色的对话都应该有独特的声音——符合其身份、教育背景和性格特征。
• 意象与象征：在关键场景中使用意象和象征来增强叙事深度。一把断剑可以象征破碎的信念，一场暴雨可以暗示命运的残酷。

二、叙事节奏控制

你根据命运编织者提供的节奏指令（BUILDUP / TENSION / RELEASE / REFLECTION / CLIMAX）调整叙事风格：

• BUILDUP（铺垫）：慢节奏，大量环境描写和氛围渲染，建立张力。使用长句、舒缓的节奏，让读者沉浸在世界中。
• TENSION（紧张）：中快节奏，短句增多，信息密度提高。制造悬念，让读者感到“有什么事情要发生”。
• RELEASE（释放）：快节奏，动作密集，情感爆发。使用短促有力的句子，让读者感受到冲击力。
• REFLECTION（反思）：慢节奏，内心独白为主，情感沉淀。使用第一人称的内心独白或第三人称的心理描写，让读者与角色共同思考。
• CLIMAX（高潮）：最快节奏，最高张力。句子短促有力，信息密集，情感达到峰值。这是故事的“决定性时刻”。

三、风格切换

你支持三种核心叙事风格的动态切换，以适应不同场景的叙事需求：

• 文学风：注重意象、隐喻和文字美感。叙事语言优美含蓄，善用象征和暗示。适合情感场景、氛围渲染和哲理思考。
• 悬疑风：善于制造悬念和紧张氛围。信息释放节奏精心控制，常使用伏笔和误导。适合调查、推理和危机场景。
• 史诗风：宏大叙事，语言庄重有力。适合战争、政治、命运抉择等大场面。善用排比和修辞增强气势。

风格的切换应该是自然的、渐进的，而非突兀的。你可以根据场景性质和命运编织者的节奏指令自动选择最合适的风格。

四、情感渲染

情感是叙事的灵魂。你必须具备精准的情感渲染能力：
• 细节共鸣：通过微小但真实的细节触发情感。一个颤抖的手指、一声未说出口的叹息，往往比大段的情感宣泄更有力量。
• 对比反差：通过前后对比强化情感冲击。战前的宁静让战争的残酷更加刺眼，失去之后的日常让失去的痛苦更加深沉。
• 延迟满足：将情感释放推迟到最合适的时机。不要过早揭示真相，不要过早释放情绪。让张力积累到临界点再爆发。
• 留白艺术：有些情感不需要说透。适当的留白给读者留下想象和共情的空间，往往比直白的表达更有力量。

五、对话呈现

对话是展现角色性格和推动情节的核心手段。你必须确保：
• 声音独特：每个角色都有独特的说话方式——用词习惯、口头禅、语气特征。仅凭对话就能区分说话者。
• 潜台词丰富：角色说的和想的不一样。好的对话有“水面下的冰山”——角色真正想表达的意思隐藏在字里行间。
• 推动情节：每段对话都应该推动情节发展或揭示角色信息，而非无意义的闲聊。

行为规范
1. 忠于事实：你只能基于命运编织者提供的事实清单进行叙事渲染。你不能凭空创造新的事件、新的角色或改变事件的结果。
2. 合理推断：在事实的基础上，你可以进行合理的推断和氛围渲染——比如根据 NPC 的性格推断其微表情，根据环境数据描写氛围。但推断不能违背已有的事实。
3. 节制克制：好的叙事者知道什么时候该放手描写，什么时候该克制留白。不是每个场景都需要大段描写，有时候一句话的力量胜过一页纸。
4. 视角一致：一旦确定了叙事视角（第一人称/第二人称/第三人称），就必须在整个故事中保持一致，除非有明确的叙事需要切换视角。
5. 尊重角色：你不能让角色说出不符合其性格的话，或做出不符合其设定的事。角色的“声音”由其人物设定决定，不由你的偏好决定。
"#;

pub struct UpperNarrator {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
    channel: EventChannel,
    receiver: broadcast::Receiver<Event>,
    pending_ending_cue: Option<(u32, EndingCue)>,
}

impl UpperNarrator {
    pub fn new(channel: EventChannel) -> Result<Self, String> {
        let model = build_chat_model()?;
        let tool_executor = GenericToolExecutor::new();
        let context = Context::new()
            .layer(build_layer(
                "upper-narrator-system",
                LayerKind::System,
                json!(SYS_PORMPT),
                100,
            ))
            .layer(build_layer(
                "upper-narrator-soul",
                LayerKind::Soul,
                json!({
                    "name": "上层叙事者",
                    "role": "将事实清单转写为文学叙事文本",
                    "guidelines": [
                        "绝不改写事实结果",
                        "要体现节奏、风格与情绪基调",
                        "输出应直接面向读者"
                    ]
                }),
                90,
            ));
        let receiver = channel.subscribe();
        let agent_actor = AgentActor::new(model, tool_executor, context);
        Ok(Self {
            agent_actor,
            channel,
            receiver,
            pending_ending_cue: None,
        })
    }

    pub async fn start(mut self) {
        loop {
            match self.receiver.recv().await {
                Ok(Event::UpperNarrator(UpperNarratorEvent::EndingCueReceived {
                    round,
                    cue,
                })) => {
                    self.pending_ending_cue = Some((round, cue));
                }
                Ok(Event::UpperNarrator(UpperNarratorEvent::NarrationTaskReceived {
                    round,
                    task,
                })) => {
                    let ending_cue = self
                        .pending_ending_cue
                        .take()
                        .filter(|(cue_round, _)| *cue_round == round)
                        .map(|(_, cue)| cue);
                    if let Err(err) = self.render_round(round, task, ending_cue).await {
                        self.send_error(err).await;
                    }
                }
                Ok(Event::System(SystemEvent::ShutdownRequested))
                | Ok(Event::FateWeaver(FateWeaverEvent::StoryEnded { .. })) => break,
                Ok(_) => {}
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    }

    async fn render_round(
        &mut self,
        round: u32,
        task: NarrationTask,
        ending_cue: Option<EndingCue>,
    ) -> Result<(), String> {
        let style = NarrativeStyle::for_task(&task);
        let perspective = NarrativePerspective::reader_default();
        let scene_beats = task.scene_beats();
        let facts = serde_json::to_string_pretty(&task)
            .map_err(|err| format!("叙事者序列化事实清单失败: {err}"))?;
        let ending_payload = serde_json::to_string_pretty(&ending_cue)
            .map_err(|err| format!("叙事者序列化结局提示失败: {err}"))?;

        self.send(UpperNarratorEvent::StyleSelected {
            round,
            style: style.clone(),
            perspective: perspective.clone(),
        })
        .await;
        self.send(UpperNarratorEvent::SceneOutlined {
            round,
            beats: scene_beats.clone(),
        })
        .await;

        let prompt = format!(
            r#"你正在书写第 {round} 轮故事。
你必须只输出叙事正文本身，不要 JSON，不要标题，不要解释，不要附加说明。

叙事要求：
- 忠于 facts 中的客观事实，不新增关键结果
- 根据 pacing_instruction、tone、focus、length_hint 调整节奏与文风
- 直接从正文第一句开始写，适合被逐字流式展示
- 如果 should_end=true，要让文字具备结局收束感
- 当前建议风格：{style_name}
- 当前叙事视角：{perspective_name}

事实清单：
{facts}

场景节拍：
{scene_beats_payload}

结局提示：
{ending_payload}"#,
            style_name = style.label(),
            perspective_name = perspective.label(),
            scene_beats_payload = serde_json::to_string_pretty(&scene_beats)
                .map_err(|err| format!("叙事者序列化场景节拍失败: {err}"))?
        );

        self.agent_actor
            .context_mut()
            .add_message(Message::user(prompt));

        let channel = self.channel.clone();
        let narrative = {
            let (event_tx, mut event_rx) = mpsc::channel(128);
            let step = self.agent_actor.run_step(Some(event_tx));
            tokio::pin!(step);

            let result = loop {
                tokio::select! {
                    result = &mut step => break result,
                    maybe_event = event_rx.recv() => {
                        match maybe_event {
                            Some(AgentActorEvent::ContentChunk(content)) => {
                                channel.send(UpperNarratorEvent::NarrativeChunkProduced {
                                    round,
                                    content,
                                });
                            }
                            Some(_) => {}
                            None => {}
                        }
                    }
                }
            };

            while let Some(event) = event_rx.recv().await {
                if let AgentActorEvent::ContentChunk(content) = event {
                    channel.send(UpperNarratorEvent::NarrativeChunkProduced {
                        round,
                        content,
                    });
                }
            }

            extract_step_content(result)?
        };

        channel.send(UpperNarratorEvent::NarrativeCompleted {
            round,
            narrative: RenderedNarrative {
                text: narrative,
                style,
                perspective,
            },
        });

        Ok(())
    }

    async fn send_error(&self, message: String) {
        self.send(SystemEvent::AgentError {
            agent: "upper_narrator".to_string(),
            message,
        })
        .await;
    }

    async fn send(&self, evt: impl Into<Event>) {
        self.channel.send(evt);
    }
}
