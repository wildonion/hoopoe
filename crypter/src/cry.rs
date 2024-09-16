


use rand::SeedableRng;
use wallexerr::misc::*;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use rand::Rng;
use rand::prelude::SliceRandom;


pub fn gen_random_chars(size: u32) -> String{
    let mut rng = rand::thread_rng();
    (0..size).map(|_|{
        /* converint the generated random ascii to char */
        char::from_u32(rng.gen_range(33..126)).unwrap() // generating a char from the random output of type u32 using from_u32() method
    }).collect()
}

pub fn gen_random_chars_no_special(size: u32) -> String{
    let mut rng = rand::thread_rng();
    (0..size).map(|_|{
        /* converint the generated random ascii to char */
        let mut c = char::from_u32(rng.gen_range(48..126)).unwrap(); // generating a char from the random output of type u32 using from_u32() method
        if c == '`'{
            c = char::from_u32(64).unwrap();
        }
        c
    }).collect()
}

pub fn get_random_elem<T: Default + Clone + Send + Sync + Sized>(vec: Vec<T>) -> T{
    // making a high entropy seed to create the rng
    let random_chars = gen_random_chars(10);
    let high_entropy_seed = wallexerr::misc::Wallet::generate_sha256_from(&random_chars); // generating a 256 bits hash
    let mut crypto_seeded_rng = rand_chacha::ChaCha20Rng::from_seed(high_entropy_seed);

    // to_owned() convert the pointer to the owned type by cloning it
    // since clone returns the actual type it's kinda like dereferencing 
    // it but costs a bit more at runtime, hence the generic must impls
    // the Clone trait! tha't why i've bound the T to Clone
    let random_elem = vec.choose(&mut crypto_seeded_rng).unwrap().to_owned();
    random_elem 
}

/*  -------------------------------------------------------------------------
    how 2 make a secure connection between clients and server?
        0 - generate ed25519 wallet and AES256 secure cell config and share between trusted parties
        1 - encrypt data using AES256
        2 - sign AES256 hash of data using pvkey
        3 - send the AES256 hash of signatre to client
        4 - use the secure cell config to decrypt the signature
        5 - use signature, pubkey and AES256 hash of data to verify the signature
        6 - if the sig is valid then the connection between parties is now secured
        7 - they can now transfer data between each other
*/


pub mod wannacry{

    pub use super::*;

    /* 
        let path_on_server = "onion.png";
        let dec_path_on_server = "onion.dec.png";

        let (encrypted_buffer, mut secure_cell) = crypter::cry::wannacry::encrypt_file(path_on_server).await;
        let decrypted_buffer = crypter::cry::wannacry::decrypt_file(dec_path_on_server, &mut secure_cell).await;
        
        let hex_file_sig = hex::encode(&encrypted_buffer);
        let base58_file_sig = encrypted_buffer.to_base58();
    
        ====>>>====>>>====>>>====>>>====>>>====>>>====>>>

        tools: RSA ed25519 ECC curve with aes256 hash in wallexerr, openssl and ring for RSA + KDF like sha256 and keccak256
        ransomewere, steganography and files encryption to generate unique assets by encrypting using 
        aes256 cbc with pbkdf2 + sha384 + salt then showing key and iv like:
            
            openssl dhparam -out infra/docker/nginx/ssl-dhparams.pem 4096
            
            openssl aes-256-cbc -a -salt -pbkdf2 -in img.png -out img.png.enc -p
            openssl aes-256-cbc -d -a -pbkdf2 -in img.png.enc -out img.png.new -p 

            openssl aes-256-cbc -md sha384 -in secret.txt -out img.png.enc -p
            openssl aes-256-cbc -d -nopad -md sha384 -in img.png.enc -p

            gpg --output encrypted.data --symmetric --cipher-algo AES256 un_encrypted.data
            gpg --output un_encrypted.data --decrypt encrypted.data
    */

