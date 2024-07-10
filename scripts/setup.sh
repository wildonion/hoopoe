#!/bin/bash
cd ..
sudo apt update -y && sudo apt upgrade && sudo apt install -y libpq-dev pkg-config build-essential libudev-dev libssl-dev librust-openssl-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
# cargo install diesel_cli --no-default-features --features postgres
cargo install sqlx-cli --no-default-features --features native-tls,postgres
cargo install sea-orm-cli

sudo apt install -y protobuf-compiler libssl-dev zlib1g-dev
wget http://archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2_amd64.deb
sudo dpkg -i libssl1.1_1.1.1f-1ubuntu2_amd64.deb
sudo apt install -y snapd && sudo snap install core; sudo snap refresh core
sudo snap install --classic certbot && sudo ln -s /snap/bin/certbot /usr/bin/certbot
cargo install sqlant && sudo apt install -y openjdk-11-jdk && sudo apt install -y graphviz

# use sqlant and plantuml to extract erd from any db
sqlant postgresql://postgres:$PASSWORD@localhost/hoopoe > $(pwd)/infra/hoopoe.uml
java -jar $(pwd)/infra/plantuml.jar $(pwd)/infra/hoopoe.uml


git clone https://github.com/cossacklabs/themis.git
cd themis
make
sudo make install