use chrono::{DateTime, NaiveDateTime, Utc};

use ratatui::{
    prelude::*,
    symbols::Marker,
    widgets::{
        Axis, Bar, BarChart, BarGroup, Block, BorderType, Borders, Chart, Clear, Dataset,
        GraphType, LegendPosition, List, Padding, Paragraph, RenderDirection, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Sparkline, Tabs, Wrap,
    },
};
use style::palette::tailwind;
use throbber_widgets_tui::WhichUse;
use tui_big_text::{BigText, PixelSize};

use crate::{
    app::{Focus, InputMode, Tab},
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

pub fn ui(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]);

    // let [header_area, area] = vertical.areas(area);

    // render_tabs(app, frame, header_area);

    match app.focus.tab {
        Tab::Graph => {
            let horizontal = Layout::horizontal([Constraint::Percentage(15), Constraint::Min(20)]);
            let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(20)]);
            let [input_area, rest] = vertical.areas(area);
            let [list_area, graph_area] = horizontal.areas(rest);

            match app.focus.panel {
                Focus::SessionSave => render_save_session(app, frame, area),
                Focus::SessionLoad => render_load_session(app, frame, area),
                Focus::Dashboard => render_dashboard(app, frame, area),
                Focus::Rename => {
                    render_query_box(app, frame, input_area);
                    render_query_list(app, frame, list_area);
                    render_rename_dialog(app, frame, graph_area);
                }
                Focus::Default | Focus::QueryInput | Focus::Log | Focus::LogDetail => {
                    render_query_box(app, frame, input_area);
                    render_query_list(app, frame, list_area);
                    if let Some(dataset) = app.datasets.selected() {
                        if dataset.has_data {
                            render_graph(app, frame, graph_area);
                        } else {
                            render_loading(app, frame, graph_area);
                        }
                    } else {
                        render_splash(app, frame, graph_area);
                    }
                }
                Focus::Search => {}
            }
        }
        Tab::Logs => {
            let horizontal = Layout::horizontal([Constraint::Percentage(10), Constraint::Min(20)]);
            let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(20)]);
            let [input_area, rest] = vertical.areas(area);
            let [list_area, rest] = horizontal.areas(rest);

            let [barchart_area, seek_area, log_area] =
                Layout::vertical([Constraint::Max(10), Constraint::Max(1), Constraint::Min(30)])
                    .areas(rest);

            match app.focus.panel {
                Focus::SessionSave => render_save_session(app, frame, area),
                Focus::SessionLoad => render_load_session(app, frame, area),
                Focus::Default | Focus::QueryInput | Focus::Log | Focus::LogDetail => {
                    render_query_box(app, frame, input_area);
                    if !app.logs.is_empty() {
                        render_log_list(app, frame, list_area);
                        render_barchart(app, frame, barchart_area);
                        render_seek(app, frame, seek_area);
                        render_log(app, frame, log_area);
                        if app.focus.panel == Focus::LogDetail {
                            render_log_detail(app, frame, log_area);
                        }
                    } else if app.focus.loading {
                        render_loading(app, frame, area)
                    } else {
                        render_splash(app, frame, log_area);
                    }
                }
                Focus::Search => render_search(app, frame, area),
                _ => render_splash(app, frame, area),
            }
        }
    }
}

