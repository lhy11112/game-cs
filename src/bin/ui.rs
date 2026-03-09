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
    widgets::{Block, BorderType, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

// ── Palette ──────────────────────────────────────────────────────────────────
const C_BG: Color = Color::Rgb(10, 12, 16);
const C_PANEL: Color = Color::Rgb(20, 24, 32);
const C_BORDER: Color = Color::Rgb(45, 55, 72);
const C_ACCENT: Color = Color::Rgb(251, 191, 36);   // gold
const C_CT: Color = Color::Rgb(96, 165, 250);        // blue
const C_T: Color = Color::Rgb(251, 113, 133);        // red
const C_GREEN: Color = Color::Rgb(74, 222, 128);
const C_DIM: Color = Color::Rgb(100, 116, 139);
const C_WHITE: Color = Color::Rgb(226, 232, 240);
const C_HP: Color = Color::Rgb(239, 68, 68);
const C_ARMOR: Color = Color::Rgb(96, 165, 250);
const C_MONEY: Color = Color::Rgb(251, 191, 36);

// ── App State ─────────────────────────────────────────────────────────────────
#[derive(Clone, PartialEq)]
enum Screen {
    MainMenu,
    Hud,
    BuyMenu,
    Scoreboard,
}

#[derive(Clone)]
struct PlayerRow {
    name: String,
    team: &'static str,
    kills: u32,
    deaths: u32,
    assists: u32,
    hs_pct: u32,
    money: u32,
    ping: u32,
    hp: u32,
}

struct BuyCategory {
    label: &'static str,
    kind: Option<WeaponKind>,
}

struct App {
    screen: Screen,
    menu_sel: usize,
    buy_cat: usize,
    buy_sel: usize,
    buy_weapons: Vec<Vec<WeaponStats>>,
    players: Vec<PlayerRow>,
    // HUD state
    hp: u32,
    armor: u32,
    money: u32,
    ammo_mag: u32,
    ammo_res: u32,
    weapon: String,
    round: u32,
    time_secs: u32,
    ct_score: u32,
    t_score: u32,
    bomb_planted: bool,
    _start: Instant,
    tick: u64,
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
    BuyCategory { label: "Pistols",   kind: Some(WeaponKind::Pistol)     },
    BuyCategory { label: "Shotguns",  kind: Some(WeaponKind::Shotgun)    },
    BuyCategory { label: "SMGs",      kind: Some(WeaponKind::SMG)        },
    BuyCategory { label: "Rifles",    kind: Some(WeaponKind::Rifle)      },
    BuyCategory { label: "Snipers",   kind: Some(WeaponKind::Sniper)     },
    BuyCategory { label: "Heavy",     kind: Some(WeaponKind::MachineGun) },
    BuyCategory { label: "Grenades",  kind: Some(WeaponKind::Throwable)  },
];

impl App {
    fn new() -> Self {
        let catalog = WeaponCatalog::new();

        let buy_weapons: Vec<Vec<WeaponStats>> = BUY_CATS
            .iter()
            .map(|cat| {
                let mut ws: Vec<WeaponStats> = catalog
                    .all()
                    .filter(|w| cat.kind.map_or(false, |k| w.kind == k))
                    .cloned()
                    .collect();
                ws.sort_by(|a, b| a.price.cmp(&b.price));
                ws
            })
            .collect();

        let players = vec![
            PlayerRow { name: "Sniper_King".into(),    team: "CT", kills: 12, deaths: 4,  assists: 3, hs_pct: 65, money: 4200, ping: 24, hp: 100 },
            PlayerRow { name: "HeadshotMaster".into(), team: "CT", kills:  8, deaths: 6,  assists: 5, hs_pct: 42, money: 3400, ping: 31, hp: 85  },
            PlayerRow { name: "SmokeArtist".into(),    team: "CT", kills: 10, deaths: 5,  assists: 2, hs_pct: 55, money: 5000, ping: 18, hp: 100 },
            PlayerRow { name: "DefuseKit".into(),      team: "CT", kills:  6, deaths: 8,  assists: 4, hs_pct: 38, money: 2800, ping: 44, hp: 70  },
            PlayerRow { name: "FlashBanger".into(),    team: "CT", kills:  9, deaths: 5,  assists: 6, hs_pct: 60, money: 4750, ping: 27, hp: 100 },
            PlayerRow { name: "AWPer".into(),          team: "T",  kills: 14, deaths: 5,  assists: 2, hs_pct: 72, money: 3700, ping: 35, hp: 100 },
            PlayerRow { name: "RifleGod".into(),       team: "T",  kills:  7, deaths: 7,  assists: 8, hs_pct: 45, money: 4200, ping: 29, hp: 100 },
            PlayerRow { name: "BombPlanter".into(),    team: "T",  kills:  9, deaths: 6,  assists: 3, hs_pct: 50, money: 1200, ping: 52, hp: 55  },
            PlayerRow { name: "EntryFragger".into(),   team: "T",  kills: 11, deaths: 4,  assists: 5, hs_pct: 58, money: 5000, ping: 22, hp: 100 },
            PlayerRow { name: "Lurker".into(),         team: "T",  kills:  8, deaths: 8,  assists: 4, hs_pct: 48, money: 3600, ping: 40, hp: 100 },
        ];

        App {
            screen: Screen::MainMenu,
            menu_sel: 0,
            buy_cat: 3,
            buy_sel: 0,
            buy_weapons,
            players,
            hp: 85,
            armor: 50,
            money: 4200,
            ammo_mag: 28,
            ammo_res: 90,
            weapon: "AK-47".into(),
            round: 5,
            time_secs: 167,
            ct_score: 8,
            t_score: 6,
            bomb_planted: false,
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
                KeyCode::Down | KeyCode::Char('j') => { self.menu_sel = (self.menu_sel + 1).min(MENU_ITEMS.len() - 1); }
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
                KeyCode::Tab         => { self.screen = Screen::BuyMenu; }
                KeyCode::Char('b') | KeyCode::Char('B') => { self.screen = Screen::BuyMenu; }
                KeyCode::Char('q') | KeyCode::Char('Q') => return true,
                KeyCode::Esc         => { self.screen = Screen::MainMenu; }
                _ => {}
            },
            Screen::BuyMenu => match code {
                KeyCode::Up   | KeyCode::Char('k') => { self.buy_sel = self.buy_sel.saturating_sub(1); }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = self.current_buy_list().len().saturating_sub(1);
                    self.buy_sel = (self.buy_sel + 1).min(max);
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.buy_cat = self.buy_cat.saturating_sub(1);
                    self.buy_sel = 0;
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.buy_cat = (self.buy_cat + 1).min(BUY_CATS.len() - 1);
                    self.buy_sel = 0;
                }
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
                KeyCode::Tab         => { self.screen = Screen::MainMenu; }
                KeyCode::Esc         => { self.screen = Screen::MainMenu; }
                KeyCode::Char('q') | KeyCode::Char('Q') => return true,
                _ => {}
            },
        }
        false
    }

    fn tick_time(&mut self) {
        self.tick += 1;
        // Animate countdown every ~60 ticks (1s at 60fps)
        if self.tick % 60 == 0 && self.time_secs > 0 {
            self.time_secs -= 1;
        }
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
    let tick_rate = Duration::from_millis(16); // ~60fps
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &app))?;

        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_default();
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.handle_key(key.code) {
                        break;
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.tick_time();
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

// ── Draw dispatcher ───────────────────────────────────────────────────────────
fn draw(f: &mut Frame, app: &App) {
    // Full-screen dark background
    f.render_widget(
        Block::default().style(Style::default().bg(C_BG)),
        f.area(),
    );
    match &app.screen {
        Screen::MainMenu   => draw_main_menu(f, app),
        Screen::Hud        => draw_hud(f, app),
        Screen::BuyMenu    => draw_buy_menu(f, app),
        Screen::Scoreboard => draw_scoreboard(f, app),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────
fn styled_block(title: &str, border_color: Color) -> Block<'_> {
    Block::default()
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(C_PANEL))
}

fn bar(filled: u32, total: u32, width: usize, fill_ch: char, empty_ch: char, color: Color) -> Line<'static> {
    let n = if total == 0 { 0 } else { (filled as usize * width / total as usize).min(width) };
    let filled_str: String = std::iter::repeat(fill_ch).take(n).collect();
    let empty_str: String  = std::iter::repeat(empty_ch).take(width - n).collect();
    Line::from(vec![
        Span::styled(filled_str, Style::default().fg(color)),
        Span::styled(empty_str,  Style::default().fg(C_BORDER)),
    ])
}

