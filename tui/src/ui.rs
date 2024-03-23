use chrono::{DateTime, Utc};

use ratatui::{
    prelude::*,
    widgets::{
        Axis, Block, Borders, Chart, Clear, Dataset, GraphType, LegendPosition, List, Paragraph,
    },
};
// use style::palette::tailwind;
use tui_big_text::{BigText, PixelSize};

use crate::{
    app::{InputMode, CACHE_LOAD, QUERY, RENAME},
    App,
};

// pub const PALETTES: [tailwind::Palette; 9] = [
//     tailwind::BLUE,
//     tailwind::EMERALD,
//     tailwind::INDIGO,
//     tailwind::RED,
//     tailwind::AMBER,
//     tailwind::ROSE,
//     tailwind::LIME,
//     tailwind::FUCHSIA,
//     tailwind::SKY,
// ];

pub fn render_load_cache(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default().title("Load cache?").borders(Borders::ALL);
    let prompt = Text::from("A cache was located. Would you like to reload the queries? y/n");
    let input = Paragraph::new(app.inputs[CACHE_LOAD as usize].buffer.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Input => Style::default().fg(Color::LightGreen),
        })
        .block(block);
    let area = centered_rect(60, 20, area);
    frame.render_widget(prompt, area);
    frame.render_widget(Clear, area);
    frame.render_widget(input, area);
}

pub fn render_dashboard(app: &mut App, frame: &mut Frame, area: Rect) {
    let n_graphs = &app.datasets.len();
    let areas = match *n_graphs {
        0 => vec![],
        1 => vec![area],
        2 => {
            let layout = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]);
            let [first, second] = layout.areas(area);
            vec![first, second]
        }
        3 => {
            let vertical =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]);
            let [top, bottom] = vertical.areas(area);
            let horizontal =
                Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]);
            let [first, second] = horizontal.areas(bottom);
            vec![top, first, second]
        }
        _ => panic!(),
    };

    (0..areas.len()).for_each(|i| {
        render_ith_graph(app, frame, areas[i], i);
    });
}

pub fn render_ith_graph(app: &mut App, frame: &mut Frame, area: Rect, i: usize) {
    let datasets = app.datasets.iter().nth(i).map(|(_, data)| {
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
                        "value" => Style::default().light_magenta(),
                        _ => Style::default(),
                    })
            })
            .collect::<Vec<_>>()
    });

    match datasets {
        Some(datasets) => {
            let (_, dataset) = app
                .datasets
                .iter()
                .nth(i)
                .expect("ERROR: Could not index bounds!");

            let bounds = dataset.bounds;
            let selection = &dataset.selection;

            let (min_x, mut min_y) = bounds.mins;
            let (_, mut max_y) = bounds.maxes;
            let mut half_y = (max_y - min_y) / 2_f64;

            min_y = f64::round(min_y);
            max_y = f64::round(max_y);
            half_y = f64::round(half_y);

            // Create the X axis and define its properties
            let x_axis = Axis::default()
                .title("Time".red())
                .style(Style::default().red())
                .bounds([min_x, Utc::now().timestamp() as f64])
                .labels(vec![
                    DateTime::from_timestamp(min_x as i64, 0)
                        .unwrap()
                        .time()
                        .to_string()
                        .red()
                        .bold(),
                    DateTime::from_timestamp(Utc::now().timestamp(), 0)
                        .unwrap()
                        .to_string()
                        .red()
                        .bold(),
                ]);

            // Create the Y axis and define its properties
            let y_axis = Axis::default()
                .title(selection.clone().red())
                .style(Style::default().white())
                .bounds([min_y, max_y])
                .labels(vec![
                    min_y.to_string().red().bold(),
                    half_y.to_string().red().bold(),
                    max_y.to_string().red().bold(),
                ]);

            let legend_position = match &datasets.len() {
                1 => None,
                _ => Some(LegendPosition::TopRight),
            };

            // Create the chart and link all the parts together
            let chart = Chart::new(datasets)
                .block(
                    Block::default(), /*.borders(Borders::ALL).title("Chart")*/
                )
                .legend_position(legend_position)
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
}

pub fn render_rename_dialog(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default().title("Rename").borders(Borders::ALL);
    let input = Paragraph::new(app.inputs[RENAME as usize].buffer.as_str())
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
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED).light_green())
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true);

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

pub fn render_query_box(app: &mut App, frame: &mut Frame, area: Rect) {
    let input = Paragraph::new(app.inputs[QUERY as usize].buffer.as_str())
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
                        "value" => Style::default().light_magenta(),
                        _ => Style::default(),
                    })
            })
            .collect::<Vec<_>>()
    });

    match datasets {
        Some(datasets) => {
            let dataset = app
                .datasets
                .get(&app.selected_query)
                .expect("ERROR: No bounds found for selected query");
            let bounds = dataset.bounds;
            let selection = &dataset.selection;

            let (min_x, mut min_y) = bounds.mins;
            let (_, mut max_y) = bounds.maxes;
            let mut half_y = (max_y - min_y) / 2_f64;

            min_y = f64::round(min_y);
            max_y = f64::round(max_y);
            half_y = f64::round(half_y);

            // Create the X axis and define its properties
            let x_axis = Axis::default()
                .title("Time".red())
                .style(Style::default().red())
                .bounds([min_x, Utc::now().timestamp() as f64])
                .labels(vec![
                    DateTime::from_timestamp(min_x as i64, 0)
                        .unwrap()
                        .time()
                        .to_string()
                        .red()
                        .bold(),
                    DateTime::from_timestamp(Utc::now().timestamp(), 0)
                        .unwrap()
                        .to_string()
                        .red()
                        .bold(),
                ]);

            // Create the Y axis and define its properties
            let y_axis = Axis::default()
                .title(selection.clone().red())
                .style(Style::default().red())
                .bounds([min_y, max_y])
                .labels(vec![
                    min_y.to_string().red().bold(),
                    half_y.to_string().red().bold(),
                    max_y.to_string().red().bold(),
                ]);

            let legend_position = match &datasets.len() {
                1 => None,
                _ => Some(LegendPosition::TopRight),
            };

            // Create the chart and link all the parts together
            let chart = Chart::new(datasets)
                .block(Block::default().borders(Borders::ALL).title("Chart"))
                .legend_position(legend_position)
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
