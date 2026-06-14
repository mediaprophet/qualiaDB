use serde::Deserialize;
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};
use crate::models::{WeightRecord, SleepRecord, HeartRateRecord, StepRecord};

pub fn clean_csv_content(content: &str) -> String {
    let mut lines = content.lines();
    if let Some(first_line) = lines.next() {
        let stripped = first_line.trim();
        if stripped.starts_with("com.samsung.health.") || stripped.starts_with("com.samsung.shealth.") {
            return lines.collect::<Vec<&str>>().join("\n");
        }
    }
    content.to_string()
}

pub fn parse_time_offset_minutes(offset_str: &str) -> i32 {
    let s = offset_str.trim();
    if s.is_empty() {
        return 0;
    }

    if let Ok(val) = s.parse::<i32>() {
        return val;
    }
    if let Ok(val) = s.parse::<f64>() {
        return val as i32;
    }

    if s.to_uppercase().starts_with("UTC") {
        let clean = s[3..].trim();
        if clean.is_empty() {
            return 0;
        }
        let sign = if clean.starts_with('-') { -1 } else { 1 };
        let numeric_part: String = clean.chars().filter(|c| c.is_ascii_digit()).collect();
        if numeric_part.len() >= 2 {
            if let Ok(hours) = numeric_part[0..2].parse::<i32>() {
                let minutes = if numeric_part.len() >= 4 {
                    numeric_part[2..4].parse::<i32>().unwrap_or(0)
                } else {
                    0
                };
                return sign * (hours * 60 + minutes);
            }
        }
    }

    0
}

pub fn parse_samsung_datetime(value: &str, offset_minutes: i32) -> Result<DateTime<FixedOffset>, String> {
    let value = value.trim();
    let offset = FixedOffset::east_opt(offset_minutes * 60)
        .ok_or_else(|| format!("Invalid timezone offset: {} minutes", offset_minutes))?;

    if let Ok(val_f) = value.parse::<f64>() {
        let val_i = val_f as i64;
        if val_i > 1_000_000_000_000 {
            let seconds = val_i / 1000;
            let nanos = (val_i % 1000) * 1_000_000;
            if let Some(utc_dt) = DateTime::from_timestamp(seconds, nanos as u32) {
                return Ok(utc_dt.with_timezone(&offset));
            }
        } else if val_i > 1_000_000_000 {
            if let Some(utc_dt) = DateTime::from_timestamp(val_i, 0) {
                return Ok(utc_dt.with_timezone(&offset));
            }
        }
    }

    let formats = [
        "%Y-%m-%d %H:%M:%S%.3f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d %H:%M",
    ];

    for fmt in &formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, fmt) {
            if let Some(local_dt) = offset.from_local_datetime(&naive).single() {
                return Ok(local_dt);
            } else {
                return Ok(DateTime::<FixedOffset>::from_naive_utc_and_offset(naive, offset));
            }
        }
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Ok(dt.with_timezone(&offset));
    }

    Err(format!("Could not parse datetime: {}", value))
}

#[derive(Debug, Deserialize)]
struct WeightCsvRow {
    uuid: String,
    start_time: String,
    time_offset: Option<String>,
    weight: f64,
    body_fat: Option<f64>,
    muscle_mass: Option<f64>,
    body_water: Option<f64>,
    skeletal_muscle: Option<f64>,
    bmi: f64,
}

pub fn parse_weight_csv(content: &str) -> Result<Vec<WeightRecord>, String> {
    let cleaned = clean_csv_content(content);
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(cleaned.as_bytes());
    
    let mut records = Vec::new();
    for result in rdr.deserialize::<WeightCsvRow>() {
        let row = result.map_err(|e| format!("CSV deserialize error: {}", e))?;
        let offset_str = row.time_offset.unwrap_or_default();
        let offset = parse_time_offset_minutes(&offset_str);
        let start_datetime = parse_samsung_datetime(&row.start_time, offset)?;
        
        records.push(WeightRecord {
            uuid: row.uuid,
            weight: row.weight,
            body_fat: row.body_fat,
            muscle_mass: row.muscle_mass,
            body_water: row.body_water,
            skeletal_muscle: row.skeletal_muscle,
            bmi: row.bmi,
            start_datetime,
        });
    }
    Ok(records)
}

