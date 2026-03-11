//! world.rs — 3D First-Person CS Map Explorer (Raycasting Engine)
//!
//! Controls:
//!   W / ↑        Move forward
//!   S / ↓        Move backward
//!   A            Strafe left
//!   D            Strafe right
//!   ← / →        Turn left / right (keyboard)
//!   Mouse X      Look left / right
//!   Scroll       Look up / down (pitch)
//!   Q / Esc      Quit

use crossterm::{
    event::{
        self, Event, KeyCode, KeyEventKind, MouseEventKind,
        EnableMouseCapture, DisableMouseCapture,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    collections::HashSet,
    io,
    time::{Duration, Instant},
};

// ── Constants ──────────────────────────────────────────────────────────────────
const FOV: f64 = std::f64::consts::PI / 3.0; // 60° horizontal field of view
const MOVE_SPEED: f64 = 4.0;                  // units / second
const TURN_SPEED: f64 = 2.5;                  // radians / second (keyboard)
const MOUSE_SENS: f64 = 0.006;               // radians / pixel (horizontal mouse)
const PLAYER_MARGIN: f64 = 0.22;             // collision radius

// ── Map definition ────────────────────────────────────────────────────────────
// Cell legend:
//   0 = open floor
//   1 = regular wall (tan/sandy)
//   2 = Bomb Site A wall (green)
//   3 = Bomb Site B wall (red)
//   4 = CT spawn marker wall (blue)
//   5 = T spawn marker wall (orange)
const MAP_W: usize = 24;
const MAP_H: usize = 24;

#[rustfmt::skip]
const MAP: [[u8; MAP_W]; MAP_H] = [
    [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
    [1,5,5,0,0,0,0,0,1,0,1,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,1,1,0,0,0,0,0,0,0,0,0,0,0,1,1,0,0,0,0,0,1],
    [1,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,0,2,2,2,2,2,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,0,2,0,0,0,2,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,1,1,1,0,0,0,2,0,0,0,2,0,0,0,0,0,1,1,1,1,1,1,1],
    [1,0,0,0,0,0,0,2,2,2,2,2,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,1,1,0,0,0,0,0,0,0,0,0,0,0,1,1,0,0,0,0,0,1],
    [1,0,0,0,1,0,3,3,3,3,3,0,0,0,0,0,1,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,3,0,0,0,3,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,3,0,0,0,3,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,3,3,3,3,3,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
    [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,0,0,0,0,1],
    [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4,0,0,0,0,1],
    [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4,0,0,0,0,1],
    [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
];

fn is_solid(x: i64, y: i64) -> bool {
    if x < 0 || y < 0 || x >= MAP_W as i64 || y >= MAP_H as i64 {
        return true;
    }
    MAP[y as usize][x as usize] != 0
}

fn cell_type(x: i64, y: i64) -> u8 {
    if x < 0 || y < 0 || x >= MAP_W as i64 || y >= MAP_H as i64 {
        return 1;
    }
    MAP[y as usize][x as usize]
}

// ── Player ────────────────────────────────────────────────────────────────────
struct Player {
    x: f64,
    y: f64,
    angle: f64, // yaw radians: 0 = +X (east), π/2 = +Y (south)
    pitch: f64, // cosmetic vertical offset, clamped -0.8..0.8
}

impl Player {
    fn new() -> Self {
        Player { x: 2.5, y: 2.5, angle: 0.8, pitch: 0.0 }
    }

    fn update(&mut self, keys: &HashSet<KeyCode>, dt: f64) {
        let turn = TURN_SPEED * dt;
        let ms   = MOVE_SPEED * dt;

        if keys.contains(&KeyCode::Left)  { self.angle -= turn; }
        if keys.contains(&KeyCode::Right) { self.angle += turn; }

        let ca = self.angle.cos();
        let sa = self.angle.sin();
        let mut dx = 0.0f64;
        let mut dy = 0.0f64;

        // Forward / backward
        let fwd = keys.contains(&KeyCode::Up)
            || keys.contains(&KeyCode::Char('w'));
        let bwd = keys.contains(&KeyCode::Down)
            || keys.contains(&KeyCode::Char('s'));
        if fwd  { dx += ca; dy += sa; }
        if bwd  { dx -= ca; dy -= sa; }

        // Strafing
        if keys.contains(&KeyCode::Char('a')) { dx += sa; dy -= ca; }
        if keys.contains(&KeyCode::Char('d')) { dx -= sa; dy += ca; }

        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.01 {
            self.move_slide(dx / len * ms, dy / len * ms);
        }
    }

    fn move_slide(&mut self, dx: f64, dy: f64) {
        let m = PLAYER_MARGIN;
        let nx = self.x + dx;
        let ny = self.y + dy;

        let clear = |tx: f64, ty: f64| -> bool {
            !is_solid((tx - m) as i64, (ty - m) as i64)
                && !is_solid((tx + m) as i64, (ty - m) as i64)
                && !is_solid((tx - m) as i64, (ty + m) as i64)
                && !is_solid((tx + m) as i64, (ty + m) as i64)
        };

        if clear(nx, ny) {
            self.x = nx;
            self.y = ny;
        } else if clear(nx, self.y) {
            self.x = nx;
        } else if clear(self.x, ny) {
            self.y = ny;
        }
    }
}

// ── DDA Raycasting ────────────────────────────────────────────────────────────
/// Returns (perpendicular_distance, cell_type, is_ns_wall)
///   is_ns_wall = true  → hit a North/South face (horizontal wall)
///   is_ns_wall = false → hit an East/West face  (vertical wall)
fn cast_ray(px: f64, py: f64, angle: f64) -> (f64, u8, bool) {
    let rdx = angle.cos();
    let rdy = angle.sin();

    let map_xi = px as i64;
    let map_yi = py as i64;

    let ddx = if rdx.abs() < 1e-10 { f64::MAX } else { (1.0 / rdx).abs() };
    let ddy = if rdy.abs() < 1e-10 { f64::MAX } else { (1.0 / rdy).abs() };

    let (step_x, mut sdx) = if rdx < 0.0 {
        (-1i64, (px - map_xi as f64) * ddx)
    } else {
        (1i64, (map_xi as f64 + 1.0 - px) * ddx)
    };
    let (step_y, mut sdy) = if rdy < 0.0 {
        (-1i64, (py - map_yi as f64) * ddy)
    } else {
        (1i64, (map_yi as f64 + 1.0 - py) * ddy)
    };

    let mut mx = map_xi;
    let mut my = map_yi;
    let mut hit_ns;

    for _ in 0..64 {
        if sdx < sdy {
            sdx += ddx;
            mx  += step_x;
            hit_ns = false; // East/West face
        } else {
            sdy += ddy;
            my  += step_y;
            hit_ns = true;  // North/South face
        }

        let cell = cell_type(mx, my);
        if cell != 0 {
            let perp = if !hit_ns { sdx - ddx } else { sdy - ddy };
            return (perp.max(0.01), cell, hit_ns);
        }
    }
    (60.0, 1, false)
}

// ── Wall colour by cell type and face ─────────────────────────────────────────
fn wall_color(cell: u8, brightness: u8, ns_face: bool) -> Color {
    // N/S faces are slightly darker (shading effect)
    let b = if ns_face { brightness.saturating_sub(45) } else { brightness };
    match cell {
        2 => Color::Rgb(0, b, b / 3),                        // Site A — teal/green
        3 => Color::Rgb(b, b / 8, 0),                        // Site B — red/orange
        4 => Color::Rgb(b / 6, b / 3, b),                   // CT spawn — blue
        5 => Color::Rgb(b, b / 2, 0),                        // T  spawn — amber
        _ => Color::Rgb(b, (b as u16 * 7 / 8) as u8,        // default — sandy tan
                          (b as u16 * 5 / 8) as u8),
    }
}

fn shade_char(dist: f64) -> &'static str {
    if      dist < 1.5 { "█" }
    else if dist < 3.5 { "▓" }
    else if dist < 7.0 { "▒" }
    else               { "░" }
}

// ── 3D Render ─────────────────────────────────────────────────────────────────
struct ColData {
    top:  isize,
    bot:  isize,
    dist: f64,
    cell: u8,
    ns:   bool,
}

fn render_3d(player: &Player, w: usize, h: usize) -> Vec<Line<'static>> {
    if w == 0 || h == 0 {
        return vec![];
    }

    let half_h = h as f64 / 2.0;
    let pitch_shift = (player.pitch * half_h * 0.75) as isize;

    // Pre-compute a ColData for every screen column
    let cols: Vec<ColData> = (0..w)
        .map(|x| {
            let ray_a = player.angle - FOV / 2.0 + FOV * (x as f64 / w as f64);
            let (dist, cell, ns) = cast_ray(player.x, player.y, ray_a);

            // Terminal chars are ~2× taller than wide → halve wall height
            let wall_h = ((half_h * 1.0) / dist).min(half_h * 2.0) as isize;
            let center  = half_h as isize + pitch_shift;

            ColData {
                top:  center - wall_h / 2,
                bot:  center + wall_h / 2,
                dist, cell, ns,
            }
        })
        .collect();

    // Build one Line per row
    let mut lines: Vec<Line<'static>> = (0..h)
        .map(|y| {
            let yi = y as isize;
            let spans: Vec<Span<'static>> = (0..w)
                .map(|x| {
                    let c = &cols[x];
                    if yi < c.top {
                        // Ceiling — darker near top, lighter near horizon
                        let t = (c.top - yi) as f64 / half_h;
                        let i = ((1.0 - t) * 55.0).max(4.0) as u8;
                        Span::styled(
                            " ",
                            Style::default().bg(Color::Rgb(i / 2, i / 2, i + 10)),
                        )
                    } else if yi >= c.bot {
                        // Floor — lighter near horizon, darker near bottom
                        let t = (yi - c.bot) as f64 / half_h;
                        let i = ((1.0 - t.min(1.0)) * 55.0).max(4.0) as u8;
                        let dot = if (x + y) % 5 == 0 { "·" } else { " " };
                        Span::styled(
                            dot,
                            Style::default()
                                .fg(Color::Rgb(i / 3, i / 3, i / 5))
                                .bg(Color::Rgb(6, 9, 6)),
                        )
                    } else {
                        // Wall
                        let bright =
                            (220.0 / c.dist.max(0.3)).min(220.0) as u8;
                        Span::styled(
                            shade_char(c.dist),
                            Style::default().fg(wall_color(c.cell, bright, c.ns)),
                        )
                    }
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    // ── Crosshair overlay ─────────────────────────────────────────────────────
    let cy = h / 2;
    let cx = w / 2;
    let white = Style::default().fg(Color::Rgb(220, 220, 220));
    let set = |lines: &mut Vec<Line<'static>>, ry: usize, rx: usize, ch: &'static str| {
        if ry < lines.len() {
            if let Some(sp) = lines[ry].spans.get_mut(rx) {
                *sp = Span::styled(ch, white);
            }
        }
    };
    set(&mut lines, cy, cx, "┼");
    for d in 1..=3 {
        if cx >= d { set(&mut lines, cy, cx - d, "─"); }
        if cx + d < w { set(&mut lines, cy, cx + d, "─"); }
    }
    if cy > 0 { set(&mut lines, cy - 1, cx, "│"); }
    if cy + 1 < h { set(&mut lines, cy + 1, cx, "│"); }

    lines
}

// ── Mini-map ──────────────────────────────────────────────────────────────────
fn render_minimap(player: &Player, w: usize, h: usize) -> Vec<Line<'static>> {
    let cx = player.x as usize;
    let cy = player.y as usize;
    let rx = (w / 2).min(8);
    let ry = (h / 2).min(6);

    let y0 = cy.saturating_sub(ry);
    let y1 = (cy + ry + 1).min(MAP_H);
    let x0 = cx.saturating_sub(rx);
    let x1 = (cx + rx + 1).min(MAP_W);

    (y0..y1)
        .map(|my| {
            let spans: Vec<Span<'static>> = (x0..x1)
                .map(|mx| {
                    if mx == cx && my == cy {
                        Span::styled("@", Style::default().fg(Color::Yellow))
                    } else {
                        match MAP[my][mx] {
                            0 => Span::styled(
                                "·",
                                Style::default().fg(Color::Rgb(40, 55, 40)),
                            ),
                            1 => Span::styled(
                                "█",
                                Style::default().fg(Color::Rgb(85, 90, 110)),
                            ),
                            2 => Span::styled(
                                "A",
                                Style::default().fg(Color::Rgb(0, 200, 80)),
                            ),
                            3 => Span::styled(
                                "B",
                                Style::default().fg(Color::Rgb(220, 50, 50)),
                            ),
                            4 => Span::styled(
                                "C",
                                Style::default().fg(Color::Rgb(60, 120, 255)),
                            ),
                            5 => Span::styled(
                                "T",
                                Style::default().fg(Color::Rgb(255, 140, 0)),
                            ),
                            _ => Span::styled(
                                "?",
                                Style::default().fg(Color::White),
                            ),
                        }
                    }
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

// ── Main ──────────────────────────────────────────────────────────────────────
fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut player  = Player::new();
    let mut pressed: HashSet<KeyCode> = HashSet::new();
    let mut last_mouse_col: Option<i32> = None;

    let frame_ms = Duration::from_millis(1000 / 30); // 30 fps target
    let mut prev_tick = Instant::now();

    'main: loop {
        let frame_start = Instant::now();
        let dt = frame_start
            .duration_since(prev_tick)
            .as_secs_f64()
            .min(0.1);
        prev_tick = frame_start;

        // ── Input ─────────────────────────────────────────────────────────────
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(k) => match k.kind {
                    KeyEventKind::Press | KeyEventKind::Repeat => {
                        match k.code {
                            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                break 'main;
                            }
                            // Normalise Char to lowercase before inserting
                            KeyCode::Char(c) => {
                                pressed.insert(KeyCode::Char(c.to_ascii_lowercase()));
                            }
                            other => {
                                pressed.insert(other);
                            }
                        }
                    }
                    KeyEventKind::Release => {
                        let code = match k.code {
                            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
                            other => other,
                        };
                        pressed.remove(&code);
                    }
                },
                Event::Mouse(m) => {
                    match m.kind {
                        MouseEventKind::Moved => {
                            let col = m.column as i32;
                            if let Some(lc) = last_mouse_col {
                                let dx = col - lc;
                                player.angle += dx as f64 * MOUSE_SENS;
                            }
                            last_mouse_col = Some(col);
                        }
                        MouseEventKind::ScrollUp => {
                            player.pitch = (player.pitch - 0.12).max(-0.8);
                        }
                        MouseEventKind::ScrollDown => {
                            player.pitch = (player.pitch + 0.12).min(0.8);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // ── Update ────────────────────────────────────────────────────────────
        player.update(&pressed, dt);

        // ── Draw ──────────────────────────────────────────────────────────────
        terminal.draw(|f| draw_ui(f, &player))?;

        // ── Frame throttle ────────────────────────────────────────────────────
        let elapsed = frame_start.elapsed();
        if elapsed < frame_ms {
            std::thread::sleep(frame_ms - elapsed);
        }
    }

    // ── Cleanup ───────────────────────────────────────────────────────────────
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

// ── UI Layout ─────────────────────────────────────────────────────────────────
fn draw_ui(f: &mut Frame, player: &Player) {
    let area = f.area();

    // Vertical split: 3D view (top) | HUD bar (bottom 3 rows)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(3)])
        .split(area);

    let view_area = chunks[0];
    let hud_area  = chunks[1];

    // ── 3D View ───────────────────────────────────────────────────────────────
    let border = Block::default()
        .borders(Borders::ALL)
        .title(" ◆ CS:GO — 3D Map Explorer ◆ ")
        .border_style(Style::default().fg(Color::Rgb(45, 55, 72)));
    let inner = border.inner(view_area);
    f.render_widget(border, view_area);

    if inner.width > 2 && inner.height > 2 {
        let lines = render_3d(player, inner.width as usize, inner.height as usize);
        f.render_widget(Paragraph::new(lines), inner);

        // ── Mini-map overlay (top-right corner) ───────────────────────────────
        let mm_w = (20u16).min(inner.width.saturating_div(3));
        let mm_h = (10u16).min(inner.height.saturating_div(2));
        if mm_w > 4 && mm_h > 3 {
            let mm_area = Rect::new(
                inner.x + inner.width.saturating_sub(mm_w + 2),
                inner.y + 1,
                mm_w,
                mm_h,
            );
            let mm_block = Block::default()
                .borders(Borders::ALL)
                .title("Map")
                .border_style(Style::default().fg(Color::Rgb(55, 70, 95)))
                .style(Style::default().bg(Color::Rgb(4, 7, 10)));
            let mm_inner = mm_block.inner(mm_area);
            f.render_widget(mm_block, mm_area);
            let mm_lines = render_minimap(
                player,
                mm_inner.width as usize,
                mm_inner.height as usize,
            );
            f.render_widget(Paragraph::new(mm_lines), mm_inner);
        }
    }

    // ── HUD bar ───────────────────────────────────────────────────────────────
    let deg = player.angle.to_degrees().rem_euclid(360.0);
    let compass = match deg as u32 {
        0..=44 | 316..=360 => "E",
        45..=134            => "S",
        135..=224           => "W",
        _                   => "N",
    };
    let zone = {
        let (xi, yi) = (player.x as usize, player.y as usize);
        match MAP[yi.min(MAP_H - 1)][xi.min(MAP_W - 1)] {
            2 => "BOMB SITE A",
            3 => "BOMB SITE B",
            4 => "CT SPAWN",
            5 => "T  SPAWN",
            _ => "",
        }
    };
    let zone_str = if zone.is_empty() {
        String::new()
    } else {
        format!("  [{}]", zone)
    };

    let hud_txt = format!(
        " Pos ({:.1},{:.1})  {compass} {deg:.0}deg  Pitch {:.2}{zone_str}\
         \n W/Up:Fwd  S/Dn:Back  A:StrafeL  D:StrafeR  Arrows:Turn  \
         Mouse:Look  Scroll:Pitch  Q/Esc:Quit",
        player.x, player.y, player.pitch
    );

    f.render_widget(
        Paragraph::new(hud_txt)
            .style(Style::default().fg(Color::Rgb(251, 191, 36)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" HUD ")
                    .border_style(Style::default().fg(Color::Rgb(45, 55, 72))),
            ),
        hud_area,
    );
}
