//! CS 1.5 / FreeCS inspired TUI  ─  four interactive screens
//!
//! Screens:
//!  1. Main Menu       — animated logo, menu selection
//!  2. HUD             — top score-bar, crosshair, player list, bottom panel
//!                       + bomb-timer bar, defuse progress, kill feed, grenades
//!  3. Buy Menu        — category tabs + weapon list + detail panel
//!  4. Scoreboard      — K/D/A/HS%/Money/Ping table with round-history ring

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use game_cs::weapons::catalog::WeaponCatalog;
use game_cs::weapons::types::{WeaponKind, WeaponStats};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

// ── Palette ───────────────────────────────────────────────────────────────────
const C_BG:     Color = Color::Rgb(10,  12,  16);
const C_PANEL:  Color = Color::Rgb(20,  24,  32);
const C_BORDER: Color = Color::Rgb(45,  55,  72);
const C_ACCENT: Color = Color::Rgb(251, 191, 36);   // gold
const C_CT:     Color = Color::Rgb(96,  165, 250);  // blue
const C_T:      Color = Color::Rgb(251, 113, 133);  // red/rose
const C_GREEN:  Color = Color::Rgb(74,  222, 128);
const C_DIM:    Color = Color::Rgb(100, 116, 139);
const C_WHITE:  Color = Color::Rgb(226, 232, 240);
const C_HP:     Color = Color::Rgb(239, 68,  68);
const C_ARMOR:  Color = Color::Rgb(96,  165, 250);
const C_MONEY:  Color = Color::Rgb(251, 191, 36);
const C_SMOKE:  Color = Color::Rgb(148, 163, 184);
const C_FIRE:   Color = Color::Rgb(249, 115, 22);

// ── App State ─────────────────────────────────────────────────────────────────
#[derive(Clone, PartialEq)]
enum Screen { MainMenu, Hud, BuyMenu, Scoreboard }

/// One entry in the HUD kill feed
#[derive(Clone)]
struct KillFeedEntry {
    killer: String,
    victim: String,
    weapon: String,
    headshot: bool,
    age_ticks: u64,   // remove after ~300 ticks (~5s)
}

#[derive(Clone)]
struct PlayerRow {
    name:    String,
    team:    &'static str,
    kills:   u32,
    deaths:  u32,
    assists: u32,
    hs_pct:  u32,
    money:   u32,
    ping:    u32,
    hp:      u32,
    blind:   bool,
    has_nade: bool,
}

/// Compact round result for the history ring at the top
#[derive(Clone, Copy, PartialEq)]
enum RoundOutcome { CT, T }

