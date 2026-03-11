//! play.rs — PLAYABLE Counter-Strike game
//! Controls:
//!   TeamSelect:  [C] CT  [T] T
//!   BuyPhase:    [B] toggle buy  ←→ category  ↑↓ select  Enter buy  [K] buy kit
//!   Combat:      [S]/Left-Click shoot  Right-Click scope  [G] HE grenade  [P] plant  [D] defuse  [B] buy (freeze only)
//!   Any:         [Q] quit

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use game_cs::{
    economy::shop::BuyMenu,
    game::{
        r#match::{Match, MatchState},
        types::{GameMode, HitZone, MatchConfig, Rank, RoundPhase, RoundResult, Team, Vec3},
    },
    map::{
        bomb::{BombAction, BombPhase},
        maps::MapRegistry,
    },
    player::types::{Player, PlayerProfile},
    weapons::{catalog::WeaponCatalog, types::{WeaponKind, WeaponStats}},
};
use rand::Rng;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant},
};

// ── Palette ───────────────────────────────────────────────────────────────────
const C_BG:     Color = Color::Rgb(10,  12,  16);
const C_PANEL:  Color = Color::Rgb(20,  24,  32);
const C_BORDER: Color = Color::Rgb(45,  55,  72);
const C_ACCENT: Color = Color::Rgb(251, 191, 36);
const C_CT:     Color = Color::Rgb(96,  165, 250);
const C_T:      Color = Color::Rgb(251, 113, 133);
const C_GREEN:  Color = Color::Rgb(74,  222, 128);
const C_DIM:    Color = Color::Rgb(100, 116, 139);
const C_WHITE:  Color = Color::Rgb(226, 232, 240);
const C_HP:     Color = Color::Rgb(239, 68,  68);
const C_MONEY:  Color = Color::Rgb(251, 191, 36);

const BOT_TICK: f32 = 1.5;   // bots act every 1.5s
const ROUND_END_SHOW: f32 = 4.0;

#[derive(Clone)]
enum PlayScreen { TeamSelect, InRound, RoundEnd(RoundResult, String), MatchEnd }

enum BotAction {
    Shoot { bot_id: String, victim_id: String, damage: f32, weapon: String, is_hs: bool },
    Plant { bot_id: String },
    Defuse { bot_id: String },
}

const BUY_CATS: &[(&str, WeaponKind)] = &[
    ("Pistols",  WeaponKind::Pistol),
    ("SMGs",     WeaponKind::SMG),
    ("Rifles",   WeaponKind::Rifle),
    ("Snipers",  WeaponKind::Sniper),
    ("Grenades", WeaponKind::Throwable),
];

struct PlayApp {
    game: Match,
    player_id: String,
    bot_ids: Vec<String>,
    screen: PlayScreen,
    log: VecDeque<String>,
    bot_acc: f32,
    round_end_acc: f32,
    buy_open: bool,
    buy_cat: usize,
    buy_sel: usize,
    buy_lists: Vec<Vec<WeaponStats>>,
    buy_menu: BuyMenu,
    tick: u64,
    last_frame: Instant,
    scoped: bool,
}

impl PlayApp {
    fn new() -> Self {
        let registry = MapRegistry::new();
        let map = registry.get("de_dust2").unwrap().clone();
        let config = MatchConfig { game_mode: GameMode::BombDefusal, ..Default::default() };
        let mut game = Match::new(config, map);

        // Add player placeholder (team assigned on TeamSelect)
        let pid = "player1".to_string();

        // Bot IDs
        let bot_ids: Vec<String> = (2..=5).map(|i| format!("ct{}", i))
            .chain((1..=5).map(|i| format!("t{}", i)))
            .collect();

        // Add bots (teams assigned later when player picks)
        for id in &bot_ids[0..4] {
            let p = Player::new(
                PlayerProfile::new(id.clone(), format!("CT-{}", &id[2..])),
                Team::CT, 800,
            );
            game.add_player(p).ok();
        }
        for id in &bot_ids[4..] {
            let num = &id[1..];
            let p = Player::new(
                PlayerProfile::new(id.clone(), format!("T-{}", num)),
                Team::T, 800,
            );
            game.add_player(p).ok();
        }

        let catalog = WeaponCatalog::new();
        let buy_lists: Vec<Vec<WeaponStats>> = BUY_CATS.iter().map(|(_, kind)| {
            let mut ws: Vec<WeaponStats> = catalog.all()
                .filter(|w| w.kind == *kind && w.price > 0)
                .cloned().collect();
            ws.sort_by_key(|w| w.price);
            ws
        }).collect();

        PlayApp {
            game, player_id: pid, bot_ids,
            screen: PlayScreen::TeamSelect,
            log: VecDeque::new(),
            bot_acc: 0.0, round_end_acc: 0.0,
            buy_open: false, buy_cat: 2, buy_sel: 0,
            buy_lists,
            buy_menu: BuyMenu::new(),
            tick: 0,
            last_frame: Instant::now(),
            scoped: false,
        }
    }

    fn start_as(&mut self, team: Team) {
        // Add player with chosen team
        let opp = if team == Team::CT { Team::T } else { Team::CT };
        let p = Player::new(
            PlayerProfile { id: self.player_id.clone(), nickname: "YOU".into(),
                rank: Rank::GoldNovaIII, experience: 0, total_kills: 0,
                total_deaths: 0, total_wins: 0, total_matches: 0,
                total_headshots: 0, created_at: chrono::Utc::now() },
            team, 800,
        );
        self.game.add_player(p).ok();

        // Reassign bots: first 4 bots same team as player, next 5 opponents
        let all_ids: Vec<String> = self.bot_ids.clone();
        let (allies, enemies) = all_ids.split_at(4);
        for id in allies {
            if let Some(p) = self.game.players.get_mut(id) { p.team = team; }
        }
        for id in enemies {
            if let Some(p) = self.game.players.get_mut(id) { p.team = opp; }
        }

        self.game.start().ok();
        self.equip_bots();
        self.add_log("=== Match started on de_dust2 ===".into());
        self.add_log(format!("You play as {:?}. Bots have been assigned.", team));
        self.add_log("Buy phase — use [B] to open buy menu".into());
        self.screen = PlayScreen::InRound;
    }

