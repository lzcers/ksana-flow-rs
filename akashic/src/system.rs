use crate::{
    agent_component::FateWeaver,
    fate_weaver::{
        component::{FateLine, FateWeaverContext},
        types::FateNode,
    },
    model_resource::ModelTaskManager,
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    protagonist::{ProtagonistActionRequest, ProtagonistDecision},
    shared::{build_chat_model, build_layer, parse_json_response},
    upper_narrator::types::{UpperNarration, UpperNarratorRequest},
};
use agent::{
    agent::{Context, LayerKind},
    core::Message,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{With, Without},
    schedule::{IntoScheduleConfigs, Schedule},
    system::{Commands, Query, ResMut},
    world::World,
};
use serde_json::json;

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub enum RoundStatus {
    #[default]
    Idle,
    WeavingFate,
    AwaitingProtagonist,
    ResolvingProtagonist,
    RoundCompleted,
    Narrating,
    ReadyForNextRound,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Default)]
pub struct RoundRecord {
    pub round: u32,
    pub fate_node: Option<FateNode>,
    pub protagonist_decision: Option<ProtagonistDecision>,
    pub narration: Option<UpperNarration>,
}

#[derive(Component, Debug, Default)]
pub struct StoryRuntime {
    pub status: RoundStatus,
    pub active_round: Option<RoundRecord>,
    pub history: Vec<RoundRecord>,
    pub last_error: Option<String>,
}

fn fail_runtime(runtime: &mut StoryRuntime, error: impl Into<String>) {
    runtime.status = RoundStatus::Failed;
    runtime.last_error = Some(error.into());
}

fn take_finished_task_result(
    task_manager: &mut ModelTaskManager,
    entity: Entity,
) -> Option<Result<String, String>> {
    if task_manager.has_pending(entity) {
        return None;
    }

    let snapshot = task_manager.get_task(entity)?;
    let result = snapshot
        .result
        .clone()
        .unwrap_or_else(|| Err("LLM task finished without result".to_string()));
    task_manager.remove_tasks(entity);
    Some(result)
}

fn parse_fate_node(raw: &str, round: u32) -> Result<RoundRecord, String> {
    let fate_node = parse_json_response::<FateNode>(raw)?;
    Ok(RoundRecord {
        round,
        fate_node: Some(fate_node),
        protagonist_decision: None,
        narration: None,
    })
}

fn build_protagonist_request(round: &RoundRecord) -> Option<ProtagonistActionRequest> {
    let fate_node = round.fate_node.as_ref()?;
    Some(ProtagonistActionRequest {
        round: round.round,
        compact_context: fate_node.to_compact_context(round.round),
        situation: fate_node.situation.clone(),
        choices: fate_node.choices.clone(),
    })
}

fn build_protagonist_context(request: &ProtagonistActionRequest) -> Context {
    let mut context = Context::new().layer(build_layer(
        "protagonist-system",
        LayerKind::System,
        json!({
            "role": "故事主角",
            "goal": "根据命运编织者提供的局势与可选项，给出单一明确动作",
            "output": "只返回 JSON"
        }),
        100,
    ));
    context.add_message(Message::user(build_action_request_prompt(request)));
    context
}

fn build_action_request_prompt(request: &ProtagonistActionRequest) -> String {
    let request_json = serde_json::to_string_pretty(request)
        .unwrap_or_else(|_| "{\"error\":\"serialize request failed\"}".to_string());
    format!(
        "请基于以下世界快照与候选行动，为主角选择一个最合适的动作。\n\
必须只返回 JSON，不要输出额外说明。\n\
返回格式:\n\
{{\n  \"choice_id\": \"选项 id\",\n  \"action\": \"主角将执行的具体动作\",\n  \"rationale\": \"不超过60字的简短理由\"\n}}\n\
\n请求数据:\n{}",
        request_json
    )
}

fn fallback_decision(request: &ProtagonistActionRequest, reason: String) -> ProtagonistDecision {
    if let Some(choice) = request.choices.first() {
        return ProtagonistDecision {
            choice_id: choice.id.clone(),
            action: choice.next_trigger.clone(),
            rationale: format!("模型解析失败，按首个选项回退: {}", reason),
        };
    }

    ProtagonistDecision {
        choice_id: "wait".to_string(),
        action: "主角暂时观察局势，等待新的可行动线索。".to_string(),
        rationale: format!("无可选项，默认保守推进: {}", reason),
    }
}

