#!/bin/bash


echo "-> wanna add (sub)domain? "
read is_new_domain
cd ..

if [[ $is_new_domain == "Y" || $is_new_domain == "y" ]]; then
    echo "-> enter domain? "
    read DOMAIN
    echo "creating new SSL certificate and key files for $DOMAIN using certbot,
    ensure that you have a (sub)domain points to this machine and it can accepts inbound connections 
    from the internet also make sure that necessary ports like 80 and 443 are opened"
    NGINX_CONTAINER_ID=$(docker container ls  | grep 'nginx' | awk '{print $1}')
    # stop nginx docker cause certbox needs the port 80 and 443
    sudo docker stop $NGINX_CONTAINER_ID && sudo certbot certonly --standalone -d $DOMAIN && sudo docker start $NGINX_CONTAINER_ID
    sudo cp /etc/letsencrypt/live/$DOMAIN/fullchain.pem $(pwd)/infra/cert/cert-$DOMAIN.pem && sudo cp /etc/letsencrypt/live/$DOMAIN/fullchain.pem $(pwd)/infra/docker/nginx/cert-$DOMAIN.pem
    sudo cp /etc/letsencrypt/live/$DOMAIN/privkey.pem $(pwd)/infra/cert/key-$DOMAIN.pem && sudo cp /etc/letsencrypt/live/$DOMAIN/privkey.pem $(pwd)/infra/docker/nginx/key-$DOMAIN.pem
    echo "okay now you can use $(pwd)/infra/docker/nginx/key-$DOMAIN.pem and $(pwd)/infra/docker/nginx/cert-$DOMAIN.pem in your nginx Dockerfile and $DOMAIN.conf"
else
    echo "🤔"
fi