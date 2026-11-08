# Define variables
CARGO := cargo
TARGET := debug

.PHONY: all build dev release test clean fmt clippy

# Default target
all: build

dev:
	$(CARGO) watch -w src -x run

# Build the project in debug mode
build:
	$(CARGO) build

# Build optimized release version
release:
	$(CARGO) build --release

# Run unit tests
test:
	$(CARGO) test

# Run lints
clippy:
	$(CARGO) clippy

# Format code
fmt:
	$(CARGO) fmt

# Clean workspace target directory
clean:
	$(CARGO) clean
