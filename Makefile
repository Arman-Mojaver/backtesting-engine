SHELL = /bin/bash

.PHONY: help bash logs up in down build buildr push pull pushr pullr upr inr downr fmt test lint deny machete ci

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


# Dev image commands
up:  ## Start containers
	docker compose -f docker-compose.yaml up -d

in:  ## Start a bash shell in started container
	docker compose -f docker-compose.yaml exec -it engine /bin/bash

down:  ## Remove containers
	docker compose -f docker-compose.yaml down

build:  ## Build dev image
	docker image build --target dev -t armanmojaver/backtesting-engine:dev .

push:  ## Push dev image
	cat .docker_password | docker login --username armanmojaver --password-stdin
	docker push armanmojaver/backtesting-engine:dev

pull:  ## Pull dev image
	cat .docker_password | docker login --username armanmojaver --password-stdin
	docker pull armanmojaver/backtesting-engine:dev


# Runtime image commands
upr:   ## Start runtime container
	docker compose -f docker-compose.runtime.yaml up -d

inr:   ## Start a bash shell in started runtime container
	docker compose -f docker-compose.runtime.yaml exec -it engine /bin/bash

downr: ## Remove runtime containers
	docker compose -f docker-compose.runtime.yaml down

buildr:  ## Build runtime image
	docker image build --target runtime -t armanmojaver/backtesting-engine:latest .

pushr:  ## Push runtime image
	cat .docker_password | docker login --username armanmojaver --password-stdin
	docker push armanmojaver/backtesting-engine:latest

pullr:  ## Pull runtime image
	cat .docker_password | docker login --username armanmojaver --password-stdin
	docker pull armanmojaver/backtesting-engine:latest


fmt:  ## Run rust formatter
	cargo fmt

test: ## Run tests
	cargo nextest run --locked

lint: ## Check formatting, run clippy and build docs (same as CI)
	cargo fmt --check
	cargo clippy --locked --all-targets --all-features -- -D warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked

deny: ## Run supply-chain checks with cargo-deny
	cargo deny check

machete: ## Check for unused dependencies with cargo-machete
	cargo machete

ci: lint test deny machete ## Run the CI gates locally (same as CI)