// ── Screen 1: Main Menu ───────────────────────────────────────────────────────
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
    let pulse = ((app.tick / 30) % 2) == 0;

    let vert = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(LOGO.len() as u16 + 2),
        Constraint::Length(2),
        Constraint::Length(MENU_ITEMS.len() as u16 + 4),
        Constraint::Length(2),
        Constraint::Fill(1),
    ]).split(area);

    // Logo
    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(C_CT).add_modifier(Modifier::BOLD))))
        .collect();
    f.render_widget(
        Paragraph::new(logo_lines).alignment(Alignment::Center),
        vert[1],
    );

    // Subtitle
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("COUNTER-STRIKE: TACTICAL ENGINE  ", Style::default().fg(C_DIM)),
            Span::styled("Rust Edition v0.1.0", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        ])).alignment(Alignment::Center),
        vert[2],
    );

    // Menu box
    let menu_w = 30u16;
    let menu_x = area.width.saturating_sub(menu_w) / 2;
    let menu_rect = Rect::new(menu_x, vert[3].y, menu_w, vert[3].height);

    let items: Vec<ListItem> = MENU_ITEMS.iter().enumerate().map(|(i, &label)| {
        if i == app.menu_sel {
            ListItem::new(Line::from(vec![
                Span::styled(
                    if pulse { "▶ " } else { "  " },
                    Style::default().fg(C_ACCENT),
                ),
                Span::styled(
                    label,
                    Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                ),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(label, Style::default().fg(C_WHITE)),
            ]))
        }
    }).collect();

    f.render_widget(
        List::new(items)
            .block(styled_block("MAIN MENU", C_BORDER))
            .style(Style::default().bg(C_PANEL)),
        menu_rect,
    );

    // Footer
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑↓] ", Style::default().fg(C_ACCENT)),
            Span::styled("Navigate  ", Style::default().fg(C_DIM)),
            Span::styled("[Enter] ", Style::default().fg(C_ACCENT)),
            Span::styled("Select  ", Style::default().fg(C_DIM)),
            Span::styled("[Tab] ", Style::default().fg(C_ACCENT)),
            Span::styled("HUD Demo  ", Style::default().fg(C_DIM)),
            Span::styled("[Q] ", Style::default().fg(C_ACCENT)),
            Span::styled("Quit", Style::default().fg(C_DIM)),
        ])).alignment(Alignment::Center),
        vert[4],
    );
}