    pub async fn encrypt_file(fpath: &str) -> (Vec<u8>, SecureCellConfig){

        let file = tokio::fs::File::open(fpath).await;
    
        let mut buffer = vec![]; // file content in form of utf8
        file.unwrap().read_to_end(&mut buffer).await; // await on it to fill the buffer    
        let mut default_secure_cell_config = &mut SecureCellConfig::default();

        default_secure_cell_config.secret_key = {
            hex::encode(
                wallexerr::misc::Wallet::generate_keccak256_hash_from(
                    &{
                        // high entropy seed with god level security 
                        // even the server doesn't know the fucking secret key!
                        let random_chars = gen_random_chars(64);
                        let hash_of_random_chars = wallexerr::misc::Wallet::generate_sha256_from(&random_chars); // generating a 256 bits hash
                        let mut crypto_seeded_rng = rand_chacha::ChaCha20Rng::from_seed(hash_of_random_chars); // generate rng with the has of random chars as the seed
                        let seed = crypto_seeded_rng.get_seed();
                        hex::encode(seed)
                    }
                )
            )
        };
    
        // store the config into a json file so we can use later to decrypt the file
        let config_path = format!("{}.config.json", fpath);
        let config_file = tokio::fs::File::create(config_path).await;
        config_file.unwrap().write_all(
            &serde_json::to_string_pretty(&default_secure_cell_config).unwrap().as_bytes()
        ).await;
        default_secure_cell_config.data = buffer;
    
        let encrypted_data = wallexerr::misc::Wallet::secure_cell_encrypt(default_secure_cell_config).unwrap();
        default_secure_cell_config.data = encrypted_data.clone(); //*** important part */
    
        let enc_file_path = format!("{}.enc", fpath);
        let file = tokio::fs::File::create(&enc_file_path).await;
        file.unwrap().write_all(&encrypted_data).await;
    
        (encrypted_data, default_secure_cell_config.to_owned())
    
    }
    
    pub async fn decrypt_file(decpath: &str, default_secure_cell_config: &mut SecureCellConfig) -> Vec<u8>{
    
        let file = tokio::fs::File::create(decpath).await;
    
        let decrypted_data = wallexerr::misc::Wallet::secure_cell_decrypt(default_secure_cell_config).unwrap();
    
        file.unwrap().write_all(&decrypted_data).await;
    
        decrypted_data
    
    }
    
}


/** 
     ---------------------------------------------------------------------
    |          EdDSA Ed25519 WITH SYMMETRIC SIGNING SUING AES256
    |---------------------------------------------------------------------
    |
    | CURVE           -> ed25519
    | DATA ENCRYPTION -> SYMMETRIC WITH AES256 ALGORITHM
    | RETURN DATA     -> base58 Signature
    |

**/
pub mod eddsa_with_symmetric_signing{

    pub use super::*;

    pub fn ed25519_encrypt_and_sign_tcp_packet_with_aes256_secure_cell(mut wallet: Wallet, aes256_config: &mut SecureCellConfig) -> String{

        let raw_data_vec = aes256_config.clone().data;
        let raw_data_str = std::str::from_utf8(&raw_data_vec).unwrap();

        let edprvkey = wallet.ed25519_secret_key.clone().unwrap();
        let base58_sig = wallet.self_ed25519_secure_cell_sign(
            &edprvkey, 
            aes256_config
        );

        /* aes256_config.data now contains the aes256 hash of the raw data */
        let hash_of_data = aes256_config.clone().data;
        println!("secure cell aes256 encrypted data :::: {:?}", hex::encode(&hash_of_data));
        println!("signature :::: {:?}", base58_sig.clone());
        
        let is_verified = wallet.self_verify_ed25519_signature(
            &base58_sig.clone().unwrap(), 
            &hash_of_data, 
            &wallet.clone().ed25519_public_key.unwrap()
        );
        
        match is_verified{
            Ok(is_verified) => {

                aes256_config.data = hash_of_data.clone(); /* update data field with encrypted form of raw data */
                let dec = wallet.self_secure_cell_decrypt(aes256_config).unwrap();
                println!("aes256 decrypted data :::: {:?}", std::str::from_utf8(&dec));

                let deserialized_data = std::str::from_utf8(&dec).unwrap();
                if deserialized_data == raw_data_str{

                    wallet.self_save_to_json("ed25519-secure_cell");
                    println!("✅ got same data");
                    return base58_sig.unwrap();

                } else{

                    eprintln!("🔴 invalid data");
                    return String::from("");
                }

            },
            Err(e) => return String::from("")
        }

    }

