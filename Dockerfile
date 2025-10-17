# Multi-stage Dockerfile para construcción multiplataforma de overlay-native

# Etapa 1: Construcción para Linux
FROM rust:1.75-bullseye AS linux-builder

# Instalar dependencias de sistema para Linux
RUN apt-get update && apt-get install -y \
    pkg-config \
    libgtk-3-dev \
    libglib2.0-dev \
    libpango1.0-dev \
    libcairo2-dev \
    libgdk-pixbuf2.0-dev \
    libx11-dev \
    libxrandr-dev \
    && rm -rf /var/lib/apt/lists/*

# Establecer directorio de trabajo
WORKDIR /app

# Copiar archivos del proyecto
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Construir para Linux en modo release
RUN cargo build --release --target x86_64-unknown-linux-gnu

# Etapa 2: Construcción para macOS (cross-compilation)
FROM rust:1.75-bullseye AS macos-builder

# Instalar herramientas de cross-compilation
RUN apt-get update && apt-get install -y \
    clang \
    lld \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Instalar osxcross (simplificado para demostración)
# Nota: En producción, necesitarías configurar osxcross correctamente
RUN cargo install cargo-xbuild

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Construir para macOS (requiere configuración adicional)
RUN echo "Construcción para macOS requiere configuración adicional de osxcross"

# Etapa 3: Imagen final de runtime para Linux
FROM debian:bullseye-slim AS linux-runtime

# Instalar dependencias de runtime
RUN apt-get update && apt-get install -y \
    libgtk-3-0 \
    libglib2.0-0 \
    libpango-1.0-0 \
    libcairo2 \
    libgdk-pixbuf2.0-0 \
    libx11-6 \
    libxrandr2 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Crear usuario no-root
RUN useradd -m -u 1000 overlayuser
WORKDIR /app

# Copiar binarios desde la etapa de construcción
COPY --from=linux-builder /app/target/x86_64-unknown-linux-gnu/release/overlay-native /usr/local/bin/
COPY --from=linux-builder /app/target/x86_64-unknown-linux-gnu/release/test_emotes /usr/local/bin/

# Copiar archivos de configuración si existen
COPY config*.json ./

# Cambiar a usuario no-root
USER overlayuser

# Exponer puertos si es necesario
# EXPOSE 8080

# Comando por defecto
CMD ["overlay-native"]

# Etapa 4: Imagen para desarrollo
FROM rust:1.75-bullseye AS development

# Instalar dependencias de desarrollo
RUN apt-get update && apt-get install -y \
    pkg-config \
    libgtk-3-dev \
    libglib2.0-dev \
    libpango1.0-dev \
    libcairo2-dev \
    libgdk-pixbuf2.0-dev \
    libx11-dev \
    libxrandr-dev \
    gdb \
    valgrind \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiar archivos del proyecto
COPY . .

# Construir en modo debug
RUN cargo build

# Configurar entorno de desarrollo
ENV RUST_LOG=debug
ENV RUST_BACKTRACE=1

# Comando por defecto para desarrollo
CMD ["cargo", "run"]
