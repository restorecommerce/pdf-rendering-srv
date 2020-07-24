FROM alpeware/chrome-headless-trunk:rev-786673
# install dependencies
RUN echo "ttf-mscorefonts-installer msttcorefonts/accepted-mscorefonts-eula select true" | debconf-set-selections
RUN apt-get update -y &&\
 DEBIAN_FRONTEND=noninteractive apt-get install -yq\
 fontconfig fonts-dejavu ttf-mscorefonts-installer curl gnupg git &&\
 rm -rf /var/lib/apt/lists/*
# install node
RUN curl -sL https://deb.nodesource.com/setup_12.x  | bash -
RUN apt-get -y install nodejs
# Create an app user so our application doesn't run as root.
RUN groupadd -r app &&\
    useradd -r -g app -d /home/app -s /sbin/nologin -c "Docker image user" app
# Create app directory
ENV HOME=/home/app
ENV APP_HOME=/home/app/pdf-rendering-srv
## SETTING UP THE APP ##
RUN mkdir $HOME
WORKDIR $HOME
# Chown all the files to the app user.
RUN chown -R app:app $HOME
RUN pwd
# Change to the app user.
USER app
RUN git clone https://github.com/alvarcarto/url-to-pdf-api pdf-rendering-srv
WORKDIR $APP_HOME
RUN npm install --only=prod

HEALTHCHECK CMD curl -I http://localhost:9000/

EXPOSE 9000
CMD [ "node", "." ]