    pub fn ed25519_decrypt_and_verify_tcp_packet_with_aes256_secure_cell(mut wallet: Wallet, base58_sig: &str, aes256_config: &mut SecureCellConfig) -> (bool, String){

        /* aes256_config.data now contains the aes256 hash of the raw data */
        let hash_of_data = aes256_config.clone().data;
        println!("secure cell aes256 encrypted data :::: {:?}", hex::encode(&hash_of_data));
        println!("signature :::: {:?}", base58_sig);
        
        let is_verified = wallet.self_verify_ed25519_signature(
            &base58_sig, 
            &hash_of_data, 
            &wallet.clone().ed25519_public_key.unwrap()
        );
        
        match is_verified{
            Ok(is_verified) => {

                aes256_config.data = hash_of_data.clone(); /* update data field with encrypted form of raw data */
                let dec = wallet.self_secure_cell_decrypt(aes256_config).unwrap();
                println!("aes256 decrypted data :::: {:?}", std::str::from_utf8(&dec)); // dec is not the vector of hex it's the raw vector of data so we can map it to str like this

                let deserialized_data = std::str::from_utf8(&dec).unwrap();
                wallet.self_save_to_json("ed25519-secure_cell");
                println!("✅ aes256 hash is valid");
                return (true, deserialized_data.to_string());

            },
            Err(e) => (false, String::from(""))
        }

    }

    //---------------------------------------------
    //----------- aes256ctr_poly1305aes -----------
    //---------------------------------------------
    pub fn ed25519_aes256_signing(data: &str, mut wallet: Wallet) -> String{

        // note that nonce must be unique per each user or a unique identity
        let mut default_aes256_config = &mut Aes256Config::default();
        default_aes256_config.secret_key = gen_random_chars(64); /*** ---- secret key must be 64 bytes or 512 bits */
        default_aes256_config.nonce = gen_random_chars(16); /*** ---- secret key must be 16 bytes or 128 bits */
        default_aes256_config.data = data.as_bytes().to_vec();

        let edprvkey = wallet.ed25519_secret_key.clone().unwrap();
        let base58_sig = wallet.self_ed25519_aes256_sign(
            &edprvkey, 
            default_aes256_config
        );
        
        /* default_aes256_config.data now contains the aes256 hash of the raw data */
        let hash_of_data = default_aes256_config.clone().data;
        println!("aes256 encrypted data :::: {:?}", hex::encode(&hash_of_data));
        println!("signature :::: {:?}", base58_sig.clone());
        
        let is_verified = wallet.self_verify_ed25519_signature(
            &base58_sig.clone().unwrap(), 
            &hash_of_data, 
            &wallet.clone().ed25519_public_key.unwrap()
        );
        
        match is_verified{
            Ok(is_verified) => {

                default_aes256_config.data = hash_of_data.clone(); /* update data field with encrypted form of raw data */
                let dec = wallet.self_generate_data_from_aes256(default_aes256_config);
                println!("aes256 decrypted data :::: {:?}", std::str::from_utf8(&dec));

                let deserialized_data = std::str::from_utf8(&dec).unwrap();
                if deserialized_data == data{

                    wallet.self_save_to_json("ed25519-aes256");
                    println!("✅ got same data");
                    return base58_sig.unwrap();

                } else{

                    eprintln!("🔴 invalid data");
                    return String::from("");
                }

            },
            Err(e) => return String::from("")
        }

    }

