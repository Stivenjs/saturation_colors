# Saturation Colors

Programa para Windows que te deja subir o bajar la saturación de los colores de tu pantalla. Simple, ligero y hecho en Rust.

## Descargar

👉 [Descargar la última versión](https://github.com/Stivenjs/saturation_colors/releases/latest)

Descárgalo, ábrelo y listo. No necesitas instalar nada más.

## Características

- Ajusta la saturación con un control deslizante.
- Se queda en segundo plano y lo manejas desde el icono junto al reloj.
- Atajos de teclado. ⚠️
- Recuerda tu configuración.
- Puede iniciarse junto con Windows.

## Para desarrolladores

Esta parte solo es necesaria si quieres modificar el código.

**Requisitos**

- Windows 10 o superior
- [Rust](https://www.rust-lang.org/tools/install) (edición 2024)

**Compilar**

```bash
git clone https://github.com/Stivenjs/saturation_colors.git
cd saturation_colors
cargo build --release
```

El ejecutable queda en `target/release/saturation_colors.exe`.

## Tecnologías

[Rust](https://www.rust-lang.org/) · [egui](https://github.com/emilk/egui) · [tray-icon](https://github.com/tauri-apps/tray-icon)
