use crate::{
    data::DataPoint,
    messages::{Messages, MsgId},
    Result,
};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
use tracing::{debug, instrument, trace};
use tui::{
    backend::CrosstermBackend,
    layout::Constraint,
    layout::Rect,
    style::Modifier,
    style::{Color, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame, Terminal,
};

#[instrument(err, skip(data_points, msg))]
pub fn draw(data_points: &[DataPoint], msg: Messages) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut index = 0;
    loop {
        debug!("Drawing charts with range {:?}", index..);
        terminal.draw(|f| draw_charts(f, data_points.get(index..).unwrap_or_default(), &msg))?;

        let event = loop {
            match event::read()? {
                CEvent::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Char('q') => break Event::Quit,
                    KeyCode::Up | KeyCode::Right | KeyCode::Char('k') | KeyCode::Char('l') => {
                        break Event::ZoomIn(1)
                    }
                    KeyCode::Down | KeyCode::Left | KeyCode::Char('j') | KeyCode::Char('h') => {
                        break Event::ZoomOut(1)
                    }
                    KeyCode::PageUp => break Event::ZoomIn(10),
                    KeyCode::PageDown => break Event::ZoomOut(10),
                    KeyCode::Char('1') | KeyCode::End => break Event::LastWeek(1),
                    KeyCode::Char('2') => break Event::LastWeek(2),
                    KeyCode::Char('3') => break Event::LastWeek(3),
                    KeyCode::Char('4') => break Event::LastWeek(4),
                    KeyCode::Char('5') => break Event::LastWeek(5),
                    KeyCode::Char('6') => break Event::LastWeek(6),
                    KeyCode::Char('7') => break Event::LastWeek(7),
                    KeyCode::Char('8') => break Event::LastWeek(8),
                    KeyCode::Char('9') => break Event::LastWeek(9),
                    KeyCode::Esc | KeyCode::Home | KeyCode::Char('0') => break Event::AllData,
                    _ => {}
                },
                CEvent::Mouse(me) if matches!(me.kind, MouseEventKind::ScrollUp) => {
                    break Event::ZoomIn(1)
                }
                CEvent::Mouse(me) if matches!(me.kind, MouseEventKind::ScrollDown) => {
                    break Event::ZoomOut(1)
                }
                CEvent::Resize(_, _) => break Event::ZoomOut(0), // re-render
                _ => {}
            }
        };
        trace!("Input event: {:?}", event);
        match event {
            Event::Quit => break,
            Event::ZoomIn(diff) => {
                index = index
                    .saturating_add(diff)
                    .min(data_points.len().saturating_sub(1))
            }
            Event::ZoomOut(diff) => index = index.saturating_sub(diff),
            Event::LastWeek(week) => index = data_points.len().saturating_sub(7 * week),
            Event::AllData => index = 0,
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw_charts<B>(f: &mut Frame<'_, B>, data_points: &[DataPoint], msg: &Messages)
where
    B: tui::backend::Backend,
{
    let data = chart_data(f.size(), data_points);
    draw_chart_data(f, data, msg);
}

fn chart_data(area: Rect, data_points: &[DataPoint]) -> ChartData {
    let x_label_counts = usize::from(area.width.saturating_sub(6) / 7);
    let x_label_steps = (data_points.len() as f64 / x_label_counts.max(1) as f64)
        .ceil()
        .max(1.0) as usize;
    let x_labels = data_points
        .iter()
        .step_by(x_label_steps)
        .map(|d| Span::raw(d.dates.date.format("%d.%m").to_string()))
        .collect::<Vec<_>>();

    let x_axis = Axis::default()
        .style(Style::default().fg(Color::Gray))
        .bounds([0.0, data_points.len() as f64])
        .labels(x_labels);

    let min_bound = data_points
        .iter()
        .map(|d| {
            d.cases
                .total
                .min(d.deaths.total)
                .min(d.recoveries.total)
                .min(d.hospitalisations.total)
        })
        .min()
        .unwrap_or_default() as f64;

    let max_bound = data_points
        .iter()
        .map(|d| {
            d.cases
                .total
                .max(d.deaths.total)
                .max(d.recoveries.total)
                .max(d.hospitalisations.total)
        })
        .max()
        .unwrap_or_default() as f64;

    let max_incidence = data_points
        .iter()
        .map(|d| d.incidence_calculated)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();

    let incidence_scale = max_bound / max_incidence;

    let bound_step = (max_bound - min_bound) / data_points.len() as f64;
    let y_bounds = std::iter::successors(Some(min_bound), |y| {
        Some(*y + bound_step).filter(|y| *y <= max_bound)
    })
    .map(|y| y.round() as u32)
    .scan(None::<u32>, |seen, y| {
        Some(match seen {
            None => {
                *seen = Some(y);
                (y, true)
            }
            Some(prev) => {
                let prev = std::mem::replace(prev, y);
                (y, y > prev)
            }
        })
    })
    .filter_map(|(y, distinct)| if distinct { Some(y) } else { None })
    .map(|y| Span::raw(format!("{:.1}", y)))
    .collect::<Vec<_>>();

    let y_axis = Axis::default()
        .style(Style::default().fg(Color::Gray))
        .bounds([min_bound, max_bound])
        .labels(y_bounds);

    let recoveries = data_points
        .iter()
        .enumerate()
        .map(|(x, y)| (x as f64, y.recoveries.total as f64))
        .collect::<Vec<_>>();

    let hospitalisations = data_points
        .iter()
        .enumerate()
        .map(|(x, y)| (x as f64, y.hospitalisations.total as f64))
        .collect::<Vec<_>>();

    let deaths = data_points
        .iter()
        .enumerate()
        .map(|(x, y)| (x as f64, y.deaths.total as f64))
        .collect::<Vec<_>>();

    let actives = data_points
        .iter()
        .enumerate()
        .map(|(x, y)| (x as f64, y.active_cases() as f64))
        .collect::<Vec<_>>();

    let cases = data_points
        .iter()
        .enumerate()
        .map(|(x, y)| (x as f64, y.cases.total as f64))
        .collect::<Vec<_>>();

    let incidences = data_points
        .iter()
        .enumerate()
        .filter(|(_, y)| y.incidence_calculated > 0.0)
        .map(|(x, y)| (x as f64, y.incidence_calculated * incidence_scale))
        .collect::<Vec<_>>();

    let (
        cases_increase,
        deaths_increase,
        hospitalizations_increase,
        recoveries_increase,
        current_incidence,
    ) = data_points
        .last()
        .map(|d| {
            (
                d.cases.increase,
                d.deaths.increase,
                d.hospitalisations.increase,
                d.recoveries.increase,
                d.incidence_calculated,
            )
        })
        .unwrap_or_default();

    // let incidence_increase = data_points
    //     .iter()
    //     .rev()
    //     .take(2)
    //     .map(|d| d.incidence_calculated)
    //     .fold(0.0, |inc, d| if inc == 0.0 { d } else { inc - d });

    let (incidence_increase, actives_increase) = data_points
        .windows(2)
        .last()
        .map(|win| {
            let incidence = win[1].incidence_calculated - win[0].incidence_calculated;
            let active = win[1].active_cases() as i32 - win[0].active_cases() as i32;
            (incidence, active)
        })
        .unwrap_or_default();

    ChartData {
        recoveries,
        hospitalisations,
        deaths,
        actives,
        cases,
        incidences,
        current_incidence,
        cases_increase,
        actives_increase,
        deaths_increase,
        hospitalizations_increase,
        recoveries_increase,
        incidence_increase,
        x_axis,
        y_axis,
    }
}

fn draw_chart_data<B: tui::backend::Backend>(
    f: &mut Frame<'_, B>,
    data: ChartData,
    msg: &Messages,
) {
    let recovered = msg.get(
        MsgId::Recovered,
        data.recoveries.last().copied().unwrap_or_default().1 as u32,
        Some(data.recoveries_increase),
    );
    let hospitalised = msg.get(
        MsgId::Hospitalised,
        data.hospitalisations.last().copied().unwrap_or_default().1 as u32,
        Some(data.hospitalizations_increase),
    );
    let deaths = msg.get(
        MsgId::Deaths,
        data.deaths.last().copied().unwrap_or_default().1 as u32,
        Some(data.deaths_increase),
    );
    let active = msg.get(
        MsgId::Active,
        data.actives.last().copied().unwrap_or_default().1 as u32,
        Some(data.actives_increase),
    );
    let cases = msg.get(
        MsgId::Cases,
        data.cases.last().copied().unwrap_or_default().1 as u32,
        Some(data.cases_increase),
    );
    let incidence = msg.get(
        MsgId::Incidence,
        data.current_incidence,
        Some(data.incidence_increase),
    );

    let datasets = vec![
        Dataset::default()
            .name(recovered)
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Green))
            .graph_type(GraphType::Line)
            .data(&data.recoveries),
        Dataset::default()
            .name(hospitalised)
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Cyan))
            .graph_type(GraphType::Line)
            .data(&data.hospitalisations),
        Dataset::default()
            .name(deaths)
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Magenta))
            .graph_type(GraphType::Line)
            .data(&data.deaths),
        Dataset::default()
            .name(active)
            .marker(symbols::Marker::Braille)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::DIM),
            )
            .graph_type(GraphType::Line)
            .data(&data.actives),
        Dataset::default()
            .name(incidence)
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Red))
            .graph_type(GraphType::Line)
            .data(&data.incidences),
        Dataset::default()
            .name(cases)
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .graph_type(GraphType::Line)
            .data(&data.cases),
    ];

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL))
        .hidden_legend_constraints((Constraint::Percentage(100), Constraint::Percentage(100)))
        .x_axis(data.x_axis)
        .y_axis(data.y_axis);

    f.render_widget(chart, f.size());
}

#[derive(Debug)]
struct ChartData {
    recoveries: Vec<(f64, f64)>,
    hospitalisations: Vec<(f64, f64)>,
    deaths: Vec<(f64, f64)>,
    actives: Vec<(f64, f64)>,
    cases: Vec<(f64, f64)>,
    incidences: Vec<(f64, f64)>,
    current_incidence: f64,
    cases_increase: i32,
    deaths_increase: i32,
    actives_increase: i32,
    hospitalizations_increase: i32,
    recoveries_increase: i32,
    incidence_increase: f64,
    x_axis: Axis<'static>,
    y_axis: Axis<'static>,
}

#[derive(Debug, Copy, Clone)]
enum Event {
    Quit,
    ZoomIn(usize),
    ZoomOut(usize),
    LastWeek(usize),
    AllData,
}
