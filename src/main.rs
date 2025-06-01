#![feature(duration_constructors_lite)]

use std::{
    fmt::{Display, Write},
    io::Result as IoResult,
    net::SocketAddr,
    time::Duration,
};

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
};
use rand::distr::{Alphabetic, SampleString};
use time::{
    OffsetDateTime,
    macros::{format_description, offset},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

#[derive(Clone, Copy)]
enum Status {
    CheckingIn,
    Sleep,
    Upload,
    Download,
    Hostname,
    Netstat,
    ProcessList,
    Invoke,
    Shutdown,
    Done,
}

struct TestResult {
    time: OffsetDateTime,
    addr: SocketAddr,
    status: Status,
    log: String,
}

#[tokio::main]
async fn main() -> IoResult<()> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7777").await?;
    println!("Strated acme server port: 7777");
    let results: &'static _ = Box::leak(Box::new(Mutex::new(Vec::new())));
    tokio::spawn(web_job(results));
    tokio::spawn(cleanup_job(results));
    while let Ok((conn, addr)) = listener.accept().await {
        println!("New rr connection from:{addr}");
        tokio::spawn(async move {
            let mut log = String::new();
            let status = match handle_connection(&mut log, conn).await {
                Ok(s) => s,
                Err((s, error)) => {
                    log += error.as_str();
                    s
                }
            };
            let time = time::UtcDateTime::now().to_offset(offset!(-4));
            results.lock().await.push(TestResult {
                time,
                addr,
                status,
                log,
            });
        });
    }
    Ok(())
}

async fn cleanup_job(results: &'static Mutex<Vec<TestResult>>) {
    loop {
        tokio::time::sleep(Duration::from_mins(5)).await;
        let now = OffsetDateTime::now_utc().to_offset(offset!(-4));
        results
            .lock()
            .await
            .retain(|i| (i.time + Duration::from_mins(5)) > now);
    }
}

const CHECKIN_MESSAGE: &'static [u8] = b"roadrunner checkin\0";
const SHUTDOWN_MESSAGE: &'static [u8] = b"shutting down\0";

async fn handle_connection(
    log: &mut String,
    mut conn: TcpStream,
) -> Result<Status, (Status, String)> {
    writeln!(log, "============\n").unwrap();
    writeln!(log, "Testing Checkin").unwrap();
    let mut status = Status::CheckingIn;
    handle_response(log, &mut conn, Some(CHECKIN_MESSAGE))
        .await
        .map_err(|msg| (status, msg))?;
    writeln!(log, "Checkin Successful").unwrap();
    writeln!(log, "============\n").unwrap();
    writeln!(log, "Testing Sleep Command").unwrap();
    status = Status::Sleep;
    send_recieve(log, &mut conn, b"sleep\0", b"1\0", None)
        .await
        .map_err(|msg| (status, msg))?;
    writeln!(log, "Sleep Command Successful").unwrap();
    writeln!(log, "============\n").unwrap();
    writeln!(log, "Testing Upload Command").unwrap();
    status = Status::Upload;
    let path_rnd = Alphabetic.sample_string(&mut rand::rng(), 16);
    let path = format!("/tmp/{path_rnd}.rr.txt\0").into_bytes();
    let content_rnd = Alphabetic.sample_string(&mut rand::rng(), 16);
    let content = format!("File from test server: {content_rnd}").into_bytes();
    let upload_arg = generate_upload_arg(&path, &content);
    send_recieve(log, &mut conn, b"upload\0", &upload_arg, None)
        .await
        .map_err(|msg| (status, msg))?;
    writeln!(log, "Upload Command Successful").unwrap();
    writeln!(log, "============\n").unwrap();
    writeln!(log, "Testing Download Command").unwrap();
    status = Status::Download;
    send_recieve(log, &mut conn, b"download\0", &path, Some(&content))
        .await
        .map_err(|msg| (status, msg))?;
    writeln!(log, "Download Command Successful").unwrap();
    writeln!(log, "============\n").unwrap();
    writeln!(log, "Testing Hostname Command").unwrap();
    status = Status::Hostname;
    send_recieve(log, &mut conn, b"hostname\0", b"\0", None)
        .await
        .map_err(|msg| (status, msg))?;
    writeln!(log, "Hostname Command Successful").unwrap();
    writeln!(log, "============\n").unwrap();
    writeln!(log, "Testing Netstat Command").unwrap();
    status = Status::Netstat;
    send_recieve(log, &mut conn, b"netstat\0", b"\0", None)
        .await
        .map_err(|msg| (status, msg))?;
    writeln!(log, "Netstat Command Successful").unwrap();
    writeln!(log, "============\n").unwrap();
    writeln!(log, "Testing Process List Command").unwrap();
    status = Status::ProcessList;
    send_recieve(log, &mut conn, b"proclist\0", b"\0", None)
        .await
        .map_err(|msg| (status, msg))?;
    writeln!(log, "Process List Command Successful").unwrap();
    writeln!(log, "============\n").unwrap();
    writeln!(log, "Testing Invoke Command").unwrap();
    status = Status::Invoke;
    send_recieve(log, &mut conn, b"invoke\0", b"ls -al\0", None)
        .await
        .map_err(|msg| (status, msg))?;
    writeln!(log, "Invoke Command Successful").unwrap();
    writeln!(log, "============\n").unwrap();
    writeln!(log, "Testing Shutdown Command").unwrap();
    status = Status::Shutdown;
    send_recieve(log, &mut conn, b"shutdown\0", b"\0", Some(SHUTDOWN_MESSAGE))
        .await
        .map_err(|msg| (status, msg))?;
    writeln!(log, "Shutdown Command Successful").unwrap();
    writeln!(log, "============\n").unwrap();
    status = Status::Done;
    Ok(status)
}