struct BuyCategory { label: &'static str, kind: Option<WeaponKind> }

struct App {
    screen: Screen,
    menu_sel: usize,
    buy_cat:  usize,
    buy_sel:  usize,
    buy_weapons: Vec<Vec<WeaponStats>>,
    players:  Vec<PlayerRow>,
    kill_feed: Vec<KillFeedEntry>,
    round_history: Vec<RoundOutcome>,
    // HUD state
    hp:        u32,
    armor:     u32,
    money:     u32,
    ammo_mag:  u32,
    ammo_res:  u32,
    weapon:    String,
    round:     u32,
    time_secs: u32,
    ct_score:  u32,
    t_score:   u32,
    bomb_planted: bool,
    bomb_frac: f32,        // 0.0 = exploded, 1.0 = just planted
    defusing:  bool,
    defuse_frac: f32,
    // Active grenades HUD
    smokes_active: u32,
    fires_active:  u32,
    // MVP of last round
    mvp_name: Option<String>,
    mvp_show_ticks: u64,
    // Animation
    _start: Instant,
    tick:   u64,
}

const MENU_ITEMS: &[&str] = &[
    "FIND SERVER",
    "CREATE SERVER",
    "BUY MENU",
    "SCOREBOARD",
    "OPTIONS",
    "QUIT",
];

const BUY_CATS: &[BuyCategory] = &[
    BuyCategory { label: "Pistols",  kind: Some(WeaponKind::Pistol)     },
    BuyCategory { label: "Shotguns", kind: Some(WeaponKind::Shotgun)    },
    BuyCategory { label: "SMGs",     kind: Some(WeaponKind::SMG)        },
    BuyCategory { label: "Rifles",   kind: Some(WeaponKind::Rifle)      },
    BuyCategory { label: "Snipers",  kind: Some(WeaponKind::Sniper)     },
    BuyCategory { label: "Heavy",    kind: Some(WeaponKind::MachineGun) },
    BuyCategory { label: "Grenades", kind: Some(WeaponKind::Throwable)  },
];

impl App {
    fn new() -> Self {
        let catalog = WeaponCatalog::new();
        let buy_weapons: Vec<Vec<WeaponStats>> = BUY_CATS.iter().map(|cat| {
            let mut ws: Vec<WeaponStats> = catalog.all()
                .filter(|w| cat.kind.map_or(false, |k| w.kind == k))
                .cloned().collect();
            ws.sort_by(|a, b| a.price.cmp(&b.price));
            ws
        }).collect();

        let players = vec![
            PlayerRow { name: "Sniper_King".into(),    team: "CT", kills: 12, deaths: 4, assists: 3, hs_pct: 65, money: 4200, ping: 24, hp: 100, blind: false, has_nade: true  },
            PlayerRow { name: "HeadshotMaster".into(), team: "CT", kills:  8, deaths: 6, assists: 5, hs_pct: 42, money: 3400, ping: 31, hp: 85,  blind: true,  has_nade: false },
            PlayerRow { name: "SmokeArtist".into(),    team: "CT", kills: 10, deaths: 5, assists: 2, hs_pct: 55, money: 5000, ping: 18, hp: 100, blind: false, has_nade: true  },
            PlayerRow { name: "DefuseKit".into(),      team: "CT", kills:  6, deaths: 8, assists: 4, hs_pct: 38, money: 2800, ping: 44, hp: 70,  blind: false, has_nade: false },
            PlayerRow { name: "FlashBanger".into(),    team: "CT", kills:  9, deaths: 5, assists: 6, hs_pct: 60, money: 4750, ping: 27, hp: 100, blind: false, has_nade: true  },
            PlayerRow { name: "AWPer".into(),          team: "T",  kills: 14, deaths: 5, assists: 2, hs_pct: 72, money: 3700, ping: 35, hp: 100, blind: false, has_nade: false },
            PlayerRow { name: "RifleGod".into(),       team: "T",  kills:  7, deaths: 7, assists: 8, hs_pct: 45, money: 4200, ping: 29, hp: 100, blind: true,  has_nade: true  },
            PlayerRow { name: "BombPlanter".into(),    team: "T",  kills:  9, deaths: 6, assists: 3, hs_pct: 50, money: 1200, ping: 52, hp: 55,  blind: false, has_nade: false },
            PlayerRow { name: "EntryFragger".into(),   team: "T",  kills: 11, deaths: 4, assists: 5, hs_pct: 58, money: 5000, ping: 22, hp: 100, blind: false, has_nade: true  },
            PlayerRow { name: "Lurker".into(),         team: "T",  kills:  8, deaths: 8, assists: 4, hs_pct: 48, money: 3600, ping: 40, hp: 100, blind: false, has_nade: false },
        ];

        let kill_feed = vec![
            KillFeedEntry { killer: "AWPer".into(),      victim: "DefuseKit".into(),    weapon: "AWP".into(),   headshot: true,  age_ticks: 0  },
            KillFeedEntry { killer: "Sniper_King".into(), victim: "RifleGod".into(),    weapon: "AK-47".into(), headshot: false, age_ticks: 80 },
            KillFeedEntry { killer: "BombPlanter".into(), victim: "FlashBanger".into(), weapon: "AK-47".into(), headshot: true,  age_ticks: 160 },
        ];

        let round_history = vec![
            RoundOutcome::CT, RoundOutcome::CT, RoundOutcome::T,
            RoundOutcome::CT, RoundOutcome::T, RoundOutcome::CT,
            RoundOutcome::CT, RoundOutcome::T,
        ];

        App {
            screen: Screen::MainMenu,
            menu_sel: 0,
            buy_cat: 3,
            buy_sel: 0,
            buy_weapons,
            players,
            kill_feed,
            round_history,
            hp: 85, armor: 50, money: 4200,
            ammo_mag: 28, ammo_res: 90,
            weapon: "AK-47".into(),
            round: 9, time_secs: 98,
            ct_score: 6, t_score: 3,
            bomb_planted: false,
            bomb_frac: 0.65,
            defusing: false,
            defuse_frac: 0.4,
            smokes_active: 1,
            fires_active: 0,
            mvp_name: Some("AWPer".into()),
            mvp_show_ticks: 240,
            _start: Instant::now(),
            tick: 0,
        }
    }

    fn current_buy_list(&self) -> &[WeaponStats] {
        self.buy_weapons.get(self.buy_cat).map(|v| v.as_slice()).unwrap_or(&[])
    }

