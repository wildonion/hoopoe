
use crate::*;
use crate::interfaces::crypter::Crypter;
use wallexerr::misc::Wallet;
use wallexerr::misc::SecureCellConfig;

impl Crypter for String{
    fn decrypt(&self, secure_cell_config: &mut SecureCellConfig){
        let decrypted = Wallet::secure_cell_decrypt(secure_cell_config).unwrap();
        secure_cell_config.data = decrypted;
    }
    fn encrypt(&self, secure_cell_config: &mut SecureCellConfig){
        let encrypted = Wallet::secure_cell_encrypt(secure_cell_config).unwrap();
        secure_cell_config.data = encrypted;
    }
}

pub fn test(){

    // define your data you want to encrypt it 
    let name = String::from("wildonion");

    // build the secure cell config, make sure the secret_key and passphrase are not empty
    let mut secure_cell_config = SecureCellConfig{
        secret_key: String::from("1secret"),
        passphrase: String::from("2pass"),
        data: name.as_bytes().to_vec(),
    };
    
    // simply encrypt the name by calling the encrypt method on it
    name.encrypt(&mut secure_cell_config);
    println!("after encryption: {:?}", secure_cell_config.data);

    // simply decrypt the name by calling the decrypt method on it
    name.decrypt(&mut secure_cell_config);
    println!("after decryption: {:?}", secure_cell_config.data);

}