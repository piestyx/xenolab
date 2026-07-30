use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::{Alignment, Frame};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use thiserror::Error;

use crate::engine::contamination::ContaminationLevel;
use crate::engine::ids::{NodeId, ObjectiveId};
use crate::engine::interventions::Intervention;
use crate::engine::measurement::MeasurementRecord;
use crate::engine::notebook::{HypothesisDirection, HypothesisId, ObservableVariable};
use crate::engine::publication::PublicationError;
use crate::engine::repair::{RepairError, RepairTrack};
use crate::engine::run::{RunStatus, ACTION_LIMIT};
use crate::engine::sim::{SimError, Simulator};
use crate::engine::world::WorldState;
use crate::ui::{view_debrief, view_journal, view_lab, view_log, view_notebook, view_repairs};

pub const MIN_TERMINAL_WIDTH: u16 = 80;
pub const MIN_TERMINAL_HEIGHT: u16 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFilter {
    All,
    Interventions,
    Measurements,
    Publications,
    Repairs,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SeedParseError {
    #[error("seed is empty")]
    Empty,
    #[error("seed must be a decimal u64")]
    Invalid,
    #[error("seed is larger than u64::MAX")]
    Overflow,
}

pub fn parse_seed(input: &str) -> Result<u64, SeedParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(SeedParseError::Empty);
    }
    if !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(SeedParseError::Invalid);
    }
    trimmed.parse::<u64>().map_err(|_| SeedParseError::Overflow)
}

impl LogFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Interventions => "interventions",
            Self::Measurements => "measurements",
            Self::Publications => "publications",
            Self::Repairs => "repairs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Lab,
    Journal,
    Log,
    Notebook,
    Repairs,
}

