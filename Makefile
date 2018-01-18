UNAME_S := $(shell uname -s)
HAS_DOCKER := $(shell command -v docker 2> /dev/null)
ifeq ($(UNAME_S),Darwin)
	forego := support/mac/bin/forego
else
	forego := support/linux/bin/forego
endif
ifneq (${IN_DOCKER},)
	IN_DOCKER := ${IN_DOCKER}
else ifeq ($(UNAME_S),Darwin)
	IN_DOCKER := true
endif

ifeq ($(IN_DOCKER),true)
	build_args := --build-arg HAB_BLDR_URL=$(HAB_BLDR_URL)
	run_args := -e HAB_BLDR_URL=$(HAB_BLDR_URL)
	run_args := $(run_args) -e HAB_ORIGIN=$(HAB_ORIGIN)
	ifneq (${http_proxy},)
		build_args := $(build_args) --build-arg http_proxy="${http_proxy}"
		run_args := $(run_args) -e http_proxy="${http_proxy}"
	endif
	ifneq (${https_proxy},)
		build_args := $(build_args) --build-arg https_proxy="${https_proxy}"
		run_args := $(run_args) -e https_proxy="${https_proxy}"
	endif

	dimage := habitat/devshell
	docker_cmd := env http_proxy= https_proxy= docker
	compose_cmd := env http_proxy= https_proxy= docker-compose
	common_run := $(compose_cmd) run --rm $(run_args)
	run := $(common_run) shell
	bldr_run := $(common_run) -p 9636:9636 -p 8080:8080 shell
	docs_run := $(common_run) -p 9633:9633 shell
	forego := support/linux/bin/forego
else
	run :=
	bldr_run :=
	docs_run :=
endif
ifneq ($(DOCKER_HOST),)
	docs_host := ${DOCKER_HOST}
else
	docs_host := 127.0.0.1
endif

BIN = hab hab-butterfly pkg-export-docker pkg-export-kubernetes sup airlock
LIB = butterfly builder-db builder-core builder-protocol common core builder-depot-client http-client net
SRV = builder-api builder-admin builder-depot builder-router builder-jobsrv builder-sessionsrv builder-originsrv builder-worker
ALL = $(BIN) $(LIB) $(SRV)
VERSION := $(shell cat VERSION)

.DEFAULT_GOAL := build-bin

build: build-bin build-lib build-srv ## builds all the components
build-all: build
.PHONY: build build-all

build-bin: $(addprefix build-,$(BIN)) ## builds the binary components
.PHONY: build-bin

build-lib: $(addprefix build-,$(LIB)) ## builds the library components
.PHONY: build-lib

build-srv: $(addprefix build-,$(SRV)) ## builds the service components
.PHONY: build-srv

unit: unit-bin unit-lib unit-srv ## executes all the components' unit test suites
unit-all: unit
.PHONY: unit unit-all

unit-bin: $(addprefix unit-,$(BIN)) ## executes the binary components' unit test suites
.PHONY: unit-bin

unit-lib: $(addprefix unit-,$(LIB)) ## executes the library components' unit test suites
.PHONY: unit-lib

unit-srv: $(addprefix unit-,$(SRV)) ## executes the service components' unit test suites
.PHONY: unit-srv

lint: lint-bin lint-lib lint-srv ## executs all components' lints
lint-all: lint
.PHONY: lint lint-all

lint-bin: $(addprefix lint-,$(BIN))
.PHONY: lint-bin

lint-lib: $(addprefix lint-,$(LIB))
.PHONY: lint-lib

lint-srv: $(addprefix lint-,$(SRV))
.PHONY: lint-srv

functional: functional-bin functional-lib functional-srv ## executes all the components' functional test suites
functional-all: functional
test: functional ## executes all components' test suites
.PHONY: functional functional-all test

functional-bin: $(addprefix unit-,$(BIN)) ## executes the binary components' unit functional suites
.PHONY: functional-bin

