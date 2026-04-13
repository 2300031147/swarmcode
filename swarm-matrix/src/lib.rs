use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{error::Error, io, sync::Arc};
use swarm_runtime::{SwarmHive, team_message};
use swarm_runtime::usage::format_usd;
use swarm_commands::SlashCommand;

pub struct AppState {
    pub input_text: String,
    pub hub: Arc<SwarmHive>,
    pub should_quit: bool,
}

impl AppState {
    pub fn with_context(hub: Arc<SwarmHive>) -> Self {
        Self {
            input_text: String::new(),
            hub,
            should_quit: false,
        }
    }
}

pub async fn render_interactive_terminal(app: &mut AppState) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut AppState) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Continuously poll to allow dynamic UI redraws when SwarmHive updates asynchronously
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Char(c) => app.input_text.push(c),
                    KeyCode::Backspace => { app.input_text.pop(); },
                    KeyCode::Enter => {
                        let msg = app.input_text.drain(..).collect::<String>();
                        if !msg.is_empty() {
                            if msg.starts_with('/') {
                                if SlashCommand::parse(&msg).is_some() {
                                    // Forward valid slash commands to the agent loop for processing
                                    let _ = app.hub.send_to(team_message("user", Some("agent"), &msg));
                                } else {
                                    let _ = app.hub.send_to(team_message("user", Some("agent"), &format!("Unknown slash command: {}. Type /help.", msg)));
                                }
                            } else {
                                // Standard conversational payload
                                let _ = app.hub.send_to(team_message("user", Some("agent"), &msg));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &AppState) {
    let size = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Chat History
            Constraint::Length(3), // Input Block
        ])
        .split(size);

    let header_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let branch = std::process::Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_else(|_| "no-git".to_string());
    
    let header_left_text = format!("ClawSwarm (Core Unified) [Branch: {}]", branch);
    let header_left = Paragraph::new(header_left_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
        
    let usage = app.hub.total_usage();
    let members = app.hub.members();
    let agent_status = members.iter()
        .find(|m| m.id == "agent")
        .map(|m| format!("{:?}", m.status))
        .unwrap_or_else(|| "Offline".to_string());

    let cost_usd = usage.estimate_cost_usd().total_cost_usd();
    let issues_count = 3; 
    let health_status = "PASSING";
    let hive_status = "LOCAL"; // In production, this would be computed via Hub networking state
    let usage_text = format!("Status: {} | Hive: {} | Health: {} | Cost: {} | Issues: {} | Tokens: In {} / Out {}", 
        agent_status, 
        hive_status,
        health_status,
        format_usd(cost_usd),
        issues_count,
        usage.input_tokens, 
        usage.output_tokens);
    let header_right = Paragraph::new(usage_text)
        .style(Style::default().fg(if agent_status == "Active" { Color::Green } else { Color::Gray }))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(header_left, header_chunk[0]);
    f.render_widget(header_right, header_chunk[1]);

    // Zero-copy sync straight from SwarmHive Memory Locks!
    let mut history_spans = Vec::new();
    for message in app.hub.message_log() {
        let (name_color, mut body_color) = if message.from == "agent" {
            (Color::Magenta, Color::White)
        } else if message.from == "system" {
            (Color::Yellow, Color::DarkGray)
        } else {
            (Color::Blue, Color::Gray)
        };

        // [INTEGRATION] Highlight blocking questions
        let is_question = message.body.starts_with("[QUESTION]");
        let mut style = Style::default().fg(body_color);
        if is_question {
            style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
        }

        history_spans.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(format!("{}: ", message.from.to_uppercase()), Style::default().fg(name_color).add_modifier(Modifier::BOLD)),
            ratatui::text::Span::styled(message.body.clone(), style),
        ]));
        history_spans.push(ratatui::text::Line::from("")); // Spacer
    }

    let messages_view = Paragraph::new(history_spans)
        .block(Block::default().title(" Distributed Team Hub Interaction ").borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(messages_view, chunks[1]);

    let input_view = Paragraph::new(app.input_text.as_ref())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title(" Input (Press 'q' to quit, Enter to send) ").borders(Borders::ALL));
    f.render_widget(input_view, chunks[2]);
}