    fn handle_key(&mut self, code: KeyCode) -> bool {
        match &self.screen {
            Screen::MainMenu => match code {
                KeyCode::Up   | KeyCode::Char('k') => { self.menu_sel = self.menu_sel.saturating_sub(1); }
                KeyCode::Down | KeyCode::Char('j') => { self.menu_sel = (self.menu_sel + 1).min(MENU_ITEMS.len()-1); }
                KeyCode::Enter => match self.menu_sel {
                    2 => { self.screen = Screen::BuyMenu; }
                    3 => { self.screen = Screen::Scoreboard; }
                    5 => return true,
                    _ => { self.screen = Screen::Hud; }
                },
                KeyCode::Tab => { self.screen = Screen::Hud; }
                KeyCode::Char('q') | KeyCode::Char('Q') => return true,
                _ => {}
            },
            Screen::Hud => match code {
                KeyCode::Tab          => { self.screen = Screen::BuyMenu; }
                KeyCode::Char('b') | KeyCode::Char('B') => { self.screen = Screen::BuyMenu; }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.bomb_planted = !self.bomb_planted;
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.defusing = !self.defusing;
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => return true,
                KeyCode::Esc          => { self.screen = Screen::MainMenu; }
                _ => {}
            },
            Screen::BuyMenu => match code {
                KeyCode::Up   | KeyCode::Char('k') => { self.buy_sel = self.buy_sel.saturating_sub(1); }
                KeyCode::Down | KeyCode::Char('j') => {
                    let mx = self.current_buy_list().len().saturating_sub(1);
                    self.buy_sel = (self.buy_sel + 1).min(mx);
                }
                KeyCode::Left  | KeyCode::Char('h') => { self.buy_cat = self.buy_cat.saturating_sub(1); self.buy_sel = 0; }
                KeyCode::Right | KeyCode::Char('l') => { self.buy_cat = (self.buy_cat+1).min(BUY_CATS.len()-1); self.buy_sel = 0; }
                KeyCode::Char('1') => { self.buy_cat = 0; self.buy_sel = 0; }
                KeyCode::Char('2') => { self.buy_cat = 1; self.buy_sel = 0; }
                KeyCode::Char('3') => { self.buy_cat = 2; self.buy_sel = 0; }
                KeyCode::Char('4') => { self.buy_cat = 3; self.buy_sel = 0; }
                KeyCode::Char('5') => { self.buy_cat = 4; self.buy_sel = 0; }
                KeyCode::Char('6') => { self.buy_cat = 5; self.buy_sel = 0; }
                KeyCode::Char('7') => { self.buy_cat = 6; self.buy_sel = 0; }
                KeyCode::Tab         => { self.screen = Screen::Scoreboard; }
                KeyCode::Esc         => { self.screen = Screen::Hud; }
                KeyCode::Char('q') | KeyCode::Char('Q') => return true,
                _ => {}
            },
            Screen::Scoreboard => match code {
                KeyCode::Tab | KeyCode::Esc => { self.screen = Screen::MainMenu; }
                KeyCode::Char('q') | KeyCode::Char('Q') => return true,
                _ => {}
            },
        }
        false
    }

    fn tick_state(&mut self) {
        self.tick += 1;
        if self.tick % 60 == 0 && self.time_secs > 0 { self.time_secs -= 1; }
        // Age kill-feed entries
        for kf in &mut self.kill_feed { kf.age_ticks += 1; }
        self.kill_feed.retain(|kf| kf.age_ticks < 300);
        // Animate bomb / defuse
        if self.bomb_planted {
            self.bomb_frac = (self.bomb_frac - 0.0005).max(0.0);
        }
        if self.defusing {
            self.defuse_frac = (self.defuse_frac - 0.003).max(0.0);
        }
        // Age MVP banner
        if self.mvp_show_ticks > 0 { self.mvp_show_ticks -= 1; }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────
fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let tick_rate = Duration::from_millis(16);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &app))?;

        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_default();
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.handle_key(key.code) { break; }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.tick_state();
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

// ── Draw dispatcher ───────────────────────────────────────────────────────────
fn draw(f: &mut Frame, app: &App) {
    f.render_widget(Block::default().style(Style::default().bg(C_BG)), f.area());
    match &app.screen {
        Screen::MainMenu   => draw_main_menu(f, app),
        Screen::Hud        => draw_hud(f, app),
        Screen::BuyMenu    => draw_buy_menu(f, app),
        Screen::Scoreboard => draw_scoreboard(f, app),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────
fn styled_block<'a>(title: &'a str, border_color: Color) -> Block<'a> {
    Block::default()
        .title(Span::styled(format!(" {title} "), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(C_PANEL))
}

fn bar_line(filled: u32, total: u32, width: usize, color: Color) -> Line<'static> {
    let n = if total == 0 { 0 } else { (filled as usize * width / total as usize).min(width) };
    let f: String = std::iter::repeat('█').take(n).collect();
    let e: String = std::iter::repeat('░').take(width - n).collect();
    Line::from(vec![
        Span::styled(f, Style::default().fg(color)),
        Span::styled(e, Style::default().fg(C_BORDER)),
    ])
}

// ── Screen 1 : Main Menu ─────────────────────────────────────────────────────
const LOGO: &[&str] = &[
    r" ██████╗███████╗    ██████╗  ██████╗ ",
    r"██╔════╝██╔════╝   ██╔════╝ ██╔═══██╗",
    r"██║     ███████╗   ██║  ███╗██║   ██║",
    r"██║     ╚════██║   ██║   ██║██║   ██║",
    r"╚██████╗███████║   ╚██████╔╝╚██████╔╝",
    r" ╚═════╝╚══════╝    ╚═════╝  ╚═════╝ ",
];

fn draw_main_menu(f: &mut Frame, app: &App) {
    let area = f.area();
    let pulse = (app.tick / 30) % 2 == 0;

    let vert = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(LOGO.len() as u16 + 2),
        Constraint::Length(2),
        Constraint::Length(MENU_ITEMS.len() as u16 + 4),
        Constraint::Length(2),
        Constraint::Fill(1),
    ]).split(area);

    let logo_lines: Vec<Line> = LOGO.iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(C_CT).add_modifier(Modifier::BOLD))))
        .collect();
    f.render_widget(Paragraph::new(logo_lines).alignment(Alignment::Center), vert[1]);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("COUNTER-STRIKE: TACTICAL ENGINE  ", Style::default().fg(C_DIM)),
            Span::styled("Rust Edition v0.1.0  ·  FreeCS Inspired", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        ])).alignment(Alignment::Center),
        vert[2],
    );

    let menu_w = 32u16;
    let menu_x = area.width.saturating_sub(menu_w) / 2;
    let menu_rect = Rect::new(menu_x, vert[3].y, menu_w, vert[3].height);

    let items: Vec<ListItem> = MENU_ITEMS.iter().enumerate().map(|(i, &label)| {
        if i == app.menu_sel {
            ListItem::new(Line::from(vec![
                Span::styled(if pulse { "▶ " } else { "  " }, Style::default().fg(C_ACCENT)),
                Span::styled(label, Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(label, Style::default().fg(C_WHITE)),
            ]))
        }
    }).collect();

    f.render_widget(
        List::new(items).block(styled_block("MAIN MENU", C_BORDER)).style(Style::default().bg(C_PANEL)),
        menu_rect,
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑↓] ", Style::default().fg(C_ACCENT)), Span::styled("Navigate  ", Style::default().fg(C_DIM)),
            Span::styled("[Enter] ", Style::default().fg(C_ACCENT)), Span::styled("Select  ", Style::default().fg(C_DIM)),
            Span::styled("[Tab] ", Style::default().fg(C_ACCENT)), Span::styled("HUD  ", Style::default().fg(C_DIM)),
            Span::styled("[Q] ", Style::default().fg(C_ACCENT)), Span::styled("Quit", Style::default().fg(C_DIM)),
        ])).alignment(Alignment::Center),
        vert[4],
    );
}

