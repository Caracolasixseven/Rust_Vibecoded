import * as THREE from 'three';
import init, { GameState } from './pkg/client.js';

const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => [...document.querySelectorAll(selector)];
const clamp = (value, min, max) => Math.max(min, Math.min(max, value));
const lerp = (a, b, t) => a + (b - a) * t;
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;

function mulberry32(seed) {
  return function random() {
    let t = seed += 0x6d2b79f5;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const canvas = $('#game-canvas');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, powerPreference: 'high-performance' });
renderer.setPixelRatio(Math.min(devicePixelRatio, 1.65));
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.08;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;

const scene = new THREE.Scene();
scene.background = new THREE.Color('#7f8b73');
scene.fog = new THREE.Fog('#7f8b73', 48, 105);
const camera = new THREE.OrthographicCamera(-20, 20, 15, -15, 0.1, 220);
const cameraTarget = new THREE.Vector3();
const cameraOffset = new THREE.Vector3(17, 23, 17);
const raycaster = new THREE.Raycaster();
const mouseNdc = new THREE.Vector2();
const groundPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
const aimPoint = new THREE.Vector3(3, 0, 0);
const mouseScreen = new THREE.Vector2(innerWidth * 0.5, innerHeight * 0.5);
let usingTouch = matchMedia('(pointer: coarse)').matches;

const hemi = new THREE.HemisphereLight('#d9e2c5', '#3e4434', 2.0);
scene.add(hemi);
const sun = new THREE.DirectionalLight('#ffe2b0', 3.1);
sun.position.set(-28, 42, 20);
sun.castShadow = true;
sun.shadow.mapSize.set(1024, 1024);
sun.shadow.camera.left = -32;
sun.shadow.camera.right = 32;
sun.shadow.camera.top = 32;
sun.shadow.camera.bottom = -32;
sun.shadow.camera.near = 1;
sun.shadow.camera.far = 100;
sun.shadow.bias = -0.0007;
scene.add(sun);

const worldGroup = new THREE.Group();
const terrainGroup = new THREE.Group();
scene.add(worldGroup, terrainGroup);
const dummy = new THREE.Object3D();
const tempColor = new THREE.Color();
const tempVec = new THREE.Vector3();
const skinPalette = ['#a9573e', '#567a78', '#a89252', '#6e644f', '#7c4f4d', '#4e6c50', '#776482', '#a36c3f'];

let game;
let mapData;
let latest;
let mode = 'title';
let quality = 'high';
let lowFpsIntervals = 0;
let lastFrame = performance.now();
let fps = 60;
let fpsAccumulator = 0;
let fpsFrames = 0;
let buildMode = null;
let actionQueue = [];
let joystickVector = { x: 0, z: 0 };
let lastInventorySignature = '';
let lastLeaderboardSignature = '';
let lastChatId = -1;
let lastResourceRevision = -1;
let lastStructureRevision = -1;
let deathReason = 'La isla se cobra otra víctima.';
let titleTime = 0;
let shake = 0;
let lastUiUpdate = 0;
let playerMesh;
let playerLabel;
let botBody;
let botHead;
let botLegs;
let boarBody;
let boarHead;
let dropMesh;
let wallMesh;
let doorMesh;
let campfireStoneMesh;
let campfireFlameMesh;
let treeTrunkMesh;
let treeCrownMesh;
let rockMesh;
let bushMesh;
let ghostMesh;
let particleSystem;
const botLabels = [];
const playerPositionCache = new THREE.Vector3();
const botPositionCache = Array.from({ length: 40 }, () => new THREE.Vector3());
const boarPositionCache = Array.from({ length: 12 }, () => new THREE.Vector3());
const keys = new Set();

const ITEM_META = {
  Wood: ['MAD', 'Madera'], Stone: ['PIE', 'Piedra'], MetalOre: ['MIN', 'Mineral'],
  Metal: ['MET', 'Metal'], Berry: ['BAY', 'Bayas'], RawMeat: ['CRU', 'Carne cruda'],
  CookedMeat: ['COC', 'Carne cocida'], StoneHatchet: ['H1', 'Hacha de piedra'],
  MetalHatchet: ['H2', 'Hacha de metal'], Campfire: ['HOG', 'Hoguera'],
  WoodWall: ['MUR', 'Muro'], WoodDoor: ['PTA', 'Puerta'],
};
const RECIPES = {
  stone_hatchet: { Wood: 6, Stone: 4, output: 'StoneHatchet' },
  metal_hatchet: { Wood: 4, Metal: 6, Stone: 8, output: 'MetalHatchet' },
  campfire: { Wood: 10, Stone: 6, output: 'Campfire' },
  wood_wall: { Wood: 8, output: 'WoodWall' },
  wood_door: { Wood: 12, output: 'WoodDoor' },
};

class SoundEngine {
  constructor() {
    this.context = null;
    this.master = null;
    this.muted = false;
    this.noiseBuffer = null;
    try { this.muted = localStorage.getItem('ferrous.muted') === '1'; } catch { this.muted = false; }
    this.updateButton();
  }
  start() {
    if (!this.context) {
      const AudioContext = window.AudioContext || window.webkitAudioContext;
      if (!AudioContext) return;
      this.context = new AudioContext();
      this.master = this.context.createGain();
      this.master.gain.value = this.muted ? 0 : 0.35;
      this.master.connect(this.context.destination);
      const length = this.context.sampleRate * 2;
      this.noiseBuffer = this.context.createBuffer(1, length, this.context.sampleRate);
      const channel = this.noiseBuffer.getChannelData(0);
      const random = mulberry32(90210);
      for (let i = 0; i < length; i++) channel[i] = random() * 2 - 1;
      this.startWind();
    }
    if (this.context.state === 'suspended') this.context.resume();
  }
  startWind() {
    const source = this.context.createBufferSource();
    source.buffer = this.noiseBuffer;
    source.loop = true;
    const filter = this.context.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.value = 380;
    const gain = this.context.createGain();
    gain.gain.value = 0.035;
    source.connect(filter).connect(gain).connect(this.master);
    source.start();
  }
  tone(frequency, duration = 0.1, type = 'square', volume = 0.12, endFrequency = frequency) {
    if (!this.context || this.muted) return;
    const now = this.context.currentTime;
    const oscillator = this.context.createOscillator();
    const gain = this.context.createGain();
    oscillator.type = type;
    oscillator.frequency.setValueAtTime(frequency, now);
    oscillator.frequency.exponentialRampToValueAtTime(Math.max(30, endFrequency), now + duration);
    gain.gain.setValueAtTime(volume, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + duration);
    oscillator.connect(gain).connect(this.master);
    oscillator.start(now);
    oscillator.stop(now + duration + 0.02);
  }
  noise(duration = 0.08, volume = 0.08, cutoff = 1000) {
    if (!this.context || this.muted) return;
    const now = this.context.currentTime;
    const source = this.context.createBufferSource();
    source.buffer = this.noiseBuffer;
    const filter = this.context.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.value = cutoff;
    const gain = this.context.createGain();
    gain.gain.setValueAtTime(volume, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + duration);
    source.connect(filter).connect(gain).connect(this.master);
    source.start(now);
    source.stop(now + duration);
  }
  play(kind) {
    if (kind === 'gather') { this.noise(0.07, 0.12, 800); this.tone(120, 0.06, 'triangle', 0.07, 80); }
    else if (kind === 'hit' || kind === 'hurt') { this.noise(0.1, 0.18, 1300); this.tone(90, 0.12, 'sawtooth', 0.08, 45); }
    else if (kind === 'craft' || kind === 'build') { this.tone(360, 0.08, 'square', 0.08, 520); setTimeout(() => this.tone(520, 0.1, 'square', 0.06, 680), 70); }
    else if (kind === 'pickup' || kind === 'deplete') this.tone(520, 0.09, 'triangle', 0.07, 760);
    else if (kind === 'eat' || kind === 'drink') this.noise(0.12, 0.05, 500);
    else if (kind === 'kill' || kind === 'death') this.tone(140, 0.4, 'sawtooth', 0.12, 42);
    else if (kind === 'respawn') this.tone(240, 0.25, 'triangle', 0.08, 620);
  }
  toggle() {
    this.muted = !this.muted;
    if (this.master) this.master.gain.setTargetAtTime(this.muted ? 0 : 0.35, this.context.currentTime, 0.02);
    try { localStorage.setItem('ferrous.muted', this.muted ? '1' : '0'); } catch { /* private mode */ }
    this.updateButton();
  }
  updateButton() {
    const button = $('#mute-button');
    button?.classList.toggle('muted', this.muted);
    button?.setAttribute('aria-label', this.muted ? 'Activar audio' : 'Silenciar audio');
  }
}
const audio = new SoundEngine();

