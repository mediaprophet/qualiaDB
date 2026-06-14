pub mod models;
pub mod parser;
pub mod rdf;
pub mod store;
pub mod shapes;
pub mod webizen;
pub mod n3_rules;
pub mod qualia_bindings;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(test)]
mod tests {
    use super::*;
    use models::*;
    use parser::*;
    use rdf::*;
    use chrono::Timelike;

    #[test]
    fn test_parse_time_offset_minutes() {
        assert_eq!(parse_time_offset_minutes("60"), 60);
        assert_eq!(parse_time_offset_minutes("UTC+1000"), 600);
        assert_eq!(parse_time_offset_minutes("UTC-0500"), -300);
        assert_eq!(parse_time_offset_minutes(""), 0);
    }

    #[test]
    fn test_parse_samsung_datetime() {
        let dt = parse_samsung_datetime("1777632000000", 60).unwrap();
        assert_eq!(dt.timezone().local_minus_utc(), 3600);
        
        let dt2 = parse_samsung_datetime("2026-05-25 12:00:00", 60).unwrap();
        assert_eq!(dt2.timezone().local_minus_utc(), 3600);
        assert_eq!(dt2.hour(), 12);
    }

    #[test]
    fn test_parse_weight_csv() {
        let csv_data = "uuid,start_time,end_time,time_offset,weight,body_fat,muscle_mass,body_water,skeletal_muscle,bmi\n\
                        a1000001-0000-4000-8000-000000000001,1777632000000,1777632060000,60,72.0,18.5,32.1,55.2,30.5,23.1\n";
        let records = parse_weight_csv(csv_data).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].weight, 72.0);
        assert_eq!(records[0].bmi, 23.1);
        
        let rdf = weight_to_turtle(&records);
        assert!(rdf.contains("fhir:Observation.valueQuantity 72"));
        assert!(rdf.contains("health:bodyFatPercentage 18.5"));
    }

    #[test]
    fn test_parse_sleep_csv() {
        let csv_data = "uuid,start_time,end_time,time_offset,sleep_duration,efficiency,deep_sleep,rem_sleep,light_sleep,wake_up_count\n\
                        b2000001-0000-4000-8000-000000000001,1777681200000,1777707600000,60,440,78,90,110,240,0\n";
        let records = parse_sleep_csv(csv_data).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].sleep_duration, 440.0);
        assert_eq!(records[0].efficiency, 78.0);
        
        let rdf = sleep_to_turtle(&records);
        assert!(rdf.contains("health:sleepEfficiency 78"));
    }

    #[test]
    fn test_parse_heart_rate_csv() {
        let csv_data = "uuid,start_time,time_offset,heart_rate,min,max,heart_rate_zone,binning_data\n\
                        c3000001-0000-4000-8000-000000000001,1777668000000,60,68,58,120,rest,hr_bin_0.json\n";
        let records = parse_heart_rate_csv(csv_data).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].heart_rate, 68);
        
        let rdf = heart_rate_to_turtle(&records);
        assert!(rdf.contains("fhir:Observation.valueQuantity 68"));
    }

    #[test]
    fn test_parse_steps_csv() {
        let csv_data = "uuid,start_time,end_time,time_offset,count,calorie,distance\n\
                        e5000001-0000-4000-8000-000000000001,1777632000000,1777718340000,60,6500,210,4800\n";
        let records = parse_steps_csv(csv_data).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].count, 6500);
        
        let rdf = steps_to_turtle(&records);
        assert!(rdf.contains("fhir:Observation.valueQuantity 6500"));
        assert!(rdf.contains("health:caloriesBurned 210"));
    }
}
