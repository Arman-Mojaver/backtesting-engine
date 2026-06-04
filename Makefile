SHELL = /bin/bash

.PHONY: help bash logs up in down ps build fmt push pull

.DEFAULT_GOAL := help


help: ## Show this help message
	@echo "Available targets:"
	@grep -E '(^[a-zA-Z_-]+:.*?##|^# [A-Za-z])' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS = ":.*?## "}; \
	/^# / {printf "\n%s\n", substr($$0, 3); next} \
	{printf "  %-20s %s\n", $$1, $$2}'


# Dev tools
bash:  ## Start a bash shell in container
	docker compose -f docker-compose.yaml run --rm -it -v ~/.bash_history:/root/.bash_history -v $(PWD):/app engine bash

logs:  ## Display docker-compose logs
	docker compose logs --tail=100 -f


# Docker commands
up:  ## Start containers
	docker compose -f docker-compose.yaml up -d

in:  ## Start a bash shell in started container
	docker compose -f docker-compose.yaml exec -it engine /bin/bash

down:  ## Remove containers
	docker compose -f docker-compose.yaml down

ps:  ## Display containers
	docker compose ps


# Docker image commands
build:  ## Build image
	docker image build -t armanmojaver/backtesting-engine:latest .

push:  ## Push images
	cat .docker_password | docker login --username armanmojaver --password-stdin
	docker push armanmojaver/backtesting-engine:latest

pull:  ## Pull images
	cat .docker_password | docker login --username armanmojaver --password-stdin
	docker push armanmojaver/backtesting-engine:latest


fmt:  ## Run rust formatter
	cd $(PWD) && cargo fmt