    fn equip_bots(&mut self) {
        let catalog = WeaponCatalog::new();
        let ak = catalog.get("AK-47").unwrap().clone();
        let m4 = catalog.get("M4A4").unwrap().clone();
        let glock = catalog.get("Glock-18").unwrap().clone();
        let usp = catalog.get("USP-S").unwrap().clone();
        use game_cs::weapons::types::Weapon;

        for id in &self.bot_ids.clone() {
            if let Some(p) = self.game.players.get_mut(id) {
                let rifle = if p.team == Team::T { Weapon::new(ak.clone()) } else { Weapon::new(m4.clone()) };
                let pistol = if p.team == Team::T { Weapon::new(glock.clone()) } else { Weapon::new(usp.clone()) };
                p.inventory.equip(rifle);
                p.inventory.equip(pistol);
            }
        }
    }

    fn player(&self) -> Option<&Player> { self.game.players.get(&self.player_id) }
    fn player_mut(&mut self) -> Option<&mut Player> { self.game.players.get_mut(&self.player_id) }

    fn round_phase(&self) -> RoundPhase {
        self.game.current_round.as_ref().map(|r| r.phase).unwrap_or(RoundPhase::Ended)
    }
    fn bomb_phase(&self) -> BombPhase {
        self.game.current_round.as_ref().map(|r| r.bomb.phase).unwrap_or(BombPhase::NotPlanted)
    }

    fn add_log(&mut self, msg: String) {
        self.log.push_front(msg);
        if self.log.len() > 60 { self.log.pop_back(); }
    }

    fn player_alive(&self) -> bool { self.player().map(|p| p.is_alive()).unwrap_or(false) }

    // ── Player actions ────────────────────────────────────────────────────────
    fn action_shoot(&mut self) {
        if !self.player_alive() { self.add_log("You are dead.".into()); return; }
        if !matches!(self.round_phase(), RoundPhase::Live | RoundPhase::BombPlanted) {
            self.add_log("Can't shoot during freeze time.".into()); return;
        }
        let player_team = self.player().map(|p| p.team).unwrap_or(Team::Spectator);
        let targets: Vec<String> = self.game.players.values()
            .filter(|p| p.team != player_team && p.team != Team::Spectator && p.is_alive())
            .map(|p| p.profile.id.clone()).collect();

        if targets.is_empty() { self.add_log("No enemies alive!".into()); return; }

        let mut rng = rand::thread_rng();
        let tid = targets[rng.gen_range(0..targets.len())].clone();
        let hit = rng.gen::<f32>() < 0.72;
        if !hit { self.add_log("Missed!".into()); return; }

        let (dmg, is_hs, weapon_name) = {
            let p = self.game.players.get(&self.player_id).unwrap();
            let base = p.inventory.active_weapon().map(|w| w.stats.base_damage).unwrap_or(30.0);
            let wname = p.inventory.active_weapon().map(|w| w.stats.name.clone()).unwrap_or("Knife".into());
            let hs = rng.gen::<f32>() < 0.25;
            let dmg = if hs { base * 4.0 } else { base * rng.gen_range(0.6f32..1.0) };
            (dmg, hs, wname)
        };

        let victim_name = self.game.players.get(&tid)
            .map(|p| p.profile.nickname.clone()).unwrap_or_default();
        let died = self.game.players.get_mut(&tid).map(|p| p.take_damage(dmg)).unwrap_or(false);
        let ts = (self.game.match_time_secs * 1000.0) as u64;

        if died {
            let hs_tag = if is_hs { " [HEADSHOT]" } else { "" };
            self.add_log(format!("YOU killed {} with {}{}!", victim_name, weapon_name, hs_tag));
            let hz = if is_hs { HitZone::Head } else { HitZone::Chest };
            self.game.record_kill(&self.player_id.clone(), &tid, &weapon_name, hz, is_hs, ts).ok();
        } else {
            let hp = self.game.players.get(&tid).map(|p| p.health).unwrap_or(0);
            self.add_log(format!("Hit {} for {:.0} dmg ({} HP left)", victim_name, dmg, hp));
        }
    }

    fn action_throw_nade(&mut self) {
        if !self.player_alive() { self.add_log("You are dead.".into()); return; }
        if !matches!(self.round_phase(), RoundPhase::Live | RoundPhase::BombPlanted) {
            self.add_log("Can only throw during live phase.".into()); return;
        }
        // Check inventory for HE grenade
        let has_he = self.player().map(|p| p.inventory.throwables.iter()
            .any(|w| w.stats.name.contains("HE") || w.stats.name.contains("Grenade"))).unwrap_or(false);
        if !has_he { self.add_log("No HE grenade! Buy one in freeze time.".into()); return; }

        let pos = self.player().map(|p| p.position).unwrap_or(Vec3::zero());
        let pid = self.player_id.clone();
        if let Some(round) = self.game.current_round.as_mut() {
            use game_cs::game::GrenadeType;
            round.throw_grenade(pid, GrenadeType::HE, Vec3::new(pos.x + 100.0, pos.y + 50.0, pos.z));
        }
        // Use one grenade from inventory
        if let Some(p) = self.player_mut() { p.inventory.use_throwable(); }
        self.add_log("YOU threw a HE grenade!".into());
    }