    //------------------------------
    //----------- themis -----------
    //------------------------------
    pub fn ed25519_secure_cell_signing(data: &str, mut wallet: Wallet) -> String{

        let mut default_secure_cell_config = &mut SecureCellConfig::default();
        // following secret key is the sha3 keccak256 hash of random chars
        default_secure_cell_config.secret_key = {
            hex::encode(
                wallet.self_generate_keccak256_hash_from(
                    &gen_random_chars(64)
                )
            )
        };
        default_secure_cell_config.data = data.as_bytes().to_vec();

        let edprvkey = wallet.ed25519_secret_key.clone().unwrap();
        let base58_sig = wallet.self_ed25519_secure_cell_sign(
            &edprvkey, 
            default_secure_cell_config
        );

        /* default_secure_cell_config.data now contains the aes256 hash of the raw data */
        let hash_of_data = default_secure_cell_config.clone().data;
        println!("secure cell aes256 encrypted data :::: {:?}", hex::encode(&hash_of_data));
        println!("signature :::: {:?}", base58_sig.clone());
        
        let is_verified = wallet.self_verify_ed25519_signature(
            &base58_sig.clone().unwrap(), 
            &hash_of_data, 
            &wallet.clone().ed25519_public_key.unwrap()
        );
        
        match is_verified{
            Ok(is_verified) => {

                default_secure_cell_config.data = hash_of_data.clone(); /* update data field with encrypted form of raw data */
                let dec = wallet.self_secure_cell_decrypt(default_secure_cell_config).unwrap();
                println!("aes256 decrypted data :::: {:?}", std::str::from_utf8(&dec));

                let deserialized_data = std::str::from_utf8(&dec).unwrap();
                if deserialized_data == data{

                    wallet.self_save_to_json("ed25519-secure_cell");
                    println!("✅ got same data");
                    return base58_sig.unwrap();

                } else{

                    eprintln!("🔴 invalid data");
                    return String::from("");
                }

            },
            Err(e) => return String::from("")
        }

    }

    
}

/** 
     ---------------------------------------------------------------------
    |          EdDSA Ed25519 USING KECCAK256 SIGNING
    |---------------------------------------------------------------------
    |
    | CURVE           -> ed25519
    | DATA ENCRYPTION -> KECCAK256
    | RETURN DATA     -> base58 Signature
    |

**/
pub mod eddsa_with_keccak256_signing{

    pub use super::*;
    
    pub fn ed25519_keccak256_signing(data: &str, mut wallet: Wallet) -> String{

        let edprvkey = wallet.ed25519_secret_key.clone().unwrap();
        let base58_sig = wallet.self_ed25519_sign(
            data,
            &edprvkey, 
        );

        let hash_of_data = wallet.self_generate_keccak256_hash_from(data);
        let is_verified = wallet.self_verify_ed25519_signature(
            &base58_sig.clone().unwrap(), 
            &hash_of_data, 
            &wallet.clone().ed25519_public_key.unwrap()
        );
        
        match is_verified{
            Ok(is_verified) => {

                wallet.self_save_to_json("ed25519-keccak256");
                return base58_sig.unwrap();

            },
            Err(e) => return String::from("")
        }

    }
    
}

pub mod zkp{

    // https://docs.circom.io/
    // https://docs.ton.org/develop/dapps/tutorials/simple-zk-on-ton
    // https://noir-lang.org/index.html
    // https://github.com/rust-cc/awesome-cryptography-rust#zero-knowledge-proofs
    // https://github.com/cossacklabs/themis/blob/master/docs/examples/rust/secure_compare.rs => themis zkp

    pub use super::*;

    pub struct ZkpError;
    pub struct Verifier;
    pub struct Prover;

    pub async fn auth() -> Result<(), ZkpError>{

        Ok(())
    }

    pub async fn secure_session(){

        // use to secure the communication between client and server
        // using zero knowledge proof
        // ...

    }
    
    fn get_zkp_comparator() -> themis::secure_comparator::SecureComparator{
        wallexerr::misc::Wallet::generate_zkp_comparator()
    }

}