// ── Screen 2 : HUD ────────────────────────────────────────────────────────────
fn draw_hud(f: &mut Frame, app: &App) {
    let area = f.area();
    let rows = Layout::vertical([
        Constraint::Length(3),  // top score bar
        Constraint::Length(3),  // round history ring
        Constraint::Fill(1),    // world view + sidebar
        Constraint::Length(3),  // bomb / defuse bar (conditional height)
        Constraint::Length(7),  // bottom HUD panel
    ]).split(area);

    draw_hud_topbar(f, rows[0], app);
    draw_round_history(f, rows[1], app);
    draw_hud_main(f, rows[2], app);
    draw_bomb_bar(f, rows[3], app);
    draw_hud_bottom(f, rows[4], app);
}

fn draw_hud_topbar(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([Constraint::Fill(1), Constraint::Length(22), Constraint::Fill(1)]).split(area);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  CT  ", Style::default().fg(C_CT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>2}", app.ct_score), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            Span::styled("  ●●●●●●●●", Style::default().fg(C_CT)),
        ])).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_CT)).style(Style::default().bg(C_PANEL)))
          .alignment(Alignment::Right),
        cols[0],
    );

    let mm = app.time_secs / 60;
    let ss = app.time_secs % 60;
    let (tc, tl) = if app.bomb_planted {
        (C_HP, format!("💣 BOMB  {:02}:{:02}", mm, ss))
    } else {
        (C_ACCENT, format!("◉ RD {:2}/30  {:02}:{:02}", app.round, mm, ss))
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(tl, Style::default().fg(tc).add_modifier(Modifier::BOLD))))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL)))
            .alignment(Alignment::Center),
        cols[1],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("●●●●●●●●  ", Style::default().fg(C_T)),
            Span::styled(format!("{:<2}", app.t_score), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            Span::styled("  T  ", Style::default().fg(C_T).add_modifier(Modifier::BOLD)),
        ])).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_T)).style(Style::default().bg(C_PANEL)))
          .alignment(Alignment::Left),
        cols[2],
    );
}