function resize() {
  const width = innerWidth;
  const height = innerHeight;
  renderer.setSize(width, height, false);
  renderer.setPixelRatio(quality === 'low' ? 1 : Math.min(devicePixelRatio, width < 600 ? 1.35 : 1.65));
  const aspect = width / height;
  const viewHeight = width < 760 ? 32 : 29;
  camera.left = -viewHeight * aspect * 0.5;
  camera.right = viewHeight * aspect * 0.5;
  camera.top = viewHeight * 0.5;
  camera.bottom = -viewHeight * 0.5;
  camera.updateProjectionMatrix();
}
addEventListener('resize', resize);
resize();

function makeNoiseTexture(base, speck, seed = 1) {
  const surface = document.createElement('canvas');
  surface.width = surface.height = 64;
  const context = surface.getContext('2d');
  context.fillStyle = base;
  context.fillRect(0, 0, 64, 64);
  const random = mulberry32(seed);
  context.fillStyle = speck;
  for (let i = 0; i < 420; i++) {
    const size = random() > 0.85 ? 2 : 1;
    context.globalAlpha = 0.08 + random() * 0.2;
    context.fillRect(Math.floor(random() * 64), Math.floor(random() * 64), size, size);
  }
  context.globalAlpha = 1;
  const texture = new THREE.CanvasTexture(surface);
  texture.wrapS = texture.wrapT = THREE.RepeatWrapping;
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.magFilter = THREE.NearestFilter;
  return texture;
}

const terrainTexture = makeNoiseTexture('#d0d0c0', '#666b5c', 14);
terrainTexture.repeat.set(18, 18);
const woodTexture = makeNoiseTexture('#786044', '#2f2a21', 27);
woodTexture.repeat.set(2, 3);
const rockTexture = makeNoiseTexture('#898a7d', '#3e423b', 39);
const leafTexture = makeNoiseTexture('#668052', '#263c2c', 51);

function disposeGroup(group) {
  for (const child of [...group.children]) {
    group.remove(child);
    child.geometry?.dispose();
    if (Array.isArray(child.material)) child.material.forEach((material) => material.dispose());
    else child.material?.dispose();
  }
}

