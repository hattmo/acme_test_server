use crate::setup::GenericListener;

use super::{Status, TestResult};

use std::{collections::VecDeque, io::Result as IoResult};

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
};
use time::macros::format_description;
use tokio::sync::Mutex;

pub async fn web_job(
    results: &'static Mutex<VecDeque<TestResult>>,
    listener: GenericListener,
) -> IoResult<()> {
    // let listener = tokio::net::TcpListener::bind("127.0.0.1:8888").await?;
    let router = axum::Router::new()
        .route("/", get(root))
        .with_state(results);
    println!("Web server started");
    axum::serve(listener, router).await?;
    Ok(())
}

static TABLE_HEAD: &str = r##"
<table>
    <tr>
        <th>Time</th>
        <th>IP</th>
        <th>CheckingIn</th>
        <th>Sleep</th>
        <th>Upload</th>
        <th>Download</th>
        <th>Hostname</th>
        <th>Netstat</th>
        <th>ProcessList</th>
        <th>Invoke</th>
        <th>Shutdown</th>
        <th>Done</th>
        <th>Logs</th>
    </tr>
"##;

static TABLE_TAIL: &str = r##"
</table>
"##;

async fn root(State(state): State<&'static Mutex<VecDeque<TestResult>>>) -> impl IntoResponse {
    let rows: String = state
        .lock()
        .await
        .iter()
        .rev()
        .enumerate()
        .map(
            |(
                i,
                TestResult {
                    time,
                    addr,
                    status,
                    log,
                },
            )| {
                let mut marks = [' '; 10];
                let n = match status {
                    Status::CheckingIn => 0,
                    Status::Sleep => 1,
                    Status::Upload => 2,
                    Status::Download => 3,
                    Status::Hostname => 4,
                    Status::Netstat => 5,
                    Status::ProcessList => 6,
                    Status::Invoke => 7,
                    Status::Shutdown => 8,
                    Status::Done => 9,
                };
                marks[..n].fill('\u{2705}');
                marks[n] = '\u{274C}';
                if let Status::Done = status {
                    marks[n] = '\u{2705}';
                }
                let marks: String = marks
                    .into_iter()
                    .map(|c| format!("<td>{}</td>", c))
                    .collect();
                let time = time
                    .format(format_description!("[hour]:[minute]"))
                    .unwrap_or_else(|_| "00:00".to_string());
                let tds = format!("<td>{time}</td><td>{addr}</td>") + &marks;
                // let log: String = log
                //     .chars()
                //     .map(|c| {
                //         if c == '\n' {
                //             "<br>".to_string()
                //         } else {
                //             c.to_string()
                //         }
                //     })
                //     .collect();
                format!(
                    "<tr>{tds}<td><a href=\"#{i}\">\u{2795}</a><a href=\"#\">\u{2796}</a></td></tr>
                    <tr id=\"{i}\" class=\"expandable\"><td colspan=13><pre>{log}</pre></td></tr>"
                )
            },
        )
        .collect();
    Html(format!(
        "<html>
            <head>
                <style>
table, th, td {{
    border: 1px solid black;
}}
.expandable {{
    display: none;
}}
.expandable:target {{
    display: block;
}}
a {{
    all: unset;
    cursor: pointer;
}}
                </style>
            </head>
            <body>{}{}{}</body>
        </html>",
        TABLE_HEAD, rows, TABLE_TAIL
    ))
}