fn draw_round_history(f: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![Span::styled(" History: ", Style::default().fg(C_DIM))];
    for (i, &outcome) in app.round_history.iter().enumerate() {
        let (sym, col) = match outcome {
            RoundOutcome::CT => ("▲", C_CT),
            RoundOutcome::T  => ("▼", C_T),
        };
        spans.push(Span::styled(sym, Style::default().fg(col)));
        if (i + 1) % 5 == 0 { spans.push(Span::styled(" │ ", Style::default().fg(C_BORDER))); }
        else { spans.push(Span::raw(" ")); }
    }
    // Smoke / fire indicators
    if app.smokes_active > 0 {
        spans.push(Span::styled(format!("  ☁ ×{}", app.smokes_active), Style::default().fg(C_SMOKE)));
    }
    if app.fires_active > 0 {
        spans.push(Span::styled(format!("  🔥×{}", app.fires_active), Style::default().fg(C_FIRE)));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL)))
            .alignment(Alignment::Left),
        area,
    );
}

fn draw_hud_main(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([Constraint::Fill(1), Constraint::Length(28)]).split(area);
    draw_hud_world(f, cols[0], app);
    draw_hud_sidebar(f, cols[1], app);
}

fn draw_hud_world(f: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::vertical([Constraint::Fill(1), Constraint::Length(6)]).split(area);

    // World view with crosshair
    let mut world_lines = vec![
        Line::from(""),
        Line::from(Span::styled("[ de_dust2 — Bomb Defusal ]", Style::default().fg(C_DIM))),
        Line::from(""),
        Line::from(Span::styled("       ·─────·", Style::default().fg(C_DIM))),
        Line::from(Span::styled("       │  +  │", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("       ·─────·", Style::default().fg(C_DIM))),
        Line::from(""),
    ];
    if app.smokes_active > 0 {
        world_lines.push(Line::from(Span::styled("  ☁ SMOKE ACTIVE — visibility reduced", Style::default().fg(C_SMOKE))));
    }
    world_lines.push(Line::from(Span::styled(
        "  [B] Buy  [P] Toggle Bomb  [D] Toggle Defuse  [Tab] Scores  [Esc] Menu",
        Style::default().fg(C_DIM),
    )));

    f.render_widget(
        Paragraph::new(world_lines).block(styled_block("IN-GAME VIEW", C_BORDER)).alignment(Alignment::Center),
        rows[0],
    );

    // Kill feed
    draw_kill_feed(f, rows[1], app);
}

fn draw_kill_feed(f: &mut Frame, area: Rect, app: &App) {
    let recent: Vec<&KillFeedEntry> = app.kill_feed.iter().rev().take(4).collect();
    let items: Vec<ListItem> = recent.iter().map(|kf| {
        let alpha = 1.0 - (kf.age_ticks as f32 / 300.0);
        let col = if alpha > 0.6 { C_WHITE } else { C_DIM };
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:<14}", &kf.killer[..kf.killer.len().min(13)]), Style::default().fg(C_CT)),
            Span::styled(if kf.headshot { "☠HS " } else { "  ✖  " }, Style::default().fg(C_ACCENT)),
            Span::styled(format!("{:<12}", kf.weapon), Style::default().fg(col)),
            Span::styled(format!("{}", &kf.victim[..kf.victim.len().min(13)]), Style::default().fg(C_T)),
        ]))
    }).collect();

    f.render_widget(
        List::new(items)
            .block(Block::default().title(Span::styled(" Kill Feed ", Style::default().fg(C_DIM)))
                .borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(Style::default().fg(C_BORDER))
                .style(Style::default().bg(C_PANEL))),
        area,
    );
}

fn draw_hud_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let halves = Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).split(area);

    for (idx, (label, tag, color)) in [("CT SIDE", "CT", C_CT), ("T SIDE", "T", C_T)].iter().enumerate() {
        let team: Vec<&PlayerRow> = app.players.iter().filter(|p| p.team == *tag).collect();
        let items: Vec<ListItem> = team.iter().map(|p| {
            let hp_color = if p.hp > 60 { C_GREEN } else if p.hp > 30 { C_ACCENT } else { C_HP };
            let blind_icon = if p.blind { "☻" } else { " " };
            let nade_icon  = if p.has_nade { "●" } else { " " };
            ListItem::new(Line::from(vec![
                Span::styled(blind_icon, Style::default().fg(C_ACCENT)),
                Span::styled(nade_icon, Style::default().fg(C_GREEN)),
                Span::styled(format!(" {:<12}", &p.name[..p.name.len().min(12)]), Style::default().fg(C_WHITE)),
                Span::styled(format!("{:>3}", p.hp), Style::default().fg(hp_color)),
            ]))
        }).collect();

        f.render_widget(
            List::new(items)
                .block(Block::default()
                    .title(Span::styled(format!(" {label} "), Style::default().fg(*color).add_modifier(Modifier::BOLD)))
                    .borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(*color))
                    .style(Style::default().bg(C_PANEL))),
            halves[idx],
        );
    }
}