function buildTerrain() {
  disposeGroup(terrainGroup);
  const { grid, size, heights, biomes, ponds } = mapData;
  const positions = new Float32Array(grid * grid * 3);
  const colors = new Float32Array(grid * grid * 3);
  const uvs = new Float32Array(grid * grid * 2);
  const indices = [];
  const biomeColors = [new THREE.Color('#64794e'), new THREE.Color('#77786b'), new THREE.Color('#b49a64')];
  const random = mulberry32(81);
  for (let z = 0; z < grid; z++) {
    for (let x = 0; x < grid; x++) {
      const index = z * grid + x;
      const px = -size * 0.5 + x * size / (grid - 1);
      const pz = -size * 0.5 + z * size / (grid - 1);
      positions[index * 3] = px;
      positions[index * 3 + 1] = heights[index];
      positions[index * 3 + 2] = pz;
      tempColor.copy(biomeColors[biomes[index]]).multiplyScalar(0.91 + random() * 0.16);
      colors[index * 3] = tempColor.r;
      colors[index * 3 + 1] = tempColor.g;
      colors[index * 3 + 2] = tempColor.b;
      uvs[index * 2] = x / (grid - 1);
      uvs[index * 2 + 1] = z / (grid - 1);
      if (x < grid - 1 && z < grid - 1) {
        const a = index;
        const b = index + 1;
        const c = index + grid;
        const d = index + grid + 1;
        indices.push(a, c, b, b, c, d);
      }
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  geometry.setAttribute('uv', new THREE.BufferAttribute(uvs, 2));
  geometry.setIndex(indices);
  geometry.computeVertexNormals();
  const material = new THREE.MeshStandardMaterial({ vertexColors: true, map: terrainTexture, roughness: 1, flatShading: true });
  const terrain = new THREE.Mesh(geometry, material);
  terrain.receiveShadow = true;
  terrainGroup.add(terrain);

  const ocean = new THREE.Mesh(
    new THREE.PlaneGeometry(360, 360),
    new THREE.MeshStandardMaterial({ color: '#456c70', roughness: 0.35, metalness: 0.05 }),
  );
  ocean.rotation.x = -Math.PI * 0.5;
  ocean.position.y = -1.65;
  ocean.receiveShadow = true;
  terrainGroup.add(ocean);

  for (const pond of ponds) {
    const water = new THREE.Mesh(
      new THREE.CircleGeometry(pond.radius * 1.07, 48),
      new THREE.MeshPhysicalMaterial({ color: '#5797a0', transparent: true, opacity: 0.82, roughness: 0.22, metalness: 0.08 }),
    );
    water.rotation.x = -Math.PI * 0.5;
    water.position.set(pond.pos.x, -0.68, pond.pos.z);
    terrainGroup.add(water);
    const shore = new THREE.Mesh(
      new THREE.RingGeometry(pond.radius * 1.05, pond.radius * 1.14, 48),
      new THREE.MeshBasicMaterial({ color: '#a49b72', transparent: true, opacity: 0.65, side: THREE.DoubleSide }),
    );
    shore.rotation.x = -Math.PI * 0.5;
    shore.position.set(pond.pos.x, -0.7, pond.pos.z);
    terrainGroup.add(shore);
  }

  const grass = new THREE.InstancedMesh(
    new THREE.ConeGeometry(0.13, 0.75, 3),
    new THREE.MeshStandardMaterial({ color: '#506840', roughness: 1 }),
    180,
  );
  const grassRandom = mulberry32(1287);
  let placed = 0;
  while (placed < 180) {
    const x = (grassRandom() - 0.5) * size * 0.94;
    const z = (grassRandom() - 0.5) * size * 0.94;
    const gridX = clamp(Math.round((x / size + 0.5) * (grid - 1)), 0, grid - 1);
    const gridZ = clamp(Math.round((z / size + 0.5) * (grid - 1)), 0, grid - 1);
    if (biomes[gridZ * grid + gridX] !== 0 || ponds.some((pond) => Math.hypot(x - pond.pos.x, z - pond.pos.z) < pond.radius * 1.2)) continue;
    dummy.position.set(x, heightAt(x, z) + 0.25, z);
    dummy.rotation.set(0, grassRandom() * Math.PI, (grassRandom() - 0.5) * 0.25);
    const scale = 0.6 + grassRandom() * 0.9;
    dummy.scale.set(scale, scale, scale);
    dummy.updateMatrix();
    grass.setMatrixAt(placed++, dummy.matrix);
  }
  grass.receiveShadow = true;
  terrainGroup.add(grass);
}

function heightAt(x, z) {
  if (!mapData) return 0;
  const { grid, size, heights } = mapData;
  const gx = clamp((x / size + 0.5) * (grid - 1), 0, grid - 1.001);
  const gz = clamp((z / size + 0.5) * (grid - 1), 0, grid - 1.001);
  const x0 = Math.floor(gx);
  const z0 = Math.floor(gz);
  const tx = gx - x0;
  const tz = gz - z0;
  const a = heights[z0 * grid + x0];
  const b = heights[z0 * grid + x0 + 1];
  const c = heights[(z0 + 1) * grid + x0];
  const d = heights[(z0 + 1) * grid + x0 + 1];
  return lerp(lerp(a, b, tx), lerp(c, d, tx), tz);
}

function instanced(geometry, material, count, shadows = true) {
  const mesh = new THREE.InstancedMesh(geometry, material, count);
  mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
  mesh.castShadow = shadows;
  mesh.receiveShadow = shadows;
  mesh.frustumCulled = false;
  worldGroup.add(mesh);
  return mesh;
}

function createHumanoid(color, player = false) {
  const group = new THREE.Group();
  const bodyMaterial = new THREE.MeshStandardMaterial({ color, roughness: 0.85, map: player ? null : woodTexture });
  const skinMaterial = new THREE.MeshStandardMaterial({ color: '#c8956e', roughness: 0.9 });
  const darkMaterial = new THREE.MeshStandardMaterial({ color: '#292f2a', roughness: 1 });
  const body = new THREE.Mesh(new THREE.CylinderGeometry(0.42, 0.52, 1.1, 7), bodyMaterial);
  body.position.y = 1.18;
  const head = new THREE.Mesh(new THREE.SphereGeometry(0.34, 9, 7), skinMaterial);
  head.position.y = 2.03;
  const legs = new THREE.Mesh(new THREE.BoxGeometry(0.57, 0.68, 0.38), darkMaterial);
  legs.position.y = 0.38;
  const face = new THREE.Mesh(new THREE.BoxGeometry(0.23, 0.09, 0.05), darkMaterial);
  face.position.set(0, 2.06, 0.31);
  const weapon = new THREE.Group();
  const handle = new THREE.Mesh(new THREE.CylinderGeometry(0.035, 0.035, 1.1, 5), new THREE.MeshStandardMaterial({ color: '#5e4934' }));
  const blade = new THREE.Mesh(new THREE.BoxGeometry(0.35, 0.24, 0.07), new THREE.MeshStandardMaterial({ color: '#858a80', roughness: 0.65 }));
  blade.position.y = 0.47;
  weapon.add(handle, blade);
  weapon.position.set(0.57, 1.13, 0.12);
  weapon.rotation.z = -0.35;
  group.add(body, head, legs, face, weapon);
  group.userData.weapon = weapon;
  group.traverse((child) => { if (child.isMesh) { child.castShadow = true; child.receiveShadow = true; } });
  worldGroup.add(group);
  return group;
}

function makeLabel(name, isPlayer = false) {
  const surface = document.createElement('canvas');
  surface.width = 256;
  surface.height = 64;
  const texture = new THREE.CanvasTexture(surface);
  texture.colorSpace = THREE.SRGBColorSpace;
  const material = new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false });
  const sprite = new THREE.Sprite(material);
  sprite.scale.set(3.8, 0.95, 1);
  sprite.renderOrder = 8;
  sprite.userData = { surface, texture, name: '', health: -1, isPlayer };
  worldGroup.add(sprite);
  updateLabel(sprite, name, 100);
  return sprite;
}

function updateLabel(sprite, name, health) {
  const healthStep = Math.round(clamp(health, 0, 100) / 5);
  if (sprite.userData.name === name && sprite.userData.health === healthStep) return;
  sprite.userData.name = name;
  sprite.userData.health = healthStep;
  const { surface, texture, isPlayer } = sprite.userData;
  const context = surface.getContext('2d');
  context.clearRect(0, 0, 256, 64);
  context.fillStyle = 'rgba(16,21,17,0.72)';
  context.fillRect(20, 3, 216, 33);
  context.font = '600 19px monospace';
  context.textAlign = 'center';
  context.fillStyle = isPlayer ? '#f2a069' : '#eef0e7';
  const clipped = name.length > 23 ? `${name.slice(0, 22)}…` : name;
  context.fillText(clipped, 128, 26);
  context.fillStyle = 'rgba(16,21,17,0.8)';
  context.fillRect(70, 42, 116, 7);
  context.fillStyle = healthStep > 6 ? '#9eb66c' : '#e25f4f';
  context.fillRect(71, 43, 114 * healthStep / 20, 5);
  texture.needsUpdate = true;
}