impl ActiveView {
    pub fn as_index(self) -> usize {
        match self {
            Self::Lab => 0,
            Self::Journal => 1,
            Self::Log => 2,
            Self::Notebook => 3,
            Self::Repairs => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotebookField {
    Cause,
    Direction,
    Effect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotebookEditor {
    pub id: Option<HypothesisId>,
    pub cause: usize,
    pub direction: HypothesisDirection,
    pub effect: usize,
    pub field: NotebookField,
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

    pub fn contamination_cost(self) -> u32 {
        self.to_intervention().contamination_cost()
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::SetUvLow | Self::SetUvHigh => "Sets the environmental UV level",
            Self::AddNutrient => "Increases nutrient concentration",
            Self::AddToxin => "Increases toxin concentration",
            Self::NeutralizeToxin => "Reduces toxin concentration",
            Self::RemoveFungus => "Removes fungus population",
            Self::RemoveBacteria => "Removes bacteria population",
            Self::SterilizeSample => "Removes fungus from the sample",
            Self::ScanPopulation => "Measures organism populations",
            Self::ScanChemicals => "Measures toxin and nutrient concentrations",
            Self::AdvanceTime => "Advances ecosystem simulation without an intervention",
        }
    }

    pub fn measurement_category(self) -> Option<&'static str> {
        match self {
            Self::ScanPopulation => Some("population instrument"),
            Self::ScanChemicals => Some("chemical instrument"),
            _ => None,
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
    pub notebook_selected: usize,
    pub notebook_editor: Option<NotebookEditor>,
    pub notebook_delete_confirmation: Option<HypothesisId>,
    pub notebook_publish_confirmation: Option<HypothesisId>,
    pub repair_selected: usize,
    pub repair_confirmation: Option<RepairTrack>,
    pub log_filter: LogFilter,
    pub repeat_last_action: Option<Intervention>,
    pub help_open: bool,
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
            notebook_selected: 0,
            notebook_editor: None,
            notebook_delete_confirmation: None,
            notebook_publish_confirmation: None,
            repair_selected: 0,
            repair_confirmation: None,
            log_filter: LogFilter::All,
            repeat_last_action: None,
            help_open: false,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.size());

        let titles = ["1 Lab", "2 Journal", "3 Log", "4 Notebook", "5 Repairs"];
        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .title("xenolab v0.9.0")
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

        let status = Paragraph::new(self.persistent_status_text())
            .block(Block::default().title("Run Status").borders(Borders::ALL))
            .wrap(ratatui::widgets::Wrap { trim: false })
            .style(Style::default().fg(Color::White));
        frame.render_widget(status, chunks[1]);

        if frame.size().width < MIN_TERMINAL_WIDTH || frame.size().height < MIN_TERMINAL_HEIGHT {
            let message = Paragraph::new(format!(
                "Terminal too small. Resize to at least {}×{} to continue. Press q to quit.",
                MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT
            ))
            .block(
                Block::default()
                    .title("Resize required")
                    .borders(Borders::ALL),
            );
            frame.render_widget(message, chunks[2]);
            self.render_footer(frame, chunks[3]);
            return;
        }

        if self.help_open {
            let help = Paragraph::new(self.help_text())
                .block(
                    Block::default()
                        .title("Help (press ? to close)")
                        .borders(Borders::ALL),
                )
                .wrap(ratatui::widgets::Wrap { trim: false });
            frame.render_widget(help, chunks[2]);
            self.render_footer(frame, chunks[3]);
            return;
        }

        if self.is_resolved()
            && self.active_view != ActiveView::Notebook
            && self.active_view != ActiveView::Repairs
        {
            view_debrief::render(frame, chunks[2], self);
        } else {
            match self.active_view {
                ActiveView::Lab => view_lab::render_lab(frame, self, chunks[2]),
                ActiveView::Journal => view_journal::render_journal(frame, self, chunks[2]),
                ActiveView::Log => view_log::render(frame, chunks[2], self),
                ActiveView::Notebook => view_notebook::render(frame, chunks[2], self),
                ActiveView::Repairs => view_repairs::render(frame, chunks[2], self),
            }
        }
        self.render_footer(frame, chunks[3]);
    }

    fn render_footer(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let footer = Paragraph::new(self.footer_text())
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right);
        frame.render_widget(footer, area);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<(), SimError> {
        if key.code == KeyCode::Char('q') {
            self.should_quit = true;
            return Ok(());
        }
        if key.code == KeyCode::Char('?') {
            self.help_open = !self.help_open;
            return Ok(());
        }
        if self.help_open {
            return Ok(());
        }
        if self.pending_seed_input.is_some() {
            return self.handle_seed_input(key);
        }
        if self.notebook_editor.is_some() {
            self.handle_notebook_editor_key(key);
            return Ok(());
        }
        if self.notebook_delete_confirmation.is_some() {
            match key.code {
                KeyCode::Enter => self.confirm_notebook_delete(),
                KeyCode::Esc => self.notebook_delete_confirmation = None,
                _ => {}
            }
            return Ok(());
        }
        if self.notebook_publish_confirmation.is_some() {
            match key.code {
                KeyCode::Enter => self.confirm_notebook_publish(),
                KeyCode::Esc => self.notebook_publish_confirmation = None,
                _ => {}
            }
            return Ok(());
        }
        if self.repair_confirmation.is_some() {
            match key.code {
                KeyCode::Enter => self.confirm_repair_purchase(),
                KeyCode::Esc => self.repair_confirmation = None,
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('1') => self.active_view = ActiveView::Lab,
            KeyCode::Char('2') => self.active_view = ActiveView::Journal,
            KeyCode::Char('3') => self.active_view = ActiveView::Log,
            KeyCode::Char('4') => self.active_view = ActiveView::Notebook,
            KeyCode::Char('5') => self.active_view = ActiveView::Repairs,
            KeyCode::Char('x') if self.active_view == ActiveView::Lab && !self.is_resolved() => {
                self.repeat_last_action()?;
            }
            KeyCode::Char('a') if self.active_view == ActiveView::Log => {
                self.log_filter = LogFilter::All;
                self.log_scroll = 0;
            }
            KeyCode::Char('i') if self.active_view == ActiveView::Log => {
                self.log_filter = LogFilter::Interventions;
                self.log_scroll = 0;
            }
            KeyCode::Char('m') if self.active_view == ActiveView::Log => {
                self.log_filter = LogFilter::Measurements;
                self.log_scroll = 0;
            }
            KeyCode::Char('p') if self.active_view == ActiveView::Log => {
                self.log_filter = LogFilter::Publications;
                self.log_scroll = 0;
            }
            KeyCode::Char('r') if self.active_view == ActiveView::Log => {
                self.log_filter = LogFilter::Repairs;
                self.log_scroll = 0;
            }
            KeyCode::Char('a') if self.active_view == ActiveView::Notebook => {
                self.begin_notebook_add();
            }
            KeyCode::Char('e') if self.active_view == ActiveView::Notebook => {
                self.begin_notebook_edit();
            }
            KeyCode::Char('d') if self.active_view == ActiveView::Notebook => {
                self.begin_notebook_delete();
            }
            KeyCode::Char('p') if self.active_view == ActiveView::Notebook => {
                self.begin_notebook_publish();
            }
            KeyCode::Enter if self.active_view == ActiveView::Repairs && !self.is_resolved() => {
                self.begin_repair_purchase();
            }
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
        match self.objective {
            ObjectiveId::StabilizePlant => format!(
                "Plant stability: {} / {} consecutive evaluations at 60+",
                progress.current, progress.required
            ),
            ObjectiveId::Detox => format!(
                "Detox hold: {} / {} consecutive evaluations at toxin 15 or below",
                progress.current, progress.required
            ),
            ObjectiveId::PreventCollapse => format!(
                "Collapse prevention: {} / {} evaluations with plant and bacteria at 25+",
                progress.current, progress.required
            ),
        }
    }

    pub fn actions_remaining(&self) -> u32 {
        self.simulator.run_state().actions_remaining()
    }

    pub fn action_limit(&self) -> u32 {
        ACTION_LIMIT
    }

    pub fn contamination_level(&self) -> ContaminationLevel {
        self.simulator.contamination_level()
    }

    pub fn contamination_next_threshold(&self) -> Option<u32> {
        self.contamination_level().next_threshold()
    }

    pub fn contamination_noise_multiplier(&self) -> f32 {
        self.contamination_level().noise_multiplier()
    }

    pub fn terminal_size_supported(width: u16, height: u16) -> bool {
        width >= MIN_TERMINAL_WIDTH && height >= MIN_TERMINAL_HEIGHT
    }

    fn persistent_status_text(&self) -> String {
        let status = match self.simulator.run_state().status {
            RunStatus::Active => "ACTIVE",
            RunStatus::Won => "RUN COMPLETE",
            RunStatus::Failed(_) => "RUN FAILED",
        };
        let pressure = if self.actions_remaining() <= 5 {
            " | LOW ACTIONS"
        } else {
            ""
        };
        format!(
            "{} | {} | actions {}/{} | contamination {:.0} {} | scan ×{:.2} | credits {} | repairs C{} / K{}{} | {}",
            status,
            self.objective_progress_text(),
            self.actions_remaining(),
            self.action_limit(),
            self.simulator.contamination(),
            self.contamination_level().label(),
            self.contamination_noise_multiplier() * self.simulator.calibration_multiplier(),
            self.simulator.credits_available(),
            self.simulator.calibration_level().level(),
            self.simulator.containment_level().level(),
            pressure,
            self.status_message,
        )
    }

    fn help_text(&self) -> String {
        "Tabs: 1 Lab | 2 Journal | 3 Log | 4 Notebook | 5 Repairs

Lab: Up/Down select | Enter apply | x repeat last action
Journal: Up/Down or j/k scroll | PageUp/PageDown jump
Log: Up/Down scroll | a all | i interventions | m measurements | p publications | r repairs
Notebook: a add | e edit | d delete | p publish | Enter confirm | Esc cancel
Repairs: Up/Down select | Enter purchase | Esc cancel
Dialogs: Enter confirm | Esc cancel | ? close help
Run: r same-seed restart | n new-seed restart after resolution | q quit

Scans cost one action but no tick. Publications cost one action. Repairs cost no action."
            .to_string()
    }

    fn footer_text(&self) -> &'static str {
        if self.pending_seed_input.is_some() {
            return "digits enter confirm | esc cancel | q quit";
        }
        if self.is_resolved() {
            return "r same seed | n new seed | q quit";
        }
        match self.active_view {
            ActiveView::Lab => {
                "? help | q quit | tabs 1-5 | arrows select | enter apply | x repeat"
            }
            ActiveView::Journal => "? help | q quit | tabs 1-5 | up/down or j/k scroll | pgup/pgdn",
            ActiveView::Log => "? help | q quit | tabs 1-5 | scroll | a/i/m/p/r filters",
            ActiveView::Notebook => {
                "? help | q quit | tabs 1-5 | a add | e edit | d delete | p publish"
            }
            ActiveView::Repairs => "? help | q quit | tabs 1-5 | arrows select | enter purchase",
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
            ActiveView::Notebook => {
                self.notebook_selected = self.notebook_selected.saturating_sub(1);
            }
            ActiveView::Repairs => {
                self.repair_selected = self.repair_selected.saturating_sub(1);
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
            ActiveView::Notebook => {
                let len = self.simulator.notebook().hypotheses().len();
                if len > 0 {
                    self.notebook_selected = (self.notebook_selected + 1) % len;
                }
            }
            ActiveView::Repairs => {
                self.repair_selected = (self.repair_selected + 1) % 2;
            }
        }
    }

    fn apply_selected_action(&mut self) -> Result<(), SimError> {
        let action = self.actions()[self.menu_index];
        self.apply_intervention(action.to_intervention())
    }

    fn repeat_last_action(&mut self) -> Result<(), SimError> {
        let Some(intervention) = self.repeat_last_action.clone() else {
            self.status_message = "No accepted Lab action to repeat".to_string();
            return Ok(());
        };
        self.apply_intervention(intervention)
    }

    fn apply_intervention(&mut self, intervention: Intervention) -> Result<(), SimError> {
        let level_before = self.simulator.contamination_level();
        let before_state = *self.simulator.state();
        let before_tick = self.simulator.tick_index();
        let progress_before = self.simulator.run_state().objective_progress;

        let event = self.simulator.apply(intervention.clone())?;
        let after_state = *self.simulator.state();
        let state_changed = before_state != after_state;

        self.update_snapshot(before_state, before_tick, event.tick_index, state_changed);

        self.status_message = format!("tick={} action={}", event.tick_index, intervention.label());
        let progress_after = self.simulator.run_state().objective_progress;
        if progress_after.current < progress_before.current {
            self.status_message = format!(
                "Objective progress reset: {}",
                self.objective_reset_message()
            );
        }
        let level_after = self.simulator.contamination_level();
        if level_after != level_before {
            match level_after {
                ContaminationLevel::Compromised => {
                    self.status_message =
                        "Warning: contamination is COMPROMISED; scans are less precise".to_string();
                }
                ContaminationLevel::Critical => {
                    self.status_message =
                        "Warning: contamination is CRITICAL; containment fails at 40".to_string();
                }
                ContaminationLevel::Stable | ContaminationLevel::Lost => {}
            }
        }
        self.last_measurements = event.measurements.clone();
        self.last_event_summary = Some(format!(
            "action={} tick={} measurements={}",
            intervention.label(),
            event.tick_index,
            event.measurements.len()
        ));
        self.repeat_last_action = Some(intervention);

        Ok(())
    }

    fn objective_reset_message(&self) -> &'static str {
        match self.objective {
            ObjectiveId::StabilizePlant => "plant fell below 60",
            ObjectiveId::Detox => "toxin rose above 15",
            ObjectiveId::PreventCollapse => "plant or bacteria fell below 25",
        }
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

    fn begin_notebook_add(&mut self) {
        if self.is_resolved() {
            self.status_message = "Notebook is read-only after run resolution".to_string();
            return;
        }
        let notebook = self.simulator.notebook();
        if notebook.remaining_slots() == 0 {
            self.status_message = "Notebook is full (8 / 8)".to_string();
            return;
        }
        self.notebook_editor = Some(NotebookEditor {
            id: None,
            cause: 0,
            direction: HypothesisDirection::Increases,
            effect: 1,
            field: NotebookField::Cause,
        });
    }

    fn selected_repair_track(&self) -> RepairTrack {
        if self.repair_selected == 0 {
            RepairTrack::Calibration
        } else {
            RepairTrack::Containment
        }
    }

    fn begin_repair_purchase(&mut self) {
        if self.is_resolved() {
            self.status_message = "Repairs are unavailable after run resolution".to_string();
            return;
        }
        let track = self.selected_repair_track();
        let next_cost = match track {
            RepairTrack::Calibration => self.simulator.calibration_level().next_cost(),
            RepairTrack::Containment => self.simulator.containment_level().next_cost(),
        };
        let Some(cost) = next_cost else {
            self.status_message = "Repair track is already at maximum level".to_string();
            return;
        };
        let available = self.simulator.credits_available();
        if available < cost {
            self.status_message =
                format!("Insufficient credits: requires {}, has {}", cost, available);
            return;
        }
        self.repair_confirmation = Some(track);
    }

    fn confirm_repair_purchase(&mut self) {
        let Some(track) = self.repair_confirmation.take() else {
            return;
        };
        match self.simulator.purchase_repair(track) {
            Ok(purchase) => {
                self.status_message = format!(
                    "{} repaired to L{}; spent {} credits, {} available",
                    track.label(),
                    purchase.level_after,
                    purchase.credits_spent,
                    purchase.credits_remaining
                );
            }
            Err(error) => self.status_message = repair_error_text(error),
        }
    }

    fn begin_notebook_edit(&mut self) {
        if self.is_resolved() {
            self.status_message = "Notebook is read-only after run resolution".to_string();
            return;
        }
        let Some(hypothesis) = self
            .simulator
            .notebook()
            .hypotheses()
            .get(self.notebook_selected)
        else {
            self.status_message = "No hypothesis selected".to_string();
            return;
        };
        let cause = ObservableVariable::ALL
            .iter()
            .position(|variable| *variable == hypothesis.cause)
            .unwrap_or(0);
        let effect = ObservableVariable::ALL
            .iter()
            .position(|variable| *variable == hypothesis.effect)
            .unwrap_or(0);
        self.notebook_editor = Some(NotebookEditor {
            id: Some(hypothesis.id),
            cause,
            direction: hypothesis.direction,
            effect,
            field: NotebookField::Cause,
        });
    }

    fn begin_notebook_delete(&mut self) {
        if self.is_resolved() {
            self.status_message = "Notebook is read-only after run resolution".to_string();
            return;
        }
        if let Some(hypothesis) = self
            .simulator
            .notebook()
            .hypotheses()
            .get(self.notebook_selected)
        {
            self.notebook_delete_confirmation = Some(hypothesis.id);
        } else {
            self.status_message = "No hypothesis selected".to_string();
        }
    }

    fn begin_notebook_publish(&mut self) {
        if self.is_resolved() {
            self.status_message = "Publication is unavailable after run resolution".to_string();
            return;
        }
        let Some(hypothesis) = self
            .simulator
            .notebook()
            .hypotheses()
            .get(self.notebook_selected)
        else {
            self.status_message = "No hypothesis selected".to_string();
            return;
        };
        if self.simulator.publication_for(hypothesis.id).is_some() {
            self.status_message = "This hypothesis has already been published".to_string();
            return;
        }
        if self.simulator.publications().len() as u32 >= self.simulator.publication_limit() {
            self.status_message = "Publication limit reached".to_string();
            return;
        }
        self.notebook_publish_confirmation = Some(hypothesis.id);
    }

    fn confirm_notebook_publish(&mut self) {
        let Some(id) = self.notebook_publish_confirmation.take() else {
            return;
        };
        match self.simulator.publish_hypothesis(id) {
            Ok(publication) => {
                self.status_message = format!(
                    "Published: {} — +{} credits; costs 1 action",
                    publication.evidence_strength.label(),
                    publication.credits_awarded
                );
            }
            Err(error) => self.status_message = publication_error_text(error),
        }
    }

    fn confirm_notebook_delete(&mut self) {
        let Some(id) = self.notebook_delete_confirmation.take() else {
            return;
        };
        match self.simulator.remove_hypothesis(id) {
            Ok(()) => {
                let len = self.simulator.notebook().hypotheses().len();
                if len == 0 {
                    self.notebook_selected = 0;
                } else {
                    self.notebook_selected = self.notebook_selected.min(len - 1);
                }
                self.status_message = "Hypothesis removed".to_string();
            }
            Err(error) => self.status_message = error.to_string(),
        }
    }

    fn handle_notebook_editor_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.notebook_editor = None;
            self.status_message = "Notebook edit cancelled".to_string();
            return;
        }
        let Some(editor) = self.notebook_editor.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                let delta = if key.code == KeyCode::Down { 1 } else { -1 };
                match editor.field {
                    NotebookField::Cause => {
                        editor.cause =
                            cycle_index(editor.cause, delta, ObservableVariable::ALL.len())
                    }
                    NotebookField::Direction => {
                        editor.direction = match (editor.direction, delta) {
                            (HypothesisDirection::Increases, 1) => HypothesisDirection::Decreases,
                            (HypothesisDirection::Decreases, -1) => HypothesisDirection::Increases,
                            (HypothesisDirection::Increases, -1) => HypothesisDirection::Decreases,
                            (HypothesisDirection::Decreases, 1) => HypothesisDirection::Increases,
                            _ => editor.direction,
                        }
                    }
                    NotebookField::Effect => {
                        editor.effect =
                            cycle_index(editor.effect, delta, ObservableVariable::ALL.len())
                    }
                }
            }
            KeyCode::Tab | KeyCode::Right => editor.field = next_notebook_field(editor.field),
            KeyCode::Left => editor.field = previous_notebook_field(editor.field),
            KeyCode::Enter => {
                if editor.field != NotebookField::Effect {
                    editor.field = next_notebook_field(editor.field);
                } else {
                    self.submit_notebook_editor();
                }
            }
            _ => {}
        }
    }