// ── Screen 2: HUD ─────────────────────────────────────────────────────────────
fn draw_hud(f: &mut Frame, app: &App) {
    let area = f.area();

    let rows = Layout::vertical([
        Constraint::Length(3),  // top bar
        Constraint::Fill(1),    // main area
        Constraint::Length(6),  // bottom HUD
    ]).split(area);

    draw_hud_topbar(f, rows[0], app);
    draw_hud_main(f, rows[1], app);
    draw_hud_bottom(f, rows[2], app);
}

fn draw_hud_topbar(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(22),
        Constraint::Fill(1),
    ]).split(area);

    // CT score side
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  CT  ", Style::default().fg(C_CT).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:>2}", app.ct_score),
                Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ●●●●●●●●", Style::default().fg(C_CT)),
        ])).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_CT)).style(Style::default().bg(C_PANEL))
        ).alignment(Alignment::Right),
        cols[0],
    );

    // Center: round + time
    let mm = app.time_secs / 60;
    let ss = app.time_secs % 60;
    let (time_color, time_label) = if app.bomb_planted {
        (C_HP, format!("💣 BOMB PLANTED  {:02}:{:02}", mm, ss))
    } else {
        (C_ACCENT, format!("◉ ROUND {:2}/30   {:02}:{:02}", app.round, mm, ss))
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            time_label,
            Style::default().fg(time_color).add_modifier(Modifier::BOLD),
        ))).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))
        ).alignment(Alignment::Center),
        cols[1],
    );

    // T score side
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("●●●●●●●●  ", Style::default().fg(C_T)),
            Span::styled(
                format!("{:<2}", app.t_score),
                Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  T  ", Style::default().fg(C_T).add_modifier(Modifier::BOLD)),
        ])).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_T)).style(Style::default().bg(C_PANEL))
        ).alignment(Alignment::Left),
        cols[2],
    );
}

