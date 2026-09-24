//! Unix timestamp ↔ human-readable UTC date-time.
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

use super::prelude::*;

struct FromTs;
impl Node for FromTs {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let s = in_text(inputs, "text")?.trim();
        let secs: i64 = s
            .parse()
            .map_err(|_| CoreError::Parse("需要 Unix 秒时间戳(整数)".into()))?;
        let dt = Utc
            .timestamp_opt(secs, 0)
            .single()
            .ok_or_else(|| CoreError::Parse("时间戳超出范围".into()))?;
        Ok(out_text(dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()))
    }
}

struct ToTs;
impl Node for ToTs {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let s = in_text(inputs, "text")?.trim();
        let ts = DateTime::parse_from_rfc3339(s)
            .map(|d| d.timestamp())
            .or_else(|_| {
                NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .map(|d| d.and_utc().timestamp())
            })
            .or_else(|_| {
                NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .map(|d| d.and_utc().timestamp())
            })
            .map_err(|_| CoreError::Parse("无法解析日期时间(试试 YYYY-MM-DD HH:MM:SS)".into()))?;
        Ok(out_text(ts.to_string()))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "from_timestamp",
            UTIL,
            "时间戳转日期",
            CYAN,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(FromTs)),
    );
    reg.register(
        desc(
            "to_timestamp",
            UTIL,
            "日期转时间戳",
            CYAN,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(ToTs)),
    );
}