fn draw_bomb_bar(f: &mut Frame, area: Rect, app: &App) {
    if app.bomb_planted {
        // Bomb detonation timer
        let pct = (app.bomb_frac * 100.0) as u16;
        let col = if app.bomb_frac > 0.5 { C_ACCENT } else if app.bomb_frac > 0.2 { C_FIRE } else { C_HP };
        let label = format!(" C4 TIMER  {:>3}%  {} ", pct, if app.defusing { "[ DEFUSING... ]" } else { "" });
        let g = Gauge::default()
            .block(Block::default().title(Span::styled(label, Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL).border_style(Style::default().fg(col)).style(Style::default().bg(C_PANEL)))
            .gauge_style(Style::default().fg(col).bg(C_PANEL))
            .ratio(app.bomb_frac as f64);
        f.render_widget(g, area);

        // If defusing, overlay a second bar
        if app.defusing {
            let inner = Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1);
            let defuse_pct = (app.defuse_frac * 100.0) as u16;
            let dg = Gauge::default()
                .gauge_style(Style::default().fg(C_CT).bg(C_PANEL))
                .label(format!("Defuse {defuse_pct}%"))
                .ratio(app.defuse_frac as f64);
            f.render_widget(dg, inner);
        }
    } else {
        // MVP banner
        if app.mvp_show_ticks > 0 {
            if let Some(ref name) = app.mvp_name {
                let pulse = (app.tick / 20) % 2 == 0;
                let star = if pulse { "★" } else { "☆" };
                f.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(format!(" {star} MVP: "), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                        Span::styled(name.clone(), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
                        Span::styled("  — Best play of the round", Style::default().fg(C_DIM)),
                    ])).block(Block::default().borders(Borders::ALL)
                        .border_style(Style::default().fg(C_ACCENT))
                        .style(Style::default().bg(C_PANEL))),
                    area,
                );
            } else {
                draw_empty_bar(f, area);
            }
        } else {
            draw_empty_bar(f, area);
        }
    }
}

fn draw_empty_bar(f: &mut Frame, area: Rect) {
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  [B] Buy Menu  [P] Plant Bomb Demo  [D] Defuse Demo",
            Style::default().fg(C_DIM),
        ))).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        area,
    );
}

fn draw_hud_bottom(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([
        Constraint::Length(16),
        Constraint::Fill(1),
        Constraint::Length(18),
    ]).split(area);

    // Health & Armor
    let hp_bar  = bar_line(app.hp,    100, 10, C_HP);
    let arm_bar = bar_line(app.armor, 100, 10, C_ARMOR);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("♥ ", Style::default().fg(C_HP).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:>3}", app.hp), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            ]),
            hp_bar,
            Line::from(vec![
                Span::styled("⬡ ", Style::default().fg(C_ARMOR).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:>3}", app.armor), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            ]),
            arm_bar,
        ]).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        cols[0],
    );

    // Center: money + weapon + ammo + grenades
    let ammo_bar = bar_line(app.ammo_mag, 30, 14, C_GREEN);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("$", Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:>5}", app.money), Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD)),
                Span::raw("   "),
                Span::styled(&app.weapon, Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {:>2}/{:<3}", app.ammo_mag, app.ammo_res), Style::default().fg(C_WHITE)),
            ]),
            ammo_bar,
            Line::from(vec![
                Span::styled("Nades: ", Style::default().fg(C_DIM)),
                Span::styled("HE ", Style::default().fg(C_HP)),
                Span::styled("Flash ", Style::default().fg(C_ACCENT)),
                Span::styled("Smoke ", Style::default().fg(C_SMOKE)),
            ]),
            Line::from(Span::styled("[B] Buy  [Tab] Scores  [Esc] Menu  [P/D] Demo", Style::default().fg(C_DIM))),
        ]).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        cols[1],
    );

    // Radar
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("╔══════╗", Style::default().fg(C_GREEN))),
            Line::from(vec![
                Span::styled("║", Style::default().fg(C_GREEN)),
                Span::styled("CT   T", Style::default().fg(C_DIM)),
                Span::styled("║", Style::default().fg(C_GREEN)),
            ]),
            Line::from(vec![
                Span::styled("║", Style::default().fg(C_GREEN)),
                Span::styled("◉    ◉", Style::default().fg(C_CT)),
                Span::styled("║", Style::default().fg(C_GREEN)),
            ]),
            Line::from(Span::styled("╚══════╝", Style::default().fg(C_GREEN))),
        ]).block(Block::default()
            .title(Span::styled(" RADAR ", Style::default().fg(C_DIM)))
            .borders(Borders::ALL).border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_PANEL)))
          .alignment(Alignment::Center),
        cols[2],
    );
}