    fn action_plant(&mut self) {
        if !self.player_alive() { self.add_log("You are dead.".into()); return; }
        let is_t = self.player().map(|p| p.team == Team::T).unwrap_or(false);
        let has_bomb = self.player().map(|p| p.is_carrying_bomb).unwrap_or(false);
        if !is_t || !has_bomb { self.add_log("Only T-side bomb carrier can plant!".into()); return; }
        if self.round_phase() != RoundPhase::Live { self.add_log("Can only plant during live phase.".into()); return; }
        // Move player to bomb site A center
        let site_pos = self.game.map.bomb_sites.first()
            .map(|s| s.bounds.center()).unwrap_or(Vec3::zero());
        if let Some(p) = self.player_mut() { p.position = site_pos; }

        let pid = self.player_id.clone();
        match self.game.apply_bomb_action(BombAction::StartPlant { player_id: pid, position: site_pos }) {
            Ok(_) => self.add_log("YOU started planting the bomb...".into()),
            Err(e) => self.add_log(format!("Plant failed: {}", e)),
        }
    }

    fn action_defuse(&mut self) {
        if !self.player_alive() { self.add_log("You are dead.".into()); return; }
        let is_ct = self.player().map(|p| p.team == Team::CT).unwrap_or(false);
        if !is_ct { self.add_log("Only CT can defuse!".into()); return; }
        if self.bomb_phase() != BombPhase::Planted { self.add_log("Bomb is not planted.".into()); return; }
        let has_kit = self.player().map(|p| p.armor.has_defuse_kit).unwrap_or(false);
        let pid = self.player_id.clone();
        // Move player to bomb position
        let bomb_pos = self.game.current_round.as_ref().and_then(|r| r.bomb.position);
        if let (Some(pos), Some(p)) = (bomb_pos, self.player_mut()) { p.position = pos; }

        match self.game.apply_bomb_action(BombAction::StartDefuse { player_id: pid, has_kit }) {
            Ok(_) => {
                let t = if has_kit { 5 } else { 10 };
                self.add_log(format!("YOU started defusing ({t}s){}...",
                    if has_kit { " [KIT]" } else { "" }));
            }
            Err(e) => self.add_log(format!("Defuse failed: {}", e)),
        }
    }

    fn action_buy(&mut self) {
        let list = self.buy_lists.get(self.buy_cat).cloned().unwrap_or_default();
        let Some(w) = list.get(self.buy_sel) else { return; };
        let wname = w.name.clone();
        let phase = self.round_phase();
        let pid = self.player_id.clone();
        let player = match self.game.players.get_mut(&pid) {
            Some(p) => p,
            None => return,
        };
        match self.buy_menu.buy_weapon(player, &wname, phase, 16000) {
            Ok(_) => self.add_log(format!("Bought {}!", wname)),
            Err(e) => self.add_log(format!("Buy failed: {}", e)),
        }
    }

    fn action_buy_kit(&mut self) {
        let phase = self.round_phase();
        let pid = self.player_id.clone();
        if let Some(p) = self.game.players.get_mut(&pid) {
            match self.buy_menu.buy_defuse_kit(p, phase) {
                Ok(_) => self.add_log("Bought Defuse Kit!".into()),
                Err(e) => self.add_log(format!("Kit: {}", e)),
            }
        }
    }

    fn action_buy_armor(&mut self) {
        let phase = self.round_phase();
        let pid = self.player_id.clone();
        if let Some(p) = self.game.players.get_mut(&pid) {
            match self.buy_menu.buy_kevlar_helmet(p, phase) {
                Ok(_) => self.add_log("Bought Kevlar + Helmet!".into()),
                Err(e) => self.add_log(format!("Armor: {}", e)),
            }
        }
    }

    // ── Bot AI ────────────────────────────────────────────────────────────────
    fn run_bots(&mut self) {
        let actions = self.collect_bot_actions();
        self.apply_bot_actions(actions);
    }

    fn collect_bot_actions(&self) -> Vec<BotAction> {
        let mut rng = rand::thread_rng();
        let mut actions = Vec::new();
        let phase = self.round_phase();
        let bphase = self.bomb_phase();
        let time_rem = self.game.current_round.as_ref().map(|r| r.time_remaining_secs).unwrap_or(0.0);

        if !matches!(phase, RoundPhase::Live | RoundPhase::BombPlanted) {
            return actions;
        }

        for bot_id in &self.bot_ids {
            let bot = match self.game.players.get(bot_id) {
                Some(p) if p.is_alive() => p,
                _ => continue,
            };
            let team = bot.team;

            // Shoot
            let enemies: Vec<String> = self.game.players.values()
                .filter(|p| p.team != team && p.team != Team::Spectator && p.is_alive())
                .map(|p| p.profile.id.clone()).collect();

            if !enemies.is_empty() && rng.gen::<f32>() < 0.55 {
                let vid = enemies[rng.gen_range(0..enemies.len())].clone();
                if rng.gen::<f32>() < 0.38 {
                    let base_dmg = bot.inventory.primary.as_ref()
                        .map(|w| w.stats.base_damage).unwrap_or(30.0);
                    let wname = bot.inventory.primary.as_ref()
                        .map(|w| w.stats.name.clone()).unwrap_or("Knife".into());
                    let is_hs = rng.gen::<f32>() < 0.12;
                    let dmg = if is_hs { base_dmg * 4.0 * 0.3 } else { base_dmg * rng.gen_range(0.4f32..0.75) };
                    actions.push(BotAction::Shoot { bot_id: bot_id.clone(), victim_id: vid, damage: dmg, weapon: wname, is_hs });
                }
            }

            // T bomb carrier plants at ~35s mark
            if team == Team::T && bot.is_carrying_bomb
                && bphase == BombPhase::NotPlanted && time_rem < 80.0
                && rng.gen::<f32>() < 0.6
            {
                actions.push(BotAction::Plant { bot_id: bot_id.clone() });
            }

            // CT tries to defuse
            if team == Team::CT && bphase == BombPhase::Planted && rng.gen::<f32>() < 0.25 {
                actions.push(BotAction::Defuse { bot_id: bot_id.clone() });
            }
        }
        actions
    }

