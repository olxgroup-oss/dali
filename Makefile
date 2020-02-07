# (c) Copyright 2019-2020 OLX
DOCKER_REGISTRY ?= docker.pkg.github.com
DOCKER_ORG ?= olxgroup-oss

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
	DOCKER_REGISTRY=$(DOCKER_REGISTRY) DOCKER_ORG=$(DOCKER_ORG) $(SUDO) docker-compose -p $(PROJECT_NAME) \
		-f docker-compose.yaml \
		up --remove-orphans --exit-code-from dali

test:
	DOCKER_REGISTRY=$(DOCKER_REGISTRY) DOCKER_ORG=$(DOCKER_ORG) $(SUDO) docker-compose -p $(PROJECT_NAME) \
		-f docker-compose.yaml -f docker-compose.tests.yaml \
		up --no-recreate --remove-orphans --exit-code-from cargo

build-base-image:
	$(SUDO) docker build -f Dockerfile.base -t "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-rust-image:$(VERSION_TAG)" .
	$(SUDO) docker build -f Dockerfile.vips -t "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-dali:$(VERSION_TAG)" .
	$(SUDO) docker tag "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-rust-image:$(VERSION_TAG)" "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-rust-image:latest"
	$(SUDO) docker push "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-rust-image:$(VERSION_TAG)"
	$(SUDO) docker push "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-rust-image:latest"
	$(SUDO) docker tag "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-dali:$(VERSION_TAG)" "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-dali:latest"
	$(SUDO) docker push "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-dali:$(VERSION_TAG)"
	$(SUDO) docker push "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/base-dali:latest"

build-image:
	$(SUDO) docker build -t "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/dali:$(VERSION_TAG)" .
	$(SUDO) docker tag "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/dali:$(VERSION_TAG)" "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/dali:latest"
	$(SUDO) docker push "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/dali:$(VERSION_TAG)"
	$(SUDO) docker push "$(DOCKER_REGISTRY)/$(DOCKER_ORG)/dali/dali:latest"
