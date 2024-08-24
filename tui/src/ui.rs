use chrono::{DateTime, Utc};

use ratatui::{
    prelude::*,
    symbols::Marker,
    widgets::{
        Axis, Block, BorderType, Borders, Chart, Clear, Dataset, GraphType, LegendPosition, List,
        Padding, Paragraph,
    },
};
use style::palette::tailwind;
use throbber_widgets_tui::WhichUse;
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

pub fn render_splash(_app: &mut App, frame: &mut Frame, area: Rect) {
    let dummy = BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::new().blue())
        .lines(vec!["XRELIC".light_green().into()])
        .build();

    let center = centered_rect(30, 30, area);
    frame.render_widget(dummy, center);
}

pub fn render_loading(_app: &mut App, frame: &mut Frame, area: Rect) {
    let center = centered_rect(5, 5, area);
    let throbber = throbber_widgets_tui::Throbber::default()
        .label("Loading data...")
        .style(Style::default().fg(Color::LightGreen))
        .throbber_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .use_type(WhichUse::Spin);
    frame.render_widget(throbber, center);
}

pub fn render_load_session(app: &mut App, frame: &mut Frame, area: Rect) {
    let area = centered_rect(60, 20, area);
    let vertical = Layout::vertical([Constraint::Length(3), Constraint::Length(3)]);
    let [prompt_area, input_area] = vertical.areas(area);

    let prompt =
        Text::from("A previous session was found. Would you like to reload its queries? y/n");
    let input = Paragraph::new(app.inputs.get(Focus::SessionLoad))
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Input => Style::default().fg(app.theme.focus_fg),
        })
        .block(
            Block::default()
                .padding(Padding::ZERO)
                .borders(Borders::BOTTOM)
                .border_type(BorderType::Rounded),
        );
    frame.render_widget(Clear, area);
    frame.render_widget(prompt, prompt_area);
    frame.render_widget(input, input_area);
}

pub fn render_save_session(app: &mut App, frame: &mut Frame, area: Rect) {
    let area = centered_rect(60, 20, area);
    let vertical = Layout::vertical([Constraint::Length(3), Constraint::Length(3)]);
    let [prompt_area, input_area] = vertical.areas(area);

    let prompt = Text::from("Save session? y/n");
    let input = Paragraph::new(app.inputs.get(Focus::SessionSave))
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Input => Style::default().fg(app.theme.focus_fg),
        })
        .block(
            Block::default()
                .padding(Padding::ZERO)
                .borders(Borders::BOTTOM)
                .border_type(BorderType::Rounded),
        );
    frame.render_widget(Clear, area);
    frame.render_widget(prompt, prompt_area);
    frame.render_widget(input, input_area);
}

pub fn render_dashboard(app: &mut App, frame: &mut Frame, area: Rect) {
    let n_graphs = &app.datasets.len();
    let areas = match n_graphs {
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
                    .style(Style::default().fg(app.facet_colours.get(facet).unwrap().to_owned()))
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
                .style(Style::default().fg(app.theme.chart_fg))
                .bounds([min_x, Utc::now().timestamp() as f64])
                .labels(vec![
                    DateTime::from_timestamp(min_x as i64, 0)
                        .unwrap()
                        .time()
                        .to_string()
                        .fg(app.theme.chart_fg)
                        .bold(),
                    DateTime::from_timestamp(Utc::now().timestamp(), 0)
                        .unwrap()
                        .to_string()
                        .fg(app.theme.chart_fg)
                        .bold(),
                ]);

            // Create the Y axis and define its properties
            let y_axis = Axis::default()
                .title(selection.clone().fg(app.theme.chart_fg))
                .style(Style::default().fg(app.theme.chart_fg))
                .bounds([min_y, max_y])
                .labels(vec![
                    min_y.to_string().fg(app.theme.chart_fg).bold(),
                    half_y.to_string().fg(app.theme.chart_fg).bold(),
                    max_y.to_string().fg(app.theme.chart_fg).bold(),
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
                .build();

            let center = centered_rect(30, 30, area);
            frame.render_widget(dummy, center);
        }
    }
}

pub fn render_rename_dialog(app: &mut App, frame: &mut Frame, area: Rect) {
    let area = centered_rect(60, 20, area);
    let vertical = Layout::vertical([Constraint::Length(3), Constraint::Length(3)]);
    let [prompt_area, input_area] = vertical.areas(area);

    let prompt = Text::from("Rename query");
    let input = Paragraph::new(app.inputs.get(Focus::Rename))
        .style(match app.focus {
            Focus::Rename => Style::default().fg(app.theme.focus_fg),
            _ => Style::default(),
        })
        .block(
            Block::default()
                .padding(Padding::ZERO)
                .borders(Borders::BOTTOM),
        );

    frame.render_widget(Clear, area);
    frame.render_widget(prompt, prompt_area);
    frame.render_widget(input, input_area);
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
                .border_type(BorderType::Rounded)
                .title("Active Queries"),
        )
        .highlight_style(
            Style::new()
                .add_modifier(Modifier::REVERSED)
                .fg(app.theme.chart_fg),
        )
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true);

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

pub fn render_query_box(app: &mut App, frame: &mut Frame, area: Rect) {
    let input = Paragraph::new(app.inputs.get(Focus::QueryInput))
        .style(match app.focus {
            Focus::QueryInput => Style::default().fg(app.theme.focus_fg),
            _ => Style::default(),
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Enter query: "),
        );
    frame.render_widget(input, area);
}

pub fn render_graph(app: &mut App, frame: &mut Frame, area: Rect) {
    let datasets = app.datasets.selected().map(|data| {
        data.facets
            .iter()
            .map(|(facet, points)| {
                Dataset::default()
                    .name(facet.to_owned())
                    .data(&points[..])
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(app.facet_colours.get(facet).unwrap().to_owned()))
            })
            .collect::<Vec<_>>()
    });

    if let Some(datasets) = datasets {
        let dataset = app
            .datasets
            .selected()
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
            .title("Time".fg(app.theme.chart_fg))
            .style(Style::default().fg(app.theme.chart_fg))
            .bounds([min_x, Utc::now().timestamp() as f64])
            .labels(vec![
                DateTime::from_timestamp(min_x as i64, 0)
                    .unwrap()
                    .time()
                    .to_string()
                    .fg(app.theme.chart_fg)
                    .bold(),
                DateTime::from_timestamp(Utc::now().timestamp(), 0)
                    .unwrap()
                    .to_string()
                    .fg(app.theme.chart_fg)
                    .bold(),
            ]);

        // Create the Y axis and define its properties
        let y_axis = Axis::default()
            .title(selection.clone().fg(app.theme.chart_fg))
            .style(Style::default().fg(app.theme.chart_fg))
            .bounds([min_y, max_y])
            .labels(vec![
                min_y.to_string().fg(app.theme.chart_fg).bold(),
                half_y.to_string().fg(app.theme.chart_fg).bold(),
                max_y.to_string().fg(app.theme.chart_fg).bold(),
            ]);

        let legend_position = match &datasets.len() {
            1 => None,
            _ => Some(LegendPosition::TopRight),
        };

        // Create the chart and link all the parts together
        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.chart_fg))
                    .border_type(BorderType::Thick)
                    .border_type(BorderType::Rounded),
            )
            .legend_position(legend_position)
            .x_axis(x_axis)
            .y_axis(y_axis);
        frame.render_widget(chart, area);
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
