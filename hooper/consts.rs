


use crate::*;
use rand::Rng; // Rng trait is implemented for rng instances, just use it here

pub static APP_NAME: &str = "Hoopoe";
pub const SERVERS: &[&str] = &["p2p", "tcp", "grpc"];

pub fn gen_random_chars(size: u32) -> String{
    let mut rng = rand::thread_rng();
    (0..size).map(|_|{
        /* converint the generated random ascii to char */
        char::from_u32(rng.gen_range(33..126)).unwrap() // generating a char from the random output of type u32 using from_u32() method
    }).collect()
}

#[derive(Serialize, Deserialize)]
struct TcpSharedSetup{
    pub wallet: wallexerr::misc::Wallet,
    pub secure_cell_config: wallexerr::misc::SecureCellConfig
}
pub static SECURECELLCONFIG_TCPWALLET: Lazy<(wallexerr::misc::SecureCellConfig, wallexerr::misc::Wallet)> = Lazy::new(||{
    
    let mut wallet = wallexerr::misc::Wallet::new_ed25519();
    // creating the secure cell config structure, the aes256_config is a mutable pointer
    // which itself is mutable means we can change the content of the pointer later with 
    // a new binding this would change the underlying data as well as the address inside 
    // the pointer which the pointer is pointing to.
    let mut aes256_config = &mut wallexerr::misc::SecureCellConfig::default();
    // following secret key is the sha3 keccak256 hash of random chars
    aes256_config.secret_key = {
        hex::encode(
            wallet.self_generate_keccak256_hash_from(
                &gen_random_chars(64)
            )
        )
    };

    // clonning neccessary things before going into the spawn scope
    let cloned_aes256_config = aes256_config.clone();
    let cloned_wallet = wallet.clone();

    // save the file config as an async task in the background 
    // using tokio spawn 
    tokio::spawn(async move{
        // save the config so we can share it between clients
        let mut file = tokio::fs::File::create("tcp_wallet_secure_cell_config.json").await.unwrap();
        file.write_all(&serde_json::to_string_pretty(&TcpSharedSetup{
            wallet: cloned_wallet.clone(), 
            secure_cell_config: cloned_aes256_config.clone()
        }).unwrap().as_bytes()).await.unwrap();
    });

    (aes256_config.to_owned(), wallet)
}); 