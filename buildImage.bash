#!/bin/bash

SERVICE_NAME="pdf-rendering-srv"

DOCKER_BUILDKIT=1 docker build \
  --tag restorecommerce/$SERVICE_NAME \
  -f ./Dockerfile \
  --cache-from restorecommerce/$SERVICE_NAME \
  .
