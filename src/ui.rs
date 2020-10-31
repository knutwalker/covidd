// use crate::demo::{ui, App};
use crate::Result;
use chrono::{Date, Utc};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent,
        MouseEvent,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{stdout, Write};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    style::{Color, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame, Terminal,
};

pub fn draw(
    dates: &[Date<Utc>],
    cases: &[u32],
    recoveries: &[u32],
    deaths: &[u32],
    hospitalisations: &[u32],
) -> Result<()> {
    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let mut index = 0;

    loop {
        terminal.draw(|f| {
            draw_charts(
                f,
                dates.get(index..).unwrap_or_default(),
                cases.get(index..).unwrap_or_default(),
                recoveries.get(index..).unwrap_or_default(),
                deaths.get(index..).unwrap_or_default(),
                hospitalisations.get(index..).unwrap_or_default(),
            )
        })?;

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
                    _ => {}
                },
                CEvent::Mouse(MouseEvent::ScrollUp(..)) => break Event::ZoomIn(1),
                CEvent::Mouse(MouseEvent::ScrollDown(..)) => break Event::ZoomOut(1),
                _ => {}
            }
        };
        match event {
            Event::Quit => break,
            Event::ZoomIn(diff) => {
                index = index
                    .saturating_add(diff)
                    .min(dates.len().saturating_sub(1))
            }
            Event::ZoomOut(diff) => index = index.saturating_sub(diff),
        }
    }

    disable_raw_mode()?;

    let c = terminal.backend_mut();

    // Queue each command, then flush
    Ok(())
        .and_then(|()| crossterm::handle_command!(c, LeaveAlternateScreen))
        .and_then(|()| crossterm::handle_command!(c, DisableMouseCapture))
        .and_then(|()| Backend::flush(c).map_err(crossterm::ErrorKind::IoError))?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw_charts<B>(
    f: &mut Frame<B>,
    dates: &[Date<Utc>],
    cases: &[u32],
    recoveries: &[u32],
    deaths: &[u32],
    hospitalisations: &[u32],
) where
    B: Backend,
{
    let data = chart_data(f.size(), dates, cases, recoveries, deaths, hospitalisations);
    draw_chart_data(f, data);
}

#[instrument(level = "debug")]
fn chart_data(
    area: Rect,
    dates: &[Date<Utc>],
    cases: &[u32],
    recoveries: &[u32],
    deaths: &[u32],
    hospitalisations: &[u32],
) -> ChartData {
    let x_label_counts = usize::from(area.width.saturating_sub(6) / 7);
    let x_label_steps = (dates.len() as f64 / x_label_counts.max(1) as f64)
        .ceil()
        .max(1.0) as usize;
    let x_labels = dates
        .iter()
        .step_by(x_label_steps)
        .map(|d| Span::raw(d.format("%d.%m").to_string()))
        .collect::<Vec<_>>();

    let x_axis = Axis::default()
        .style(Style::default().fg(Color::Gray))
        .bounds([0.0, dates.len() as f64])
        .labels(x_labels);

    let min_bound = cases
        .iter()
        .min()
        .min(deaths.iter().min())
        .min(recoveries.iter().min())
        .min(hospitalisations.iter().min())
        .copied()
        .unwrap_or_default() as f64;

    let max_bound = cases
        .iter()
        .max()
        .max(deaths.iter().max())
        .max(recoveries.iter().max())
        .max(hospitalisations.iter().max())
        .copied()
        .unwrap_or_default() as f64;

    let bound_step = (max_bound - min_bound) / cases.len() as f64;
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

    let cases = cases
        .iter()
        .enumerate()
        .map(|(x, &y)| (x as f64, y as f64))
        .collect::<Vec<_>>();

    let deaths = deaths
        .iter()
        .enumerate()
        .map(|(x, &y)| (x as f64, y as f64))
        .collect::<Vec<_>>();

    let recoveries = recoveries
        .iter()
        .enumerate()
        .map(|(x, &y)| (x as f64, y as f64))
        .collect::<Vec<_>>();

    let hospitalisations = hospitalisations
        .iter()
        .enumerate()
        .map(|(x, &y)| (x as f64, y as f64))
        .collect::<Vec<_>>();

    ChartData {
        cases,
        deaths,
        recoveries,
        hospitalisations,
        x_axis,
        y_axis,
    }
}

#[instrument(level = "debug", skip(f))]
fn draw_chart_data<B: Backend>(f: &mut Frame<B>, data: ChartData) {
    let latest_recoveries = data.recoveries.last().copied().unwrap_or_default().1 as u32;
    let latest_hospitalisations =
        data.hospitalisations.last().copied().unwrap_or_default().1 as u32;
    let latest_cases = data.cases.last().copied().unwrap_or_default().1 as u32;
    let latest_deaths = data.deaths.last().copied().unwrap_or_default().1 as u32;

    let datasets = vec![
        Dataset::default()
            .name(format!("{:>6} recovered   ", latest_recoveries))
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Green))
            .graph_type(GraphType::Line)
            .data(&data.recoveries),
        Dataset::default()
            .name(format!("{:>6} hospitalised", latest_hospitalisations))
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Cyan))
            .graph_type(GraphType::Line)
            .data(&data.hospitalisations),
        Dataset::default()
            .name(format!("{:>6} deaths      ", latest_deaths))
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::LightRed))
            .graph_type(GraphType::Line)
            .data(&data.deaths),
        Dataset::default()
            .name(format!("{:>6} total cases ", latest_cases))
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .graph_type(GraphType::Line)
            .data(&data.cases),
    ];

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL))
        .x_axis(data.x_axis)
        .y_axis(data.y_axis);

    f.render_widget(chart, f.size());
}

#[derive(Debug)]
struct ChartData {
    cases: Vec<(f64, f64)>,
    deaths: Vec<(f64, f64)>,
    recoveries: Vec<(f64, f64)>,
    hospitalisations: Vec<(f64, f64)>,
    x_axis: Axis<'static>,
    y_axis: Axis<'static>,
}

#[derive(Debug, Copy, Clone)]
enum Event {
    Quit,
    ZoomIn(usize),
    ZoomOut(usize),
}