fn draw_hud_main(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(26),
    ]).split(area);

    // Game world view (crosshair + map)
    let world_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "[ de_dust2 — Bomb Defusal ]",
            Style::default().fg(C_DIM),
        )),
        Line::from(""),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "          ·─────·",
            Style::default().fg(C_DIM),
        )),
        Line::from(Span::styled(
            "          │  +  │",
            Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "          ·─────·",
            Style::default().fg(C_DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  [B] Buy Menu  [Tab] Scoreboard  [Esc] Main Menu",
            Style::default().fg(C_DIM),
        )),
    ];
    f.render_widget(
        Paragraph::new(world_lines)
            .block(styled_block("IN-GAME VIEW", C_BORDER))
            .alignment(Alignment::Center),
        cols[0],
    );

    // Player list sidebar
    draw_hud_sidebar(f, cols[1], app);
}

fn draw_hud_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let halves = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Fill(1),
    ]).split(area);

    for (idx, (team_label, team_tag, color)) in
        [("CT SIDE", "CT", C_CT), ("T SIDE", "T", C_T)].iter().enumerate()
    {
        let team_players: Vec<&PlayerRow> = app.players.iter().filter(|p| p.team == *team_tag).collect();
        let items: Vec<ListItem> = team_players.iter().map(|p| {
            let hp_color = if p.hp > 60 { C_GREEN } else if p.hp > 30 { C_ACCENT } else { C_HP };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<14}", &p.name[..p.name.len().min(13)]), Style::default().fg(C_WHITE)),
                Span::styled(format!("{:>3}hp", p.hp), Style::default().fg(hp_color)),
            ]))
        }).collect();

        f.render_widget(
            List::new(items)
                .block(Block::default()
                    .title(Span::styled(
                        format!(" {team_label} "),
                        Style::default().fg(*color).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(*color))
                    .style(Style::default().bg(C_PANEL)))
                .style(Style::default().fg(C_DIM)),
            halves[idx],
        );
    }
}

