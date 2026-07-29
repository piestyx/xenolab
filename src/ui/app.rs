use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::{Alignment, Frame};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::engine::ids::{NodeId, ObjectiveId};
use crate::engine::interventions::Intervention;
use crate::engine::measurement::MeasurementRecord;
use crate::engine::run::{RunStatus, ACTION_LIMIT};
use crate::engine::sim::{SimError, Simulator};
use crate::engine::world::WorldState;
use crate::ui::{view_debrief, view_journal, view_lab, view_log};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Lab,
    Journal,
    Log,
}

impl ActiveView {
    pub fn as_index(self) -> usize {
        match self {
            Self::Lab => 0,
            Self::Journal => 1,
            Self::Log => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    SetUvLow,
    SetUvHigh,
    AddNutrient,
    AddToxin,
    NeutralizeToxin,
    RemoveFungus,
    RemoveBacteria,
    SterilizeSample,
    ScanPopulation,
    ScanChemicals,
    AdvanceTime,
}

impl MenuAction {
    pub const ALL: [Self; 11] = [
        Self::SetUvLow,
        Self::SetUvHigh,
        Self::AddNutrient,
        Self::AddToxin,
        Self::NeutralizeToxin,
        Self::RemoveFungus,
        Self::RemoveBacteria,
        Self::SterilizeSample,
        Self::ScanPopulation,
        Self::ScanChemicals,
        Self::AdvanceTime,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::SetUvLow => "Set UV Low",
            Self::SetUvHigh => "Set UV High",
            Self::AddNutrient => "Add Nutrient (+20)",
            Self::AddToxin => "Add Toxin (+20)",
            Self::NeutralizeToxin => "Neutralize Toxin (-20)",
            Self::RemoveFungus => "Remove Fungus",
            Self::RemoveBacteria => "Remove Bacteria",
            Self::SterilizeSample => "Sterilize Sample",
            Self::ScanPopulation => "Scan Population",
            Self::ScanChemicals => "Scan Chemicals",
            Self::AdvanceTime => "Advance Time",
        }
    }

    pub fn to_intervention(self) -> Intervention {
        match self {
            Self::SetUvLow => Intervention::SetUvLow,
            Self::SetUvHigh => Intervention::SetUvHigh,
            Self::AddNutrient => Intervention::add_nutrient_default(),
            Self::AddToxin => Intervention::add_toxin_default(),
            Self::NeutralizeToxin => Intervention::neutralize_toxin_default(),
            Self::RemoveFungus => Intervention::RemoveFungus,
            Self::RemoveBacteria => Intervention::RemoveBacteria,
            Self::SterilizeSample => Intervention::SterilizeSample,
            Self::ScanPopulation => Intervention::ScanPopulation,
            Self::ScanChemicals => Intervention::ScanChemicals,
            Self::AdvanceTime => Intervention::AdvanceTime,
        }
    }
}

pub struct App {
    pub simulator: Simulator,
    pub seed: u64,
    pub objective: ObjectiveId,
    pub active_view: ActiveView,
    pub menu_index: usize,
    pub log_scroll: usize,
    pub journal_scroll: u16,
    pub should_quit: bool,
    pub status_message: String,
    pub last_measurements: Vec<MeasurementRecord>,
    pub last_state_snapshot: Option<WorldState>,
    pub last_event_summary: Option<String>,
    pub pending_seed_input: Option<String>,
}

impl App {
    pub fn new(seed: u64) -> Self {
        let recipe = crate::worldgen::generate_playable(seed);
        let objective = recipe.objective;
        Self {
            simulator: Simulator::new(recipe),
            seed,
            objective,
            active_view: ActiveView::Lab,
            menu_index: 0,
            log_scroll: 0,
            journal_scroll: 0,
            should_quit: false,
            status_message: format!("Seed {seed}"),
            last_measurements: Vec::new(),
            last_state_snapshot: None,
            last_event_summary: None,
            pending_seed_input: None,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(frame.size());

        let titles = ["1 Lab", "2 Journal", "3 Log"];
        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .title("xenolab v0.2.0")
                    .borders(Borders::ALL),
            )
            .select(self.active_view.as_index())
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(tabs, chunks[0]);

        if self.is_resolved() {
            view_debrief::render(frame, chunks[1], self);
        } else {
            match self.active_view {
                ActiveView::Lab => view_lab::render_lab(frame, self, chunks[1]),
                ActiveView::Journal => view_journal::render_journal(frame, self, chunks[1]),
                ActiveView::Log => view_log::render(frame, chunks[1], self),
            }
        }

        let footer = Paragraph::new(self.footer_text())
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right);
        let footer_area = ratatui::layout::Rect {
            x: chunks[1].x,
            y: chunks[1]
                .y
                .saturating_add(chunks[1].height.saturating_sub(1)),
            width: chunks[1].width.saturating_sub(1),
            height: 1,
        };
        frame.render_widget(footer, footer_area);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<(), SimError> {
        if self.pending_seed_input.is_some() {
            return self.handle_seed_input(key);
        }

        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('1') => self.active_view = ActiveView::Lab,
            KeyCode::Char('2') => self.active_view = ActiveView::Journal,
            KeyCode::Char('3') => self.active_view = ActiveView::Log,
            KeyCode::Char('r') if self.is_resolved() => self.restart_same_seed(),
            KeyCode::Char('n') if self.is_resolved() => self.begin_new_seed(),
            KeyCode::Char('j') => {
                if self.active_view == ActiveView::Journal {
                    self.journal_scroll = self.journal_scroll.saturating_add(1);
                }
            }
            KeyCode::Char('k') => {
                if self.active_view == ActiveView::Journal {
                    self.journal_scroll = self.journal_scroll.saturating_sub(1);
                }
            }
            KeyCode::PageDown => {
                if self.active_view == ActiveView::Journal {
                    self.journal_scroll = self.journal_scroll.saturating_add(8);
                }
            }
            KeyCode::PageUp => {
                if self.active_view == ActiveView::Journal {
                    self.journal_scroll = self.journal_scroll.saturating_sub(8);
                }
            }
            KeyCode::Up => self.handle_up(),
            KeyCode::Down => self.handle_down(),
            KeyCode::Enter => {
                if self.active_view == ActiveView::Lab && !self.is_resolved() {
                    self.apply_selected_action()?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn actions(&self) -> &'static [MenuAction] {
        &MenuAction::ALL
    }

    pub fn delta_for(&self, node: NodeId) -> f32 {
        match self.last_state_snapshot {
            Some(previous) => self.simulator.state().get(node) - previous.get(node),
            None => 0.0,
        }
    }

    pub fn objective_summary_short(&self) -> &'static str {
        match self.objective {
            ObjectiveId::StabilizePlant => "Target: plant >= 60 for 3 evaluations",
            ObjectiveId::Detox => "Target: toxin <= 15 for 3 evaluations",
            ObjectiveId::PreventCollapse => "Target: plant and bacteria >= 25 for 3 evaluations",
        }
    }

    pub fn objective_goal_text(&self) -> &'static str {
        match self.objective {
            ObjectiveId::StabilizePlant => {
                "Keep plant population at or above 60 for 3 consecutive evaluations."
            }
            ObjectiveId::Detox => {
                "Reduce toxin concentration to 15 or below for 3 consecutive evaluations."
            }
            ObjectiveId::PreventCollapse => {
                "Maintain bacteria >= 25 and plant >= 25 for 3 consecutive evaluations."
            }
        }
    }