function buildDynamicMeshes() {
  playerMesh = createHumanoid('#d87945', true);
  playerLabel = makeLabel(latest.player.name, true);
  const botMaterial = new THREE.MeshStandardMaterial({ color: '#ffffff', roughness: 0.9 });
  botBody = instanced(new THREE.CylinderGeometry(0.4, 0.5, 1.08, 6), botMaterial, latest.bots.length);
  botHead = instanced(new THREE.SphereGeometry(0.33, 8, 6), new THREE.MeshStandardMaterial({ color: '#b98b68', roughness: 0.95 }), latest.bots.length);
  botLegs = instanced(new THREE.BoxGeometry(0.55, 0.65, 0.37), new THREE.MeshStandardMaterial({ color: '#30372f' }), latest.bots.length);
  latest.bots.forEach((bot, index) => {
    botBody.setColorAt(index, tempColor.set(skinPalette[bot.skin % skinPalette.length]));
    botLabels.push(makeLabel(bot.name));
  });
  botBody.instanceColor.needsUpdate = true;

  boarBody = instanced(new THREE.DodecahedronGeometry(0.72, 0), new THREE.MeshStandardMaterial({ color: '#4e4437', roughness: 1 }), latest.boars.length);
  boarHead = instanced(new THREE.BoxGeometry(0.52, 0.48, 0.58), new THREE.MeshStandardMaterial({ color: '#665342' }), latest.boars.length);
  dropMesh = instanced(new THREE.BoxGeometry(0.65, 0.5, 0.65), new THREE.MeshStandardMaterial({ color: '#c78b45', emissive: '#4f2c12', emissiveIntensity: 0.3 }), 90);
  wallMesh = instanced(new THREE.BoxGeometry(3.8, 2.7, 0.32), new THREE.MeshStandardMaterial({ color: '#8a6a45', map: woodTexture, roughness: 1 }), 180);
  doorMesh = instanced(new THREE.BoxGeometry(3.3, 2.55, 0.28), new THREE.MeshStandardMaterial({ color: '#6c533a', map: woodTexture, roughness: 1 }), 180);
  campfireStoneMesh = instanced(new THREE.DodecahedronGeometry(0.48, 0), new THREE.MeshStandardMaterial({ color: '#74766d', map: rockTexture }), 180);
  campfireFlameMesh = instanced(new THREE.ConeGeometry(0.34, 1.15, 7), new THREE.MeshBasicMaterial({ color: '#ef914d', transparent: true, opacity: 0.9 }), 180, false);

  const treeCount = latest.resources.filter((resource) => resource.kind === 'Tree').length;
  const rockCount = latest.resources.filter((resource) => resource.kind === 'Rock').length;
  const bushCount = latest.resources.filter((resource) => resource.kind === 'Bush').length;
  treeTrunkMesh = instanced(new THREE.CylinderGeometry(0.26, 0.38, 2.4, 6), new THREE.MeshStandardMaterial({ color: '#665039', map: woodTexture }), treeCount);
  treeCrownMesh = instanced(new THREE.IcosahedronGeometry(1.38, 0), new THREE.MeshStandardMaterial({ color: '#4f7046', map: leafTexture, flatShading: true }), treeCount);
  rockMesh = instanced(new THREE.DodecahedronGeometry(0.94, 0), new THREE.MeshStandardMaterial({ color: '#818477', map: rockTexture, flatShading: true }), rockCount);
  bushMesh = instanced(new THREE.IcosahedronGeometry(0.74, 1), new THREE.MeshStandardMaterial({ color: '#6f874d', map: leafTexture, flatShading: true }), bushCount);

  ghostMesh = new THREE.Mesh(
    new THREE.BoxGeometry(3.8, 2.7, 0.32),
    new THREE.MeshBasicMaterial({ color: '#b7d68b', transparent: true, opacity: 0.42, depthWrite: false }),
  );
  ghostMesh.visible = false;
  worldGroup.add(ghostMesh);
  buildParticles();
}

function buildParticles() {
  const count = 180;
  const positions = new Float32Array(count * 3);
  const colors = new Float32Array(count * 3);
  positions.fill(-1000);
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  const points = new THREE.Points(geometry, new THREE.PointsMaterial({ size: 0.19, vertexColors: true, transparent: true, opacity: 0.9, depthWrite: false }));
  points.frustumCulled = false;
  worldGroup.add(points);
  particleSystem = {
    points, positions, colors, cursor: 0,
    particles: Array.from({ length: count }, () => ({ life: 0, x: 0, y: -1000, z: 0, vx: 0, vy: 0, vz: 0 })),
  };
}

function burst(position, color = '#d7b56d', amount = 10) {
  if (!particleSystem || reducedMotion) return;
  const random = mulberry32(Math.floor((latest?.time || 1) * 1000) + amount);
  tempColor.set(color);
  for (let i = 0; i < amount; i++) {
    const index = particleSystem.cursor++ % particleSystem.particles.length;
    const particle = particleSystem.particles[index];
    particle.life = 0.45 + random() * 0.35;
    particle.x = position.x;
    particle.z = position.z ?? position.y;
    particle.y = heightAt(particle.x, particle.z) + 1.1;
    particle.vx = (random() - 0.5) * 4;
    particle.vz = (random() - 0.5) * 4;
    particle.vy = 1.5 + random() * 3;
    particleSystem.colors[index * 3] = tempColor.r;
    particleSystem.colors[index * 3 + 1] = tempColor.g;
    particleSystem.colors[index * 3 + 2] = tempColor.b;
  }
  particleSystem.points.geometry.attributes.color.needsUpdate = true;
}

function updateParticles(dt) {
  if (!particleSystem) return;
  particleSystem.particles.forEach((particle, index) => {
    if (particle.life > 0) {
      particle.life -= dt;
      particle.x += particle.vx * dt;
      particle.y += particle.vy * dt;
      particle.z += particle.vz * dt;
      particle.vy -= 7 * dt;
      particleSystem.positions[index * 3] = particle.x;
      particleSystem.positions[index * 3 + 1] = particle.y;
      particleSystem.positions[index * 3 + 2] = particle.z;
    } else {
      particleSystem.positions[index * 3 + 1] = -1000;
    }
  });
  particleSystem.points.geometry.attributes.position.needsUpdate = true;
}

let attackAnimation = 0;
scene.add(sun.target);

function interpolated(entity, alpha, target = tempVec) {
  const x = lerp(entity.prev.x, entity.pos.x, alpha);
  const z = lerp(entity.prev.z, entity.pos.z, alpha);
  return target.set(x, heightAt(x, z), z);
}

function setInstance(mesh, index, x, y, z, yaw = 0, scale = 1, scaleY = scale) {
  dummy.position.set(x, y, z);
  dummy.rotation.set(0, yaw, 0);
  dummy.scale.set(scale, scaleY, scale);
  dummy.updateMatrix();
  mesh.setMatrixAt(index, dummy.matrix);
}