    fn apply_bot_actions(&mut self, actions: Vec<BotAction>) {
        for action in actions {
            match action {
                BotAction::Shoot { bot_id, victim_id, damage, weapon, is_hs } => {
                    let bot_name = self.game.players.get(&bot_id)
                        .map(|p| p.profile.nickname.clone()).unwrap_or_default();
                    let victim_name = self.game.players.get(&victim_id)
                        .map(|p| p.profile.nickname.clone()).unwrap_or_default();
                    let died = self.game.players.get_mut(&victim_id)
                        .map(|p| p.take_damage(damage)).unwrap_or(false);
                    let ts = (self.game.match_time_secs * 1000.0) as u64;
                    if died {
                        let hs_s = if is_hs { " [HS]" } else { "" };
                        self.add_log(format!("{} killed {}{} with {}", bot_name, victim_name, hs_s, weapon));
                        let hz = if is_hs { HitZone::Head } else { HitZone::Chest };
                        self.game.record_kill(&bot_id, &victim_id, &weapon, hz, is_hs, ts).ok();
                    } else {
                        let hp = self.game.players.get(&victim_id).map(|p| p.health).unwrap_or(0);
                        self.add_log(format!("{} hit {} for {:.0} ({} HP)", bot_name, victim_name, damage, hp));
                    }
                }
                BotAction::Plant { bot_id } => {
                    let site_pos = self.game.map.bomb_sites.first()
                        .map(|s| s.bounds.center()).unwrap_or(Vec3::zero());
                    if let Some(p) = self.game.players.get_mut(&bot_id) { p.position = site_pos; }
                    let bot_name = self.game.players.get(&bot_id)
                        .map(|p| p.profile.nickname.clone()).unwrap_or_default();
                    match self.game.apply_bomb_action(BombAction::StartPlant {
                        player_id: bot_id.clone(), position: site_pos,
                    }) {
                        Ok(_) => self.add_log(format!("{} is planting the bomb at A!", bot_name)),
                        Err(_) => {}
                    }
                }
                BotAction::Defuse { bot_id } => {
                    let has_kit = self.game.players.get(&bot_id)
                        .map(|p| p.armor.has_defuse_kit).unwrap_or(false);
                    let bot_name = self.game.players.get(&bot_id)
                        .map(|p| p.profile.nickname.clone()).unwrap_or_default();
                    match self.game.apply_bomb_action(BombAction::StartDefuse {
                        player_id: bot_id.clone(), has_kit,
                    }) {
                        Ok(_) => self.add_log(format!("{} is defusing!", bot_name)),
                        Err(_) => {}
                    }
                }
            }
        }
    }

    // ── Frame tick ────────────────────────────────────────────────────────────
    fn tick_state(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.tick += 1;

        match &self.screen.clone() {
            PlayScreen::InRound => {
                if self.game.state == MatchState::Halftime {
                    self.add_log("--- HALFTIME --- Teams swapped!".into());
                    self.game.resume_from_halftime().ok();
                    self.equip_bots();
                }
                if self.game.state == MatchState::Ended {
                    self.screen = PlayScreen::MatchEnd;
                    return;
                }

                // Bot AI tick
                self.bot_acc += dt;
                if self.bot_acc >= BOT_TICK {
                    self.bot_acc = 0.0;
                    self.run_bots();
                }

                // Match tick
                let old_round = self.game.round_number;
                let old_phase = self.round_phase();
                if let Some(result) = self.game.tick(dt) {
                    let mvp_name = self.game.round_history.last()
                        .and_then(|s| s.mvp_nickname.clone())
                        .unwrap_or_else(|| "Unknown".into());
                    let result_msg = match result {
                        RoundResult::CTWin(r) => format!("CT WIN ({:?})", r),
                        RoundResult::TWin(r)  => format!("T WIN ({:?})", r),
                        RoundResult::InProgress => "Round still ongoing".into(),
                    };
                    self.add_log(format!("=== {} === Score CT:{} T:{}",
                        result_msg, self.game.score.ct, self.game.score.t));
                    self.screen = PlayScreen::RoundEnd(result, mvp_name);
                    self.round_end_acc = 0.0;
                } else {
                    // Log phase transitions
                    let new_phase = self.round_phase();
                    if old_phase == RoundPhase::FreezeTime && new_phase == RoundPhase::Live {
                        self.add_log(format!("=== Round {} LIVE! Fight! ===", self.game.round_number));
                    }
                    if self.game.round_number > old_round {
                        self.add_log("Buy phase — use [B] to open buy menu".into());
                    }
                    // Bomb status messages
                    let bp = self.bomb_phase();
                    if bp == BombPhase::Planted {
                        if self.tick % 60 == 0 {
                            let t = self.game.current_round.as_ref().map(|r| r.bomb.timer_secs).unwrap_or(0.0);
                            self.add_log(format!("*** BOMB PLANTED — {:.0}s remaining! ***", t));
                        }
                    }
                }
            }
            PlayScreen::RoundEnd(_, _) => {
                self.round_end_acc += dt;
                if self.round_end_acc >= ROUND_END_SHOW {
                    if self.game.state == MatchState::Ended { self.screen = PlayScreen::MatchEnd; }
                    else { self.screen = PlayScreen::InRound; self.equip_bots(); }
                }
            }
            _ => {}
        }
    }

