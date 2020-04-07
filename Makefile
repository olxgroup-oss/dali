# (c) Copyright 2019-2020 OLX
DOCKER_REGISTRY ?= docker.pkg.github.com
DOCKER_ORG ?= olxgroup-oss
BUILD_IMAGE ?= $(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-rust-image
BASE_IMAGE ?= $(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-dali
RUNTIME_IMAGE ?= $(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/dali

PROJECT_NAME=dali

REVISION ?= $(shell git rev-parse --short HEAD)
ifeq ($(BUILD_NUMBER),)
	VERSION_TAG ?= $(REVISION)
else
	VERSION_TAG ?= $(REVISION)-$(BUILD_NUMBER)
endif

SUDO := $(shell docker info >/dev/null 2>&1 || echo sudo)

.PHONY: up test build-base-image build-image

up:
	RUNTIME_IMAGE=$(RUNTIME_IMAGE) $(SUDO) docker-compose -p $(PROJECT_NAME) \
		-f docker-compose.yaml \
		up --remove-orphans --exit-code-from dali

test:
	RUNTIME_IMAGE=$(RUNTIME_IMAGE) BUILD_IMAGE=$(BUILD_IMAGE) $(SUDO) docker-compose -p $(PROJECT_NAME) \
		-f docker-compose.yaml -f docker-compose.tests.yaml \
		up --no-recreate --remove-orphans --exit-code-from cargo

build-base-image:
	$(SUDO) docker build -f Dockerfile.base -t "$(BUILD_IMAGE):$(VERSION_TAG)" .
	$(SUDO) docker build -f Dockerfile.vips -t "$(BASE_IMAGE):$(VERSION_TAG)" .
	$(SUDO) docker tag "$(BUILD_IMAGE):$(VERSION_TAG)" "$(BUILD_IMAGE):latest"
	$(SUDO) docker push "$(BUILD_IMAGE):$(VERSION_TAG)"
	$(SUDO) docker push "$(BUILD_IMAGE):latest"
	$(SUDO) docker tag "$(BASE_IMAGE):$(VERSION_TAG)" "$(BASE_IMAGE):latest"
	$(SUDO) docker push "$(BASE_IMAGE):$(VERSION_TAG)"
	$(SUDO) docker push "$(BASE_IMAGE):latest"

build-image:
	$(SUDO) docker build --build-arg BASE_IMAGE=$(BASE_IMAGE) --build-arg BUILD_IMAGE=$(BUILD_IMAGE) -t "$(RUNTIME_IMAGE):$(VERSION_TAG)" .
	$(SUDO) docker tag "$(RUNTIME_IMAGE):$(VERSION_TAG)" "$(RUNTIME_IMAGE):latest"
	$(SUDO) docker push "$(RUNTIME_IMAGE):$(VERSION_TAG)"
	$(SUDO) docker push "$(RUNTIME_IMAGE):latest"
