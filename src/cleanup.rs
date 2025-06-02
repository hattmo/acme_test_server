use super::TestResult;
use std::{collections::VecDeque, time::Duration};
use time::{OffsetDateTime, macros::offset};
use tokio::sync::Mutex;

pub async fn cleanup_job(results: &'static Mutex<VecDeque<TestResult>>) {
    loop {
        tokio::time::sleep(Duration::from_mins(5)).await;
        let now = OffsetDateTime::now_utc().to_offset(offset!(-4));
        results
            .lock()
            .await
            .retain(|i| (i.time + Duration::from_mins(5)) > now);
    }
}