functional-lib: $(addprefix unit-,$(LIB)) ## executes the library components' unit functional suites
.PHONY: functional-lib

functional-srv: $(addprefix unit-,$(SRV)) ## executes the service components' unit functional suites
.PHONY: functional-srv

clean: clean-bin clean-lib clean-srv ## cleans all the components' clean test suites
clean-all: clean
.PHONY: clean clean-all

clean-bin: $(addprefix clean-,$(BIN)) ## cleans the binary components' project trees
.PHONY: clean-bin

clean-lib: $(addprefix clean-,$(LIB)) ## cleans the library components' project trees
.PHONY: clean-lib

clean-srv: $(addprefix clean-,$(SRV)) ## cleans the service components' project trees
.PHONY: clean-srv

fmt: fmt-bin fmt-lib fmt-srv ## formats all the components' codebases
fmt-all: fmt
.PHONY: fmt fmt-all

fmt-bin: $(addprefix fmt-,$(BIN)) ## formats the binary components' codebases
.PHONY: clean-bin

fmt-lib: $(addprefix fmt-,$(LIB)) ## formats the library components' codebases
.PHONY: clean-lib

fmt-srv: $(addprefix fmt-,$(SRV)) ## formats the service components' codebases
.PHONY: clean-srv

help:
	@perl -nle'print $& if m{^[a-zA-Z_-]+:.*?## .*$$}' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
.PHONY: help

shell: image ## launches a development shell
	$(run)
.PHONY: shell

bldr-shell: build-srv ## launches a development shell with forwarded ports but doesn't run anything
	$(bldr_run)
.PHONY: bldr-shell

bldr-run: build-srv ## launches a development shell running the API
	$(bldr_run) sh -c '$(forego) start -f support/Procfile -e support/bldr.env'
.PHONY: bldr-run

bldr-run-no-build: ## launches a development shell without rebuilding the world
	$(bldr_run) sh -c '$(forego) start -f support/Procfile -e support/bldr.env'
.PHONY: bldr-run-no-build

bldr-kill: ## kills every bldr process as well as hab processes
	$(bldr_run) sh -c ' \
	for name in api admin router jobsrv sessionsrv originsrv worker; do \
		sudo killall -9 bldr-$$name; \
	done; \
	sudo killall -9 hab-launch; \
	sudo killall -9 hab-sup; \
	sudo killall -9 lite-server; \
	sudo killall -9 postmaster; \
	SRC_NM_DIR=/src/components/builder-web/node_modules; \
	sudo mountpoint -q $$SRC_NM_DIR && sudo umount $$SRC_NM_DIR; \
	HOME_NM_DIR=$$HOME/.builder_web_node_modules; \
	sudo mountpoint -q $$HOME_NM_DIR && sudo umount $$HOME_NM_DIR; \
	'
.PHONY: bldr-kill

serve-docs: docs ## serves the project documentation from an HTTP server
	@echo "==> View the docs at:\n\n        http://`\
		echo $(docs_host) | sed -e 's|^tcp://||' -e 's|:[0-9]\{1,\}$$||'`:9633/\n\n"
	$(docs_run) sh -c 'set -e; cd ./target/doc; python -m SimpleHTTPServer 9633;'
.PHONY: serve-docs

ifeq ($(IN_DOCKER),true)
distclean: ## fully cleans up project tree and any associated Docker images and containers
	$(compose_cmd) stop
	$(compose_cmd) rm -f -v
	$(docker_cmd) rmi $(dimage) || true
	($(docker_cmd) images -q -f dangling=true | xargs $(docker_cmd) rmi -f) || true
.PHONY: distclean

image: ## create an image
ifeq ($(HAS_DOCKER),)
	$(error "Docker does not seem installed, please install docker first.")
