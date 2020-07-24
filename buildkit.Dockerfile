# syntax = docker/dockerfile:experimental

FROM alpeware/chrome-headless-trunk:rev-786673

RUN echo "ttf-mscorefonts-installer msttcorefonts/accepted-mscorefonts-eula select true" | debconf-set-selections \
  && apt update -y \
  && apt install -y fontconfig fonts-dejavu ttf-mscorefonts-installer curl gnupg git \
  && curl -sL https://deb.nodesource.com/setup_14.x | bash - \
  && apt install -y nodejs \
  && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1000 node \
  && useradd --uid 1000 --gid node --shell /bin/bash --create-home node

USER node

ENV HOME=/home/node
ARG APP_HOME=/home/node/srv
WORKDIR $APP_HOME

RUN git clone --depth 1 https://github.com/alvarcarto/url-to-pdf-api . \
  && npm install --only=production

HEALTHCHECK CMD sh -c 'if [ $(curl -I -s -o /dev/null -w "%{http_code}" http://localhost:9000/) = "403" ]; then exit 0; else exit 1; fi'

EXPOSE 9000
CMD [ "node", "." ]