fn generate_upload_arg(path: &[u8], content: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let path_len = path.len() as u32;
    let content_len = content.len() as u32;
    out.extend(path_len.to_be_bytes());
    out.extend(path);
    out.extend(content_len.to_be_bytes());
    out.extend(content);
    out
}

async fn send_recieve(
    log: &mut String,
    conn: &mut TcpStream,
    command: &[u8],
    args: &[u8],
    expected: Option<&[u8]>,
) -> Result<(), String> {
    send_command(log, conn, command, args).await?;
    handle_response(log, conn, expected).await?;
    Ok(())
}

async fn send_command(
    log: &mut String,
    conn: &mut TcpStream,
    command: &[u8],
    args: &[u8],
) -> Result<(), String> {
    let cmd = generate_command(command, args);
    writeln!(log, "---Command---").unwrap();
    write!(log, "{:16.64}", ByteFormat(&cmd)).unwrap();
    writeln!(log, "-------------").unwrap();
    conn.write_all(&cmd)
        .await
        .map_err(|e| format!("Failed to send {} command: {e}", ByteFormat(command)))?;
    Ok(())
}

fn generate_command(command: &[u8], args: &[u8]) -> Vec<u8> {
    let command_len = command.len() as u32;
    let args_len = args.len() as u32;
    let total_len = command_len + args_len + 12;
    let mut out = Vec::new();
    out.extend(total_len.to_be_bytes());
    out.extend(command_len.to_be_bytes());
    out.extend(command);
    out.extend(args_len.to_be_bytes());
    out.extend(args);
    out
}

async fn handle_response(
    log: &mut String,
    conn: &mut TcpStream,
    expected_message: Option<&[u8]>,
) -> Result<(), String> {
    let body = parse_response(log, conn).await?;
    if let Some(expected) = expected_message {
        if body != expected {
            return Err(format!(
                "Invalid response\nGot: {body:?}\nExpected: {expected:?}"
            ));
        }
    }
    Ok(())
}

async fn parse_response(log: &mut String, conn: &mut TcpStream) -> Result<Vec<u8>, String> {
    let total_size = conn
        .read_u32()
        .await
        .map_err(|e| format!("Failed to read total size: {e}"))?;
    writeln!(log, "Total size: {total_size}").unwrap();
    let ret_code = conn
        .read_u32()
        .await
        .map_err(|e| format!("Failed to read return code: {e}"))?;
    writeln!(log, "Return code: {ret_code}").unwrap();
    let message_length = conn
        .read_u32()
        .await
        .map_err(|e| format!("Failed to read message length: {e}"))?;
    writeln!(log, "Message length: {message_length}").unwrap();
    if message_length > 10000 {
        return Err("Too much data in message".to_owned());
    }
    let message_length = message_length as usize;
    let mut body = vec![0u8; message_length];
    let res = conn
        .read_exact(&mut body)
        .await
        .map_err(|e| format!("Failed to read message body: {e}"))?;
    if res != message_length {
        return Err(format!(
            "Message body length does not match header: expected: {message_length}, actual: {res}"
        ));
    }
    writeln!(log, "---Response Body---").unwrap();
    write!(log, "{:16.64}", ByteFormat(&body)).unwrap();
    writeln!(log, "-------------------").unwrap();
    Ok(body)
}

struct ByteFormat<'a>(&'a [u8]);

impl<'a> Display for ByteFormat<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(bytes) = self;
        let bytes_iter = bytes.into_iter().map(|&b| {
            if b.is_ascii_alphanumeric() {
                if f.alternate() {
                    format!("\x1b[31m{:>2.2}\x1b[m ", (b as char).to_string())
                } else {
                    format!("{:>2.2} ", (b as char).to_string())
                }
            } else {
                format!("{b:>02.2X} ")
            }
        });
        let bytes_vec: Vec<String> = if let Some(precision) = f.precision() {
            let mut bytes_vec: Vec<String> = bytes_iter.take(precision).collect();
            if bytes_vec.len() < bytes.len() {
                if let Some(last) = bytes_vec.last_mut() {
                    *last = "...".to_string();
                }
            }
            bytes_vec
        } else {
            bytes_iter.collect()
        };
        if let Some(width) = f.width() {
            for row in bytes_vec.chunks(width) {
                let row: String = row.iter().map(String::as_str).collect();
                writeln!(f, "{row}")?;
            }
        } else {
            let out: String = bytes_vec.iter().map(String::as_str).collect();
            write!(f, "{out}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::ByteFormat;

    #[test]
    fn test_bytes() {
        let b = [1, 2, 3, 4, 5, 6, 7, 8, 9, 65];
        let bytes = ByteFormat(&b);
        println!("{bytes:#}");
        assert!(false);
    }
}

async fn web_job(results: &'static Mutex<Vec<TestResult>>) -> IoResult<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8888").await?;
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
async fn root(State(state): State<&'static Mutex<Vec<TestResult>>>) -> impl IntoResponse {
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