fn build_narrator_request(round: &RoundRecord) -> Option<UpperNarratorRequest> {
    let fate_node = round.fate_node.as_ref()?;
    let mut compact_context = fate_node.to_compact_context(round.round);
    let mut event = fate_node.event.clone();

    if let Some(decision) = &round.protagonist_decision {
        compact_context.push_str(&format!(
            " 主角决策：选择 `{}`，执行动作：{}。",
            decision.choice_id, decision.action
        ));
        event = format!("{} 主角后续动作：{}。", event, decision.action);
    }

    Some(UpperNarratorRequest {
        round: round.round,
        chapter: fate_node.chapter.clone(),
        section: fate_node.section.clone(),
        compact_context,
        event,
        situation: fate_node.situation.clone(),
        info_gained: fate_node.info_gained.clone(),
    })
}

fn build_narration_context(request: &UpperNarratorRequest) -> Context {
    let mut context = Context::new().layer(build_layer(
        "upper-narrator-system",
        LayerKind::System,
        json!({
            "role": "上层叙事者",
            "goal": "把已完成轮次的事实快照改写为读者可见的叙事文本",
            "output": "直接输出正文"
        }),
        100,
    ));
    context.add_message(Message::user(build_draft_scene_prompt(request)));
    context
}

fn build_draft_scene_prompt(request: &UpperNarratorRequest) -> String {
    let request_json = serde_json::to_string_pretty(request)
        .unwrap_or_else(|_| "{\"error\":\"serialize request failed\"}".to_string());
    format!(
        "请把以下事实快照改写为面向读者的短篇叙事段落。\n\
要求：\n\
1. 不增删事实，不篡改因果。\n\
2. 可润色节奏、氛围和人物感受，但不要引入新设定。\n\
3. 直接输出正文，不要使用 JSON，不要附加说明。\n\
\n事实数据:\n{}",
        request_json
    )
}

fn build_narration(round: &RoundRecord, content: String) -> Option<UpperNarration> {
    let fate_node = round.fate_node.as_ref()?;
    Some(UpperNarration {
        round: round.round,
        title: format!("{} · {}", fate_node.chapter, fate_node.section),
        content,
    })
}

fn commit_round_to_context(round: &RoundRecord, fate_weaver_context: &mut FateWeaverContext) {
    let Some(fate_node) = round.fate_node.as_ref() else {
        return;
    };

    let compact_context = fate_node.to_compact_context(round.round);
    let context = fate_weaver_context.get_context_mut();
    context.add_message(Message::assistant(compact_context));

    if let Some(decision) = &round.protagonist_decision {
        context.add_message(Message::user(format!(
            "主角在第{}轮选择了 `{}`，具体动作：{}。理由：{}",
            round.round, decision.choice_id, decision.action, decision.rationale
        )));
    }

    if let Some(narration) = &round.narration {
        context.add_message(Message::assistant(format!(
            "第{}轮叙事成稿《{}》：{}",
            narration.round, narration.title, narration.content
        )));
    }
}

fn spawn_next_round(
    entity: Entity,
    fate_line: &mut FateLine,
    fate_weaver_context: &FateWeaverContext,
    runtime: &mut StoryRuntime,
    task_manager: &mut ModelTaskManager,
) {
    if fate_line.current_round >= fate_line.max_rounds {
        runtime.status = RoundStatus::Finished;
        return;
    }

    if task_manager.has_pending(entity) {
        return;
    }

    fate_line.current_round += 1;
    runtime.active_round = Some(RoundRecord {
        round: fate_line.current_round,
        ..RoundRecord::default()
    });
    runtime.last_error = None;
    task_manager.spawn_task(entity, fate_weaver_context.get_context());
    runtime.status = RoundStatus::WeavingFate;
}

pub fn story_runtime_init_system(
    mut commands: Commands,
    query: Query<Entity, (With<FateWeaver>, Without<StoryRuntime>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(StoryRuntime::default());
    }
}

// 单轮生命周期总览
// 1. FateWeaver 发起当前轮的 FateNode 推演任务。
// 2. FateNode 完成后分两种情况：
//    - 有 choice：进入等待主角答复状态，主角答复完成后，本轮才算完成。
//    - 无 choice：FateNode 解析完成时，本轮立即完成。
// 3. 本轮完成后，才能把该轮事实快照交给叙事者生成故事文本。
// 4. 叙事者完成当前轮叙事后，当前轮才真正收束，系统才允许进入下一轮。
// 5. 因此“本轮完成”和“允许进入下一轮”不是同一个时刻：
//    - 本轮完成：世界事实已闭合，若需要主角答复则也已拿到答复。
//    - 允许下一轮：叙事文本已产出，读者已经看到当前轮故事。
//
// task_system 负责推进所有异步模型任务的 stream，并把“进行中 / 已完成 / 超时”
// 的任务状态刷新到 ModelTaskManager 中，供 FateWeaver / Narration 系统读取。
pub fn task_system(mut task_manager: ResMut<ModelTaskManager>) {
    task_manager.poll_tasks();
}