    // ── Input ─────────────────────────────────────────────────────────────────
    fn handle_key(&mut self, code: KeyCode) -> bool {
        match &self.screen.clone() {
            PlayScreen::TeamSelect => match code {
                KeyCode::Char('c') | KeyCode::Char('C') => self.start_as(Team::CT),
                KeyCode::Char('t') | KeyCode::Char('T') => self.start_as(Team::T),
                KeyCode::Char('q') | KeyCode::Char('Q') => return true,
                _ => {}
            },
            PlayScreen::InRound => {
                if self.buy_open {
                    match code {
                        KeyCode::Up   | KeyCode::Char('k') => { self.buy_sel = self.buy_sel.saturating_sub(1); }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let mx = self.buy_lists.get(self.buy_cat).map(|l| l.len().saturating_sub(1)).unwrap_or(0);
                            self.buy_sel = (self.buy_sel + 1).min(mx);
                        }
                        KeyCode::Left  | KeyCode::Char('h') => { self.buy_cat = self.buy_cat.saturating_sub(1); self.buy_sel = 0; }
                        KeyCode::Right | KeyCode::Char('l') => { self.buy_cat = (self.buy_cat + 1).min(BUY_CATS.len()-1); self.buy_sel = 0; }
                        KeyCode::Enter => self.action_buy(),
                        KeyCode::Char('k') => self.action_buy_kit(),
                        KeyCode::Char('a') | KeyCode::Char('A') => self.action_buy_armor(),
                        KeyCode::Esc | KeyCode::Char('b') | KeyCode::Char('B') => self.buy_open = false,
                        _ => {}
                    }
                } else {
                    match code {
                        KeyCode::Char('s') | KeyCode::Char('S') => self.action_shoot(),
                        KeyCode::Char('g') | KeyCode::Char('G') => self.action_throw_nade(),
                        KeyCode::Char('p') | KeyCode::Char('P') => self.action_plant(),
                        KeyCode::Char('d') | KeyCode::Char('D') => self.action_defuse(),
                        KeyCode::Char('b') | KeyCode::Char('B') => {
                            if self.round_phase() == RoundPhase::FreezeTime { self.buy_open = true; }
                            else { self.add_log("Buy only available during freeze time!".into()); }
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => return true,
                        _ => {}
                    }
                }
            }
            PlayScreen::RoundEnd(_, _) => {
                if code == KeyCode::Char('q') || code == KeyCode::Char('Q') { return true; }
            }
            PlayScreen::MatchEnd => {
                if code == KeyCode::Char('q') || code == KeyCode::Char('Q') { return true; }
            }
        }
        false
    }

    // ── Mouse input ───────────────────────────────────────────────────────────
    fn handle_mouse(&mut self, kind: MouseEventKind) {
        if let PlayScreen::InRound = &self.screen.clone() {
            if self.buy_open { return; }
            match kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // Left click = shoot
                    self.action_shoot();
                }
                MouseEventKind::Down(MouseButton::Right) => {
                    // Right click = toggle scope
                    self.scoped = !self.scoped;
                    if self.scoped {
                        self.add_log(">>> SCOPED IN <<<".into());
                    } else {
                        self.add_log("<<< SCOPED OUT >>>".into());
                    }
                }
                _ => {}
            }
        }
    }
}

// ── Drawing ────────────────────────────────────────────────────────────────────
fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = PlayApp::new();
    app.last_frame = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &app))?;
        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.handle_key(key.code) { break; }
                }
                Event::Mouse(me) => {
                    app.handle_mouse(me.kind);
                }
                _ => {}
            }
        }
        app.tick_state();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn draw(f: &mut Frame, app: &PlayApp) {
    f.render_widget(Block::default().style(Style::default().bg(C_BG)), f.area());
    match &app.screen {
        PlayScreen::TeamSelect    => draw_team_select(f, app),
        PlayScreen::InRound       => draw_in_round(f, app),
        PlayScreen::RoundEnd(r,m) => draw_round_end(f, app, *r, m),
        PlayScreen::MatchEnd      => draw_match_end(f, app),
    }
}

fn panel<'a>(title: &'a str, color: Color) -> Block<'a> {
    Block::default()
        .title(Span::styled(format!(" {title} "), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(C_PANEL))
}

// ── Team select ───────────────────────────────────────────────────────────────
fn draw_team_select(f: &mut Frame, _app: &PlayApp) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Fill(1), Constraint::Length(16), Constraint::Fill(1)]).split(area);
    let cols = Layout::horizontal([Constraint::Fill(1), Constraint::Length(56), Constraint::Fill(1)]).split(rows[1]);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("COUNTER-STRIKE", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED))),
            Line::from(""),
            Line::from(Span::styled("Choose your side:", Style::default().fg(C_WHITE))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [C]  ", Style::default().fg(C_CT).add_modifier(Modifier::BOLD)),
                Span::styled("Counter-Terrorist", Style::default().fg(C_CT).add_modifier(Modifier::BOLD)),
                Span::styled("  — Defend, defuse the bomb", Style::default().fg(C_DIM)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [T]  ", Style::default().fg(C_T).add_modifier(Modifier::BOLD)),
                Span::styled("Terrorist", Style::default().fg(C_T).add_modifier(Modifier::BOLD)),
                Span::styled("          — Attack, plant the bomb", Style::default().fg(C_DIM)),
            ]),
            Line::from(""),
            Line::from(Span::styled("  Map: de_dust2   5v5   30 rounds", Style::default().fg(C_DIM))),
            Line::from(""),
            Line::from(Span::styled("  [Q] Quit", Style::default().fg(C_DIM))),
        ])
        .block(panel("SELECT TEAM", C_BORDER))
        .alignment(Alignment::Center),
        cols[1],
    );
}

