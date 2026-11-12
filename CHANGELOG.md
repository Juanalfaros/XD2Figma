# Registro de cambios

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).
El proyecto sigue [Versionado Semántico](https://semver.org/lang/es/).

## [0.6.2] - 2026-11-12

### Añadido

- Identidad de marca en `brand/`: símbolo, logotipos vertical y horizontal (color y monocromo, para fondos claros y oscuros) e íconos cuadrados, con una convención de nombres documentada.
- Logo en el encabezado del README, con variante para tema oscuro.

### Cambiado

- Nuevo ícono de la aplicación (macOS, Windows y Linux) generado desde `brand/xd2figma-app-icon.svg`.

## [0.6.1] - 2026-11-11

### Cambiado

- El escritor `.fig` (FigCraft) se obtiene de un fork propio, `Juanalfaros/figcraft`, fijado al mismo commit, para que la compilación no dependa del repositorio original.
- La documentación de distribución indica que los instaladores no se firman y cómo abrirlos.

## [0.6.0] - 2026-11-11

### Añadido

- Cancelación real: **Cancelar conversión** interrumpe el archivo en curso (también durante el recorrido de capas), no deja archivos a medias y devuelve el lote a pendiente.
- Historial de resultados persistente entre sesiones (hasta 50 entradas) con **Borrar historial**.
- Exportación del registro de la sesión a un archivo de texto.
- Herramienta de línea de comandos `xd2figma-cli` para convertir sin la interfaz.
- Pruebas del parser (rutas peligrosas, contenedores inválidos, modo tolerante y estricto, extracción bajo demanda, cancelación y nombres sin sobrescribir) y de la interfaz (cancelación, reintento e historial).
- CI con lint, `cargo fmt`, `clippy -D warnings`, todas las pruebas y comprobación del binario de escritorio en cada push y pull request. La publicación de instaladores depende de que pasen.
- Lint del frontend con oxlint (`pnpm lint`) y `pnpm check`.

### Cambiado

- El XD ya no se descomprime entero al abrirlo: el contenedor se valida y cada entrada se extrae solo cuando se usa, con lo que los recursos que no se emplean nunca tocan el disco.
- Límite de JSON de capas elevado de 32 a 128 MiB y mensajes de límite más claros.
- La interfaz se divide en componentes (`src/components`).
- El importador del plugin calcula las huellas de componentes con un hash en lugar de cadenas JSON anidadas.
- Descripción del paquete Rust actualizada.

### Corregido

- Un contorno vectorial corrupto ya no provoca un pánico: se informa como advertencia o error.
- **Mostrar en carpeta** en Windows ya no muestra un error falso (el Explorador devuelve 1 aunque abra) y en Linux acepta carpetas.
- Se elimina el evento de progreso duplicado que la interfaz nunca escuchaba.

## [0.5.1] - 2026-11-07

### Corregido

- Compilación de escritorio en Linux: límite ZIP64 convertido explícitamente a u64 para evitar ambigüedad con GLib.
- Runner de macOS actualizado a macos-15 tras el retiro de macos-14.
- Publicación de los instaladores exitosos aunque falle otra plataforma.
- Acciones de GitHub actualizadas y caché de compilación Rust.

## [0.5.0] - 2026-11-07

### Cambiado

- Implementación del diseño 4 aprobado en Figma: dashboard con navegación lateral y pasos Importar, Configurar, Convertir y Revisar.
- Resumen de sesión persistente, progreso del archivo activo y cola de lote.
- Resultados de la sesión consultables después de iniciar una nueva conversión.
- Preferencias independientes con apariencia Claro, Oscuro o Sistema, formato y opciones avanzadas.
- Grafitos planos, Inter incluida localmente e iconos vectoriales sin emojis.
- Disposición adaptable a ventanas pequeñas, foco visible y estados de botones.

### Añadido

- Empaquetado `pnpm package:mac` con instalador verificado y accesible en la raíz del proyecto.
- Ventana DMG personalizada con fondo grafito Retina, zona de arrastre, acceso a Aplicaciones y posiciones de iconos guardadas sin AppleScript.
- Pruebas de interfaz para lotes, progreso IPC, advertencias, reintentos, detención, cancelación del destino y preferencias persistentes.

## [0.4.0] - 2026-11-07

### Añadido

- Botón para mostrar el archivo generado en su carpeta.
- Memoria de las opciones de conversión y la carpeta de destino.
- Detención del lote al finalizar el archivo actual.
- Limpieza del registro y aviso de advertencias en el resultado.

### Corregido

- Bloqueo inmediato de solicitudes de conversión duplicadas.
- Selector de apariencia dentro de Opciones avanzadas.

## [0.3.1] - 2026-11-06

- Selector de apariencia en Opciones avanzadas.

## [0.3.0] - 2026-11-06

### Interfaz

- Temas Claro, Oscuro y Sistema con superficies de grafito, iconos de trazo y preferencia persistente.
- Formato y carpeta de destino visibles al inicio. Opciones avanzadas agrupadas.
- Retiro individual de archivos de la cola y avisos consultables sin interrumpir la conversión.

## [0.2.3] - 2026-11-06

### Corregido

- Conservación del ancho visible en capturas diagonales divididas en tramos.
- Conservación del anclaje de texto posicionado con alineación centrada o derecha.

## [0.2.2] - 2026-11-06

### Corregido

- Conservación de la posición y escala de los recortes de imagen como rellenos editables en la exportación directa a .fig.
- Lectura y reducción de imágenes grandes antes de incrustarlas en el .fig.

## [0.2.1] - 2026-11-06

### Corregido

- Exportación de los degradados editables, las máscaras vectoriales y los desenfoques de fondo de Adobe XD.
- Altura inicial de las cajas de texto basada en las líneas y el interlineado del documento.
- Conversión a Auto Layout de pilas simples con separación uniforme y geometría compatible.

## [0.2.0] - 2026-11-06

### Añadido

- Exportación directa a .fig con capas editables como salida predeterminada.
- Importador local que crea nodos editables en Figma Design.
- Redes vectoriales, Auto Layout para Repeat Grids, extracción de imágenes y modo estricto.
- Log de conversión en vivo y panel lateral ajustable.
- Empaquetado de escritorio multiplataforma al publicar tags.

### Cambiado

- El paquete utiliza el esquema JSON 2 y reserva SVG para casos irrecuperables o efectos incompatibles.

### Corregido

- Conservación de mayúsculas como estilo de texto editable, incluso cuando XD guarda un contador de caracteres obsoleto.
- Alineación de las etiquetas de botones de una línea a partir del fondo y del relleno interno definido en XD.
- Conservación de colores y pesos por tramo, anclaje de textos centrados y modos de mezcla Multiplicar y Superponer.
- Ajuste del tamaño de imágenes grandes para la importación en Figma.

### Interfaz

- Estados diferenciados para selección, conversión, resultado y error.
- Reintento individual, estado del lote visible y selección explícita del resultado.

## [0.1.0] - 2026-11-05

### Añadido

- Aplicación de escritorio con Tauri v2, interfaz oscura en español y cola de conversión.
- Motor Rust para leer Adobe XD y exportar un paquete con SVG, JSON y recursos.
- Opciones de exportación y resumen por archivo con advertencias de compatibilidad.
- Configuración de instaladores nativos y prueba de integración con el XD local.
