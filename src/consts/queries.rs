


pub const SELECT_BY_NOTIF_ID: &str = r#"
    select * from notifs where id = $1
"#;

pub const INSERT_HOOP: &str = r#"
    insert into hoops (etype, manager, entrance_fee) 
    values ($1, $2, $3)
"#;