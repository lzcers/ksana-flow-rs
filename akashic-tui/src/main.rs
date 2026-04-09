use std::{io, time::Duration};

use agent::agent::AgentActorEvent;
use akashic::{
    channel::{AgentChannel, AgentMessage, AkashicEvent, FateWeaverMessage}, fate_weaver::FateWeaver, profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE}
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

struct App {
    story: String,
    fate_chunks: String,
    actions: String,
    status: String,
    fate_scroll: u16,
    follow_fate: bool,
}

impl App {
    fn new() -> Self {
        Self {
            story: "上层叙事者尚未接入，这里先预留为故事展示区。".to_string(),
            fate_chunks: "等待命运编织者输出 ContentChunk ...".to_string(),
            actions: "主角行动选择尚未接入，这里先预留为选择窗口。".to_string(),
            status: "运行中".to_string(),
            fate_scroll: 0,
            follow_fate: true,
        }
    }

    fn on_agent_event(&mut self, event: AgentActorEvent) {
        match event {
            AgentActorEvent::ContentChunk(chunk) => {
                if self.fate_chunks == "等待命运编织者输出 ContentChunk ..." {
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

    fn on_agent_closed(&mut self) {
        if self.status == "运行中" {
            self.status = "命运编织者已结束".to_string();
        }
    }

    fn scroll_fate_up(&mut self, amount: u16) {
        self.follow_fate = false;
        self.fate_scroll = self.fate_scroll.saturating_sub(amount);
    }

    fn scroll_fate_down(&mut self, amount: u16) {
        let max_scroll = self.max_fate_scroll();
        self.fate_scroll = self.fate_scroll.saturating_add(amount).min(max_scroll);
        self.follow_fate = self.fate_scroll >= max_scroll;
    }

    fn scroll_fate_to_top(&mut self) {
        self.follow_fate = false;
        self.fate_scroll = 0;
    }

    fn scroll_fate_to_end(&mut self) {
        self.fate_scroll = self.max_fate_scroll();
        self.follow_fate = true;
    }

    fn max_fate_scroll(&self) -> u16 {
        self.fate_chunks.lines().count().saturating_sub(1) as u16
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
    let mut channel = AgentChannel::new();
    let fate_weaver = FateWeaver::new(
        DEFAULT_PROTAGONIST_PROFILE.to_string(),
        DEFAULT_WORLD_PROFILE.to_string(),
        channel.clone(),
        10,
    );
    let mut app = App::new();
    let mut agent_closed = false;

    tokio::spawn(async move {
        fate_weaver.start().await;
    });
    
    channel.send_msg(AgentMessage::FateWeaver(FateWeaverMessage::Start));

    loop {
        while let Ok(event) = channel.subscribe_event().try_recv() {
            match event {
                AkashicEvent::AgentActor(event) => app.on_agent_event(event),
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
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_fate_up(1),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_fate_down(1),
                    KeyCode::PageUp => app.scroll_fate_up(8),
                    KeyCode::PageDown => app.scroll_fate_down(8),
                    KeyCode::Home => app.scroll_fate_to_top(),
                    KeyCode::End => app.scroll_fate_to_end(),
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
        .block(Block::default().title("上层叙事者").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    let fate = Paragraph::new(app.fate_chunks.as_str())
        .block(
            Block::default()
                .title(format!("命运编织者 · {} · ↑↓/PgUp/PgDn 滚动", app.status))
                .borders(Borders::ALL),
        )
        .scroll((app.fate_scroll, 0))
        .wrap(Wrap { trim: false });
    let actions = Paragraph::new(app.actions.as_str())
        .block(Block::default().title("主角行动").borders(Borders::ALL))
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