#[derive(Debug, Deserialize)]
struct SleepCsvRow {
    uuid: String,
    start_time: String,
    end_time: String,
    time_offset: Option<String>,
    sleep_duration: f64,
    efficiency: f64,
    deep_sleep: Option<f64>,
    rem_sleep: Option<f64>,
    light_sleep: Option<f64>,
    wake_up_count: Option<u32>,
}

pub fn parse_sleep_csv(content: &str) -> Result<Vec<SleepRecord>, String> {
    let cleaned = clean_csv_content(content);
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(cleaned.as_bytes());
    
    let mut records = Vec::new();
    for result in rdr.deserialize::<SleepCsvRow>() {
        let row = result.map_err(|e| format!("CSV deserialize error: {}", e))?;
        let offset_str = row.time_offset.unwrap_or_default();
        let offset = parse_time_offset_minutes(&offset_str);
        let start_datetime = parse_samsung_datetime(&row.start_time, offset)?;
        let end_datetime = parse_samsung_datetime(&row.end_time, offset)?;
        
        records.push(SleepRecord {
            uuid: row.uuid,
            sleep_duration: row.sleep_duration,
            efficiency: row.efficiency,
            deep_sleep: row.deep_sleep,
            rem_sleep: row.rem_sleep,
            light_sleep: row.light_sleep,
            wake_up_count: row.wake_up_count,
            start_datetime,
            end_datetime,
        });
    }
    Ok(records)
}

#[derive(Debug, Deserialize)]
struct HeartRateCsvRow {
    uuid: String,
    start_time: String,
    time_offset: Option<String>,
    heart_rate: u32,
    min: Option<u32>,
    max: Option<u32>,
    heart_rate_zone: Option<String>,
    binning_data: Option<String>,
}

pub fn parse_heart_rate_csv(content: &str) -> Result<Vec<HeartRateRecord>, String> {
    let cleaned = clean_csv_content(content);
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(cleaned.as_bytes());
    
    let mut records = Vec::new();
    for result in rdr.deserialize::<HeartRateCsvRow>() {
        let row = result.map_err(|e| format!("CSV deserialize error: {}", e))?;
        let offset_str = row.time_offset.unwrap_or_default();
        let offset = parse_time_offset_minutes(&offset_str);
        let start_datetime = parse_samsung_datetime(&row.start_time, offset)?;
        
        records.push(HeartRateRecord {
            uuid: row.uuid,
            heart_rate: row.heart_rate,
            min: row.min,
            max: row.max,
            heart_rate_zone: row.heart_rate_zone,
            binning_data: row.binning_data,
            start_datetime,
        });
    }
    Ok(records)
}

#[derive(Debug, Deserialize)]
struct StepCsvRow {
    uuid: String,
    start_time: String,
    end_time: String,
    time_offset: Option<String>,
    count: u32,
    calorie: f64,
    distance: f64,
}

pub fn parse_steps_csv(content: &str) -> Result<Vec<StepRecord>, String> {
    let cleaned = clean_csv_content(content);
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(cleaned.as_bytes());
    
    let mut records = Vec::new();
    for result in rdr.deserialize::<StepCsvRow>() {
        let row = result.map_err(|e| format!("CSV deserialize error: {}", e))?;
        let offset_str = row.time_offset.unwrap_or_default();
        let offset = parse_time_offset_minutes(&offset_str);
        let start_datetime = parse_samsung_datetime(&row.start_time, offset)?;
        let end_datetime = parse_samsung_datetime(&row.end_time, offset)?;
        
        records.push(StepRecord {
            uuid: row.uuid,
            count: row.count,
            calorie: row.calorie,
            distance: row.distance,
            start_datetime,
            end_datetime,
        });
    }
    Ok(records)
}