// ── Character model ───────────────────────────────────────────────────────────
fn draw_character_model(f: &mut Frame, area: Rect, app: &PlayApp) {
    let alive  = app.player_alive();
    let team   = app.player().map(|p| p.team).unwrap_or(Team::CT);
    let hp     = app.player().map(|p| p.health).unwrap_or(0);
    let scoped = app.scoped;

    let body_color = if hp > 60 { C_GREEN } else if hp > 30 { C_ACCENT } else { C_HP };
    let head_color = if team == Team::CT { C_CT } else { C_T };

    let (title, border_color, lines): (&str, Color, Vec<Line>) = if !alive {
        ("✗ DEAD", C_DIM, vec![
            Line::from(""),
            Line::from(Span::styled(" _______ ", Style::default().fg(C_DIM))),
            Line::from(Span::styled("/x     x\\", Style::default().fg(C_DIM))),
            Line::from(Span::styled("\\___+___/", Style::default().fg(C_DIM))),
            Line::from(Span::styled("  /___\\  ", Style::default().fg(C_DIM))),
            Line::from(Span::styled(" / RIP \\ ", Style::default().fg(C_HP).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("/________\\", Style::default().fg(C_DIM))),
            Line::from(""),
        ])
    } else if team == Team::CT {
        let eyes = if scoped { "|(*)  o|" } else { "| o  o |" };
        ("◉ CT", C_CT, vec![
            Line::from(Span::styled("  _____  ", Style::default().fg(head_color))),
            Line::from(Span::styled(eyes,       Style::default().fg(head_color))),
            Line::from(Span::styled("|=_CT_=|", Style::default().fg(head_color).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(" _|   |_ ", Style::default().fg(body_color))),
            Line::from(Span::styled("/=|   |=\\", Style::default().fg(body_color))),
            Line::from(Span::styled("  |   |  ", Style::default().fg(body_color))),
            Line::from(Span::styled("  |___|  ", Style::default().fg(body_color))),
            Line::from(Span::styled("  /   \\  ", Style::default().fg(body_color))),
            Line::from(Span::styled(" /     \\ ", Style::default().fg(body_color))),
        ])
    } else {
        let eyes = if scoped { "#x}  x##" } else { "## x  x#" };
        ("◉ T", C_T, vec![
            Line::from(Span::styled(" /####\\ ", Style::default().fg(head_color))),
            Line::from(Span::styled(eyes,       Style::default().fg(head_color))),
            Line::from(Span::styled("\\__T__/ ", Style::default().fg(head_color).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(" _|   |_ ", Style::default().fg(body_color))),
            Line::from(Span::styled("/=|   |=\\", Style::default().fg(body_color))),
            Line::from(Span::styled("  |   |  ", Style::default().fg(body_color))),
            Line::from(Span::styled("  |___|  ", Style::default().fg(body_color))),
            Line::from(Span::styled("  /   \\  ", Style::default().fg(body_color))),
            Line::from(Span::styled(" /     \\ ", Style::default().fg(body_color))),
        ])
    };

    f.render_widget(
        Paragraph::new(lines)
            .block(panel(title, border_color))
            .alignment(Alignment::Center),
        area,
    );
}

// ── In-round ──────────────────────────────────────────────────────────────────
fn draw_in_round(f: &mut Frame, app: &PlayApp) {
    let area = f.area();
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(9),
    ]).split(area);

    draw_hud_bar(f, rows[0], app);

    let cols = Layout::horizontal([
        Constraint::Length(18),
        Constraint::Fill(1),
        Constraint::Length(32),
    ]).split(rows[1]);
    draw_character_model(f, cols[0], app);
    if app.buy_open {
        draw_buy(f, cols[1], app);
    } else {
        draw_log(f, cols[1], app);
    }
    draw_teams(f, cols[2], app);

    draw_bottom(f, rows[2], app);
}

fn draw_hud_bar(f: &mut Frame, area: Rect, app: &PlayApp) {
    let phase = app.round_phase();
    let bp = app.bomb_phase();
    let time = app.game.current_round.as_ref().map(|r| r.time_remaining_secs).unwrap_or(0.0);
    let mm = (time / 60.0) as u32; let ss = time as u32 % 60;
    let phase_str = match phase {
        RoundPhase::FreezeTime => "BUY PHASE",
        RoundPhase::Live => "LIVE",
        RoundPhase::BombPlanted => "BOMB PLANTED!",
        RoundPhase::Ended => "ENDED",
    };
    let (pc, bomb_str) = match bp {
        BombPhase::Planting => (C_ACCENT, " [PLANTING]"),
        BombPhase::Planted  => (C_HP,     " [BOMB TICKING]"),
        BombPhase::Defusing => (C_CT,     " [DEFUSING]"),
        BombPhase::Defused  => (C_GREEN,  " [DEFUSED]"),
        BombPhase::Exploded => (C_HP,     " [EXPLODED]"),
        _ => (C_DIM, ""),
    };
    let scope_span = if app.scoped {
        Span::styled(" [SCOPED] ", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("", Style::default())
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" CT {:>2} ", app.game.score.ct), Style::default().fg(C_CT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("— {:>2} T ", app.game.score.t), Style::default().fg(C_T).add_modifier(Modifier::BOLD)),
            Span::styled(format!("| Round {:>2}/30 ", app.game.round_number), Style::default().fg(C_WHITE)),
            Span::styled(format!("| {:>02}:{:>02} ", mm, ss), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("| {} ", phase_str), Style::default().fg(pc).add_modifier(Modifier::BOLD)),
            Span::styled(bomb_str, Style::default().fg(pc)),
            scope_span,
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        area,
    );
}

fn draw_log(f: &mut Frame, area: Rect, app: &PlayApp) {
    let h = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = app.log.iter().take(h).rev().map(|msg| {
        let (color, bold) = if msg.contains("YOU killed") { (C_GREEN, true) }
            else if msg.contains("killed YOU") || msg.contains("killed you") { (C_HP, true) }
            else if msg.starts_with("===") { (C_ACCENT, true) }
            else if msg.starts_with("***") { (C_T, true) }
            else if msg.starts_with("---") { (C_DIM, false) }
            else { (C_WHITE, false) };
        let style = if bold { Style::default().fg(color).add_modifier(Modifier::BOLD) }
            else { Style::default().fg(color) };
        Line::from(Span::styled(msg.clone(), style))
    }).collect();

    f.render_widget(
        Paragraph::new(lines).block(panel("GAME LOG", C_BORDER)),
        area,
    );
}

fn draw_buy(f: &mut Frame, area: Rect, app: &PlayApp) {
    let player_money = app.player().map(|p| p.money).unwrap_or(0);
    let rows = Layout::vertical([Constraint::Length(3), Constraint::Fill(1), Constraint::Length(5)]).split(area);

    // Category tabs
    let tabs: Vec<Span> = BUY_CATS.iter().enumerate().map(|(i, (name, _))| {
        if i == app.buy_cat {
            Span::styled(format!(" [{}]{} ", i+1, name), Style::default().fg(C_BG).bg(C_ACCENT).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(format!(" [{}]{} ", i+1, name), Style::default().fg(C_DIM))
        }
    }).collect();
    f.render_widget(
        Paragraph::new(Line::from(tabs))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        rows[0],
    );

    // Weapon list
    let list = app.buy_lists.get(app.buy_cat).cloned().unwrap_or_default();
    let items: Vec<ListItem> = list.iter().enumerate().map(|(i, w)| {
        let can = w.price <= player_money;
        let pc = if can { C_MONEY } else { C_HP };
        if i == app.buy_sel {
            ListItem::new(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(C_ACCENT)),
                Span::styled(format!("{:<18}", w.name), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("${:>4}", w.price), Style::default().fg(pc).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" dmg:{:.0}", w.base_damage), Style::default().fg(C_DIM)),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:<18}", w.name), Style::default().fg(if can { C_WHITE } else { C_DIM })),
                Span::styled(format!("${:>4}", w.price), Style::default().fg(pc)),
            ]))
        }
    }).collect();
    f.render_widget(
        List::new(items).block(panel("BUY MENU", C_CT)).style(Style::default().bg(C_PANEL)),
        rows[1],
    );

    // Hints
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("Money: ${}", player_money), Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled("[Enter] Buy  [A] Armor $1000  [K] Kit $400", Style::default().fg(C_DIM))),
            Line::from(Span::styled("[←→/1-5] Category  [↑↓] Select  [B/Esc] Close", Style::default().fg(C_DIM))),
        ])
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
        rows[2],
    );
}

