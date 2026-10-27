# FERROUS.IO

Prototipo de supervivencia 3D con sensación `.io`, completamente **single-player y client-side**. Los 40 bots, el chat, el contador de jugadores, el ping y la clasificación son simulados localmente; no hay servidor, WebSocket ni multijugador real.

## Estructura

```text
Cargo.toml                 Workspace de dos crates
sim/
  Cargo.toml
  src/lib.rs               World, bots, IA, combate, recursos y supervivencia
client/
  Cargo.toml
  src/lib.rs               API wasm-bindgen y persistencia defensiva
www/
  index.html               HUD y estados de partida
  style.css                UI responsive y controles táctiles
  main.js                  Three.js, interpolación, audio y entrada
  pkg/                     Salida generada por wasm-pack
  vendor/                  Three.js local para servir www directamente
.github/workflows/deploy.yml
vite.config.js             Empaquetado estático opcional para Pages
```

## Ejecutar en local

Requisitos: Rust con el target `wasm32-unknown-unknown`, `wasm-pack` y Node.js.

```bash
rustup target add wasm32-unknown-unknown
wasm-pack build client --target web --out-dir ../www/pkg
npx serve www
```

Abre la URL local que imprime `serve`. No uses `file://`: el navegador necesita HTTP para cargar WASM.

### Build de producción

```bash
npm ci
npm run build
npm run preview
```

El resultado estático queda en `dist/`. No requiere backend.

## Controles

| Entrada | Acción |
|---|---|
| `WASD` / flechas | Mover al jugador |
| Ratón | Apuntar |
| Clic / `Espacio` | Atacar |
| `E` | Recolectar o recoger botín |
| `C` | Mochila y fabricación |
| `B` | Menú de construcción; clic para colocar |
| `1` | Comer bayas |
| `2` | Comer carne cocida |
| `F` | Cocinar carne o fundir mineral junto a una hoguera |
| `Q` | Beber junto a una orilla |
| `Enter` | Escribir en el chat |
| `P` / `Esc` | Pausa; `Esc` también cancela construcción o cierra paneles |
| `M` | Activar o silenciar audio |

En pantallas táctiles aparecen un joystick, botones de acción y accesos a mochila, construcción, chat y clasificación.

## Sistemas

- **Bucle fijo:** `requestAnimationFrame` llama a `GameState.tick(dt)`. Rust acumula tiempo y simula a 20 Hz; Three.js interpola posiciones previas y actuales para renderizar a la frecuencia del navegador.
- **Mundo:** ruido Perlin, tres biomas, lagos, recursos con HP y respawn, jabalíes y estructuras con snap a grid.
- **Bots:** 40 entidades con personalidad, estados `Idle → Wander → Gather → Craft → Build → Fight → Flee`, PvP local, botín y respawn a los 30 segundos con nombre nuevo.
- **Supervivencia:** salud, hambre, sed, temperatura, intoxicación por carne cruda, cocina, fundición, herramientas y construcción.
- **Ilusión de sala:** chat cada 5–15 segundos, respuestas con latencia de 1–3 segundos, leaderboard cada 2 segundos, ping y contador local claramente identificados como simulación.
- **Sin assets descargados en runtime:** texturas de ruido creadas en Canvas, geometría procedural y audio sintetizado con Web Audio.

## Guardado

Inventario, estadísticas, puntuación, posición y estructuras del jugador se guardan en `localStorage` cada cuatro segundos y al pausar/cerrar. Los bots y sus estructuras se regeneran en cada sesión.

El guardado pertenece **solo a este navegador**: no se sincroniza entre dispositivos y se pierde si borras los datos del sitio. La opción «Borrar progreso y reiniciar» del menú de pausa también lo elimina.

## Pruebas

```bash
cargo test -p sim
cargo check -p client --target wasm32-unknown-unknown
```

Las pruebas cubren determinismo del tick fijo, consumo de ingredientes y saneado del chat.

## GitHub Pages

1. Sube el proyecto a GitHub y usa `main` como rama principal.
2. En **Settings → Pages → Build and deployment**, selecciona **GitHub Actions** como origen.
3. Haz push a `main`. El workflow instala Rust/WASM, compila `client`, genera `dist/` y publica con `actions/deploy-pages`.
4. La primera ejecución puede requerir aceptar el entorno `github-pages`; después la URL aparece en la acción y en **Settings → Pages**.

La base relativa de Vite admite tanto páginas de usuario como repositorios publicados en un subdirectorio.

## Licencias

Three.js se distribuye bajo MIT; su licencia se incluye en `www/vendor/THREE-LICENSE.txt`. No se usan texturas, modelos, sonidos ni fuentes externas.