function updateActors(dt) {
  const alpha = clamp(latest.alpha, 0, 1);
  const playerPosition = interpolated(latest.player, alpha, playerPositionCache);
  playerMesh.visible = latest.player.alive;
  playerMesh.position.copy(playerPosition);
  playerMesh.rotation.y = Math.PI * 0.5 - latest.player.facing;
  const moving = keys.has('KeyW') || keys.has('KeyA') || keys.has('KeyS') || keys.has('KeyD') || Math.hypot(joystickVector.x, joystickVector.z) > 0.1;
  const bob = moving && !reducedMotion ? Math.sin(latest.time * 13) * 0.055 : 0;
  playerMesh.position.y += bob;
  attackAnimation = Math.max(0, attackAnimation - dt);
  const weapon = playerMesh.userData.weapon;
  weapon.rotation.z = -0.35 - Math.sin((attackAnimation / 0.28) * Math.PI) * 1.5;
  playerLabel.visible = latest.player.alive;
  playerLabel.position.set(playerPosition.x, playerPosition.y + 3.1, playerPosition.z);
  updateLabel(playerLabel, latest.player.name, latest.player.health);

  latest.bots.forEach((bot, index) => {
    const position = interpolated(bot, alpha, botPositionCache[index]);
    const scale = bot.alive ? 1 : 0;
    const yaw = Math.PI * 0.5 - bot.facing;
    setInstance(botLegs, index, position.x, position.y + 0.37, position.z, yaw, scale);
    setInstance(botBody, index, position.x, position.y + 1.18 + (bot.alive && !reducedMotion ? Math.sin(latest.time * 8 + index) * 0.025 : 0), position.z, yaw, scale);
    setInstance(botHead, index, position.x, position.y + 2.03, position.z, yaw, scale);
    botBody.setColorAt(index, tempColor.set(skinPalette[bot.skin % skinPalette.length]));
    const label = botLabels[index];
    label.visible = bot.alive && position.distanceTo(playerPosition) < 43;
    label.position.set(position.x, position.y + 3.05, position.z);
    if (label.visible) updateLabel(label, bot.name, bot.health);
  });
  botBody.instanceMatrix.needsUpdate = true;
  botHead.instanceMatrix.needsUpdate = true;
  botLegs.instanceMatrix.needsUpdate = true;
  botBody.instanceColor.needsUpdate = true;

  latest.boars.forEach((boar, index) => {
    const position = interpolated(boar, alpha, boarPositionCache[index]);
    const scale = boar.alive ? 1 : 0;
    const yaw = Math.PI * 0.5 - boar.facing;
    setInstance(boarBody, index, position.x, position.y + 0.72, position.z, yaw, scale, scale * 0.78);
    const forwardX = Math.cos(boar.facing) * 0.68;
    const forwardZ = Math.sin(boar.facing) * 0.68;
    setInstance(boarHead, index, position.x + forwardX, position.y + 0.73, position.z + forwardZ, yaw, scale);
  });
  boarBody.instanceMatrix.needsUpdate = true;
  boarHead.instanceMatrix.needsUpdate = true;

  const dropCount = Math.min(latest.drops.length, 90);
  dropMesh.visible = dropCount > 0;
  dropMesh.count = dropCount;
  for (let index = 0; index < dropCount; index++) {
    const drop = latest.drops[index];
    const y = heightAt(drop.pos.x, drop.pos.z);
    setInstance(dropMesh, index, drop.pos.x, y + 0.5 + Math.sin(latest.time * 3 + index) * 0.12, drop.pos.z, latest.time * 0.7 + index, 1);
  }
  dropMesh.instanceMatrix.needsUpdate = true;
  return playerPosition;
}

function updateResources() {
  if (latest.resource_revision === lastResourceRevision) return;
  lastResourceRevision = latest.resource_revision;
  let treeIndex = 0;
  let rockIndex = 0;
  let bushIndex = 0;
  for (const resource of latest.resources) {
    const y = heightAt(resource.pos.x, resource.pos.z);
    const scale = resource.alive ? 0.82 + 0.18 * resource.hp / resource.max_hp : 0;
    if (resource.kind === 'Tree') {
      setInstance(treeTrunkMesh, treeIndex, resource.pos.x, y + 1.15, resource.pos.z, resource.id[0] || 0, scale);
      setInstance(treeCrownMesh, treeIndex, resource.pos.x, y + 3.02, resource.pos.z, resource.id[0] || 0, scale);
      treeIndex++;
    } else if (resource.kind === 'Rock') {
      setInstance(rockMesh, rockIndex++, resource.pos.x, y + 0.66, resource.pos.z, resource.id[0] || 0, scale, scale * 0.8);
    } else {
      setInstance(bushMesh, bushIndex++, resource.pos.x, y + 0.55, resource.pos.z, resource.id[0] || 0, scale, scale * 0.82);
    }
  }
  for (const mesh of [treeTrunkMesh, treeCrownMesh, rockMesh, bushMesh]) mesh.instanceMatrix.needsUpdate = true;
}

function updateStructures() {
  if (latest.structure_revision !== lastStructureRevision) {
    lastStructureRevision = latest.structure_revision;
    let wallIndex = 0;
    let doorIndex = 0;
    let fireIndex = 0;
    for (const structure of latest.structures) {
      const y = heightAt(structure.pos.x, structure.pos.z);
      if (structure.kind === 'Wall') setInstance(wallMesh, wallIndex++, structure.pos.x, y + 1.35, structure.pos.z, structure.rotation, 1);
      else if (structure.kind === 'Door') setInstance(doorMesh, doorIndex++, structure.pos.x, y + 1.28, structure.pos.z, structure.rotation, 1);
      else setInstance(campfireStoneMesh, fireIndex++, structure.pos.x, y + 0.22, structure.pos.z, 0, 1.25, 0.42);
    }
    wallMesh.count = wallIndex;
    doorMesh.count = doorIndex;
    campfireStoneMesh.count = fireIndex;
    campfireFlameMesh.count = fireIndex;
    for (const mesh of [wallMesh, doorMesh, campfireStoneMesh]) mesh.instanceMatrix.needsUpdate = true;
  }
  let fireIndex = 0;
  for (const structure of latest.structures) {
    if (structure.kind !== 'Campfire') continue;
    const y = heightAt(structure.pos.x, structure.pos.z);
    const flicker = reducedMotion ? 1 : 0.85 + Math.sin(latest.time * 17 + fireIndex * 2) * 0.16;
    setInstance(campfireFlameMesh, fireIndex++, structure.pos.x, y + 0.85, structure.pos.z, latest.time, flicker, flicker);
  }
  campfireFlameMesh.instanceMatrix.needsUpdate = true;
}

function updateGhost(playerPosition) {
  ghostMesh.visible = Boolean(buildMode) && mode === 'play';
  if (!ghostMesh.visible) return;
  const offset = aimPoint.clone().sub(playerPosition);
  if (offset.length() > 7) offset.setLength(7);
  const target = playerPosition.clone().add(offset);
  if (buildMode !== 'campfire') {
    target.x = Math.round(target.x / 2) * 2;
    target.z = Math.round(target.z / 2) * 2;
  }
  const y = heightAt(target.x, target.z);
  ghostMesh.position.set(target.x, y + (buildMode === 'campfire' ? 0.3 : 1.35), target.z);
  ghostMesh.rotation.y = Math.abs(aimPoint.x - playerPosition.x) > Math.abs(aimPoint.z - playerPosition.z) ? Math.PI * 0.5 : 0;
  ghostMesh.scale.setScalar(buildMode === 'campfire' ? 0.28 : 1);
}

