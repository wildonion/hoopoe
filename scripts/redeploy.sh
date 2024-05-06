#!/bin/bash

cd ..

echo \t"------------------------------------------------------"\n
echo \t"  --[are you completed with .env.prod screct vars?]--"
echo \t"------------------------------------------------------"\n
read ENVCOMPLETED

if [[ $ENVCOMPLETED == "Y" || $ENVCOMPLETED == "y" ]]; then
    sudo rm .env && sudo mv .env.prod .env
    echo "[?] Enter Machine Id: "
    read MACHINE_ID
    echo MACHINE_ID=$MACHINE_ID >> .env
    echo "[?] Enter Node Id: "
    read NODE_ID
    echo NODE_ID=$NODE_ID >> .env

    echo "[?] Redis/Timescaledb Password: "
    read PSWD

    echo "[?] App Name: "
    read APPNAME

    sudo chmod 666 /var/run/docker.sock
    export SERVER_IP=$(hostname -I | awk '{print $1}')
    export PASSWORD=$PSWD
    export APP_NAME=$APPNAME

    echo "[?] Wanna Redeploy Infrastructure? "
    read REDPLOY_INFRASTRUCTURE

    if [[ $REDPLOY_INFRASTRUCTURE == "Y" || $REDPLOY_INFRASTRUCTURE == "y" ]]; then

        echo "> Redeploying Infrastructure Pipelines Only"
        echo "☕ Okay, sit back and drink your coffee :)"

        sudo docker stop grafana && sudo docker rm -f grafana
        sudo docker stop postgres && sudo docker rm -f postgres
        sudo docker stop adminer && sudo docker rm -f adminer
        sudo docker stop nginx && sudo docker rm -f nginx
        sudo docker stop redis && sudo docker rm -f redis
        sudo docker stop portainer && sudo docker rm -f portainer
        sudo docker stop timescaledb && sudo docker rm -f timescaledb
        sudo docker stop rabbitmq && sudo docker rm -f rabbitmq
        sudo docker stop dbeaver && sudo docker rm -f dbeaver

        docker run -d --restart unless-stopped --name dbeaver --network hoopoe -ti -p 8080:8978 -v $(pwd)/infra/data/opt/cloudbeaver/workspace dbeaver/cloudbeaver:latest

        # host is a network is used to serve other containers if you use the 
        # host network mode for a container, that container’s network stack is 
        # not isolated from the Docker host (the container shares the host's networking namespace), 
        # and the container does not get its own IP-address allocated. 
        # For instance, if you run a container which binds to port 80 and 
        # you use host networking, the container's application is available 
        # on port 80 on the host's IP address thus we use the host network 
        # so we can access containers on 127.0.0.1 with their exposed ports 
        # inside the nginx conf without their dns name or ip address. 
        sudo docker build -t --no-cache nginx -f $(pwd)/infra/docker/nginx/Dockerfile .
        sudo docker run -d -it -p 80:80 -p 443:443 -v $(pwd)/infra/data/nginx/confs/:/etc/nginx -v $(pwd)/infra/data/nginx/wwws/:/usr/share/nginx/ -v $(pwd)/assets/:/etc/nginx/assets -v $(pwd)/infra/logs/:/etc/nginx/logs --name nginx --network host nginx

        sudo docker run -d --network hoopoe -p 7050:3000 --name=grafana --user "$(id -u)" --volume $(pwd)/infra/data:/var/lib/grafana grafana/grafana # admin | admin

        sudo docker volume create portainer_data
        sudo docker run -d \
            --network hoopoe \
            -p 8000:8000 \
            -p 9443:9443 \
            --name portainer \
            --restart=always \
            --volume /var/run/docker.sock:/var/run/docker.sock \
            --volume $(pwd)/infra/data/portainer_data:/data \
            portainer/portainer-ce:latest

        sudo docker run -d \
            -h redis \
            -e REDIS_PASSWORD=$PASSWORD \
            -v $(pwd)/infra/data/redis/:/data \
            -p 6379:6379 \
            --name redis \
            --network hoopoe \
            --restart always \
            redis:latest /bin/sh -c 'redis-server --appendonly yes --requirepass ${REDIS_PASSWORD}'

        sudo docker run -d --network hoopoe --name timescaledb -p 5432:5432 \
            -v $(pwd)/infra/data/timesaceldb/:/home/postgres/pgdata/data \
            -e POSTGRES_PASSWORD=$PASSWORD -e POSTGRES_USER=postgres \
            -e PGDATA=/var/lib/postgresql/data/pgdata timescale/timescaledb-ha:pg16

        sudo docker run -d --network hoopoe --hostname rabbitmq -p 5672:5672 -p 15672:15672 --name rabbitmq -e RABBITMQ_DEFAULT_USER=hoopoe -e RABBITMQ_DEFAULT_PASS=$PASSWORD rabbitmq:3-management

        sudo docker run -d --link timescaledb --network hoopoe --name adminer -p 7543:8080 adminer
        sqlx database create
        sqlant postgresql://postgres:$PASSWORD@localhost/hoopoe > $(pwd)/infra/hoopoe.uml
        java -jar $(pwd)/infra/plantuml.jar $(pwd)/infra/hoopoe.uml

        jobs="jobs/*"
        for f in $jobs
        do
            crontab $f
        done  
        crontab -u root -l 

        sudo docker ps -a && sudo docker compose ps -a && sudo docker images
    
    else
        echo "> Redeploying Rust Services Only"\n
        echo "☕ Okay, sit back and drink your coffee :)"

        sudo rm -r $(pwd)/target

        ANY_HOOPOE_HTTP_CONTAINER_ID=$(docker container ls  | grep 'hoopoe-http' | awk '{print $1}')
        sudo docker stop $ANY_HOOPOE_HTTP_CONTAINER_ID && sudo docker rm -f $ANY_HOOPOE_HTTP_CONTAINER_ID

        ANY_HOOPOE_GRPC_CONTAINER_ID=$(docker container ls  | grep 'hoopoe-grpc' | awk '{print $1}')
        sudo docker stop $ANY_HOOPOE_GRPC_CONTAINER_ID && sudo docker rm -f $ANY_HOOPOE_GRPC_CONTAINER_ID

        ANY_HOOPOE_TCP_CONTAINER_ID=$(docker container ls  | grep 'hoopoe-tcp' | awk '{print $1}')
        sudo docker stop $ANY_HOOPOE_TCP_CONTAINER_ID && sudo docker rm -f $ANY_HOOPOE_TCP_CONTAINER_ID

        TIMESTAMP=$(date +%s)

        echo \t"--[make sure you 1. setup a subdomain for wehbook endpoint in DNS records 2. register the webhook endpoint in your stripe dashabord 3. setup the nginx config file for the endpoint with SSL points to this VPS]--"        
       
        sudo docker build -t hoopoe-http-$TIMESTAMP -f $(pwd)/infra/docker/hoopoe/Dockerfile . --no-cache
        sudo docker run -d --restart unless-stopped --link timescaledb --network hoopoe --name hoopoe-http-$TIMESTAMP -p 2346:2344 -v $(pwd)/assets/:/app/assets -v $(pwd)/infra/logs/:/app/logs hoopoe-http-$TIMESTAMP

        sudo docker build -t hoopoe-grpc-$TIMESTAMP -f $(pwd)/infra/docker/hoopoe/Dockerfile . --no-cache
        sudo docker run -d --restart unless-stopped --link timescaledb --network hoopoe --name hoopoe-grpc-$TIMESTAMP -p 2257:2256 -v $(pwd)/assets/:/app/assets -v $(pwd)/infra/logs/:/app/logs hoopoe-grpc-$TIMESTAMP

        sudo docker build -t hoopoe-tcp-$TIMESTAMP -f $(pwd)/infra/docker/hoopoe/Dockerfile . --no-cache
        sudo docker run -d --restart unless-stopped --link timescaledb --network hoopoe --name hoopoe-tcp-$TIMESTAMP -p 2735:2734 -v $(pwd)/assets/:/app/assets -v $(pwd)/infra/logs/:/app/logs hoopoe-tcp-$TIMESTAMP

        echo \n"you can run ./renew.sh to bring the containers into a life!"

    fi

    sudo docker system prune --all

else
    echo \t"run me again once you get done with filling .env.prod vars"
fi