// ── Screen 3 : Buy Menu ───────────────────────────────────────────────────────
fn draw_buy_menu(f: &mut Frame, app: &App) {
    let area = f.area();
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(2),
    ]).split(area);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("BUY MENU", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("   Available: ", Style::default().fg(C_DIM)),
            Span::styled(format!("${}", app.money), Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD)),
            Span::styled("   Round: ", Style::default().fg(C_DIM)),
            Span::styled(format!("{}", app.round), Style::default().fg(C_WHITE)),
        ])).block(styled_block("", C_BORDER)).alignment(Alignment::Center),
        rows[0],
    );

    let tab_spans: Vec<Span> = BUY_CATS.iter().enumerate().map(|(i, cat)| {
        let lbl = format!(" [{}]{} ", i+1, cat.label);
        if i == app.buy_cat {
            Span::styled(lbl, Style::default().fg(C_BG).bg(C_ACCENT).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(lbl, Style::default().fg(C_DIM).bg(C_PANEL))
        }
    }).collect();
    f.render_widget(
        Paragraph::new(Line::from(tab_spans))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        rows[1],
    );

    let cols = Layout::horizontal([Constraint::Length(32), Constraint::Fill(1)]).split(rows[2]);
    draw_buy_list(f, cols[0], app);
    draw_buy_detail(f, cols[1], app);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑↓] ", Style::default().fg(C_ACCENT)), Span::styled("Select  ", Style::default().fg(C_DIM)),
            Span::styled("[←→/1-7] ", Style::default().fg(C_ACCENT)), Span::styled("Category  ", Style::default().fg(C_DIM)),
            Span::styled("[Enter] ", Style::default().fg(C_ACCENT)), Span::styled("Buy  ", Style::default().fg(C_DIM)),
            Span::styled("[Esc] ", Style::default().fg(C_ACCENT)), Span::styled("HUD  ", Style::default().fg(C_DIM)),
            Span::styled("[Tab] ", Style::default().fg(C_ACCENT)), Span::styled("Scoreboard  ", Style::default().fg(C_DIM)),
            Span::styled("[Q] ", Style::default().fg(C_ACCENT)), Span::styled("Quit", Style::default().fg(C_DIM)),
        ])).alignment(Alignment::Center),
        rows[3],
    );
}

fn draw_buy_list(f: &mut Frame, area: Rect, app: &App) {
    let weapons = app.current_buy_list();
    let items: Vec<ListItem> = weapons.iter().enumerate().map(|(i, w)| {
        let afford = w.price <= app.money;
        let pc = if afford { C_MONEY } else { C_HP };
        if i == app.buy_sel {
            ListItem::new(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(C_ACCENT)),
                Span::styled(format!("{:<18}", w.name), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("${:>4}", w.price), Style::default().fg(pc).add_modifier(Modifier::BOLD)),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:<18}", w.name), Style::default().fg(if afford { C_WHITE } else { C_DIM })),
                Span::styled(format!("${:>4}", w.price), Style::default().fg(pc)),
            ]))
        }
    }).collect();

    f.render_widget(
        List::new(items).block(styled_block(BUY_CATS[app.buy_cat].label, C_CT)).style(Style::default().bg(C_PANEL)),
        area,
    );
}