function updateCamera(dt, playerPosition) {
  if (mode === 'title') {
    titleTime += dt;
    if (!reducedMotion) cameraOffset.set(20 + Math.sin(titleTime * 0.13) * 3, 24, 20 + Math.cos(titleTime * 0.13) * 3);
  } else {
    cameraOffset.set(17, 23, 17);
  }
  const follow = 1 - Math.exp(-dt * (mode === 'title' ? 1.5 : 8));
  cameraTarget.lerp(playerPosition, follow);
  shake = Math.max(0, shake - dt * 2.8);
  const shakeAmount = reducedMotion ? 0 : shake * 0.18;
  camera.position.copy(cameraTarget).add(cameraOffset);
  camera.position.x += Math.sin(latest.time * 83) * shakeAmount;
  camera.position.z += Math.cos(latest.time * 71) * shakeAmount;
  camera.lookAt(cameraTarget.x, cameraTarget.y + 0.5, cameraTarget.z);
  sun.position.set(cameraTarget.x - 28, 42, cameraTarget.z + 20);
  sun.target.position.copy(cameraTarget);
  sun.target.updateMatrixWorld();
}

function adaptQuality() {
  if (quality === 'low' || mode !== 'play') return;
  lowFpsIntervals = fps < 28 ? lowFpsIntervals + 1 : 0;
  if (lowFpsIntervals < 3) return;
  quality = 'low';
  renderer.shadowMap.enabled = false;
  sun.castShadow = false;
  renderer.setPixelRatio(1);
  scene.traverse((object) => { if (object.material) object.material.needsUpdate = true; });
}

function toast(message, kind = '') {
  if (!message) return;
  const feed = $('#event-feed');
  while (feed.children.length >= 3) feed.firstElementChild.remove();
  const element = document.createElement('div');
  element.className = `toast ${kind}`;
  element.textContent = message;
  feed.append(element);
  setTimeout(() => element.remove(), 2600);
}

function processEvents() {
  for (const event of latest.events) {
    audio.play(event.kind);
    if (event.message) toast(event.message, event.kind);
    if (event.kind === 'death') deathReason = event.message;
    if (['hit', 'hurt', 'death'].includes(event.kind)) shake = Math.max(shake, event.kind === 'death' ? 1.5 : 0.65);
    if (event.kind === 'hurt') {
      $('#damage-flash').classList.add('active');
      setTimeout(() => $('#damage-flash').classList.remove('active'), 90);
    }
    const colors = { gather: '#c5a168', deplete: '#9eb66c', hit: '#e8c389', hurt: '#e25f4f', kill: '#dc7946', build: '#b7d68b', craft: '#b7d68b', pickup: '#e7c76c' };
    if (colors[event.kind]) burst(event.pos, colors[event.kind], event.kind === 'kill' ? 20 : 9);
  }
}

function itemCount(kind) {
  return latest?.player.inventory.slots.find((slot) => slot.item === kind)?.count || 0;
}

function updateHud() {
  const player = latest.player;
  for (const [name, value] of [['health', player.health], ['hunger', player.hunger], ['thirst', player.thirst], ['temperature', player.temperature]]) {
    $(`#${name}-bar`).style.transform = `scaleX(${clamp(value / 100, 0, 1)})`;
    $(`#${name}-value`).textContent = Math.round(value);
    $(`#${name}-bar`).closest('.stat-row').classList.toggle('critical', value < 20 || (name === 'temperature' && value > 92));
  }
  $('#online-count').textContent = latest.online_count;
  $('#ping-value').textContent = latest.ping;
  $('#player-score').textContent = latest.player.score;
  $('#berry-count').textContent = itemCount('Berry');
  $('#cooked-count').textContent = itemCount('CookedMeat');
  $('#context-hint').textContent = latest.context;
  updateInventory();
  updateLeaderboard();
  updateChat();
}

function updateInventory() {
  const slots = latest.player.inventory.slots;
  const signature = JSON.stringify(slots);
  if (signature === lastInventorySignature) return;
  lastInventorySignature = signature;
  const grid = $('#inventory-grid');
  grid.replaceChildren();
  for (let index = 0; index < 12; index++) {
    const stack = slots[index];
    const slot = document.createElement('div');
    slot.className = `inventory-slot${stack ? ' has-item' : ''}`;
    if (stack) {
      const [code, name] = ITEM_META[stack.item] || ['OBJ', stack.item];
      slot.innerHTML = `<span class="item-code"></span><span class="item-name"></span><b class="item-count"></b>`;
      slot.querySelector('.item-code').textContent = code;
      slot.querySelector('.item-name').textContent = name;
      slot.querySelector('.item-count').textContent = stack.count;
    } else {
      slot.innerHTML = `<span class="item-name">VACÍO</span>`;
    }
    grid.append(slot);
  }
  $$('#recipe-list [data-craft]').forEach((button) => {
    const recipe = RECIPES[button.dataset.craft];
    const uniqueTool = ['StoneHatchet', 'MetalHatchet'].includes(recipe.output) && itemCount(recipe.output) > 0;
    const affordable = Object.entries(recipe).every(([kind, count]) => kind === 'output' || itemCount(kind) >= count);
    button.disabled = uniqueTool || !affordable;
    button.querySelector('b').textContent = uniqueTool ? 'FABRICADA' : affordable ? 'FABRICAR' : 'FALTAN';
  });
}

function updateLeaderboard() {
  const signature = latest.leaderboard.map((entry) => `${entry.id}:${entry.score}`).join('|');
  if (signature === lastLeaderboardSignature) return;
  lastLeaderboardSignature = signature;
  const list = $('#leaderboard-list');
  list.replaceChildren();
  for (const entry of latest.leaderboard) {
    const row = document.createElement('li');
    if (entry.is_player) row.className = 'is-player';
    const name = document.createElement('span');
    name.className = 'board-name';
    name.textContent = entry.name;
    const score = document.createElement('span');
    score.className = 'board-score';
    score.textContent = entry.score;
    const kills = document.createElement('small');
    kills.textContent = `${entry.kills}K`;
    score.append(kills);
    row.append(name, score);
    list.append(row);
  }
}

function updateChat() {
  const newest = latest.chat.at(-1)?.id ?? -1;
  if (newest === lastChatId) return;
  const log = $('#chat-messages');
  for (const message of latest.chat) {
    if (message.id <= lastChatId) continue;
    const line = document.createElement('p');
    line.className = `chat-line${message.is_player ? ' player' : ''}${message.sender === 'SERVIDOR' ? ' server' : ''}`;
    const sender = document.createElement('strong');
    sender.textContent = `${message.sender}:`;
    line.append(sender, document.createTextNode(message.text));
    log.append(line);
  }
  while (log.children.length > 30) log.firstElementChild.remove();
  lastChatId = newest;
  log.scrollTop = log.scrollHeight;
}

