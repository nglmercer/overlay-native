# Makefile para construcción multiplataforma de overlay-native

# Variables
PROJECT_NAME = overlay-native
CARGO = cargo
TARGET_DIR = target
DIST_DIR = dist

# Targets soportados
LINUX_TARGET = x86_64-unknown-linux-gnu
MACOS_TARGET = x86_64-apple-darwin
MACOS_ARM_TARGET = aarch64-apple-darwin
WINDOWS_TARGET = x86_64-pc-windows-msvc

# Colores para salida
RED := \033[0;31m
GREEN := \033[0;32m
YELLOW := \033[1;33m
BLUE := \033[0;34m
CYAN := \033[0;36m
WHITE := \033[1;37m
GRAY := \033[0;37m
NC := \033[0m # No Color

# Funciones auxiliares
define print_message
	@echo -e "$(1)$(2)$(NC)"
endef

# Target por defecto
.PHONY: default
default: build

# Ayuda
.PHONY: help
help:
	@echo "$(PROJECT_NAME) - Makefile multiplataforma"
	@echo ""
	@echo "Targets disponibles:"
	@echo "  build          - Construir para la plataforma actual"
	@echo "  build-all      - Construir para todas las plataformas"
	@echo "  build-linux    - Construir para Linux x86_64"
	@echo "  build-macos    - Construir para macOS x86_64"
	@echo "  build-macos-arm - Construir para macOS ARM64"
	@echo "  build-windows  - Construir para Windows x86_64"
	@echo "  test           - Ejecutar pruebas"
	@echo "  test-all       - Ejecutar pruebas en todas las plataformas"
	@echo "  clean          - Limpiar archivos de construcción"
	@echo "  clean-all      - Limpiar completamente incluyendo targets cruzados"
	@echo "  docker-build   - Construir usando Docker"
	@echo "  docker-run     - Ejecutar usando Docker"
	@echo "  install-deps   - Instalar dependencias del sistema"
	@echo "  format         - Formatear código"
	@echo "  lint           - Ejecutar linter"
	@echo "  doc            - Generar documentación"
	@echo "  package        - Empaquetar binarios para distribución"
	@echo "  help           - Mostrar esta ayuda"

# Construir para plataforma actual
.PHONY: build
build:
	$(call print_message,$(GREEN),🔨 Construyendo para plataforma actual...)
	$(CARGO) build --release

# Construir para todas las plataformas
.PHONY: build-all
build-all: build-linux build-macos build-macos-arm build-windows
	$(call print_message,$(GREEN),✅ Construcción completada para todas las plataformas)

# Construir para Linux
.PHONY: build-linux
build-linux:
	$(call print_message,$(BLUE),📦 Construyendo para Linux x86_64...)
	$(CARGO) build --release --target $(LINUX_TARGET)
	@mkdir -p $(DIST_DIR)/linux
	@cp $(TARGET_DIR)/$(LINUX_TARGET)/release/$(PROJECT_NAME) $(DIST_DIR)/linux/ 2>/dev/null || true
	@cp $(TARGET_DIR)/$(LINUX_TARGET)/release/test_emotes $(DIST_DIR)/linux/ 2>/dev/null || true

# Construir para macOS x86_64
.PHONY: build-macos
build-macos:
	$(call print_message,$(BLUE),📦 Construyendo para macOS x86_64...)
	$(CARGO) build --release --target $(MACOS_TARGET)
	@mkdir -p $(DIST_DIR)/macos
	@cp $(TARGET_DIR)/$(MACOS_TARGET)/release/$(PROJECT_NAME) $(DIST_DIR)/macos/ 2>/dev/null || true
	@cp $(TARGET_DIR)/$(MACOS_TARGET)/release/test_emotes $(DIST_DIR)/macos/ 2>/dev/null || true

# Construir para macOS ARM64
.PHONY: build-macos-arm
build-macos-arm:
	$(call print_message,$(BLUE),📦 Construyendo para macOS ARM64...)
	$(CARGO) build --release --target $(MACOS_ARM_TARGET)
	@mkdir -p $(DIST_DIR)/macos-arm
	@cp $(TARGET_DIR)/$(MACOS_ARM_TARGET)/release/$(PROJECT_NAME) $(DIST_DIR)/macos-arm/ 2>/dev/null || true
	@cp $(TARGET_DIR)/$(MACOS_ARM_TARGET)/release/test_emotes $(DIST_DIR)/macos-arm/ 2>/dev/null || true

# Construir para Windows
.PHONY: build-windows
build-windows:
	$(call print_message,$(BLUE),📦 Construyendo para Windows x86_64...)
	$(CARGO) build --release --target $(WINDOWS_TARGET)
	@mkdir -p $(DIST_DIR)/windows
	@cp $(TARGET_DIR)/$(WINDOWS_TARGET)/release/$(PROJECT_NAME).exe $(DIST_DIR)/windows/ 2>/dev/null || true
	@cp $(TARGET_DIR)/$(WINDOWS_TARGET)/release/test_emotes.exe $(DIST_DIR)/windows/ 2>/dev/null || true

# Instalar targets de cross-compilation
.PHONY: install-targets
install-targets:
	$(call print_message,$(YELLOW),📥 Instalando targets de cross-compilation...)
	rustup target add $(LINUX_TARGET) $(MACOS_TARGET) $(MACOS_ARM_TARGET) $(WINDOWS_TARGET)

# Ejecutar pruebas
.PHONY: test
test:
	$(call print_message,$(BLUE),🧪 Ejecutando pruebas...)
	$(CARGO) test --release

# Ejecutar pruebas en todas las plataformas
.PHONY: test-all
test-all:
	$(call print_message,$(BLUE),🧪 Ejecutando pruebas en todas las plataformas...)
	$(CARGO) test --release --target $(LINUX_TARGET) || true
	$(CARGO) test --release --target $(MACOS_TARGET) || true
	$(CARGO) test --release --target $(MACOS_ARM_TARGET) || true
	$(CARGO) test --release --target $(WINDOWS_TARGET) || true

# Limpiar construcción
.PHONY: clean
clean:
	$(call print_message,$(YELLOW),🧹 Limpiando archivos de construcción...)
	$(CARGO) clean
	@rm -rf $(DIST_DIR)

# Limpiar completamente
.PHONY: clean-all
clean-all: clean
	$(call print_message,$(YELLOW),🧹 Limpiando targets cruzados...)
	@rm -rf $(TARGET_DIR)/*-*/release
	@rm -rf $(TARGET_DIR)/*-*/debug

# Construir con Docker
.PHONY: docker-build
docker-build:
	$(call print_message,$(BLUE),🐳 Construyendo con Docker...)
	docker-compose --profile build up --build

# Ejecutar con Docker
.PHONY: docker-run
docker-run:
	$(call print_message,$(BLUE),🐳 Ejecutando con Docker...)
	docker-compose --profile prod up

# Entorno de desarrollo con Docker
.PHONY: docker-dev
docker-dev:
	$(call print_message,$(BLUE),🐳 Iniciando entorno de desarrollo con Docker...)
	docker-compose --profile dev up --build

# Instalar dependencias del sistema
.PHONY: install-deps
install-deps:
	@if command -v apt-get >/dev/null 2>&1; then \
		$(call print_message,$(YELLOW),📦 Instalando dependencias para Debian/Ubuntu...); \
		sudo apt-get update; \
		sudo apt-get install -y pkg-config libgtk-3-dev libglib2.0-dev libpango1.0-dev libcairo2-dev libgdk-pixbuf2.0-dev libx11-dev libxrandr-dev; \
	elif command -v brew >/dev/null 2>&1; then \
		$(call print_message,$(YELLOW),📦 Instalando dependencias para macOS...); \
		brew install gtk+3 pkg-config; \
	elif command -v pacman >/dev/null 2>&1; then \
		$(call print_message,$(YELLOW),📦 Instalando dependencias para Arch Linux...); \
		sudo pacman -S gtk3 pkgconf; \
	else \
		$(call print_message,$(RED),❌ No se pudo detectar el gestor de paquetes. Instala manualmente GTK3 y pkg-config); \
	fi

# Formatear código
.PHONY: format
format:
	$(call print_message,$(BLUE),📝 Formateando código...)
	$(CARGO) fmt

# Ejecutar linter
.PHONY: lint
lint:
	$(call print_message,$(BLUE),🔍 Ejecutando linter...)
	$(CARGO) clippy -- -D warnings

# Generar documentación
.PHONY: doc
doc:
	$(call print_message,$(BLUE),📚 Generando documentación...)
	$(CARGO) doc --no-deps --document-private-items

# Empaquetar binarios para distribución
.PHONY: package
package: build-all
	$(call print_message,$(BLUE),📦 Empaquetando binarios para distribución...)
	@mkdir -p $(DIST_DIR)/packages
	@cd $(DIST_DIR) && \
	for dir in linux macos macos-arm windows; do \
		if [ -d "$$dir" ]; then \
			tar -czf "packages/$(PROJECT_NAME)-$$dir.tar.gz" "$$dir/" && \
			$(call print_message,$(GREEN),✅ Paquete creado: packages/$(PROJECT_NAME)-$$dir.tar.gz); \
		fi; \
	done

# Verificar construcción
.PHONY: verify
verify: format lint test
	$(call print_message,$(GREEN),✅ Verificación completada)

# CI/CD pipeline
.PHONY: ci
ci: install-targets verify build-all
	$(call print_message,$(GREEN),✅ Pipeline CI completado)

# Mostrar información del entorno
.PHONY: info
info:
	$(call print_message,$(CYAN),📋 Información del entorno:)
	@echo "  Rust: $$(rustc --version)"
	@echo "  Cargo: $$(cargo --version)"
	@echo "  SO: $$(uname -s)"
	@echo "  Arquitectura: $$(uname -m)"
	@echo "  Targets instalados:"
	@rustup target list --installed

# Watch para desarrollo
.PHONY: watch
watch:
	$(call print_message,$(BLUE),👀 Iniciando modo watch...)
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo watch -x run; \
	else \
		$(call print_message,$(YELLOW),⚠️  Instala cargo-watch: cargo install cargo-watch); \
	fi

# Instalar herramientas de desarrollo
.PHONY: install-tools
install-tools:
	$(call print_message,$(YELLOW),📥 Instalando herramientas de desarrollo...)
	cargo install cargo-watch cargo-audit cargo-outdated

# Auditoría de seguridad
.PHONY: audit
audit:
	$(call print_message,$(BLUE),🔒 Ejecutando auditoría de seguridad...)
	@if command -v cargo-audit >/dev/null 2>&1; then \
		cargo audit; \
	else \
		$(call print_message,$(YELLOW),⚠️  Instala cargo-audit: cargo install cargo-audit); \
	fi

# Verificar dependencias desactualizadas
.PHONY: outdated
outdated:
	$(call print_message,$(BLUE),📋 Verificando dependencias desactualizadas...)
	@if command -v cargo-outdated >/dev/null 2>&1; then \
		cargo outdated; \
	else \
		$(call print_message,$(YELLOW),⚠️  Instala cargo-outdated: cargo install cargo-outdated); \
	fi
