# (c) Copyright 2019-2023 OLX

DOCKER_REGISTRY ?= ghcr.io
DOCKER_ORG ?= olxgroup-oss
PROJECT_NAME=dali
DALI_IMAGE_NAME ?= $(DOCKER_REGISTRY)/$(DOCKER_ORG)/$(PROJECT_NAME)/$(PROJECT_NAME)
DALI_IMAGE_TAG ?= $(shell git branch --show-current)

SUDO := $(shell docker info >/dev/null 2>&1 || echo sudo)

.PHONY: up down test run pull

up: pull
	RUNTIME_IMAGE=$(DALI_IMAGE_NAME) RUNTIME_TAG=$(DALI_IMAGE_TAG) $(SUDO) docker compose -p $(PROJECT_NAME) \
			-f docker-compose.yaml \
			up --remove-orphans --exit-code-from dali

down:
	RUNTIME_IMAGE=$(DALI_IMAGE_NAME) RUNTIME_TAG=$(DALI_IMAGE_TAG) $(SUDO) docker compose -p $(PROJECT_NAME) \
			-f docker-compose.yaml \
			down --remove-orphans --volumes

test:
	@ ./scripts/dali-tests-runner.sh

run:
	@ cargo run

pull:
	@ $(SUDO) docker pull ${DALI_IMAGE_NAME}:${DALI_IMAGE_TAG}