pub fn render_search(app: &mut App, frame: &mut Frame, area: Rect) {
    let area = centered_rect(60, 20, area);
    let vertical = Layout::vertical([Constraint::Length(3), Constraint::Length(3)]);
    let [prompt_area, input_area] = vertical.areas(area);

    let prompt = Text::from("Search term");
    let input = Paragraph::new(app.inputs.get(Focus::Search))
        .style(match app.focus.panel {
            Focus::Search => Style::default().fg(app.config.theme.focus_fg),
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

// Creates a widget::Line for a log detail with styling based on content
pub fn style_detail_line<'a>(app: &App, value: String) -> Line<'a> {
    if value.contains("CorrelationId") || value.contains("requestId") {
        Line::from(value).style(Style::default().bold().fg(app.config.theme.focus_fg))
    } else if (value.contains("level") && value.contains("Error"))
        || (value.contains("severity.text") && value.contains("Error"))
    {
        Line::from(value).style(Style::default().bold().fg(Color::LightRed))
    } else if !app.logs.filters.is_empty()
        && value.contains(app.logs.filters.iter().next().unwrap())
    {
        Line::from(value).style(Style::default().bg(Color::LightRed))
    } else {
        Line::from(value)
    }
}

pub fn render_seek(app: &mut App, frame: &mut Frame, area: Rect) {
    let curr = app.logs.selected.clone().parse::<f64>().unwrap();
    let vec = vec![(curr, 0.5)];
    let dataset = Dataset::default()
        .data(&vec)
        .marker(Marker::HalfBlock)
        .style(Style::default().fg(app.config.theme.chart_fg))
        .graph_type(GraphType::Bar);

    let bounds = app.logs.bounds;
    let (min_x, _) = bounds.mins;
    let (max_x, _) = bounds.maxes;

    let min_x_date_time = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(min_x as i64 / 1000, 0).unwrap(),
        Utc,
    );
    let max_x_date_time = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(max_x as i64 / 1000, 0).unwrap(),
        Utc,
    );

    // Create the X axis and define its properties
    let x_axis = Axis::default()
        // .style(Style::default().white())
        .bounds([min_x, max_x]);

    // Create the Y axis and define its properties
    let y_axis = Axis::default()
        // .style(Style::default().white())
        .bounds([0.0, 1.0]);

    // Create the chart and link all the parts together
    let chart = Chart::new(vec![dataset])
        .block(
            Block::new().padding(Padding::horizontal(3)), // .borders(Borders::ALL)
                                                          // .border_type(BorderType::Rounded),
        )
        .x_axis(x_axis)
        .y_axis(y_axis);

    frame.render_widget(chart, area);
}

pub fn render_barchart(app: &mut App, frame: &mut Frame, area: Rect) {
    let error_dataset = Dataset::default()
        .data(&app.logs.chart_data.error)
        .marker(Marker::Block)
        .style(Style::default().red())
        .graph_type(GraphType::Bar);
    let debug_dataset = Dataset::default()
        .data(&app.logs.chart_data.debug)
        .marker(Marker::Block)
        .style(Style::default().magenta())
        .graph_type(GraphType::Bar);
    let info_dataset = Dataset::default()
        .data(&app.logs.chart_data.info)
        .marker(Marker::Block)
        .style(Style::default().blue())
        .graph_type(GraphType::Bar);

    let bounds = app.logs.bounds;
    let (min_x, _) = bounds.mins;
    let (max_x, _) = bounds.maxes;

    let selected_date_time = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(
            app.logs.selected.clone().parse::<i64>().unwrap() / 1000,
            0,
        )
        .unwrap(),
        Utc,
    );

    let min_x_date_time = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(min_x as i64 / 1000, 0).unwrap(),
        Utc,
    );
    let max_x_date_time = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(max_x as i64 / 1000, 0).unwrap(),
        Utc,
    );

    // Create the X axis and define its properties
    let x_axis = Axis::default()
        .style(Style::default().white())
        .labels([
            format!(
                "{} {}",
                min_x_date_time.date_naive(),
                min_x_date_time.time()
            ),
            format!(
                "{} {}",
                max_x_date_time.date_naive(),
                max_x_date_time.time()
            ),
        ])
        .labels_alignment(Alignment::Right)
        .bounds([min_x, max_x]);

    // Create the Y axis and define its properties
    let y_axis = Axis::default()
        .style(Style::default().white())
        .bounds([0.0, 3.0]);

    // Create the chart and link all the parts together
    let chart = Chart::new(vec![info_dataset, debug_dataset, error_dataset])
        .block(
            Block::new()
                .title_bottom(
                    // Line::from(format!(
                    //     "{}, {}",
                    //     selected_date_time.date_naive(),
                    //     selected_date_time.time(),
                    // ))
                    Line::from(format!(
                        "{}, {} ({} hours ago)",
                        selected_date_time.date_naive(),
                        selected_date_time.time(),
                        Utc::now()
                            .signed_duration_since(selected_date_time)
                            .num_hours(),
                    ))
                    .bold()
                    .red()
                    .centered(),
                )
                .padding(Padding::horizontal(2))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .x_axis(x_axis)
        .y_axis(y_axis);

    frame.render_widget(chart, area);
}

pub fn render_log_detail(app: &mut App, frame: &mut Frame, area: Rect) {
    let area = centered_rect(60, 20, area);
    let key_idx = app.logs.log_item_list_state.selected().unwrap();
    let log = &app.logs.selected().unwrap()[key_idx];

    let paragraph = Paragraph::new(log.clone())
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default());

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

pub fn render_log_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let items = app
        .logs
        .logs
        .iter()
        // .keys()
        .filter(|(_, v)| apply_filter(app, v))
        .map(|(k, _)| k.to_owned())
        .collect::<Vec<String>>();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if app.focus.panel == Focus::Default {
                    app.config.theme.focus_fg
                } else {
                    Color::White
                }))
                .title("[Timestamps]".bold()),
        )
        .highlight_style(
            Style::new()
                .add_modifier(Modifier::REVERSED)
                .fg(app.config.theme.chart_fg),
        )
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true);

    let mut scrollbar_state = ScrollbarState::default()
        .content_length(app.logs.len())
        .position(app.logs.log_list_state.selected().unwrap_or_default());

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);

    frame.render_stateful_widget(list, area, &mut app.logs.log_list_state);
    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    )
}

pub fn apply_filter(app: &App, log_lines: &[String]) -> bool {
    if app.logs.filters.is_empty() {
        return true;
    }
    for line in log_lines {
        for filter in &app.logs.filters {
            if line.contains(filter) {
                return true;
            }
        }
    }

    false
}