endif
	@if [ -n "${force}" -o -n "${refresh}" -o -z "`$(docker_cmd) images -q $(dimage)`" ]; then \
		if [ -n "${force}" ]; then \
		  $(docker_cmd) build --no-cache $(build_args) -t $(dimage) .; \
		else \
		  $(docker_cmd) build $(build_args) -t $(dimage) .; \
		fi \
	fi
.PHONY: image
else
image: ## no-op
.PHONY: image

distclean: clean ## fully cleans up project tree
.PHONY: distclean
endif

bundle: image
	@$(run) sh -c 'AWS_ACCESS_KEY_ID=$(AWS_ACCESS_KEY_ID) AWS_KEYPAIR_NAME=$(AWS_KEYPAIR_NAME) \
		AWS_SECRET_ACCESS_KEY=$(AWS_SECRET_ACCESS_KEY) terraform/scripts/create_bootstrap_bundle.sh \
		$(VERSION)'

changelog: image
	@$(run) sh -c 'hab pkg install core/github_changelog_generator && \
		hab pkg binlink core/git git --force && \
		hab pkg binlink core/github_changelog_generator github_changelog_generator --force && \
		github_changelog_generator --future-release $(VERSION) --token $(GITHUB_TOKEN)' --max-issues=1000

docs: image ## build the docs
	$(run) sh -c 'set -ex; \
		cd components/sup && cargo doc && cd ../../ \
		rustdoc --crate-name habitat_sup README.md -o ./target/doc/habitat_sup; \
		docco -e .sh -o target/doc/habitat_sup/hab-plan-build components/plan-build/bin/hab-plan-build.sh; \
		cp -r images ./target/doc/habitat_sup; \
		echo "<meta http-equiv=refresh content=0;url=habitat_sup/index.html>" > target/doc/index.html;'

tag-release:
	sh -c 'git tag $(VERSION)'

publish-release:
	@$(run) sh -c 'hab pkg install core/jq-static && \
		hab pkg binlink core/jq-static jq --force && \
		./support/ci/promote_rc.sh'

define BUILD
build-$1: image ## builds the $1 component
	$(run) sh -c 'cd components/$1 && cargo build'
.PHONY: build-$1

endef
$(foreach component,$(ALL),$(eval $(call BUILD,$(component))))

define UNIT
unit-$1: image ## executes the $1 component's unit test suite
	$(run) sh -c 'cd components/$1 && cargo test'
.PHONY: unit-$1
endef
$(foreach component,$(ALL),$(eval $(call UNIT,$(component))))

# Here we just add a dependency on the hab-launch binary for the
# Supervisor (integration) tests
build-launcher-for-supervisor-tests:
	$(run) sh -c 'cd components/launcher && cargo build --bin=hab-launch'
unit-sup: build-launcher-for-supervisor-tests
.PHONY: build-launcher-for-supervisor-tests

define LINT
lint-$1: image ## executes the $1 component's linter checks
	$(run) sh -c 'cd components/$1 && cargo build --features clippy'
.PHONY: lint-$1
endef
$(foreach component,$(ALL),$(eval $(call LINT,$(component))))

define FUNCTIONAL
functional-$1: image ## executes the $1 component's functional test suite
	$(run) sh -c 'cd components/$1 && cargo test --features functional'
.PHONY: functional-$1

endef
$(foreach component,$(ALL),$(eval $(call FUNCTIONAL,$(component))))

define CLEAN
clean-$1: image ## cleans the $1 component's project tree
	$(run) sh -c 'cd components/$1 && cargo clean'
.PHONY: clean-$1

endef
$(foreach component,$(ALL),$(eval $(call CLEAN,$(component))))

define FMT
fmt-$1: image ## formats the $1 component
	$(run) sh -c 'cd components/$1 && cargo fmt'
.PHONY: fmt-$1

endef
$(foreach component,$(ALL),$(eval $(call FMT,$(component))))

# Run BATS integration tests in a Docker "cleanroom" container.
bats: build-hab build-sup build-launcher-for-supervisor-tests
	./run-bats.sh