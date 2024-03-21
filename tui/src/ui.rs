use chrono::{DateTime, Utc};

use ratatui::{
    prelude::*,
    widgets::{Axis, Block, Borders, Chart, Clear, Dataset, GraphType, List, Paragraph},
};
use style::palette::tailwind;
use tui_big_text::{BigText, PixelSize};

use crate::{
    app::{Focus, InputMode},
    App,
};

pub const PALETTES: [tailwind::Palette; 9] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
    tailwind::AMBER,
    tailwind::ROSE,
    tailwind::LIME,
    tailwind::FUCHSIA,
    tailwind::SKY,
];

pub fn render_rename_dialog(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default().title("Rename").borders(Borders::ALL);
    let input = Paragraph::new(app.inputs[Focus::Rename as usize].buffer.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Input => Style::default().fg(Color::LightGreen),
        })
        .block(block);
    let area = centered_rect(60, 20, area);
    frame.render_widget(Clear, area);
    frame.render_widget(input, area);
}

pub fn render_query_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let items = app
        .datasets
        .iter()
        .map(|(query, data)| match &data.query_alias {
            Some(alias) => alias.to_owned(),
            None => query.to_owned(),
        })
        .collect::<Vec<_>>();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Active Queries"),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true);

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

pub fn render_query_box(app: &mut App, frame: &mut Frame, area: Rect) {
    let input = Paragraph::new(app.inputs[Focus::QueryInput as usize].buffer.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Input => Style::default().fg(Color::LightGreen),
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Enter query: "),
        );
    frame.render_widget(input, area);
}

pub fn render_graph(app: &mut App, frame: &mut Frame, area: Rect) {
    let datasets = app.datasets.get(&app.selected_query).map(|data| {
        data.facets
            .iter()
            .map(|(facet, points)| {
                Dataset::default()
                    .name(facet.to_owned())
                    .data(&points[..])
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(match facet.as_str() {
                        ".NET" => Style::default().cyan(),
                        "Elasticsearch" => Style::default().yellow(),
                        "Web external" => Style::default().light_red(),
                        _ => Style::default(),
                    })
            })
            .collect::<Vec<_>>()
    });

    match datasets {
        Some(datasets) => {
            let bounds = app
                .datasets
                .get(&app.selected_query)
                .expect("ERROR: No bounds found for selected query")
                .bounds;
            let (min_x, mut min_y) = bounds.mins;
            let (_, mut max_y) = bounds.maxes;
            let mut half_y = (max_y - min_y) / 2_f64;

            min_y = f64::round(min_y);
            max_y = f64::round(max_y);
            half_y = f64::round(half_y);

            // Create the X axis and define its properties
            let x_axis = Axis::default()
                .title("Time".red())
                .style(Style::default().white())
                .bounds([min_x, Utc::now().timestamp() as f64])
                .labels(vec![
                    DateTime::from_timestamp(min_x as i64, 0)
                        .unwrap()
                        .to_string()
                        .into(),
                    DateTime::from_timestamp(Utc::now().timestamp() as i64, 0)
                        .unwrap()
                        .to_string()
                        .into(),
                ]);

            // Create the Y axis and define its properties
            let y_axis = Axis::default()
                .title("Transaction Time (ms)".red())
                .style(Style::default().white())
                .bounds([min_y, max_y])
                .labels(vec![
                    min_y.to_string().into(),
                    half_y.to_string().into(),
                    max_y.to_string().into(),
                ]);

            // Create the chart and link all the parts together
            let chart = Chart::new(datasets)
                .block(Block::default().borders(Borders::ALL).title("Chart"))
                .x_axis(x_axis)
                .y_axis(y_axis);
            frame.render_widget(chart, area);
        }
        None => {
            let dummy = BigText::builder()
                .pixel_size(PixelSize::Full)
                .style(Style::new().blue())
                .lines(vec!["XRELIC".light_green().into()])
                .build()
                .unwrap();

            let center = centered_rect(30, 30, area);
            frame.render_widget(dummy, center);
        }
    }
    // frame.render_widget(chart, frame.size());
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
