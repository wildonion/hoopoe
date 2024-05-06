



// BEST LOGIC
// -ˋˏ✄┈┈┈┈ inserting new location with cumulative mileage
pub const INSERT_QUERY: &str = r#"
    WITH 
        last_record AS (
            SELECT 
                cumulative_mileage,
                latitude,
                longitude
            FROM hyper_locations
            WHERE imei = $1
            ORDER BY date DESC
            LIMIT 1
        ),
        cum AS (
            SELECT
                CASE
                    WHEN NOT EXISTS (SELECT 1 FROM last_record) THEN 0
                    ELSE (
                        COALESCE((SELECT cumulative_mileage FROM last_record), 0) + 
                        ST_DistanceSphere(
                            ST_MakePoint((SELECT longitude FROM last_record), (SELECT latitude FROM last_record)),
                            ST_MakePoint($7, $6)
                        ) / 1000
                    )
                END AS cumulative_mileage
        )
    INSERT INTO hyper_locations (
            timestamp, correlation_id, device_id, imei, latitude, 
            longitude, date, position_status, speed, heading, altitude, 
            satellites, gsm_signal, odometer, hdop, mobile_country_code, 
            mobile_network_code, cell_id, location_area_code, cumulative_mileage
        ) 
    VALUES ($2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, 
            $17, $18, $19, $20, (SELECT cumulative_mileage FROM cum))
        
"#;

// BAD LOGIC, SLOW AND HIGH LATENCY EXECUTION
// -ˋˏ✄┈┈┈┈ calculating cumulative mileage, min and max by executing query over all rows
pub const BASIC_REPORT_QUERY0: &str = r#"
    WITH cte AS (
        SELECT 
            imei, latitude, longitude, speed, date,
            LAG(latitude) OVER (ORDER BY date) AS prev_lat,
            LAG(longitude) OVER (ORDER BY date) AS prev_long
        FROM hyper_locations
        WHERE imei = $1 AND date >= $2 AND date <= $3
    ),
    cte_distance AS (
        SELECT 
            imei, latitude, longitude, speed, date, prev_lat, prev_long,
            ST_DistanceSphere(
                ST_MakePoint(prev_long, prev_lat),
                ST_MakePoint(longitude, latitude)
            ) / 1000 AS destination
        FROM cte
    )
    SELECT 
        imei, latitude, longitude, speed, destination,
        SUM(destination) OVER (ORDER BY date) AS cumulative_mileage,
        MIN(speed) OVER () AS speed_min,
        MAX(speed) OVER () AS speed_max,
        AVG(speed) OVER () AS speed_avg
    FROM cte_distance
    ORDER BY date DESC;
"#;

pub const BASIC_REPORT_QUERY1: &str = r#"
    WITH speed_stats AS(
        SELECT 
            MIN(speed) AS speed_min,
            MAX(speed) AS speed_max,
            AVG(speed) AS speed_avg
        FROM hyper_locations 
        WHERE imei = $1 AND date >= $2 AND date <= $3
    )
    SELECT MAX(cumulative_mileage) - MIN(cumulative_mileage) AS mileage,
    (SELECT speed_min from speed_stats), (SELECT speed_max from speed_stats), (SELECT speed_avg from speed_stats)
    FROM hyper_locations 
    WHERE imei = $1 AND date >= $2 AND date <= $3
"#;