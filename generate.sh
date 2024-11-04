#!/usr/bin/env bash

# For pulling updates on MacOs with Apple silicon ¯\_(ツ)_/¯

set -ex

TARGETARCH=${TARGETARCH:-amd64}

docker build -t k8sfg --platform linux/${TARGETARCH} .
CONTAINER=$(docker run --platform linux/${TARGETARCH} -d k8sfg false)
docker cp ${CONTAINER}:/RESULTS.md .

docker rm ${CONTAINER}
