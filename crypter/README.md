

a crypto provider contains utility cryptography methods empowering to verify the digital signature in communicating between two services.

## ⛩️ themis setup

> refer to https://docs.cossacklabs.com/themis/installation/installation-from-packages/ if you don't want to build themis from source, clone the repo then install the followings:

```bash
wget http://archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2_amd64.deb
sudo dpkg -i libssl1.1_1.1.1f-1ubuntu2_amd64.deb
sudo apt update -y && sudo apt upgrade && sudo apt install -y libpq-dev pkg-config build-essential libudev-dev libssl-dev librust-openssl-dev
git clone https://github.com/cossacklabs/themis.git
cd themis
make
sudo make install
# install themis on MacOS M1
brew install libthemis
```

`themis-wasm` contains an example of using a themis `secp256r1` keys in js.
