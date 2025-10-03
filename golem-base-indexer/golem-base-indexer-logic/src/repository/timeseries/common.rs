use anyhow::{anyhow, Result};
use chrono::{NaiveDate, NaiveDateTime, Utc};

#[derive(Debug)]
pub enum ChartResolution {
    Day,
    Hour,
    Week,
    Month,
}

impl TryFrom<i32> for ChartResolution {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self> {
        match value {
            0 => Ok(ChartResolution::Day),
            1 => Ok(ChartResolution::Hour),
            2 => Ok(ChartResolution::Week),
            3 => Ok(ChartResolution::Month),
            _ => Err(anyhow!("Error converting chart resolution")),
        }
    }
}

pub(super) fn parse_date_range(
    from: Option<String>,
    to: Option<String>,
) -> Result<(Option<NaiveDate>, Option<NaiveDate>)> {
    let from_date = match from {
        Some(date_str) => Some(
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|e| anyhow!("Invalid from date format: {}", e))?,
        ),
        None => None,
    };

    let to_date = match to {
        Some(date_str) => Some(
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|e| anyhow!("Invalid to date format: {}", e))?,
        ),
        None => None,
    };

    if let (Some(from), Some(to)) = (from_date, to_date) {
        if from > to {
            return Err(anyhow!(
                "From date ({}) cannot be later than to date ({})",
                from.format("%Y-%m-%d"),
                to.format("%Y-%m-%d")
            ));
        }
    }

    Ok((from_date, to_date))
}

pub(super) fn parse_datetime_range(
    from: Option<String>,
    to: Option<String>,
) -> Result<(Option<NaiveDateTime>, Option<NaiveDateTime>)> {
    let current_datetime = Utc::now().naive_utc();

    let from_datetime = match from {
        Some(datetime_str) => Some(
            NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M")
                .map_err(|e| anyhow!("Invalid from datetime format: {}", e))?,
        ),
        None => None,
    };

    let to_datetime = match to {
        Some(datetime_str) => {
            let parsed = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M")
                .map_err(|e| anyhow!("Invalid to datetime format: {}", e))?;

            Some(if from_datetime.is_some() && parsed > current_datetime {
                current_datetime
            } else {
                parsed
            })
        }
        None => Some(Utc::now().naive_utc()),
    };

    if let (Some(from), Some(to)) = (from_datetime, to_datetime) {
        if from > to {
            return Err(anyhow!(
                "From datetime ({}) cannot be later than to datetime ({})",
                from.format("%Y-%m-%d %H:%M"),
                to.format("%Y-%m-%d %H:%M")
            ));
        }
    }

    Ok((from_datetime, to_datetime))
}