    pub fn objective_failure_text(&self) -> &'static str {
        match self.objective {
            ObjectiveId::StabilizePlant | ObjectiveId::Detox | ObjectiveId::PreventCollapse => {
                "A run fails when all 30 actions are used without completing the objective."
            }
        }
    }

    pub fn is_resolved(&self) -> bool {
        self.simulator.run_state().status != RunStatus::Active
    }

    pub fn objective_progress_text(&self) -> String {
        let progress = self.simulator.run_state().objective_progress;
        format!("{} / {} evaluations", progress.current, progress.required)
    }

    pub fn actions_remaining(&self) -> u32 {
        self.simulator.run_state().actions_remaining()
    }

    pub fn action_limit(&self) -> u32 {
        ACTION_LIMIT
    }

    fn footer_text(&self) -> &'static str {
        if self.pending_seed_input.is_some() {
            return "digits enter confirm | esc cancel | q quit";
        }
        if self.is_resolved() {
            return "r same seed | n new seed | q quit";
        }
        match self.active_view {
            ActiveView::Lab => "q quit | 1/2/3 tabs | arrows actions | enter apply | 30 actions",
            ActiveView::Journal => {
                "q quit | 1/2/3 tabs | up/down or j/k scroll | pgup/pgdn fast scroll"
            }
            ActiveView::Log => "q quit | 1/2/3 tabs | up/down scroll log",
        }
    }

    fn handle_up(&mut self) {
        if self.is_resolved() {
            return;
        }
        match self.active_view {
            ActiveView::Lab => {
                if self.menu_index == 0 {
                    self.menu_index = self.actions().len().saturating_sub(1);
                } else {
                    self.menu_index = self.menu_index.saturating_sub(1);
                }
            }
            ActiveView::Journal => {
                self.journal_scroll = self.journal_scroll.saturating_sub(1);
            }
            ActiveView::Log => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
        }
    }

    fn handle_down(&mut self) {
        if self.is_resolved() {
            return;
        }
        match self.active_view {
            ActiveView::Lab => {
                let len = self.actions().len();
                self.menu_index = if len == 0 {
                    0
                } else {
                    (self.menu_index + 1) % len
                };
            }
            ActiveView::Journal => {
                self.journal_scroll = self.journal_scroll.saturating_add(1);
            }
            ActiveView::Log => {
                self.log_scroll = self.log_scroll.saturating_add(1);
            }
        }
    }

    fn apply_selected_action(&mut self) -> Result<(), SimError> {
        let action = self.actions()[self.menu_index];
        let intervention = action.to_intervention();
        let before_state = *self.simulator.state();
        let before_tick = self.simulator.tick_index();

        let event = self.simulator.apply(intervention.clone())?;
        let after_state = *self.simulator.state();
        let state_changed = before_state != after_state;

        self.update_snapshot(before_state, before_tick, event.tick_index, state_changed);

        self.status_message = format!("tick={} action={}", event.tick_index, intervention.label());
        self.last_measurements = event.measurements.clone();
        self.last_event_summary = Some(format!(
            "action={} tick={} measurements={}",
            intervention.label(),
            event.tick_index,
            event.measurements.len()
        ));

        Ok(())
    }

    fn update_snapshot(
        &mut self,
        previous_state: WorldState,
        previous_tick: u32,
        event_tick: u32,
        state_changed: bool,
    ) {
        if event_tick > previous_tick || state_changed {
            self.last_state_snapshot = Some(previous_state);
        }
    }

    fn handle_seed_input(&mut self, key: KeyEvent) -> Result<(), SimError> {
        match key.code {
            KeyCode::Esc => {
                self.pending_seed_input = None;
                self.status_message = "New seed cancelled".to_string();
            }
            KeyCode::Backspace => {
                if let Some(input) = self.pending_seed_input.as_mut() {
                    input.pop();
                }
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                if let Some(input) = self.pending_seed_input.as_mut() {
                    if input.len() < 20 {
                        input.push(ch);
                    }
                }
            }
            KeyCode::Enter => {
                let Some(input) = self.pending_seed_input.as_deref() else {
                    return Ok(());
                };
                match input.parse::<u64>() {
                    Ok(seed) => *self = Self::new(seed),
                    Err(_) => {
                        self.status_message =
                            "Invalid decimal u64 seed; enter digits or press Esc".to_string();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn restart_same_seed(&mut self) {
        let seed = self.seed;
        *self = Self::new(seed);
    }

    fn begin_new_seed(&mut self) {
        self.pending_seed_input = Some(String::new());
        self.status_message = "Enter a decimal u64 seed".to_string();
    }
}
