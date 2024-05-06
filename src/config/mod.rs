


use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Env{
    pub REDIS_HOST: String,  
    pub REDIS_PORT: String,  
    pub REDIS_USERNAME: String,  
    pub REDIS_PASSWORD: String,  
    pub REDIS_SESSION_EXP_KEY: String,  
    pub IO_BUFFER_SIZE: String,  
    pub FILE_SIZE: String,  
    pub HOST: String,  
    pub AMQP_PORT: String,
    pub AMQP_HOST: String,
    pub AMQP_USERNAME: String,
    pub AMQP_PASSWORD: String,
    pub TCP_PORT: String,  
    pub GRPC_PORT: String,
    pub HTTP_PORT: String,
    pub SECRET_KEY: String,  
    pub ENVIRONMENT: String,  
    pub DB_MIGRATION_ENV: String,  
    pub MACHINE_ID: String,  
    pub NODE_ID: String,  
    pub POSTGRES_HOST: String,  
    pub POSTGRES_PORT: String,  
    pub POSTGRES_USERNAME: String,  
    pub POSTGRES_PASSWORD: String,  
    pub DB_ENGINE: String,  
    pub DB_NAME: String,
    pub DATABASE_URL: String,
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Context<C>{
    pub vars: C

}
pub trait EnvExt{
    
    type Context;
    fn get_vars(&self) -> Self::Context;
}

impl EnvExt for Env{

    type Context = Context<Self>;

    fn get_vars(&self) -> Self::Context {

        dotenv::dotenv().expect("expected .env file be there!");
        let ctx = Context::<Env>{ // context contains a generic of type Env vars
            vars: Env{
                REDIS_HOST: std::env::var("REDIS_HOST").expect("REDIS_HOST must be there!"),
                REDIS_PORT: std::env::var("REDIS_PORT").expect("REDIS_PORT must be there!"),
                REDIS_USERNAME: std::env::var("REDIS_USERNAME").expect("REDIS_USERNAME must be there!"),
                REDIS_PASSWORD: std::env::var("REDIS_PASSWORD").expect("REDIS_PASSWORD must be there!"),
                REDIS_SESSION_EXP_KEY: std::env::var("REDIS_SESSION_EXP_KEY").expect("REDIS_SESSION_EXP_KEY must be there!"),
                IO_BUFFER_SIZE: std::env::var("IO_BUFFER_SIZE").expect("IO_BUFFER_SIZE must be there!"),
                FILE_SIZE: std::env::var("FILE_SIZE").expect("FILE_SIZE must be there!"),
                HOST: std::env::var("HOST").expect("HOST must be there!"),
                AMQP_PORT: std::env::var("AMQP_PORT").expect("AMQP_PORT must be there!"),
                AMQP_HOST: std::env::var("AMQP_HOST").expect("AMQP_HOST must be there!"),
                AMQP_USERNAME: std::env::var("AMQP_USERNAME").expect("AMQP_USERNAME must be there!"),
                AMQP_PASSWORD: std::env::var("AMQP_PASSWORD").expect("AMQP_PASSWORD must be there!"),
                TCP_PORT: std::env::var("TCP_PORT").expect("TCP_PORT must be there!"),
                GRPC_PORT: std::env::var("GRPC_PORT").expect("GRPC_PORT must be there!"),
                HTTP_PORT: std::env::var("HTTP_PORT").expect("HTTP_PORT must be there!"),
                SECRET_KEY: std::env::var("SECRET_KEY").expect("SECRET_KEY must be there!"),
                ENVIRONMENT: std::env::var("ENVIRONMENT").expect("ENVIRONMENT must be there!"),
                DB_MIGRATION_ENV: std::env::var("DB_MIGRATION_ENV").expect("DB_MIGRATION_ENV must be there!"),
                MACHINE_ID: std::env::var("MACHINE_ID").expect("MACHINE_ID must be there!"),
                NODE_ID: std::env::var("NODE_ID").expect("NODE_ID must be there!"),
                POSTGRES_HOST: std::env::var("POSTGRES_HOST").expect("POSTGRES_HOST must be there!"),
                POSTGRES_PORT: std::env::var("POSTGRES_PORT").expect("POSTGRES_PORT must be there!"),
                POSTGRES_USERNAME: std::env::var("POSTGRES_USERNAME").expect("POSTGRES_USERNAME must be there!"),
                POSTGRES_PASSWORD: std::env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be there!"),
                DB_ENGINE: std::env::var("DB_ENGINE").expect("DB_ENGINE must be there!"),
                DB_NAME: std::env::var("DB_NAME").expect("DB_NAME must be there!"),
                DATABASE_URL: std::env::var("DATABASE_URL").expect("DATABASE_URL must be there!"),
            }
        };

        ctx
        
    }

}