# (c) Copyright 2019-2024 OLX
DOCKER_REGISTRY ?= ghcr.io
DOCKER_ORG ?= olxgroup-oss
DALI_IMAGE_NAME ?= $(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/dali
BUILD_NUMBER ?= $(shell ./scripts/get-current-version.sh)
RUSTFLAGS ?= "-C target-feature=-crt-static $(pkg-config vips --libs)"

ifeq ($(REVISION),)
	VERSION_TAG ?= $(BUILD_NUMBER)
else
	VERSION_TAG ?= $(REVISION)-preview
endif

SUDO := $(shell docker info >/dev/null 2>&1 || echo sudo)

.PHONY: test run docker-build docker-publish start-local-s3

test:
	@ ./scripts/dali-tests-runner.sh

run:
	@ cargo run

docker-build:
	@ docker build -t ${DALI_IMAGE_NAME}:${VERSION_TAG} .

docker-publish: docker-build
	@ docker push ${DALI_IMAGE_NAME}:${VERSION_TAG}

dev-env.start:
	@ KES_PORT=2967 MINIO_CONSOLE_PORT=2968 MINIO_API_PORT=2969 docker-compose -f ./dev-env-resources/docker-compose.yaml up -d

dev-env.stop:
	@ KES_PORT=2967 MINIO_CONSOLE_PORT=2968 MINIO_API_PORT=2969 docker-compose -f ./dev-env-resources/docker-compose.yaml down