fn draw_hud_bottom(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([
        Constraint::Length(16),
        Constraint::Fill(1),
        Constraint::Length(18),
    ]).split(area);

    // Health & Armor
    let hp_bar  = bar(app.hp,    100, 10, '█', '░', C_HP);
    let arm_bar = bar(app.armor, 100, 10, '█', '░', C_ARMOR);
    let vitals = vec![
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
    ];
    f.render_widget(
        Paragraph::new(vitals)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        cols[0],
    );

    // Center: weapon + ammo + money
    let ammo_bar = bar(app.ammo_mag, 30, 14, '|', '·', C_GREEN);
    let center = vec![
        Line::from(vec![
            Span::styled("$", Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>5}", app.money), Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD)),
            Span::raw("   "),
            Span::styled(&app.weapon, Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("  {:>2} / {:<3}", app.ammo_mag, app.ammo_res),
                Style::default().fg(C_WHITE),
            ),
        ]),
        ammo_bar,
        Line::from(vec![
            Span::styled("Slots: ", Style::default().fg(C_DIM)),
            Span::styled("[1]Knife ", Style::default().fg(C_DIM)),
            Span::styled("[2]USP-S ", Style::default().fg(C_DIM)),
            Span::styled("[3]AK-47 ", Style::default().fg(C_ACCENT)),
            Span::styled("[4]HE ", Style::default().fg(C_DIM)),
        ]),
        Line::from(Span::styled(
            "[B] Buy  [Tab] Scores  [Esc] Menu",
            Style::default().fg(C_DIM),
        )),
    ];
    f.render_widget(
        Paragraph::new(center)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        cols[1],
    );

    // Radar placeholder
    let radar_lines = vec![
        Line::from(Span::styled("╔═══════╗", Style::default().fg(C_GREEN))),
        Line::from(vec![
            Span::styled("║", Style::default().fg(C_GREEN)),
            Span::styled(" CT  T ", Style::default().fg(C_DIM)),
            Span::styled("║", Style::default().fg(C_GREEN)),
        ]),
        Line::from(vec![
            Span::styled("║", Style::default().fg(C_GREEN)),
            Span::styled(" ◉    ◉ ", Style::default().fg(C_CT)),
            Span::styled("║", Style::default().fg(C_GREEN)),
        ]),
        Line::from(Span::styled("╚═══════╝", Style::default().fg(C_GREEN))),
    ];
    f.render_widget(
        Paragraph::new(radar_lines)
            .block(Block::default().title(Span::styled(" RADAR ", Style::default().fg(C_DIM))).borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL)))
            .alignment(Alignment::Center),
        cols[2],
    );
}

// ── Screen 3: Buy Menu ────────────────────────────────────────────────────────
fn draw_buy_menu(f: &mut Frame, app: &App) {
    let area = f.area();

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(2),
    ]).split(area);

    // Title
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("BUY MENU", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("   Money Available: ", Style::default().fg(C_DIM)),
            Span::styled(format!("${}", app.money), Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD)),
        ])).block(styled_block("", C_BORDER)).alignment(Alignment::Center),
        rows[0],
    );

    // Category tabs
    let tab_spans: Vec<Span> = BUY_CATS.iter().enumerate().map(|(i, cat)| {
        let label = format!(" [{}]{} ", i + 1, cat.label);
        if i == app.buy_cat {
            Span::styled(label, Style::default().fg(C_BG).bg(C_ACCENT).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(label, Style::default().fg(C_DIM).bg(C_PANEL))
        }
    }).collect();
    f.render_widget(
        Paragraph::new(Line::from(tab_spans))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        rows[1],
    );

    // Split: weapon list | detail panel
    let cols = Layout::horizontal([
        Constraint::Length(32),
        Constraint::Fill(1),
    ]).split(rows[2]);

    draw_buy_list(f, cols[0], app);
    draw_buy_detail(f, cols[1], app);

    // Footer
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑↓] ", Style::default().fg(C_ACCENT)),
            Span::styled("Select  ", Style::default().fg(C_DIM)),
            Span::styled("[←→/1-7] ", Style::default().fg(C_ACCENT)),
            Span::styled("Category  ", Style::default().fg(C_DIM)),
            Span::styled("[Enter] ", Style::default().fg(C_ACCENT)),
            Span::styled("Buy  ", Style::default().fg(C_DIM)),
            Span::styled("[Esc] ", Style::default().fg(C_ACCENT)),
            Span::styled("HUD  ", Style::default().fg(C_DIM)),
            Span::styled("[Tab] ", Style::default().fg(C_ACCENT)),
            Span::styled("Scoreboard  ", Style::default().fg(C_DIM)),
            Span::styled("[Q] ", Style::default().fg(C_ACCENT)),
            Span::styled("Quit", Style::default().fg(C_DIM)),
        ])).alignment(Alignment::Center),
        rows[3],
    );
}

