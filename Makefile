# (c) Copyright 2019-2020 OLX
DOCKER_REGISTRY ?= docker.pkg.github.com
DOCKER_ORG ?= olxgroup-oss
RUNTIME_IMAGE ?= $(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/dali
BUILD_NUMBER ?= $(shell ./scripts/get-current-version.sh)

ifeq ($(REVISION),)
	VERSION_TAG ?= $(BUILD_NUMBER)
else
	VERSION_TAG ?= $(REVISION)-preview
endif

SUDO := $(shell docker info >/dev/null 2>&1 || echo sudo)

.PHONY: up test build-base-image build-image

test:
	./scripts/dali-test-up.sh

run:
	cargo run

docker-build:
	docker build -t ${RUNTIME_IMAGE}:${VERSION_TAG} .

docker-publish: docker-build
	docker push ${RUNTIME_IMAGE}:${VERSION_TAG}