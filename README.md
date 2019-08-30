# pdf-rendering-srv

Docker image for rendering PDFs from HTML. Good for receipts, invoices, or any content.
It uses the [chrome-headless-trunk](https://github.com/alpeware/chrome-headless-trunk)
as base image for Chrome and packages the [url-to-pdf-api](https://github.com/alvarcarto/url-to-pdf-api)
library to form a solid and easy-to-use microservice.

The image is available on [docker hub](https://cloud.docker.com/u/restorecommerce/r).

## Running

Building Image

```sh
./buildImage.bash
```

Start Service

```sh
# Start pdf-render-srv
docker run \
 --name restorecommerce_pdf_rendering_srv \
 -e ALLOW_HTTP=true \
 -p 9000:9000 \
 restorecommerce/pdf-rendering-srv
```

## Usage

### With URL

```sh
curl -o google.pdf -XGET localhost:9000/api/render?url=http://google.com
```

### With HTML File

```sh
curl -o index.pdf -XPOST -d@index.html -H 'content-type: text/html' 'localhost:9000/api/render'
```

### With PDF Options

The service supports sll [options](https://github.com/alvarcarto/url-to-pdf-api#get-apirender) provided by which can be passed as query
parameters:

```sh
curl -o index.pdf -XPOST -d@index.html -H 'content-type: text/html' 'localhost:9000/api/render?pdf.margin.top=100px&pdf.margin.bottom=100px&pdf.displayHeaderFooter=true&pdf.footerTemplate=%3Cdiv%20style=%22width:100%25%22%3E%3Cp%20style=%22padding-right:1cm;text-align:right;font-size:10px;%20%22%3Epage%20%3Cspan%20class=%22pageNumber%22%3E%3C/span%3E%20of%20%3Cspan%20class=%22totalPages%22%3E%3C/p%3E'
```

### With API Key

The optional `X-API-KEY` can be used for authentication and can be set using `API_TOKENS` environment variable when running the container.

```sh
curl -o google.pdf -H 'X-API-KEY: XXXXXX' -XGET localhost:9000/api/render?url=http://google.com
```

### Installing Extra Fonts

When text is rendered by a computer, sometimes characters are displayed as 口 a.k.a “tofu”. They are little boxes to indicate your device doesn’t have a font to display the text. To solve this additional fonts can be installed with in the container:

```sh
docker exec -it restorecommerce_pdf_rendering_srv /bin/bash
apt-get update
apt-get install -yq fonts-symbola      # 🙄🙄🙄
apt-get install -yq fonts-noto-cjk     # 囍, 언문, にほんご
apt-get install -yq fonts-ocr-b        # PASSPORT FONT
```