fn draw_buy_list(f: &mut Frame, area: Rect, app: &App) {
    let weapons = app.current_buy_list();
    let items: Vec<ListItem> = weapons.iter().enumerate().map(|(i, w)| {
        let can_afford = w.price <= app.money;
        let price_color = if can_afford { C_MONEY } else { C_HP };
        if i == app.buy_sel {
            ListItem::new(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(C_ACCENT)),
                Span::styled(format!("{:<18}", w.name), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("${:>4}", w.price), Style::default().fg(price_color).add_modifier(Modifier::BOLD)),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:<18}", w.name), Style::default().fg(if can_afford { C_WHITE } else { C_DIM })),
                Span::styled(format!("${:>4}", w.price), Style::default().fg(price_color)),
            ]))
        }
    }).collect();

    let title = BUY_CATS[app.buy_cat].label;
    f.render_widget(
        List::new(items)
            .block(styled_block(title, C_CT))
            .style(Style::default().bg(C_PANEL)),
        area,
    );
}

fn draw_buy_detail(f: &mut Frame, area: Rect, app: &App) {
    let weapons = app.current_buy_list();
    if weapons.is_empty() {
        f.render_widget(
            Paragraph::new("No weapons in this category.")
                .block(styled_block("Details", C_BORDER))
                .style(Style::default().fg(C_DIM)),
            area,
        );
        return;
    }

    let w = &weapons[app.buy_sel.min(weapons.len() - 1)];
    let can_afford = w.price <= app.money;

    let ap_pct = (w.armor_penetration * 100.0) as u32;
    let ap_bar  = bar(ap_pct, 100, 16, '█', '░', C_T);
    let dmg_bar = bar(w.base_damage as u32, 120, 16, '█', '░', C_HP);
    let rpc_bar = bar((w.recoil * 33.0) as u32, 100, 16, '█', '░', C_ACCENT);

    let auto_tag = if w.is_automatic { "AUTO" } else { "SEMI" };
    let afford_line = if can_afford {
        Line::from(Span::styled("✔ You can afford this weapon", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)))
    } else {
        Line::from(Span::styled(
            format!("✘ Need ${} more", w.price.saturating_sub(app.money)),
            Style::default().fg(C_HP).add_modifier(Modifier::BOLD),
        ))
    };

    let kind_str = format!("{:?}", w.kind);

    let detail = vec![
        Line::from(Span::styled(&w.name, Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED))),
        Line::from(vec![
            Span::styled(kind_str, Style::default().fg(C_DIM)),
            Span::raw("  "),
            Span::styled(auto_tag, Style::default().fg(if w.is_automatic { C_GREEN } else { C_CT })),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Price       ", Style::default().fg(C_DIM)), Span::styled(format!("${}", w.price), Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("Damage      ", Style::default().fg(C_DIM)), Span::styled(format!("{:.0}", w.base_damage), Style::default().fg(C_WHITE))]),
        dmg_bar,
        Line::from(vec![Span::styled("Armor Pen   ", Style::default().fg(C_DIM)), Span::styled(format!("{ap_pct}%"), Style::default().fg(C_WHITE))]),
        ap_bar,
        Line::from(vec![Span::styled("Recoil      ", Style::default().fg(C_DIM)), Span::styled(format!("{:.1}", w.recoil), Style::default().fg(C_WHITE))]),
        rpc_bar,
        Line::from(vec![Span::styled("Fire Rate   ", Style::default().fg(C_DIM)), Span::styled(format!("{:.0} RPM", w.fire_rate_rpm), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("Magazine    ", Style::default().fg(C_DIM)), Span::styled(format!("{} / {}", w.magazine_size, w.magazine_size + w.reserve_ammo), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("Kill Reward ", Style::default().fg(C_DIM)), Span::styled(format!("${}", w.kill_reward), Style::default().fg(C_MONEY))]),
        Line::from(""),
        afford_line,
        Line::from(""),
        Line::from(Span::styled("[Enter] Buy   [Esc] Back", Style::default().fg(C_DIM))),
    ];

    f.render_widget(
        Paragraph::new(detail)
            .block(styled_block("WEAPON DETAILS", C_BORDER))
            .wrap(Wrap { trim: false }),
        area,
    );
}

// ── Screen 4: Scoreboard ──────────────────────────────────────────────────────
fn draw_scoreboard(f: &mut Frame, app: &App) {
    let area = f.area();

    let rows = Layout::vertical([
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
            Span::styled(format!("  {:02}:{:02}", mm, ss), Style::default().fg(C_ACCENT)),
        ])).block(styled_block("", C_BORDER)).alignment(Alignment::Center),
        rows[0],
    );

    let body = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Fill(1),
    ]).split(rows[1]);

    draw_score_team(f, body[0], app, "CT", C_CT, app.ct_score);
    draw_score_team(f, body[1], app, "T",  C_T,  app.t_score);

    // Footer
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Tab] ", Style::default().fg(C_ACCENT)),
            Span::styled("Main Menu  ", Style::default().fg(C_DIM)),
            Span::styled("[Esc] ", Style::default().fg(C_ACCENT)),
            Span::styled("Main Menu  ", Style::default().fg(C_DIM)),
            Span::styled("[Q] ", Style::default().fg(C_ACCENT)),
            Span::styled("Quit", Style::default().fg(C_DIM)),
        ])).alignment(Alignment::Center),
        rows[2],
    );
}

