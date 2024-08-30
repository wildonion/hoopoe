


pub const SELECT_BY_NOTIF_ID: &str = r#"
    select * from notifs where id = $1
"#;

pub const INSERT_HOOP: &str = r#"
    insert into hoops (etype, manager, entrance_fee, title, 
    description, duration, capacity, cover, started_at, end_at) 
    values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
"#;

pub const UPDATE_IS_FINISHED: &str = r#"
    update hoops set is_finished = true where title = $1;
"#;

pub const SELECT_HOOP_BY_ID: &str = r#"
    select * from hoops where id = $1
"#;

pub const SELECT_HOOP_BY_TITLE: &str = r#"
    select * from hoops where title = $1
"#;