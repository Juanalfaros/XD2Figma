# XD2Figma

Aplicación de escritorio que convierte archivos Adobe XD en archivos .fig con capas editables para Figma Design. También puede generar un paquete para el importador mediante la Plugin API.

## Descargas

Las versiones y sus instaladores se publican en [GitHub Releases](https://github.com/Juanalfaros/XD2Figma/releases). Cada archivo incluye su suma SHA-256. El workflow de tags `v*` compila macOS Apple Silicon, Windows x64 (MSI y EXE) y Linux x64 (AppImage y DEB). Consulta los archivos adjuntos y el estado de Actions para comprobar cuáles están disponibles en cada versión.

## Instalar XD2Figma 0.6.0 en macOS

Abre `XD2Figma_0.6.0_aarch64.dmg`, disponible en la raíz del proyecto, y arrastra **XD2Figma** a **Aplicaciones**. Si ya tienes instalada una versión anterior, cierra la aplicación antes de reemplazarla. La versión se muestra al pie de la navegación lateral.

La interfaz implementa la [propuesta 4 — Dashboard guiado](https://www.figma.com/design/ijSjjmuOohgwfYuYrnh5Ev?node-id=8-206). Usa **Conversiones** para seguir los pasos **Importar → Configurar → Convertir → Revisar**. El resumen lateral mantiene visibles el formato, el destino y el estado del lote.

En **Preferencias** puedes elegir **Claro**, **Oscuro** o **Sistema**, además del formato y las opciones de conversión. La aplicación recuerda estos ajustes y la carpeta de destino. El tema oscuro es el predeterminado en instalaciones nuevas. La tipografía Inter se incluye localmente bajo su licencia OFL.

**Resultados** conserva los últimos 50 archivos completados, incluso después de cerrar la app; **Borrar historial** lo vacía. **Registro** muestra los avisos de la sesión y permite exportarlos a un archivo de texto. Los originales no se modifican.

## Desarrollo

Instala Rust 1.95 o superior, Node.js 22.12 o superior, pnpm y los [requisitos de Tauri v2](https://v2.tauri.app/start/prerequisites/) de tu plataforma.

```sh
pnpm install
cargo install tauri-cli --version '^2' --locked
cargo tauri dev
cargo tauri build
```

También puedes usar `pnpm tauri dev` y `pnpm tauri build`. En macOS, `pnpm package:mac` crea y verifica un DMG en la raíz del proyecto, con fondo Retina propio, posiciones fijas de los iconos y un acceso a Aplicaciones. La ventana se configura mediante `.DS_Store`, sin automatizar Finder. Requiere Python 3.9 o posterior y las herramientas de línea de comandos de Xcode; las dependencias de empaquetado se instalan en un entorno aislado bajo `src-tauri/target`. Para cambiar solo el diseño del instalador conservando la app compilada y verificada de la misma versión, usa `pnpm package:mac --reuse-app`. La vista previa se genera en `src-tauri/target/dmg-artwork/preview.png`. Los tags `v*` disparan GitHub Actions, conservan los instaladores como artefactos y los adjuntan a la release correspondiente. Si una plataforma falla, se publican las que hayan terminado correctamente; el fallo sigue visible en Actions.

## Del XD al .fig

Añade un archivo XD, elige una carpeta de salida y pulsa **Iniciar conversión** en el paso **Configurar**. XD2Figma escribe directamente un archivo `.fig` en esa carpeta. En Figma Desktop, impórtalo desde el explorador de archivos de Figma.

La exportación binaria es experimental y usa el escritor Kiwi de [FigCraft](https://github.com/storytold/figcraft). El prototipado se conserva como datos en el paquete del plugin cuando se activa esa opción. Las instancias se escriben como marcos con sus capas editables.

### Texto y botones editables

El exportador conserva el texto original, los estilos por tramo y el ajuste de mayúsculas. En botones de una sola línea con fondo rectangular y relleno interno definido en XD, coloca la etiqueta en el área interior y conserva ambos elementos como capas editables de tamaño fijo.

La cola distingue los archivos pendientes, en proceso, listos y con error. Usa **Reintentar** en la fila de un archivo para procesarlo de nuevo o selecciona un resultado para ver su ruta y sus advertencias. El registro mantiene los avisos de toda la sesión y permite limpiarlos. El botón **Abrir carpeta de resultados** permite localizar el archivo generado.

La aplicación recuerda las opciones y la carpeta de destino entre sesiones. **Cancelar conversión** interrumpe el archivo activo sin dejar archivos a medias y deja todo el lote pendiente en la cola.

## Línea de comandos

El mismo motor está disponible sin interfaz:

```sh
cargo run --release --manifest-path src-tauri/Cargo.toml --no-default-features --bin xd2figma-cli -- -o salida "Diseño.xd"
```

Opciones: `--zip` (paquete para el plugin), `--strict`, `--extract-images`, `--no-components`, `--no-auto-layout` y `--prototype`. Termina con código distinto de cero si algún archivo falla.

## Distribución y firma

Los instaladores no están firmados. En macOS la firma es ad hoc (`signingIdentity: "-"`), así que Gatekeeper pedirá abrirlos con clic derecho → Abrir, y Windows mostrará SmartScreen. Para distribuirlos a terceros hace falta un Apple Developer ID con notarización y un certificado de firma de código en Windows; ambos requieren cuentas de pago y secretos en GitHub Actions que no están configurados.

## Dependencias a vigilar

El escritor `.fig` depende de [FigCraft](https://github.com/storytold/figcraft), fijado al commit `8eb0b87`. Es un repositorio de terceros y su formato es experimental: si desaparece o cambia, la compilación falla. Conviene mantener un fork propio y revisar su licencia antes de distribuir binarios.

## Paquete opcional para el plugin

Selecciona **Paquete para el plugin (.zip)** en **Preferencias → Formato de salida** para usar este flujo.

1. Añade los archivos XD en XD2Figma, elige el destino y pulsa **Iniciar conversión**.
2. Extrae el paquete `.xd2figma.zip`. En Figma Desktop, registra `figma-importer/manifest.json` desde **Plugins → Development → Import plugin from manifest**. Si Figma solicita un ID de desarrollo propio, crea un plugin local y sustituye el campo `id` por el que Figma genere.
3. Abre un documento de Figma Design, ejecuta **XD2Figma Importer** y selecciona la carpeta extraída.
4. Guarda el archivo nativo desde **File → Save local copy**. Figma genera el binario `.fig`. Consulta la [guía de Figma](https://help.figma.com/hc/en-us/articles/8403626871063-Save-a-local-copy-of-files).

El ZIP incluye JSON versionado, los binarios de recursos y un importador sin acceso de red. Las capas importables usan texto, formas, componentes, rellenos y efectos nativos. Las rutas Bézier usan Vector Network cuando conservan la geometría, con VectorPaths editable para contornos abiertos con relleno. Las Repeat Grids reconocidas se convierten en marcos con Auto Layout.

El modo tolerante registra las aproximaciones SVG y los marcadores que indican geometría irrecuperable. El modo estricto detiene el archivo si necesita ese fallback o encuentra una entrada AGC inválida. Las fuentes se comprueban en el sistema y Figma vuelve a comprobar su disponibilidad al importar. Algunas instancias con overrides se reconstruyen como marcos editables. Los recortes de imágenes pueden variar y las interacciones opcionales quedan en el JSON para su reconstrucción.

## Verificación

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features
```

`pnpm check` ejecuta además el lint, la compilación y las pruebas de interfaz. Las pruebas comprueban la lectura del archivo .fig generado y la presencia de textos, vectores, componentes e imágenes. Los diseños de prueba no se publican en este repositorio. Las dos pruebas con el diseño real están marcadas como ignoradas por defecto. Para ejecutarlas, proporciona tu copia autorizada del fixture Flipeame y utiliza `XD_FIXTURE="/ruta/Flipeame - Landing Page.xd" cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --test conversion_test -- --include-ignored --nocapture`. El fixture incluye aserciones específicas de ese diseño. El caso sintético comprueba Auto Layout, Bézier, efectos, extracción de recursos anidados y modo estricto.

El módulo `parser` valida el contenedor, extrae bajo demanda y analiza las capas. `converter` crea el modelo de Figma y sus redes vectoriales. `exporter` empaqueta el JSON, los recursos y el importador. `commands` ejecuta el trabajo en segundo plano y transmite progreso y advertencias a React.

### Pruebas de la interfaz

```sh
pnpm exec playwright install chromium
pnpm test:ui
```

Las pruebas ejecutan React y simulan la comunicación Tauri sin leer ni escribir archivos de usuario. Comprueban selección múltiple, progreso, resultado, advertencias, cancelación del selector, detención del lote, reintentos, persistencia y adaptación a ventanas pequeñas. En macOS se puede reutilizar Microsoft Edge si está instalado; en otros entornos se usa Chromium de Playwright. La compilación frontend se valida con `pnpm build`.
