use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::{Alignment, Frame};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::engine::ids::ObjectiveId;
use crate::engine::interventions::Intervention;
use crate::engine::measurement::MeasurementRecord;
use crate::engine::sim::{SimError, Simulator};
use crate::ui::{view_console, view_log, view_status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Status,
    Console,
    Log,
}

impl ActiveView {
    pub fn as_index(self) -> usize {
        match self {
            Self::Status => 0,
            Self::Console => 1,
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
    pub objective: ObjectiveId,
    pub active_view: ActiveView,
    pub menu_index: usize,
    pub log_scroll: usize,
    pub should_quit: bool,
    pub status_message: String,
    pub last_measurements: Vec<MeasurementRecord>,
}

impl App {
    pub fn new(seed: u64) -> Self {
        let recipe = crate::worldgen::generate_playable(seed);
        let objective = recipe.objective;
        Self {
            simulator: Simulator::new(recipe),
            objective,
            active_view: ActiveView::Status,
            menu_index: 0,
            log_scroll: 0,
            should_quit: false,
            status_message: format!("Seed {seed}"),
            last_measurements: Vec::new(),
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(frame.size());

        let titles = ["1 Status", "2 Console", "3 Log"];
        let tabs = Tabs::new(titles)
            .block(Block::default().title("xenolab v0.1").borders(Borders::ALL))
            .select(self.active_view.as_index())
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        frame.render_widget(tabs, chunks[0]);

        match self.active_view {
            ActiveView::Status => view_status::render(frame, chunks[1], self),
            ActiveView::Console => view_console::render(frame, chunks[1], self),
            ActiveView::Log => view_log::render(frame, chunks[1], self),
        }

        let footer = Paragraph::new("q quit | arrows navigate | enter apply")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right);
        let footer_area = ratatui::layout::Rect {
            x: chunks[1].x,
            y: chunks[1].y.saturating_add(chunks[1].height.saturating_sub(1)),
            width: chunks[1].width.saturating_sub(1),
            height: 1,
        };
        frame.render_widget(footer, footer_area);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<(), SimError> {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('1') => self.active_view = ActiveView::Status,
            KeyCode::Char('2') => self.active_view = ActiveView::Console,
            KeyCode::Char('3') => self.active_view = ActiveView::Log,
            KeyCode::Up => self.handle_up(),
            KeyCode::Down => self.handle_down(),
            KeyCode::Enter => {
                if self.active_view == ActiveView::Console {
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

    fn handle_up(&mut self) {
        match self.active_view {
            ActiveView::Console => {
                if self.menu_index == 0 {
                    self.menu_index = self.actions().len().saturating_sub(1);
                } else {
                    self.menu_index = self.menu_index.saturating_sub(1);
                }
            }
            ActiveView::Log => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
            ActiveView::Status => {}
        }
    }

    fn handle_down(&mut self) {
        match self.active_view {
            ActiveView::Console => {
                let len = self.actions().len();
                self.menu_index = if len == 0 { 0 } else { (self.menu_index + 1) % len };
            }
            ActiveView::Log => {
                self.log_scroll = self.log_scroll.saturating_add(1);
            }
            ActiveView::Status => {}
        }
    }

    fn apply_selected_action(&mut self) -> Result<(), SimError> {
        let action = self.actions()[self.menu_index];
        let intervention = action.to_intervention();
        let event = self.simulator.apply(intervention.clone())?;

        self.status_message = format!("tick={} action={}", event.tick_index, intervention.label());
        if !event.measurements.is_empty() {
            self.last_measurements = event.measurements;
        }
        Ok(())
    }
}