fn draw_teams(f: &mut Frame, area: Rect, app: &PlayApp) {
    let halves = Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).split(area);

    for (half_idx, (team, color, label)) in [
        (Team::CT, C_CT, "CT"), (Team::T, C_T, "T"),
    ].iter().enumerate() {
        let items: Vec<ListItem> = app.game.players.values()
            .filter(|p| p.team == *team)
            .map(|p| {
                let you = p.profile.id == app.player_id;
                let (icon, nc) = if p.is_alive() { ("◉", C_GREEN) } else { ("✗", C_DIM) };
                let hpc = if p.health > 60 { C_GREEN } else if p.health > 30 { C_ACCENT } else { C_HP };
                let bomb = if p.is_carrying_bomb { "💣" } else { "  " };
                let name_str = if you {
                    format!("{:<12}", "YOU")
                } else {
                    format!("{:<12}", &p.profile.nickname[..p.profile.nickname.len().min(12)])
                };
                ListItem::new(Line::from(vec![
                    Span::styled(icon, Style::default().fg(nc)),
                    Span::styled(bomb, Style::default()),
                    Span::styled(name_str, Style::default().fg(if you { C_ACCENT } else { C_WHITE })
                        .add_modifier(if you { Modifier::BOLD } else { Modifier::empty() })),
                    Span::styled(format!("{:>3}HP", p.health), Style::default().fg(hpc)),
                ]))
            }).collect();

        let alive = app.game.players.values()
            .filter(|p| p.team == *team && p.is_alive()).count();
        f.render_widget(
            List::new(items).block(Block::default()
                .title(Span::styled(format!(" {label} ({alive} alive) "), Style::default().fg(*color).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(Style::default().fg(*color))
                .style(Style::default().bg(C_PANEL))),
            halves[half_idx],
        );
    }
}

fn draw_bottom(f: &mut Frame, area: Rect, app: &PlayApp) {
    let rows = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(area);
    let bp = app.bomb_phase();

    // Bomb / defuse bar (if applicable)
    if matches!(bp, BombPhase::Planted | BombPhase::Defusing) {
        let bomb_t = app.game.current_round.as_ref().map(|r| r.bomb.timer_secs).unwrap_or(0.0);
        let max_t  = app.game.current_round.as_ref().map(|r| r.bomb.detonation_timer_secs).unwrap_or(40.0);
        let frac = (bomb_t / max_t).clamp(0.0, 1.0) as f64;
        let col = if frac > 0.5 { C_ACCENT } else if frac > 0.2 { Color::Rgb(249,115,22) } else { C_HP };
        let label = format!(" C4 {:.0}s  {} ", bomb_t, if bp == BombPhase::Defusing { "[ DEFUSING ]" } else { "" });
        let g = Gauge::default()
            .block(Block::default().title(Span::styled(label, Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL).border_style(Style::default().fg(col)).style(Style::default().bg(C_PANEL)))
            .gauge_style(Style::default().fg(col).bg(C_PANEL))
            .ratio(frac);
        f.render_widget(g, rows[0]);
    } else {
        // Player HP/armor bar
        let hp = app.player().map(|p| p.health).unwrap_or(0);
        let armor = app.player().map(|p| p.armor.vest_hp).unwrap_or(0);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  HP: {:>3}/100  ", hp), Style::default().fg(if hp>60 { C_GREEN } else { C_HP }).add_modifier(Modifier::BOLD)),
                Span::styled(format!("Armor: {:>3}  ", armor), Style::default().fg(C_CT)),
            ])).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)).style(Style::default().bg(C_PANEL))),
            rows[0],
        );
    }

    // Actions + player info
    let p = app.player();
    let dead = !app.player_alive();
    let money = p.map(|p| p.money).unwrap_or(0);
    let weapon = p.and_then(|p| p.inventory.active_weapon()).map(|w| w.stats.name.as_str()).unwrap_or("Knife");
    let ammo_m = p.and_then(|p| p.inventory.active_weapon()).map(|w| w.ammo.bullets_in_magazine).unwrap_or(0);
    let ammo_r = p.and_then(|p| p.inventory.active_weapon()).map(|w| w.ammo.reserve_bullets).unwrap_or(0);
    let has_kit = p.map(|p| p.armor.has_defuse_kit).unwrap_or(false);
    let team = p.map(|p| p.team).unwrap_or(Team::Spectator);
    let nades = p.map(|p| p.inventory.throwables.len()).unwrap_or(0);

    let phase = app.round_phase();
    let actions = if dead {
        "[Spectating — waiting for next round]".to_string()
    } else if app.buy_open {
        "[Buy menu open — B/Esc to close]".to_string()
    } else {
        let mut acts = vec!["[S]/LClick:Shoot", "RClick:Scope"];
        if nades > 0 { acts.push("[G]Nade"); }
        if team == Team::T { acts.push("[P]Plant"); }
        if team == Team::CT && bp == BombPhase::Planted { acts.push("[D]Defuse"); }
        if phase == RoundPhase::FreezeTime { acts.push("[B]Buy"); }
        format!("{}  |  [Q]Quit", acts.join("  "))
    };

    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("${:<6}", money), Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<16}", weapon), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:>2}/{:<3}", ammo_m, ammo_r), Style::default().fg(C_WHITE)),
                if has_kit { Span::styled("  [KIT]", Style::default().fg(C_CT)) } else { Span::raw("") },
                Span::styled(format!("  Nades:{}", nades), Style::default().fg(if nades>0{C_GREEN}else{C_DIM})),
                if dead { Span::styled("  *** DEAD ***", Style::default().fg(C_HP).add_modifier(Modifier::BOLD)) }
                else { Span::raw("") },
            ]),
            Line::from(Span::styled(actions, Style::default().fg(C_DIM))),
        ]).block(panel("PLAYER", C_BORDER)),
        rows[1],
    );
}

