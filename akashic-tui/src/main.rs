use std::{io, time::Duration};

use agent::agent::AgentActorEvent;
use akashic::{
    channel::{AgentChannel, AkashicEvent, FateWeaverMessage},
    fate_weaver::FateWeaver,
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    protagonist::Protagonist,
    upper_narrator::UpperNarrator,
};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
};

const MAX_BUFFER_CHARS: usize = 12000;
const STORY_PLACEHOLDER: &str = "等待上层叙事者生成故事片段...";
const FATE_PLACEHOLDER: &str = "等待命运编织者输出 ContentChunk ...";
const ACTION_PLACEHOLDER: &str = "等待主角产生决策...";

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusPanel {
    Story,
    Fate,
    Actions,
}

impl FocusPanel {
    fn next(self) -> Self {
        match self {
            Self::Story => Self::Fate,
            Self::Fate => Self::Actions,
            Self::Actions => Self::Story,
        }
    }
}

struct App {
    story: String,
    fate_chunks: String,
    actions: String,
    status: String,
    story_scroll: u16,
    fate_scroll: u16,
    actions_scroll: u16,
    follow_story: bool,
    follow_fate: bool,
    follow_actions: bool,
    focus: FocusPanel,
    narration_count: usize,
    decision_count: usize,
}

impl App {
    fn new() -> Self {
        Self {
            story: STORY_PLACEHOLDER.to_string(),
            fate_chunks: FATE_PLACEHOLDER.to_string(),
            actions: ACTION_PLACEHOLDER.to_string(),
            status: "运行中".to_string(),
            story_scroll: 0,
            fate_scroll: 0,
            actions_scroll: 0,
            follow_story: true,
            follow_fate: true,
            follow_actions: true,
            focus: FocusPanel::Story,
            narration_count: 0,
            decision_count: 0,
        }
    }

    fn on_narration(&mut self, round: u32, title: &str, content: &str) {
        if self.story == STORY_PLACEHOLDER {
            self.story.clear();
        }
        if !self.story.is_empty() {
            self.story.push_str("\n\n");
        }
        self.narration_count += 1;
        append_with_limit(
            &mut self.story,
            &format!("第 {} 轮  《{}》\n{}", round, title, content),
        );
        if self.follow_story {
            self.scroll_story_to_end();
        }
    }

    fn on_protagonist_decision(
        &mut self,
        round: u32,
        choice_id: &str,
        action: &str,
        rationale: &str,
    ) {
        if self.actions == ACTION_PLACEHOLDER {
            self.actions.clear();
        }
        if !self.actions.is_empty() {
            self.actions.push_str("\n\n");
        }
        self.decision_count += 1;
        append_with_limit(
            &mut self.actions,
            &format!(
                "第 {} 轮\n选择: {}\n动作: {}\n理由: {}",
                round, choice_id, action, rationale
            ),
        );
        if self.follow_actions {
            self.scroll_actions_to_end();
        }
    }

    fn on_agent_event(&mut self, event: AgentActorEvent) {
        match event {
            AgentActorEvent::ContentChunk(chunk) => {
                if self.fate_chunks == FATE_PLACEHOLDER {
                    self.fate_chunks.clear();
                }
                append_with_limit(&mut self.fate_chunks, &chunk);
                if self.follow_fate {
                    self.scroll_fate_to_end();
                }
            }
            AgentActorEvent::Error(err) => {
                self.status = format!("执行失败: {}", err);
            }
            _ => {}
        }
    }

    fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    fn focus_story(&mut self) {
        self.focus = FocusPanel::Story;
    }

    fn focus_fate(&mut self) {
        self.focus = FocusPanel::Fate;
    }

    fn focus_actions(&mut self) {
        self.focus = FocusPanel::Actions;
    }

    fn scroll_up(&mut self, amount: u16) {
        match self.focus {
            FocusPanel::Story => {
                self.follow_story = false;
                self.story_scroll = self.story_scroll.saturating_sub(amount);
            }
            FocusPanel::Fate => {
                self.follow_fate = false;
                self.fate_scroll = self.fate_scroll.saturating_sub(amount);
            }
            FocusPanel::Actions => {
                self.follow_actions = false;
                self.actions_scroll = self.actions_scroll.saturating_sub(amount);
            }
        }
    }

    fn scroll_down(&mut self, amount: u16) {
        match self.focus {
            FocusPanel::Story => {
                let max_scroll = self.max_story_scroll();
                self.story_scroll = self.story_scroll.saturating_add(amount).min(max_scroll);
                self.follow_story = self.story_scroll >= max_scroll;
            }
            FocusPanel::Fate => {
                let max_scroll = self.max_fate_scroll();
                self.fate_scroll = self.fate_scroll.saturating_add(amount).min(max_scroll);
                self.follow_fate = self.fate_scroll >= max_scroll;
            }
            FocusPanel::Actions => {
                let max_scroll = self.max_actions_scroll();
                self.actions_scroll = self.actions_scroll.saturating_add(amount).min(max_scroll);
                self.follow_actions = self.actions_scroll >= max_scroll;
            }
        }
    }

