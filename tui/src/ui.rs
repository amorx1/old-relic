use std::sync::Arc;

use chrono::{DateTime, Local, Utc};
use ratatui::{
    prelude::*,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, List, Paragraph},
};
use style::palette::tailwind;
use tui_big_text::{BigText, PixelSize};

use crate::{app::InputMode, App};

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

pub fn render_query_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let items = app.datasets.keys().cloned().collect::<Vec<_>>();
    // let items = app.queries.clone().into_iter().collect::<Vec<_>>();
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
    let input = Paragraph::new(app.input.as_str())
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
    let datasets = app.datasets.get_mut(&app.selected_query).map(|data| {
        data.iter_mut()
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
        Some(_) => {
            let now = format!("{}.0", Utc::now().timestamp());
            let bounds = app
                .bounds
                .get(&app.selected_query)
                .expect("ERROR: No bounds found for selected query");
            let (min_x, mut min_y) = bounds.mins;
            let (max_x, mut max_y) = bounds.maxes;
            let mut half_y = (max_y - min_y) / 2 as f64;

            min_y = f64::round(min_y);
            max_y = f64::round(max_y);
            half_y = f64::round(half_y);

            // Create the X axis and define its properties
            let x_axis = Axis::default()
                .title("Time".red())
                .style(Style::default().white())
                .bounds([min_x, Utc::now().timestamp() as f64])
                .labels(vec![min_x.to_string().into(), now.into()]);

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
            let chart = Chart::new(datasets.unwrap())
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

// pub fn render_popup(app: &mut App, frame: &mut Frame, area: Rect) {
//     let block = Block::default().title("Event").borders(Borders::ALL);
//     let text = app
//         .events
//         .first_key_value()
//         .map_or(Paragraph::new(""), |(_, event)| {
//             Paragraph::new(Text::styled(
//                 format!("{}\n{}", event.subject, event.organizer,),
//                 Style::default().fg(Color::Red).bold(),
//             ))
//         });

//     let inner_area = centered_rect(60, 20, area);
//     frame.render_widget(Clear, area); //this clears out the background
//     frame.render_widget(Block::default().bg(Color::LightRed), area);
//     frame.render_widget(text.block(block).on_black(), inner_area);
// }

// pub fn render_selection(app: &mut App, frame: &mut Frame, area: Rect) {
//     if let Some(i) = app.table_state.selected() {
//         let text = app
//             .events
//             .iter()
//             .nth(i)
//             .map_or(Paragraph::new(""), |(_, event)| {
//                 Paragraph::new(Text::styled(
//                     format!(
//                         "{}\n{}\n{}\n{}\n{}\n{}",
//                         event.subject,
//                         event.location,
//                         event.organizer,
//                         event
//                             .teams_meeting
//                             .clone()
//                             .map_or("".to_string(), |meeting| meeting.url),
//                         event
//                             .response
//                             .clone()
//                             .unwrap_or(EventResponse::NotResponded),
//                         event.body
//                     ),
//                     Style::default().fg(Color::Red).bold(),
//                 ))
//             });

//         let block = Block::default()
//             .title("Event")
//             .borders(Borders::ALL)
//             .style(Style::default().fg(Color::Black));
//         let block2 = Block::default()
//             .title("Options")
//             .borders(Borders::ALL)
//             .style(Style::default().fg(Color::Black));

//         let inner_area = centered_rect(60, 40, area);
//         let layout = Layout::default()
//             .direction(Direction::Vertical)
//             .constraints(vec![Constraint::Percentage(70), Constraint::Percentage(30)])
//             .split(inner_area);

//         let text2 = Paragraph::new(Text::raw("\nACCEPT | REJECT")).alignment(Alignment::Center);
//         frame.render_widget(Clear, area);
//         frame.render_widget(Block::default().bg(Color::Rgb(64, 188, 252)), area);
//         frame.render_widget(text.block(block), layout[0]);
//         frame.render_widget(text2.block(block2), layout[1]);
//     }
// }

// pub fn render_table(app: &mut App, frame: &mut Frame, area: Rect) {
//     let layout = Layout::horizontal([Constraint::Percentage(100)])
//         .flex(layout::Flex::SpaceBetween)
//         .split(area);

//     let header_style = Style::default()
//         .fg(app.colors.header_fg)
//         .bg(app.colors.header_bg);
//     let selected_style = Style::default()
//         .add_modifier(Modifier::REVERSED)
//         .fg(app.colors.selected_style_fg);
//     let header = [
//         Text::from("Event")
//             .style(Style::default().bold())
//             .alignment(Alignment::Left),
//         Text::from("Start Time")
//             .style(Style::default().bold())
//             .alignment(Alignment::Left),
//         Text::from("Duration")
//             .style(Style::default().bold())
//             .alignment(Alignment::Left),
//     ]
//     .iter()
//     .cloned()
//     .map(Cell::from)
//     .collect::<Row>()
//     .style(header_style)
//     .height(2);

//     let footer = Row::new(vec![Cell::from("up/down: k/j | open/close: l/h").bold()])
//         .height(1)
//         .top_margin(0);

//     // let rows = app.events.iter().enumerate().map(|(i, (_, e))| {
//     //     let color = match i % 2 {
//     //         0 => app.colors.normal_row_color,
//     //         _ => app.colors.alt_row_color,
//     //     };

//     //     let duration = &e.end_time.signed_duration_since(e.start_time).num_minutes();
//     //     let subject = e.subject.clone();
//     //     let local_dt: DateTime<Local> = DateTime::from(e.start_time);
//     //     let date = local_dt.date_naive();
//     //     let time = local_dt.time();

//     //     Row::new(vec![
//     //         Cell::new(Span::from(subject)).style(Style::default().bold()),
//     //         Cell::new(Span::from(format!("{date:?} @ {time:?}"))),
//     //         Cell::new(Span::from(format!("{duration:?} mins"))),
//     //     ])
//     //     .style(Style::new().fg(app.colors.row_fg).bg(color))
//     //     .height(3)
//     // });

//     let widths = [
//         Constraint::Percentage(40),
//         Constraint::Percentage(45),
//         Constraint::Percentage(15),
//     ];
//     let table = Table::new(rows, widths)
//         .header(header)
//         .footer(footer)
//         // .bg(app.colors.buffer_bg)
//         .highlight_style(selected_style);

//     frame.render_stateful_widget(table, layout[0], &mut app.table_state);
// }