// 解析当前轮次的 FateNode，并把结果转成调度层可消费的轮次状态。
// 这里需要明确处理：
// 1. 当前轮是否存在 choice。
// 2. 当前轮是否已经达到“轮次完成”。
// 3. 哪些事实会写回 FateWeaverContext，作为后续轮次的上下文。
// 4. 哪些内容需要发送给主角系统，哪些内容需要保留到叙事阶段。
// 命运编织系统
// 负责推进“世界事实”这一主链，但不直接负责故事文本生成。
//
// 单轮状态机建议理解为：
// 1. Idle：没有进行中的 FateNode 推演，也没有阻塞当前轮推进的未完成步骤。
// 2. Weaving：当前轮 FateNode 推演中。
// 3. AwaitingProtagonist：FateNode 已完成，且本轮存在 choice，等待主角答复。
// 4. RoundCompleted：本轮事实已闭合，但叙事尚未完成，因此还不能进入下一轮。
// 5. ReadyForNextRound：当前轮叙事完成，允许 FateWeaver 发起下一轮推演。
//
// 注意：
// - FateWeaver 不应仅凭“当前是否有 pending task”决定是否开启下一轮。
// - 没有 pending task 只说明当前没有模型调用在跑，不代表本轮已经允许推进。
// - 是否允许进入下一轮，最终取决于当前轮叙事是否完成。
pub fn fate_weaving_system(
    mut query: Query<(
        Entity,
        &FateWeaver,
        &mut FateLine,
        &mut FateWeaverContext,
        &mut StoryRuntime,
    )>,
    mut task_manager: ResMut<ModelTaskManager>,
) {
    for (entity, _fate_weaver, mut fate_line, mut fate_weaver_context, mut runtime) in
        query.iter_mut()
    {
        match runtime.status {
            RoundStatus::Idle => {
                spawn_next_round(
                    entity,
                    &mut fate_line,
                    &fate_weaver_context,
                    &mut runtime,
                    &mut task_manager,
                );
            }
            RoundStatus::ReadyForNextRound => {
                if let Some(round) = runtime.active_round.take() {
                    commit_round_to_context(&round, &mut fate_weaver_context);
                    runtime.history.push(round);
                }

                if fate_line.current_round >= fate_line.max_rounds {
                    runtime.status = RoundStatus::Finished;
                    continue;
                }

                runtime.status = RoundStatus::Idle;
                spawn_next_round(
                    entity,
                    &mut fate_line,
                    &fate_weaver_context,
                    &mut runtime,
                    &mut task_manager,
                );
            }
            RoundStatus::WeavingFate => {
                let Some(result) = take_finished_task_result(&mut task_manager, entity) else {
                    continue;
                };

                let round = runtime
                    .active_round
                    .as_ref()
                    .map(|active| active.round)
                    .unwrap_or(fate_line.current_round);

                match result {
                    Ok(raw) => match parse_fate_node(&raw, round) {
                        Ok(parsed_round) => {
                            let needs_protagonist_reply = parsed_round
                                .fate_node
                                .as_ref()
                                .map(|fate_node| !fate_node.choices.is_empty())
                                .unwrap_or(false);
                            runtime.active_round = Some(parsed_round);
                            runtime.last_error = None;
                            runtime.status = if needs_protagonist_reply {
                                RoundStatus::AwaitingProtagonist
                            } else {
                                RoundStatus::RoundCompleted
                            };
                        }
                        Err(err) => fail_runtime(&mut runtime, err),
                    },
                    Err(err) => fail_runtime(&mut runtime, err),
                }
            }
            RoundStatus::AwaitingProtagonist
            | RoundStatus::ResolvingProtagonist
            | RoundStatus::RoundCompleted
            | RoundStatus::Narrating
            | RoundStatus::Finished
            | RoundStatus::Failed => {}
        }
    }
}

