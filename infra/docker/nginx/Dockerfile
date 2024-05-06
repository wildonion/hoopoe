# Use the official Nginx image as the base image
FROM nginx:latest

WORKDIR /etc/nginx

# copy the whole project into /etc/nginx
ADD . ./ 

# go to infra/docker/nginx folder
WORKDIR /etc/nginx/infra/docker/nginx

# Remove the default Nginx configuration file
RUN rm /etc/nginx/conf.d/default.conf

# accessing logs dir requires password, copy it into the /etc/nginx dir
COPY .htpasswd /etc/nginx/.htpasswd
# Copy the SSL certificate and private key to the container
#hoopoe.app:
COPY cert-hoopoe.app.pem /etc/nginx/cert-hoopoe.app.pem
COPY key-hoopoe.app.pem /etc/nginx/key-hoopoe.app.pem
#api.hoopoe.app:
COPY cert-api.hoopoe.app.pem /etc/nginx/cert-api.hoopoe.app.pem
COPY key-api.hoopoe.app.pem /etc/nginx/key-api.hoopoe.app.pem
#event.api.hoopoe.app:
COPY cert-event.api.hoopoe.app /etc/nginx/cert-event.api.hoopoe.app
COPY key-event.api.hoopoe.app /etc/nginx/key-event.api.hoopoe.app
#dev.api.hoopoe.app:
COPY cert-dev.api.hoopoe.app.pem /etc/nginx/cert-dev.api.hoopoe.app.pem
COPY key-dev.api.hoopoe.app.pem /etc/nginx/key-dev.api.hoopoe.app.pem
#adminer.hoopoe.app:
COPY cert-adminer.hoopoe.app.pem /etc/nginx/cert-adminer.hoopoe.app.pem
COPY key-adminer.hoopoe.app.pem /etc/nginx/key-adminer.hoopoe.app.pem
#portainer.hoopoe.app:
COPY cert-hoopoe.app.pem /etc/nginx/cert-hoopoe.app.pem
COPY key-hoopoe.app.pem /etc/nginx/key-hoopoe.app.pem
#grafana.hoopoe.app:
COPY cert-grafana.hoopoe.app.pem /etc/nginx/cert-grafana.hoopoe.app.pem
COPY key-grafana.hoopoe.app.pem /etc/nginx/key-grafana.hoopoe.app.pem
#rmq.hoopoe.app:
COPY cert-rmq.hoopoe.app.pem /etc/nginx/cert-rmq.hoopoe.app.pem
COPY key-rmq.hoopoe.app.pem /etc/nginx/key-rmq.hoopoe.app.pem
#dbeaver.hoopoe.app:
COPY cert-dbeaver.hoopoe.app.pem /etc/nginx/cert-dbeaver.hoopoe.app.pem
COPY key-dbeaver.hoopoe.app.pem /etc/nginx/key-dbeaver.hoopoe.app.pem
#docker.hoopoe.app:
COPY cert-docker.hoopoe.app.pem /etc/nginx/cert-docker.hoopoe.app.pem
COPY key-docker.hoopoe.app.pem /etc/nginx/key-docker.hoopoe.app.pem
#grpc.api.hoopoe.app:
COPY cert-grpc.api.hoopoe.app.pem /etc/nginx/cert-grpc.api.hoopoe.app.pem
COPY key-grpc.api.hoopoe.app.pem /etc/nginx/key-grpc.api.hoopoe.app.pem

COPY options-ssl-nginx.conf /etc/nginx/options-ssl-nginx.conf
COPY ssl-dhparams.pem /etc/nginx/ssl-dhparams.pem

# Expose port 80 for HTTP traffic and port 443 for HTTPS traffic
EXPOSE 80
EXPOSE 443

COPY nginx.conf /etc/nginx/nginx.conf
COPY hoopoe.app.conf /etc/nginx/hoopoe.app.conf
COPY api.hoopoe.app.conf /etc/nginx/api.hoopoe.app.conf
COPY event.api.hoopoe.app.conf /etc/nginx/event.api.hoopoe.app.conf
COPY adminer.hoopoe.app.conf /etc/nginx/adminer.hoopoe.app.conf
COPY grafana.hoopoe.app.conf /etc/nginx/grafana.hoopoe.app.conf
COPY rmq.hoopoe.app.conf /etc/nginx/rmq.hoopoe.app.conf
COPY dbeaver.hoopoe.app.conf /etc/nginx/dbeaver.hoopoe.app.conf
COPY docker.hoopoe.app.conf /etc/nginx/docker.hoopoe.app.conf
COPY grpc.api.hoopoe.app.conf /etc/nginx/grpc.api.hoopoe.app.conf

# Start Nginx in the foreground when the Docker container starts
CMD ["nginx", "-g", "daemon off;"]