# solverforge-iap Makefile
# Colorized dev entrypoints in the SolverForge house style

# ============== Colors & Symbols ==============
GREEN := \033[92m
EMERALD := \033[38;2;16;185;129m
CYAN := \033[96m
YELLOW := \033[93m
MAGENTA := \033[95m
RED := \033[91m
GRAY := \033[90m
BOLD := \033[1m
RESET := \033[0m

CHECK := ✓
CROSS := ✗
ARROW := ▸
PROGRESS := →

# ============== Project Metadata ==============
VERSION := $(shell grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
RUST_VERSION := 1.95+
NAME := solverforge-iap
SECONDS ?= 60
SWEEP_INSTANCES ?=
SWEEP_FLAGS ?=

# ============== Phony Targets ==============
.PHONY: banner help build build-release serve solve test lint fmt fmt-check clippy check \
        sweep bench version bump-patch bump-minor bump-major bump-dry \
        pre-release install clean watch

.DEFAULT_GOAL := help

# ============== Banner ==============
banner:
	@if command -v figlet >/dev/null 2>&1; then \
	  figlet -f small -w 90 "$(NAME)" | { printf "$(EMERALD)$(BOLD)"; cat; printf "$(RESET)"; }; \
	else printf "$(EMERALD)$(BOLD)$(NAME)$(RESET)\n"; fi
	@printf "  $(GRAY)v$(VERSION)$(RESET) $(EMERALD)Invigilator Assignment Workbench$(RESET)\n\n"

define box
@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
@printf "$(CYAN)$(BOLD)║ %-36s ║$(RESET)\n" "$(1)"
@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
endef

# ============== Build Targets ==============

build: banner
	@$(call box,Build)
	@printf "$(ARROW) $(BOLD)Compiling the solver and workbench...$(RESET)\n"
	@cargo build && \
		printf "$(GREEN)$(CHECK) Build successful$(RESET)\n\n" || \
		(printf "$(RED)$(CROSS) Build failed$(RESET)\n\n" && exit 1)

build-release: banner
	@$(call box,Release Build)
	@printf "$(ARROW) $(BOLD)Compiling with optimizations...$(RESET)\n"
	@cargo build --release && \
		printf "$(GREEN)$(CHECK) Release build successful$(RESET)\n\n" || \
		(printf "$(RED)$(CROSS) Release build failed$(RESET)\n\n" && exit 1)

# ============== Solve one instance ==============

solve: banner
	@printf "$(ARROW) $(BOLD)Solving $(YELLOW)$(INSTANCE)$(RESET) $(BOLD)for $(YELLOW)$(SECONDS)$(RESET)$(BOLD)s...$(RESET)\n"
	@python3 scripts/solve_snapshot.py $(INSTANCE) $(SECONDS)

# ============== Test Targets ==============

test: banner
	@$(call box,Tests)
	@printf "$(PROGRESS) Unit + audit tests...\n"
	@cargo test --lib --quiet && \
		printf "$(GREEN)$(CHECK) All tests passed$(RESET)\n\n" || \
		(printf "$(RED)$(CROSS) Tests failed$(RESET)\n\n" && exit 1)

# ============== Lint & Model ==============

lint: banner fmt-check clippy
	@printf "$(GREEN)$(BOLD)$(CHECK) All lint checks passed$(RESET)\n\n"

fmt:
	@printf "$(PROGRESS) Formatting code...\n"
	@cargo fmt --all
	@printf "$(GREEN)$(CHECK) Code formatted$(RESET)\n"

fmt-check:
	@printf "$(PROGRESS) Checking formatting...\n"
	@cargo fmt --all -- --check && \
		printf "$(GREEN)$(CHECK) Formatting valid$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Formatting issues found$(RESET)\n" && exit 1)

clippy:
	@printf "$(PROGRESS) Running clippy...\n"
	@cargo clippy --all-targets -- -D warnings && \
		printf "$(GREEN)$(CHECK) Clippy passed$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Clippy warnings found$(RESET)\n" && exit 1)

check: banner fmt-check clippy test
	@$(call box,Model)
	@/srv/lab/dev/solverforge/solverforge-cli/target/release/solverforge check
	@printf "\n$(GREEN)$(BOLD)$(CHECK) Full gate passed$(RESET)\n\n"

# ============== Benchmarks ==============

sweep: banner
	@$(call box,Instance Sweep)
	@printf "$(ARROW) $(BOLD)Sweeping CM and CR at $(SECONDS)s per job...$(RESET)\n"
	@python3 scripts/sweep.py --seconds "$(SECONDS)" $(SWEEP_FLAGS) $(SWEEP_INSTANCES)

bench: banner
	@$(call box,Native Benchmark)
	@cargo build --release --example benchmark --quiet && \
		printf "$(GREEN)$(CHECK) Benchmark example built$(RESET)\n\n" || \
		(printf "$(RED)$(CROSS) Benchmark build failed$(RESET)\n\n" && exit 1)

# ============== Version Management ==============

version:
	@printf "$(CYAN)Current version:$(RESET) $(YELLOW)$(BOLD)$(VERSION)$(RESET)\n"

bump-patch: banner
	@printf "$(ARROW) Bumping patch version...\n"
	@csemver --release-as patch && \
		printf "$(GREEN)$(CHECK) Version bumped$(RESET)\n"

bump-minor: banner
	@printf "$(ARROW) Bumping minor version...\n"
	@csemver --release-as minor && \
		printf "$(GREEN)$(CHECK) Version bumped$(RESET)\n"

bump-major: banner
	@printf "$(ARROW) Bumping major version...\n"
	@csemver --release-as major && \
		printf "$(GREEN)$(CHECK) Version bumped$(RESET)\n"

bump-dry:
	@csemver --dry-run

# ============== Pre-Release Validation ==============

pre-release: banner
	@csemver --dry-run >/dev/null && printf "$(GREEN)$(CHECK) Release updaters resolved every configured surface$(RESET)\n"
	@$(call box,Pre-Release Validation)
	@$(MAKE) fmt-check --no-print-directory
	@$(MAKE) clippy --no-print-directory
	@printf "$(PROGRESS) Running full test suite...\n"
	@cargo test --lib --quiet && printf "$(GREEN)$(CHECK) All tests passed$(RESET)\n"
	@printf "\n$(GREEN)$(BOLD)$(CHECK) Ready for release v$(VERSION)$(RESET)\n\n"

# ============== Install & Clean ==============

install: banner
	@printf "$(ARROW) Installing the workbench binary...\n"
	@cargo install --path . --force && \
		printf "$(GREEN)$(CHECK) Installed $(NAME)$(RESET)\n\n" || \
		(printf "$(RED)$(CROSS) Install failed$(RESET)\n\n" && exit 1)

clean:
	@printf "$(ARROW) Cleaning build artifacts...\n"
	@cargo clean
	@printf "$(GREEN)$(CHECK) Clean complete$(RESET)\n"

watch:
	@printf "$(ARROW) Watching for changes...\n"
	@cargo watch -x build

# ============== Help ==============

help: banner
	@/bin/echo -e "$(CYAN)$(BOLD)Build Commands:$(RESET)"
	@/bin/echo -e "  $(GREEN)make build$(RESET)          - Build the solver and workbench"
	@/bin/echo -e "  $(GREEN)make build-release$(RESET)  - Build in release mode"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Run:$(RESET)"
	@/bin/echo -e "  $(GREEN)make serve$(RESET)          - Serve the workbench ($(GRAY)PORT, IAP_VARIANT from the environment$(RESET))"
	@/bin/echo -e "  $(GREEN)make sweep$(RESET)          - Both variants; SECONDS=600 for the paper per-run budget"
	@/bin/echo -e "  $(GREEN)make bench$(RESET)          - Build the native benchmark example"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Test & Lint:$(RESET)"
	@/bin/echo -e "  $(GREEN)make test$(RESET)           - Unit + audit tests"
	@/bin/echo -e "  $(GREEN)make lint$(RESET)           - fmt-check + clippy"
	@/bin/echo -e "  $(GREEN)make check$(RESET)          - $(YELLOW)$(BOLD)Full gate: lint, tests, model check$(RESET)"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Version Management:$(RESET)"
	@/bin/echo -e "  $(GREEN)make version$(RESET)        - Show current version"
	@/bin/echo -e "  $(GREEN)make bump-dry$(RESET)       - Preview version bump"
	@/bin/echo -e "  $(GREEN)make bump-patch$(RESET)     - Bump patch version"
	@/bin/echo -e "  $(GREEN)make bump-minor$(RESET)     - Bump minor version"
	@/bin/echo -e "  $(GREEN)make bump-major$(RESET)     - Bump major version"
	@/bin/echo -e "  $(GREEN)make pre-release$(RESET)    - All pre-release validations"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Other:$(RESET)"
	@/bin/echo -e "  $(GREEN)make install$(RESET)        - Install the binary with cargo"
	@/bin/echo -e "  $(GREEN)make clean$(RESET)          - Clean build artifacts"
	@/bin/echo -e "  $(GREEN)make watch$(RESET)          - Watch and rebuild on changes"
	@/bin/echo -e ""
	@/bin/echo -e "$(GRAY)Rust version required: $(RUST_VERSION)$(RESET)"
	@/bin/echo -e "$(GRAY)Current version: v$(VERSION)$(RESET)"
	@/bin/echo -e ""