// 主角系统
// 只处理“当前轮需要主角答复”的情况。
// todo:
// 1. 监听 FateWeaver 发来的 choice / action request。
// 2. 生成并提交主角答复。
// 3. 主角答复提交后，如果该轮原本处于 AwaitingProtagonist，
//    则该轮在这里被标记为“轮次完成”。
// 4. 主角系统不负责推进下一轮；它只负责让当前轮从“等待主角答复”
//    进入“已完成、等待叙事”的状态。
pub fn protagonist_system(
    mut query: Query<(Entity, &mut StoryRuntime)>,
    mut task_manager: ResMut<ModelTaskManager>,
) {
    for (entity, mut runtime) in query.iter_mut() {
        match runtime.status {
            RoundStatus::AwaitingProtagonist => {
                let Some(request) = runtime
                    .active_round
                    .as_ref()
                    .and_then(build_protagonist_request)
                else {
                    fail_runtime(&mut runtime, "missing active round for protagonist request");
                    continue;
                };

                if task_manager.has_pending(entity) {
                    continue;
                }

                let context = build_protagonist_context(&request);
                task_manager.spawn_task(entity, &context);
                runtime.status = RoundStatus::ResolvingProtagonist;
            }
            RoundStatus::ResolvingProtagonist => {
                let Some(result) = take_finished_task_result(&mut task_manager, entity) else {
                    continue;
                };

                let Some(request) = runtime
                    .active_round
                    .as_ref()
                    .and_then(build_protagonist_request)
                else {
                    fail_runtime(&mut runtime, "missing active round for protagonist result");
                    continue;
                };

                let decision = match result {
                    Ok(raw) => parse_json_response::<ProtagonistDecision>(&raw)
                        .unwrap_or_else(|err| fallback_decision(&request, err)),
                    Err(err) => fallback_decision(&request, err),
                };

                if let Some(active_round) = runtime.active_round.as_mut() {
                    active_round.protagonist_decision = Some(decision);
                    runtime.last_error = None;
                    runtime.status = RoundStatus::RoundCompleted;
                } else {
                    fail_runtime(&mut runtime, "missing active round while storing decision");
                }
            }
            _ => {}
        }
    }
}

// 叙事者系统
// 叙事者是观察者，不参与 FateWeaver 与主角的交互决策。
// 它只消费“已经完成但尚未叙事”的轮次，并把事实转写为读者可见的故事文本。
//
// 职责：
// 1. 查询是否存在“本轮已完成，但尚未叙事完成”的轮次。
// 2. 如果当前有故事生成任务在运行，则等待其完成并记录结果。
// 3. 如果当前没有故事生成任务，且存在待叙事轮次，则提交叙事任务。
// 4. 当叙事任务完成时，将该轮标记为“叙事完成”。
// 5. 当前轮只有在叙事完成后，调度层才允许进入下一轮。
//
// 因此 narrator 的输出虽然不参与世界事实决策，但它是故事推进的最终门控。
pub fn narration_system(
    mut query: Query<(Entity, &mut StoryRuntime)>,
    mut task_manager: ResMut<ModelTaskManager>,
) {
    for (entity, mut runtime) in query.iter_mut() {
        match runtime.status {
            RoundStatus::RoundCompleted => {
                let Some(request) = runtime
                    .active_round
                    .as_ref()
                    .and_then(build_narrator_request)
                else {
                    fail_runtime(&mut runtime, "missing completed round for narration");
                    continue;
                };

                if task_manager.has_pending(entity) {
                    continue;
                }

                let context = build_narration_context(&request);
                task_manager.spawn_task(entity, &context);
                runtime.status = RoundStatus::Narrating;
            }
            RoundStatus::Narrating => {
                let Some(result) = take_finished_task_result(&mut task_manager, entity) else {
                    continue;
                };

                let content = match result {
                    Ok(content) => content.trim().to_string(),
                    Err(err) => {
                        runtime.last_error = Some(err.clone());
                        format!("叙事生成失败：{}", err)
                    }
                };

                let Some(active_round) = runtime.active_round.as_mut() else {
                    fail_runtime(&mut runtime, "missing active round for narration result");
                    continue;
                };

                if let Some(narration) = build_narration(active_round, content) {
                    active_round.narration = Some(narration);
                    runtime.status = RoundStatus::ReadyForNextRound;
                } else {
                    fail_runtime(&mut runtime, "missing fate node while building narration");
                }
            }
            _ => {}
        }
    }
}

pub fn build_story_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            story_runtime_init_system,
            task_system,
            fate_weaving_system,
            protagonist_system,
            narration_system,
        )
            .chain(),
    );
    schedule
}

pub fn spawn_story_entity(
    world: &mut World,
    protagonist_profile: impl Into<String>,
    world_profile: impl Into<String>,
    max_rounds: u32,
) -> Entity {
    if !world.contains_resource::<ModelTaskManager>() {
        world.insert_resource(ModelTaskManager::new(build_chat_model()));
    }

    world
        .spawn((
            FateWeaver,
            FateLine::new(max_rounds),
            FateWeaverContext::new(protagonist_profile.into(), world_profile.into()),
            StoryRuntime::default(),
        ))
        .id()
}

pub fn build_default_story_world(max_rounds: u32) -> (World, Entity) {
    let mut world = World::new();
    let entity = spawn_story_entity(
        &mut world,
        DEFAULT_PROTAGONIST_PROFILE.to_string(),
        DEFAULT_WORLD_PROFILE.to_string(),
        max_rounds,
    );
    (world, entity)
}
