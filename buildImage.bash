git clone https://github.com/alvarcarto/url-to-pdf-api pdf-rendering-srv
cp Dockerfile pdf-rendering-srv/
cd pdf-rendering-srv
docker build --no-cache -t restorecommerce/pdf-rendering-srv .