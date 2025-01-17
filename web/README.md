# Web Demo

This directory of the project contains a build of pasfmt for web assembly, paired with
a web editor that runs it in a side-by-side view.

## Build WASM

### Prerequisites

1. Rust (and cargo)
2. [wasm-pack](https://github.com/rustwasm/wasm-pack):
    ```sh
    cargo install wasm-pack
    ```

### Build

```sh
cd web/
wasm-pack build --target web
```

## Build Web UI

### Prerequisites

1. Node.js
2. Dependencies
    ```sh
    cd web/demo
    npm install
    ```

### Build

```sh
cd web/demo

# for development
npx vite

# for production
npx vite build
```