// ── Round end ─────────────────────────────────────────────────────────────────
fn draw_round_end(f: &mut Frame, app: &PlayApp, result: RoundResult, mvp: &str) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Fill(1), Constraint::Length(14), Constraint::Fill(1)]).split(area);
    let cols = Layout::horizontal([Constraint::Fill(1), Constraint::Length(52), Constraint::Fill(1)]).split(rows[1]);

    let (title, color) = match result {
        RoundResult::CTWin(_) => ("COUNTER-TERRORISTS WIN", C_CT),
        RoundResult::TWin(_)  => ("TERRORISTS WIN", C_T),
        RoundResult::InProgress => ("ROUND OVER", C_DIM),
    };
    let reason = match result {
        RoundResult::CTWin(r) => format!("{:?}", r),
        RoundResult::TWin(r)  => format!("{:?}", r),
        RoundResult::InProgress => String::new(),
    };
    let player_team = app.player().map(|p| p.team).unwrap_or(Team::Spectator);
    let won = matches!((result, player_team),
        (RoundResult::CTWin(_), Team::CT) | (RoundResult::TWin(_), Team::T));
    let (outcome, oc) = if won { ("YOU WON!", C_GREEN) } else { ("YOU LOST", C_HP) };

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(title, Style::default().fg(color).add_modifier(Modifier::BOLD | Modifier::UNDERLINED))),
            Line::from(Span::styled(format!("Reason: {}", reason), Style::default().fg(C_DIM))),
            Line::from(""),
            Line::from(Span::styled(outcome, Style::default().fg(oc).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("★ MVP: ", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(mvp, Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Score  CT:", Style::default().fg(C_CT).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", app.game.score.ct), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
                Span::styled("— T:", Style::default().fg(C_T).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", app.game.score.t), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled(format!("Your money: ${}  (next round: buy phase)", app.player().map(|p|p.money).unwrap_or(0)), Style::default().fg(C_MONEY))),
            Line::from(""),
            Line::from(Span::styled("Next round starting automatically...", Style::default().fg(C_DIM))),
        ])
        .block(panel("ROUND RESULT", color))
        .alignment(Alignment::Center),
        cols[1],
    );
}

// ── Match end ─────────────────────────────────────────────────────────────────
fn draw_match_end(f: &mut Frame, app: &PlayApp) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Fill(1), Constraint::Length(16), Constraint::Fill(1)]).split(area);
    let cols = Layout::horizontal([Constraint::Fill(1), Constraint::Length(56), Constraint::Fill(1)]).split(rows[1]);

    let winner = app.game.winner();
    let player_team = app.player().map(|p| p.team).unwrap_or(Team::Spectator);
    let (title, color, outcome) = match winner {
        Some(Team::CT) => ("CT WINS THE MATCH!", C_CT, if player_team == Team::CT { "VICTORY!" } else { "DEFEAT" }),
        Some(Team::T)  => ("T WINS THE MATCH!",  C_T,  if player_team == Team::T  { "VICTORY!" } else { "DEFEAT" }),
        _ => ("MATCH TIED!", C_ACCENT, "TIED"),
    };
    let oc = if outcome == "VICTORY!" { C_GREEN } else if outcome == "DEFEAT" { C_HP } else { C_ACCENT };

    let p = app.player();
    let kills = p.map(|p| p.round_kills).unwrap_or(0);
    let money = p.map(|p| p.money).unwrap_or(0);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(title, Style::default().fg(color).add_modifier(Modifier::BOLD | Modifier::UNDERLINED))),
            Line::from(""),
            Line::from(Span::styled(outcome, Style::default().fg(oc).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Final Score  CT:", Style::default().fg(C_CT).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", app.game.score.ct), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
                Span::styled("— T:", Style::default().fg(C_T).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", app.game.score.t), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Your kills this round: ", Style::default().fg(C_DIM)),
                Span::styled(kills.to_string(), Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
                Span::styled("   Final money: $", Style::default().fg(C_DIM)),
                Span::styled(money.to_string(), Style::default().fg(C_MONEY).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled("[Q] Quit", Style::default().fg(C_DIM))),
        ])
        .block(panel("MATCH OVER", color))
        .alignment(Alignment::Center),
        cols[1],
    );
}