    fn scroll_to_top(&mut self) {
        match self.focus {
            FocusPanel::Story => {
                self.follow_story = false;
                self.story_scroll = 0;
            }
            FocusPanel::Fate => {
                self.follow_fate = false;
                self.fate_scroll = 0;
            }
            FocusPanel::Actions => {
                self.follow_actions = false;
                self.actions_scroll = 0;
            }
        }
    }

    fn scroll_to_end(&mut self) {
        match self.focus {
            FocusPanel::Story => self.scroll_story_to_end(),
            FocusPanel::Fate => self.scroll_fate_to_end(),
            FocusPanel::Actions => self.scroll_actions_to_end(),
        }
    }

    fn scroll_story_to_end(&mut self) {
        self.story_scroll = self.max_story_scroll();
        self.follow_story = true;
    }

    fn scroll_fate_to_end(&mut self) {
        self.fate_scroll = self.max_fate_scroll();
        self.follow_fate = true;
    }

    fn scroll_actions_to_end(&mut self) {
        self.actions_scroll = self.max_actions_scroll();
        self.follow_actions = true;
    }

    fn max_story_scroll(&self) -> u16 {
        self.story.lines().count().saturating_sub(1) as u16
    }

    fn max_fate_scroll(&self) -> u16 {
        self.fate_chunks.lines().count().saturating_sub(1) as u16
    }

    fn max_actions_scroll(&self) -> u16 {
        self.actions.lines().count().saturating_sub(1) as u16
    }

    fn story_title(&self) -> String {
        panel_title(
            "上层叙事者",
            self.focus == FocusPanel::Story,
            self.narration_count,
        )
    }

    fn fate_title(&self) -> String {
        format!(
            "{} · {} · Tab/1/2/3 切换面板 · ↑↓/PgUp/PgDn 滚动",
            panel_title("命运编织者", self.focus == FocusPanel::Fate, 0),
            self.status
        )
    }

    fn actions_title(&self) -> String {
        panel_title(
            "主角决策",
            self.focus == FocusPanel::Actions,
            self.decision_count,
        )
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    let (channel, inboxes) = AgentChannel::new();
    let upper_narrator = UpperNarrator::new(channel.clone());
    let fate_weaver = FateWeaver::new(
        DEFAULT_PROTAGONIST_PROFILE.to_string(),
        DEFAULT_WORLD_PROFILE.to_string(),
        channel.clone(),
        10,
    );
    let protagonist = Protagonist::new(DEFAULT_PROTAGONIST_PROFILE.to_string(), channel.clone());
    let mut app = App::new();
    let mut events = channel.subscribe_event();

    tokio::spawn(async move {
        fate_weaver.start(inboxes.fate_weaver).await;
    });
    tokio::spawn(async move {
        protagonist.start(inboxes.protagonist).await;
    });
    tokio::spawn(async move {
        upper_narrator.start(inboxes.upper_narrator).await;
    });

    let _ = channel.send_fate_weaver(FateWeaverMessage::Start).await;

    loop {
        while let Ok(event) = events.try_recv() {
            match event {
                AkashicEvent::AgentActor(event) => app.on_agent_event(event),
                AkashicEvent::NarrationGenerated(narration) => {
                    app.on_narration(narration.round, &narration.title, &narration.content)
                }
                AkashicEvent::ProtagonistDecisionMade {
                    round,
                    choice_id,
                    action,
                    rationale,
                } => app.on_protagonist_decision(round, &choice_id, &action, &rationale),
                _ => {}
            }
        }

        terminal.draw(|frame| render(frame, &app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Tab => app.focus_next(),
                    KeyCode::Char('1') => app.focus_story(),
                    KeyCode::Char('2') => app.focus_fate(),
                    KeyCode::Char('3') => app.focus_actions(),
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
                    KeyCode::PageUp => app.scroll_up(8),
                    KeyCode::PageDown => app.scroll_down(8),
                    KeyCode::Home => app.scroll_to_top(),
                    KeyCode::End => app.scroll_to_end(),
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn render(frame: &mut ratatui::Frame, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(frame.area());

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(columns[1]);

    let story = Paragraph::new(app.story.as_str())
        .block(
            Block::default()
                .title(app.story_title())
                .borders(Borders::ALL),
        )
        .scroll((app.story_scroll, 0))
        .wrap(Wrap { trim: false });
    let fate = Paragraph::new(app.fate_chunks.as_str())
        .block(
            Block::default()
                .title(app.fate_title())
                .borders(Borders::ALL),
        )
        .scroll((app.fate_scroll, 0))
        .wrap(Wrap { trim: false });
    let actions = Paragraph::new(app.actions.as_str())
        .block(
            Block::default()
                .title(app.actions_title())
                .borders(Borders::ALL),
        )
        .scroll((app.actions_scroll, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(story, columns[0]);
    frame.render_widget(fate, right[0]);
    frame.render_widget(actions, right[1]);
}

fn append_with_limit(target: &mut String, chunk: &str) {
    target.push_str(chunk);
    let len = target.chars().count();
    if len > MAX_BUFFER_CHARS {
        *target = target.chars().skip(len - MAX_BUFFER_CHARS).collect();
    }
}

fn panel_title(base: &str, focused: bool, count: usize) -> String {
    let prefix = if focused { "[当前] " } else { "" };
    if count == 0 {
        format!("{}{}", prefix, base)
    } else {
        format!("{}{} · {} 条", prefix, base, count)
    }
}