fn draw_buy_detail(f: &mut Frame, area: Rect, app: &App) {
    let weapons = app.current_buy_list();
    if weapons.is_empty() {
        f.render_widget(
            Paragraph::new("No weapons in this category.").block(styled_block("Details", C_BORDER)).style(Style::default().fg(C_DIM)),
            area,
        );
        return;
    }
    let w = &weapons[app.buy_sel.min(weapons.len()-1)];
    let can_afford = w.price <= app.money;
    let ap_pct = (w.armor_penetration * 100.0) as u32;

    let detail = vec![
        Line::from(Span::styled(w.name.as_str(), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED))),
        Line::from(vec![
            Span::styled(format!("{:?}", w.kind), Style::default().fg(C_DIM)),
            Span::raw("  "),
            Span::styled(if w.is_automatic { "AUTO" } else { "SEMI" }, Style::default().fg(if w.is_automatic { C_GREEN } else { C_CT })),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Price        ", Style::default().fg(C_DIM)), Span::styled(format!("${}", w.price), Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("Damage       ", Style::default().fg(C_DIM)), Span::styled(format!("{:.0}", w.base_damage), Style::default().fg(C_WHITE))]),
        bar_line(w.base_damage as u32, 120, 16, C_HP),
        Line::from(vec![Span::styled("Armor Pen    ", Style::default().fg(C_DIM)), Span::styled(format!("{ap_pct}%"), Style::default().fg(C_WHITE))]),
        bar_line(ap_pct, 100, 16, C_T),
        Line::from(vec![Span::styled("Recoil       ", Style::default().fg(C_DIM)), Span::styled(format!("{:.1}", w.recoil), Style::default().fg(C_WHITE))]),
        bar_line((w.recoil * 33.0) as u32, 100, 16, C_ACCENT),
        Line::from(vec![Span::styled("Fire Rate    ", Style::default().fg(C_DIM)), Span::styled(format!("{:.0} RPM", w.fire_rate_rpm), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("Magazine     ", Style::default().fg(C_DIM)), Span::styled(format!("{} + {}", w.magazine_size, w.reserve_ammo), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("Kill Reward  ", Style::default().fg(C_DIM)), Span::styled(format!("${}", w.kill_reward), Style::default().fg(C_MONEY))]),
        Line::from(""),
        if can_afford {
            Line::from(Span::styled("✔ You can afford this", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)))
        } else {
            Line::from(Span::styled(format!("✘ Need ${} more", w.price.saturating_sub(app.money)), Style::default().fg(C_HP).add_modifier(Modifier::BOLD)))
        },
        Line::from(""),
        Line::from(Span::styled("[Enter] Buy   [Esc] Back", Style::default().fg(C_DIM))),
    ];

    f.render_widget(
        Paragraph::new(detail).block(styled_block("WEAPON DETAILS", C_BORDER)).wrap(Wrap { trim: false }),
        area,
    );
}

// ── Screen 4 : Scoreboard ─────────────────────────────────────────────────────
fn draw_scoreboard(f: &mut Frame, app: &App) {
    let area = f.area();
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(2),
    ]).split(area);

    // Header
    let mm = app.time_secs / 60;
    let ss = app.time_secs % 60;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("SCOREBOARD", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("  —  de_dust2  ", Style::default().fg(C_DIM)),
            Span::styled(format!("Round {}/30", app.round), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            Span::styled(format!("   CT {}-{} T", app.ct_score, app.t_score), Style::default().fg(C_ACCENT)),
            Span::styled(format!("   {:02}:{:02}", mm, ss), Style::default().fg(C_DIM)),
        ])).block(styled_block("", C_BORDER)).alignment(Alignment::Center),
        rows[0],
    );

    // Round history ring
    let mut rh_spans = vec![Span::styled(" Rounds: ", Style::default().fg(C_DIM))];
    for &o in &app.round_history {
        let (sym, col) = match o {
            RoundOutcome::CT => ("■", C_CT),
            RoundOutcome::T  => ("■", C_T),
        };
        rh_spans.push(Span::styled(sym, Style::default().fg(col)));
        rh_spans.push(Span::raw(" "));
    }
    f.render_widget(
        Paragraph::new(Line::from(rh_spans))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        rows[1],
    );

    let body = Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).split(rows[2]);
    draw_score_team(f, body[0], app, "CT", C_CT, app.ct_score);
    draw_score_team(f, body[1], app, "T",  C_T,  app.t_score);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Tab/Esc] ", Style::default().fg(C_ACCENT)), Span::styled("Main Menu  ", Style::default().fg(C_DIM)),
            Span::styled("[Q] ", Style::default().fg(C_ACCENT)), Span::styled("Quit", Style::default().fg(C_DIM)),
        ])).alignment(Alignment::Center),
        rows[3],
    );
}

fn draw_score_team(f: &mut Frame, area: Rect, app: &App, tag: &str, color: Color, score: u32) {
    let header = Row::new(
        ["Player", "K", "D", "A", "HS%", "Money", "Ping", "HP", "Nades"]
            .iter().map(|h| Cell::from(*h).style(Style::default().fg(color).add_modifier(Modifier::BOLD)))
    ).height(1).style(Style::default().bg(Color::Rgb(20, 28, 40)));

    let team: Vec<&PlayerRow> = app.players.iter().filter(|p| p.team == tag).collect();
    let data_rows: Vec<Row> = team.iter().map(|p| {
        let hpc = if p.hp > 60 { C_GREEN } else if p.hp > 30 { C_ACCENT } else { C_HP };
        let blind_icon = if p.blind { "☻" } else { "-" };
        Row::new(vec![
            Cell::from(p.name.clone()).style(Style::default().fg(C_WHITE)),
            Cell::from(p.kills.to_string()).style(Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
            Cell::from(p.deaths.to_string()).style(Style::default().fg(C_HP)),
            Cell::from(p.assists.to_string()).style(Style::default().fg(C_DIM)),
            Cell::from(format!("{}%", p.hs_pct)).style(Style::default().fg(C_ACCENT)),
            Cell::from(format!("${}", p.money)).style(Style::default().fg(C_MONEY)),
            Cell::from(format!("{}ms", p.ping)).style(Style::default().fg(C_DIM)),
            Cell::from(p.hp.to_string()).style(Style::default().fg(hpc)),
            Cell::from(blind_icon).style(Style::default().fg(if p.blind { C_ACCENT } else { C_DIM })),
        ])
    }).collect();

    let label = if tag == "CT" { "CT SIDE" } else { "T SIDE" };
    f.render_widget(
        Table::new(data_rows, [
            Constraint::Fill(1),
            Constraint::Length(4), Constraint::Length(4), Constraint::Length(4),
            Constraint::Length(5), Constraint::Length(7),
            Constraint::Length(6), Constraint::Length(4), Constraint::Length(5),
        ])
        .header(header)
        .block(Block::default()
            .title(Span::styled(format!(" {label}  ▶  Score: {score} "), Style::default().fg(color).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(color))
            .style(Style::default().bg(C_PANEL)))
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD)),
        area,
    );
}