function queueAction(action) {
  if (mode !== 'play') return;
  if (actionQueue.length < 8) actionQueue.push(action);
  if (action === 'attack') attackAnimation = 0.28;
}

function setMode(next) {
  mode = next;
  $('#title-screen').hidden = next !== 'title';
  $('#pause-screen').hidden = next !== 'pause';
  $('#death-screen').hidden = next !== 'death';
  document.body.classList.toggle('is-title', next === 'title');
  if (next !== 'play') {
    $('#build-menu').hidden = true;
    $('#inventory-panel').hidden = true;
    document.body.classList.remove('show-chat', 'show-board');
    buildMode = null;
    keys.clear();
  }
}

function startGame() {
  if (!game) return;
  audio.start();
  setMode('play');
  canvas.focus();
}

function pauseGame() {
  if (mode !== 'play') return;
  game.save_now();
  setMode('pause');
  $('#resume-button').focus();
}

function resumeGame() {
  if (mode !== 'pause') return;
  setMode('play');
}

function onDeath() {
  if (mode !== 'play') return;
  game.save_now();
  $('#death-reason').textContent = deathReason;
  $('#death-score').textContent = latest.player.score;
  $('#death-kills').textContent = latest.player.kills;
  $('#death-built').textContent = latest.player.built;
  setMode('death');
  $('#respawn-button').focus();
}

function openInventory() {
  if (mode !== 'play') return;
  const panel = $('#inventory-panel');
  panel.hidden = !panel.hidden;
  $('#build-menu').hidden = true;
  $('#build-button').classList.remove('active');
  if (!panel.hidden) panel.querySelector('.close-button').focus();
  else $('#inventory-button').focus();
}

function toggleBuildMenu() {
  if (mode !== 'play') return;
  $('#inventory-panel').hidden = true;
  const menu = $('#build-menu');
  menu.hidden = !menu.hidden;
  $('#build-button').classList.toggle('active', !menu.hidden);
  buildMode = null;
}

function chooseBuild(kind) {
  const output = { wall: 'WoodWall', door: 'WoodDoor', campfire: 'Campfire' }[kind];
  if (!itemCount(output)) {
    toast('Primero fabrica la estructura en la mochila', 'notice');
    $('#build-menu').hidden = true;
    openInventory();
    return;
  }
  buildMode = kind;
  $('#build-menu').hidden = true;
  $('#build-button').classList.remove('active');
  $('#build-label').hidden = false;
  $('#build-kind').textContent = { wall: 'MURO', door: 'PUERTA', campfire: 'HOGUERA' }[kind];
}

function cancelBuild() {
  buildMode = null;
  $('#build-label').hidden = true;
}

function updateAim(clientX, clientY) {
  mouseScreen.set(clientX, clientY);
  mouseNdc.set(clientX / innerWidth * 2 - 1, -(clientY / innerHeight) * 2 + 1);
  raycaster.setFromCamera(mouseNdc, camera);
  raycaster.ray.intersectPlane(groundPlane, aimPoint);
  $('#aim-reticle').style.left = `${clientX}px`;
  $('#aim-reticle').style.top = `${clientY}px`;
}

function bindControls() {
  $('#start-button').addEventListener('click', startGame);
  $('#pause-button').addEventListener('click', pauseGame);
  $('#resume-button').addEventListener('click', resumeGame);
  $('#mute-button').addEventListener('click', () => { audio.start(); audio.toggle(); });
  $('#inventory-button').addEventListener('click', openInventory);
  $('#build-button').addEventListener('click', toggleBuildMenu);
  $('#mobile-chat-button').addEventListener('click', () => { document.body.classList.toggle('show-chat'); document.body.classList.remove('show-board'); });
  $('#mobile-board-button').addEventListener('click', () => { document.body.classList.toggle('show-board'); document.body.classList.remove('show-chat'); });
  $$('[data-close]').forEach((button) => button.addEventListener('click', () => { $(`#${button.dataset.close}`).hidden = true; $('#inventory-button').focus(); }));
  $$('[data-action]').forEach((button) => button.addEventListener('pointerdown', (event) => { event.preventDefault(); queueAction(button.dataset.action); }));
  $$('[data-craft]').forEach((button) => button.addEventListener('click', () => queueAction(`craft:${button.dataset.craft}`)));
  $$('[data-build]').forEach((button) => button.addEventListener('click', () => chooseBuild(button.dataset.build)));
  $('#respawn-button').addEventListener('click', () => {
    setMode('play');
    queueAction('respawn');
    deathReason = 'La isla se cobra otra víctima.';
  });
  $('#reset-button').addEventListener('click', () => { $('#reset-confirm').hidden = false; $('#confirm-reset-button').focus(); });
  $('#cancel-reset-button').addEventListener('click', () => { $('#reset-confirm').hidden = true; $('#reset-button').focus(); });
  $('#confirm-reset-button').addEventListener('click', () => {
    game.reset_save();
    mapData = game.get_map_data();
    latest = game.get_render_data();
    buildTerrain();
    lastResourceRevision = -1;
    lastStructureRevision = -1;
    lastInventorySignature = '';
    lastLeaderboardSignature = '';
    lastChatId = -1;
    $('#chat-messages').replaceChildren();
    $('#reset-confirm').hidden = true;
    setMode('play');
    cameraTarget.set(latest.player.pos.x, heightAt(latest.player.pos.x, latest.player.pos.z), latest.player.pos.z);
    toast('Nueva semilla. Progreso reiniciado.', 'respawn');
  });
  $('#reload-button').addEventListener('click', () => location.reload());

  $('#chat-form').addEventListener('submit', (event) => {
    event.preventDefault();
    const input = $('#chat-input');
    const text = input.value.trim();
    if (text && game) game.send_chat(text);
    input.value = '';
    input.blur();
    latest = game.get_render_data();
    updateChat();
  });

  addEventListener('keydown', (event) => {
    const typing = document.activeElement === $('#chat-input');
    if (typing) {
      if (event.code === 'Escape') { $('#chat-input').blur(); event.preventDefault(); }
      return;
    }
    if (event.code === 'Space' && mode === 'title' && !$('#start-button').disabled) {
      event.preventDefault();
      startGame();
      return;
    }
    if (event.code === 'Escape') {
      if (!$('#reset-confirm').hidden) { $('#reset-confirm').hidden = true; return; }
      if (!$('#inventory-panel').hidden) { $('#inventory-panel').hidden = true; return; }
      if (buildMode) { cancelBuild(); return; }
      if (mode === 'play') pauseGame(); else if (mode === 'pause') resumeGame();
      return;
    }
    if (event.code === 'KeyP') { if (mode === 'play') pauseGame(); else if (mode === 'pause') resumeGame(); return; }
    if (event.code === 'KeyM') { audio.start(); audio.toggle(); return; }
    if (mode !== 'play') return;
    if (['Space', 'KeyW', 'KeyA', 'KeyS', 'KeyD', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.code)) event.preventDefault();
    keys.add(event.code);
    if (event.repeat) return;
    if (event.code === 'KeyE') queueAction('gather');
    else if (event.code === 'Space') queueAction('attack');
    else if (event.code === 'Digit1') queueAction('eat:berry');
    else if (event.code === 'Digit2') queueAction('eat:cooked_meat');
    else if (event.code === 'KeyF') queueAction('cook');
    else if (event.code === 'KeyQ') queueAction('drink');
    else if (event.code === 'KeyC') openInventory();
    else if (event.code === 'KeyB') toggleBuildMenu();
    else if (event.code === 'Enter') { event.preventDefault(); $('#chat-input').focus(); }
  });
  addEventListener('keyup', (event) => keys.delete(event.code));
  addEventListener('blur', () => keys.clear());
  document.addEventListener('visibilitychange', () => { if (document.hidden && mode === 'play') pauseGame(); });
  addEventListener('beforeunload', () => game?.save_now());

  canvas.addEventListener('pointermove', (event) => {
    usingTouch = event.pointerType === 'touch';
    if (!usingTouch) updateAim(event.clientX, event.clientY);
  });
  canvas.addEventListener('pointerdown', (event) => {
    if (mode !== 'play' || event.button !== 0) return;
    usingTouch = event.pointerType === 'touch';
    if (!usingTouch) updateAim(event.clientX, event.clientY);
    if (buildMode) queueAction(`build:${buildMode}`);
    else queueAction('attack');
  });
  canvas.addEventListener('contextmenu', (event) => event.preventDefault());

  const joystick = $('#joystick');
  const knob = $('#joystick-knob');
  let joystickPointer = null;
  const moveJoystick = (event) => {
    const bounds = joystick.getBoundingClientRect();
    let x = event.clientX - (bounds.left + bounds.width * 0.5);
    let y = event.clientY - (bounds.top + bounds.height * 0.5);
    const radius = bounds.width * 0.34;
    const length = Math.hypot(x, y);
    if (length > radius) { x = x / length * radius; y = y / length * radius; }
    joystickVector = { x: x / radius, z: y / radius };
    knob.style.transform = `translate(${x}px, ${y}px)`;
  };
  joystick.addEventListener('pointerdown', (event) => {
    joystickPointer = event.pointerId;
    joystick.setPointerCapture(event.pointerId);
    moveJoystick(event);
  });
  joystick.addEventListener('pointermove', (event) => { if (event.pointerId === joystickPointer) moveJoystick(event); });
  const releaseJoystick = (event) => {
    if (event.pointerId !== joystickPointer) return;
    joystickPointer = null;
    joystickVector = { x: 0, z: 0 };
    knob.style.transform = '';
  };
  joystick.addEventListener('pointerup', releaseJoystick);
  joystick.addEventListener('pointercancel', releaseJoystick);

  document.addEventListener('keydown', (event) => {
    if (event.key !== 'Tab') return;
    const container = $('.drawer:not([hidden]), .compact-overlay:not([hidden])');
    if (!container) return;
    const focusable = [...container.querySelectorAll('button:not(:disabled), input, [tabindex="0"]')];
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable.at(-1);
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  });
}

