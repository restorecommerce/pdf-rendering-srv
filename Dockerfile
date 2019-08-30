FROM alpeware/chrome-headless-trunk:rev-691036
RUN apt-get update -y && apt-get install -yq fontconfig fonts-dejavu curl gnupg && rm -rf /var/lib/apt/lists/*
# install node
RUN curl -sL https://deb.nodesource.com/setup_12.x  | bash -
RUN apt-get -y install nodejs
RUN mkdir -p /home/app
# Create an app user so our application doesn't run as root.
RUN groupadd -r app &&\
    useradd -r -g app -d /home/app -s /sbin/nologin -c "Docker image user" app
# Create app directory
ENV HOME=/home/app
ENV APP_HOME=/home/app/pdf-rendering-srv
## SETTING UP THE APP ##
RUN mkdir $APP_HOME
WORKDIR $APP_HOME
ADD . $APP_HOME
# Chown all the files to the app user.
RUN chown -R app:app $HOME
RUN cd $APP_HOME
RUN pwd
# Change to the app user.
USER app
RUN npm install

EXPOSE 9000
CMD [ "npm", "start" ]