pub fn render_log(app: &mut App, frame: &mut Frame, area: Rect) {
    let logs = app.logs.clone();
    let default = vec![]; // TODO??
    let lines = logs
        .selected()
        .unwrap_or(&default)
        .iter()
        .map(|v| style_detail_line(app, v.to_string()));
    let list = List::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if app.focus.panel == Focus::Log {
                    app.config.theme.focus_fg
                } else {
                    Color::White
                }))
                .title("[Log]".bold()),
        )
        .highlight_style(
            Style::new()
                .add_modifier(Modifier::REVERSED)
                .fg(app.config.theme.chart_fg),
        )
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true);

    frame.render_stateful_widget(list, area, &mut app.logs.log_item_list_state);
}

pub fn render_tabs(app: &mut App, frame: &mut Frame, area: Rect) {
    let titles = app.tabs.clone();
    let tabs = Tabs::new(titles)
        .highlight_style(Style::default().fg(Color::Green).bold())
        .select(app.focus.tab as usize)
        .padding("", "")
        .divider(" | ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Tabs"),
        );

    frame.render_widget(tabs, area);
}

pub fn render_splash(app: &mut App, frame: &mut Frame, area: Rect) {
    let dummy = BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::new().blue())
        .lines(vec!["Old Relic".fg(app.config.theme.focus_fg).into()])
        .build();

    let center = centered_rect(60, 60, area);
    frame.render_widget(dummy, center);
}

pub fn render_loading(app: &mut App, frame: &mut Frame, area: Rect) {
    let center = centered_rect(5, 5, area);
    let throbber = throbber_widgets_tui::Throbber::default()
        .label("Loading data...")
        .style(Style::default().fg(app.config.theme.focus_fg))
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
        .style(match app.focus.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Input => Style::default().fg(app.config.theme.focus_fg),
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
        .style(match app.focus.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Input => Style::default().fg(app.config.theme.focus_fg),
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
                .style(Style::default().fg(app.config.theme.chart_fg))
                .bounds([min_x, Utc::now().timestamp() as f64])
                .labels(vec![
                    DateTime::from_timestamp(min_x as i64, 0)
                        .unwrap()
                        .time()
                        .to_string()
                        .fg(app.config.theme.chart_fg)
                        .bold(),
                    DateTime::from_timestamp(Utc::now().timestamp(), 0)
                        .unwrap()
                        .to_string()
                        .fg(app.config.theme.chart_fg)
                        .bold(),
                ]);

            // Create the Y axis and define its properties
            let y_axis = Axis::default()
                .title(selection.clone().fg(app.config.theme.chart_fg))
                .style(Style::default().fg(app.config.theme.chart_fg))
                .bounds([min_y, max_y])
                .labels(vec![
                    min_y.to_string().fg(app.config.theme.chart_fg).bold(),
                    half_y.to_string().fg(app.config.theme.chart_fg).bold(),
                    max_y.to_string().fg(app.config.theme.chart_fg).bold(),
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
                .lines(vec!["Old Relic".fg(app.config.theme.focus_fg).into()])
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
        .style(match app.focus.panel {
            Focus::Rename => Style::default().fg(app.config.theme.focus_fg),
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
                .fg(app.config.theme.chart_fg),
        )
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true);

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

pub fn render_query_box(app: &mut App, frame: &mut Frame, area: Rect) {
    let input = Paragraph::new(app.inputs.get(Focus::QueryInput).bold())
        .style(match app.focus.panel {
            Focus::QueryInput => Style::default().fg(app.config.theme.focus_fg),
            _ => Style::default(),
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("[Query]".bold()),
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
            .title("Time".fg(app.config.theme.chart_fg))
            .style(Style::default().fg(app.config.theme.chart_fg))
            .bounds([min_x, Utc::now().timestamp() as f64])
            .labels(vec![
                DateTime::from_timestamp(min_x as i64, 0)
                    .unwrap()
                    .time()
                    .to_string()
                    .fg(app.config.theme.chart_fg)
                    .bold(),
                DateTime::from_timestamp(Utc::now().timestamp(), 0)
                    .unwrap()
                    .to_string()
                    .fg(app.config.theme.chart_fg)
                    .bold(),
            ]);

        // Create the Y axis and define its properties
        let y_axis = Axis::default()
            .title(selection.clone().fg(app.config.theme.chart_fg))
            .style(Style::default().fg(app.config.theme.chart_fg))
            .bounds([min_y, max_y])
            .labels(vec![
                min_y.to_string().fg(app.config.theme.chart_fg).bold(),
                half_y.to_string().fg(app.config.theme.chart_fg).bold(),
                max_y.to_string().fg(app.config.theme.chart_fg).bold(),
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
                    .border_style(Style::default().fg(app.config.theme.chart_fg))
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