function frame(now) {
  requestAnimationFrame(frame);
  const rawDt = Math.max(0, (now - lastFrame) / 1000);
  const dt = clamp(rawDt, 0, 0.25);
  lastFrame = now;
  fpsAccumulator += rawDt;
  fpsFrames++;
  if (fpsAccumulator >= 0.75) {
    fps = fpsFrames / fpsAccumulator;
    fpsAccumulator = 0;
    fpsFrames = 0;
    adaptQuality();
  }

  if (mode === 'play') {
    const typing = document.activeElement === $('#chat-input');
    const panelOpen = !$('#inventory-panel').hidden;
    let moveX = 0;
    let moveZ = 0;
    if (!typing && !panelOpen) {
      moveX = (keys.has('KeyD') || keys.has('ArrowRight') ? 1 : 0) - (keys.has('KeyA') || keys.has('ArrowLeft') ? 1 : 0) + joystickVector.x;
      moveZ = (keys.has('KeyS') || keys.has('ArrowDown') ? 1 : 0) - (keys.has('KeyW') || keys.has('ArrowUp') ? 1 : 0) + joystickVector.z;
    }
    const movementLength = Math.hypot(moveX, moveZ);
    if (movementLength > 1) { moveX /= movementLength; moveZ /= movementLength; }
    if (usingTouch) {
      const player = latest.player;
      aimPoint.set(player.pos.x + Math.cos(player.facing) * 4, 0, player.pos.z + Math.sin(player.facing) * 4);
    }
    const action = actionQueue.shift() || null;
    game.handle_input(moveX, moveZ, action, aimPoint.x, aimPoint.z);
    game.tick(dt);
    latest = game.get_render_data();
    processEvents();
    if (!latest.player.alive) onDeath();
  }

  const playerPosition = updateActors(dt);
  updateResources();
  updateStructures();
  updateGhost(playerPosition);
  updateParticles(dt);
  updateCamera(dt, playerPosition);
  if (now - lastUiUpdate > 100) {
    updateHud();
    lastUiUpdate = now;
  }
  renderer.render(scene, camera);
  window.__SCENE_READY__ = true;
}

async function boot() {
  try {
    await init();
    game = new GameState();
    mapData = game.get_map_data();
    latest = game.get_render_data();
    buildTerrain();
    buildDynamicMeshes();
    cameraTarget.set(latest.player.pos.x, heightAt(latest.player.pos.x, latest.player.pos.z), latest.player.pos.z);
    updateResources();
    updateStructures();
    updateHud();
    bindControls();
    $('#start-button').disabled = false;
    $('#start-label').textContent = 'ENTRAR EN LA SALA';
    lastFrame = performance.now();
    window.__GAME_READY__ = true;
    window.__gameProbe = () => ({
      mode,
      player: { x: latest.player.pos.x, z: latest.player.pos.z, health: latest.player.health, score: latest.player.score },
      bots: latest.bots.filter((bot) => bot.alive).length,
      resources: latest.resources.filter((resource) => resource.alive).length,
      structures: latest.structures.length,
      drawCalls: renderer.info.render.calls,
      triangles: renderer.info.render.triangles,
      fps: Math.round(fps),
      quality,
      renderer: renderer.getContext().getParameter(renderer.getContext().RENDERER),
      buildMode,
    });
    window.__seek = (seconds) => {
      const steps = Math.min(Math.ceil(seconds * 20), 1200);
      for (let index = 0; index < steps; index++) game.tick(0.05);
      latest = game.get_render_data();
      return window.__gameProbe();
    };
    const frozenTime = Number(new URLSearchParams(location.search).get('t'));
    if (frozenTime > 0) window.__seek(frozenTime);
    requestAnimationFrame(frame);
  } catch (error) {
    $('#loading-error').hidden = false;
    $('#start-label').textContent = 'ERROR DE CARGA';
    console.error('Ferrous failed to initialize:', error);
  }
}

boot();