    fn submit_notebook_editor(&mut self) {
        let Some(editor) = self.notebook_editor.take() else {
            return;
        };
        let cause = ObservableVariable::ALL[editor.cause];
        let effect = ObservableVariable::ALL[editor.effect];
        let result = match editor.id {
            Some(id) => self
                .simulator
                .edit_hypothesis(id, cause, editor.direction, effect)
                .map(|()| id),
            None => self
                .simulator
                .add_hypothesis(cause, editor.direction, effect),
        };
        match result {
            Ok(id) => {
                if editor.id.is_none() {
                    self.notebook_selected = self.simulator.notebook().hypotheses().len() - 1;
                }
                self.status_message = format!("Hypothesis {} recorded", id.0);
            }
            Err(error) => {
                self.status_message = error.to_string();
            }
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
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch.is_ascii_whitespace() => {
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
                match parse_seed(input) {
                    Ok(seed) => *self = Self::new(seed),
                    Err(error) => self.status_message = error.to_string(),
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

fn cycle_index(index: usize, delta: i32, length: usize) -> usize {
    if length == 0 {
        return 0;
    }
    if delta < 0 {
        if index == 0 {
            length - 1
        } else {
            index - 1
        }
    } else {
        (index + 1) % length
    }
}

fn next_notebook_field(field: NotebookField) -> NotebookField {
    match field {
        NotebookField::Cause => NotebookField::Direction,
        NotebookField::Direction => NotebookField::Effect,
        NotebookField::Effect => NotebookField::Cause,
    }
}

fn previous_notebook_field(field: NotebookField) -> NotebookField {
    match field {
        NotebookField::Cause => NotebookField::Effect,
        NotebookField::Direction => NotebookField::Cause,
        NotebookField::Effect => NotebookField::Direction,
    }
}

fn publication_error_text(error: PublicationError) -> String {
    error.to_string()
}

fn repair_error_text(error: RepairError) -> String {
    error.to_string()
}
