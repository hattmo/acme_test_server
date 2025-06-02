use super::TestResult;
use std::{collections::VecDeque, process::exit, time::Duration};
use time::{OffsetDateTime, macros::offset};
use tokio::sync::Mutex;

pub async fn cleanup_job(results: &'static Mutex<VecDeque<TestResult>>, is_activated: bool) {
    loop {
        tokio::time::sleep(Duration::from_mins(20)).await;
        let now = OffsetDateTime::now_utc().to_offset(offset!(-4));
        let mut results = results.lock().await;
        results.retain(|i| (i.time + Duration::from_mins(5)) > now);
        if is_activated && results.len() == 0 {
            println!("Shuting down due to inactivity");
            exit(0);
        }
    }
}
