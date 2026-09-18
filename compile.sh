#!/bin/bash

# Останавливать скрипт, если какая-то команда завершилась ошибкой
set -e

echo "[1/3] Компилируем WASM-модуль..."
cd ~/app_wasm
cargo build --target wasm32-unknown-unknown --release

echo "[2/3] Копируем собранный .wasm в рантайм..."
cp target/wasm32-unknown-unknown/release/app_wasm.wasm ~/host_runtime/app.wasm

echo "[3/3] Запускаем Host Runtime..."
cd ~/host_runtime
cargo run