mod event_system;
mod fate_weaver;
mod protagonist;
mod shared;
mod upper_narrator;
mod profile;

use event_system::{Event, EventChannel, SystemEvent};
use fate_weaver::{FateWeaver, FateWeaverEvent};
use protagonist::{Protagonist, ProtagonistEvent};
use std::io::{self, Write};
use tokio::sync::broadcast;
use upper_narrator::{UpperNarrator, UpperNarratorEvent};

use profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE};

struct ConsolePresenter {
    streaming_round: Option<u32>,
}

impl ConsolePresenter {
    fn new() -> Self {
        Self {
            streaming_round: None,
        }
    }

    fn render_narrative_chunk(&mut self, round: u32, content: &str) {
        if self.streaming_round != Some(round) {
            if self.streaming_round.is_some() {
                println!();
            }
            println!("\n========== 第 {round} 轮 ==========\n");
            self.streaming_round = Some(round);
        }
        print!("{content}");
        let _ = io::stdout().flush();
    }

    fn render_decision_request(
        &mut self,
        round: u32,
        request: protagonist::UserDecisionRequest,
    ) {
        println!("\n[第 {round} 轮可选行动]");
        println!("情境：{}", request.situation);
        println!("内心：{}", request.inner_thought);
        for option in request.options {
            println!(
                "- [{}] {}｜后果：{}｜风险：{:?}｜契合度：{:?}｜倾向：{}",
                option.option_id,
                option.action,
                option.consequence_hint,
                option.risk_level,
                option.character_fit,
                option.protagonist_tendency
            );
        }
        if let Some(option_id) = request.recommended_option_id {
            println!("推荐选择：{option_id}");
        }
    }

    fn render_auto_selected(&mut self, round: u32, option_id: &str, reason: &str) {
        println!("\n[第 {round} 轮自动抉择] 选择 {option_id}：{reason}");
    }

    fn finish_round_if_streaming(&mut self, round: u32) {
        if self.streaming_round == Some(round) {
            println!();
            self.streaming_round = None;
        }
    }

    fn render_story_ended(&mut self, round: u32, ending: &fate_weaver::EmergentEnding) {
        if self.streaming_round == Some(round) {
            println!();
            self.streaming_round = None;
        }
        println!("\n========== 终局 · 第 {round} 轮 ==========\n");
        println!("结局类型：{:?}", ending.kind);
        println!("结局摘要：{}", ending.summary);
        println!("代价：{}", ending.cost);
    }

    fn render_agent_error(&mut self, agent: &str, message: &str) {
        eprintln!("[{agent}] {message}");
    }
}

#[tokio::main]
async fn main() {
    let channel = EventChannel::new();
    let mut monitor = channel.subscribe();
    let mut presenter = ConsolePresenter::new();

    let fate_weaver =  FateWeaver::new(
        DEFAULT_PROTAGONIST_PROFILE.to_string(),
        DEFAULT_WORLD_PROFILE.to_string(),
        channel.clone(),
        6,
    );
    let protagonist = Protagonist::new(DEFAULT_PROTAGONIST_PROFILE.to_string(), channel.clone());
    let upper_narrator = UpperNarrator::new(channel.clone());


    let fate_task = tokio::spawn(fate_weaver.start());
    let protagonist_task = tokio::spawn(protagonist.start());
    let narrator_task = tokio::spawn(upper_narrator.start());

    // 启动事件系统
    channel.send(FateWeaverEvent::StartRequested);

    loop {
        match monitor.recv().await {
            Ok(Event::UpperNarrator(UpperNarratorEvent::NarrativeChunkProduced {
                round,
                content,
            })) => {
                presenter.render_narrative_chunk(round, &content);
            }
            Ok(Event::Protagonist(ProtagonistEvent::DecisionRequested {
                round,
                request,
            })) => {
                presenter.render_decision_request(round, request);
            }
            Ok(Event::System(SystemEvent::DecisionAutoSelected {
                round,
                option_id,
                reason,
            })) => {
                presenter.render_auto_selected(round, &option_id, &reason);
            }
            Ok(Event::UpperNarrator(UpperNarratorEvent::NarrativeCompleted {
                round, ..
            })) => {
                presenter.finish_round_if_streaming(round);
            }
            Ok(Event::FateWeaver(FateWeaverEvent::StoryEnded { round, ending })) => {
                presenter.render_story_ended(round, &ending);
                break;
            }
            Ok(Event::System(SystemEvent::AgentError { agent, message })) => {
                presenter.render_agent_error(&agent, &message);
                channel.send(SystemEvent::ShutdownRequested);
                break;
            }
            Ok(_) => {}
            Err(broadcast::error::RecvError::Closed) => break,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
        }
    }

    let _ = fate_task.await;
    let _ = protagonist_task.await;
    let _ = narrator_task.await;
}
