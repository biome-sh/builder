# Keep plans in a correct build order
PLANS := components/builder-api components/builder-api-proxy components/builder-datastore components/builder-memcached components/builder-minio components/builder-graph components/builder-jobsrv

.DEFAULT_GOAL := all
all: tomlcheck shellcheck rendercheck build
.PHONY: all tomlcheck shellcheck rendercheck build
.PHONY: upload promote-testing promote-stable

setup: /.studio
setup: $(addprefix /hab/bin/,git tomlcheck shellcheck)
setup: $(addprefix /hab/bin/bio-plan-,tomlcheck shellcheck rendercheck)
.PHONY: setup

dist: /.studio
	@. results/builder-api.env && tar -zcf results/meta-$$pkg_version-$$pkg_target.tar.gz results/*.env
.PHONY: dist

pre-commit:
	@pre-commit run -a

tomlcheck: $(addprefix tomlcheck/,$(PLANS))
tomlcheck/%:
	@bio-plan-tomlcheck $*

shellcheck: $(addprefix shellcheck/,$(PLANS))
shellcheck/%:
	@bio-plan-shellcheck $*

rendercheck: $(addprefix rendercheck/,$(PLANS))
rendercheck/%:
	@bio-plan-rendercheck $*

build: $(addprefix build/,$(PLANS))
build/%: /.studio
	@MAKELEVEL= bio-plan-build $*
	@mv results/last_build.env results/$$(basename $*).env
.NOTPARALLEL: $(addprefix build/,$(PLANS))

upload: $(addprefix upload/,$(PLANS))
upload/%:
	@. results/$$(basename $*).env && bio pkg upload results/$$pkg_artifact

promote-testing: $(addprefix promote-testing/,$(PLANS))
promote-testing/%:
	@. results/$$(basename $*).env && bio pkg promote $$pkg_ident testing

promote-stable: $(addprefix promote-stable/,$(PLANS))
promote-stable/%:
	@. results/$$(basename $*).env && bio pkg promote $$pkg_ident stable

# Executables
/hab/bin/git:
	@bio pkg install -fb core/git

/hab/bin/pre-commit:
	@bio pkg install -fb ya/pre-commit

/hab/bin/tomlcheck:
	@bio pkg install -fb ya/tomlcheck

/hab/bin/shellcheck:
	@bio pkg install -fb core/shellcheck

/hab/bin/bio-plan-%:
	@bio pkg install -fb ya/bio-sdk

# Studio guard: fail build if we are not in studio
/.studio:
	@echo "Looks like you are not in studio but studio is required."
	@exit 1

.PHONY: clean
clean:
	@rm -rf results/
