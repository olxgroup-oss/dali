# (c) Copyright 2019-2023 OLX
DOCKER_REGISTRY ?= ghcr.io
DOCKER_ORG ?= olxgroup-oss
DALI_IMAGE_NAME ?= $(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/dali
BUILD_NUMBER ?= $(shell ./scripts/get-current-version.sh)

ifeq ($(REVISION),)
	VERSION_TAG ?= $(BUILD_NUMBER)
else
	VERSION_TAG ?= $(REVISION)-preview
endif

SUDO := $(shell docker info >/dev/null 2>&1 || echo sudo)

.PHONY: test run docker-build docker-publish

test:
	@ ./scripts/dali-tests-runner.sh

run:
	@ cargo run

docker-build:
	@ docker build -t ${DALI_IMAGE_NAME}:${VERSION_TAG} .

docker-publish: docker-build
	@ docker push ${DALI_IMAGE_NAME}:${VERSION_TAG}