fn draw_score_team(f: &mut Frame, area: Rect, app: &App, team_tag: &str, color: Color, score: u32) {
    let header_cells = ["Player", "K", "D", "A", "HS%", "Money", "Ping", "HP"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(color).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(0)
        .style(Style::default().bg(Color::Rgb(20, 28, 40)));

    let team_players: Vec<&PlayerRow> = app.players.iter().filter(|p| p.team == team_tag).collect();

    let data_rows: Vec<Row> = team_players.iter().map(|p| {
        let hp_color = if p.hp > 60 { C_GREEN } else if p.hp > 30 { C_ACCENT } else { C_HP };
        Row::new(vec![
            Cell::from(p.name.clone()).style(Style::default().fg(C_WHITE)),
            Cell::from(p.kills.to_string()).style(Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
            Cell::from(p.deaths.to_string()).style(Style::default().fg(C_HP)),
            Cell::from(p.assists.to_string()).style(Style::default().fg(C_DIM)),
            Cell::from(format!("{}%", p.hs_pct)).style(Style::default().fg(C_ACCENT)),
            Cell::from(format!("${}", p.money)).style(Style::default().fg(C_MONEY)),
            Cell::from(format!("{}ms", p.ping)).style(Style::default().fg(C_DIM)),
            Cell::from(format!("{}", p.hp)).style(Style::default().fg(hp_color)),
        ])
    }).collect();

    let team_label = if team_tag == "CT" { "CT SIDE" } else { "T SIDE" };
    let title = format!(" {team_label}  ▶  Score: {score} ");

    f.render_widget(
        Table::new(
            data_rows,
            [
                Constraint::Fill(1),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(5),
                Constraint::Length(7),
                Constraint::Length(6),
                Constraint::Length(4),
            ],
        )
        .header(header)
        .block(Block::default()
            .title(Span::styled(title, Style::default().fg(color).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(color))
            .style(Style::default().bg(C_PANEL)))
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD)),
        area,
    );
}
