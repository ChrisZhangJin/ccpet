import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

const appWindow = getCurrentWebviewWindow();

// === Debug logger ===
function dbg(...args) {
  const line = '[ccpet-frontend] ' + args.map(a =>
    typeof a === 'string' ? a : JSON.stringify(a)
  ).join(' ');
  console.log(line);
  document.title = 'ccpet ' + line;
}

dbg('main.js loaded');

// === DOM elements ===
const dog = document.getElementById('dog');
const bubble = document.getElementById('speech-bubble');
const audio = new Audio('/bark.mp3');
audio.autoplay = true;
audio.playsInline = true;
audio.preload = 'auto';
const IS_MAC = /Mac|iPhone|iPad/.test(navigator.platform || '') || /Mac OS X/.test(navigator.userAgent || '');
const DRAG_MODIFIER = IS_MAC ? 'Meta' : 'Control';
let audioUnlocked = false;

function unlockAudio() {
  if (audioUnlocked) return;
  audioUnlocked = true;
  audio.play().then(() => {
    audio.pause();
    audio.currentTime = 0;
  }).catch(() => {});
}

window.addEventListener('pointerdown', unlockAudio, { once: true });
window.addEventListener('keydown', unlockAudio, { once: true });
const container = document.getElementById('pet-container');

let reacting = false;

// === Drag state ===
// The window is normally click-through (clicks pass to the apps behind it),
// so the OS never sees the mouse on the pet.
//
// While the platform modifier is held we flip click-through OFF: the cursor now lands on the pet.
let ctrlHeld = false;
let pointerInside = false;
let dragging = false;
let dragOffsetX = 0; // pointer.x at mousedown - window.x at that instant
let dragOffsetY = 0;

async function setClickThrough(ignore) {
  try {
    await appWindow.setIgnoreCursorEvents(ignore);
  } catch (e) {
    dbg('setIgnoreCursorEvents(' + ignore + ') failed: ' + e.message);
  }
}

function applyCursor() {
  // Show "move" cursor only when Ctrl is held AND the pointer is inside
  // the pet window — i.e. the moment the user is about to drag.
  if (ctrlHeld && pointerInside) {
    container.style.cursor = 'move';
  } else {
    container.style.cursor = '';
  }
}

window.addEventListener('keydown', (e) => {
  if (e.key === DRAG_MODIFIER && !ctrlHeld) {
    ctrlHeld = true;
    dbg(DRAG_MODIFIER + ' down -> disable click-through');
    setClickThrough(false);
    applyCursor();
  }
  if (e.code === 'KeyB' && !e.ctrlKey && !e.metaKey) {
    playReaction('manual-key');
  }
});

window.addEventListener('keyup', (e) => {
  if (e.key === DRAG_MODIFIER && ctrlHeld) {
    ctrlHeld = false;
    dragging = false;
    dbg(DRAG_MODIFIER + ' up -> re-enable click-through');
    setClickThrough(true);
    container.style.cursor = '';
  }
});

// Safety net: lose focus while holding Ctrl (e.g. Ctrl+Tab)
window.addEventListener('blur', () => {
  if (ctrlHeld) {
    ctrlHeld = false;
    dragging = false;
    setClickThrough(true);
    container.style.cursor = '';
    dbg('blur -> re-enable click-through');
  }
});

// Track whether the pointer is over the pet (only meaningful while
// click-through is OFF, i.e. while Ctrl is held).
container.addEventListener('mouseenter', () => {
  pointerInside = true;
  applyCursor();
});
container.addEventListener('mouseleave', () => {
  pointerInside = false;
  applyCursor();
});

// === Drag implementation ===
container.addEventListener('mousedown', async (e) => {
  if (!ctrlHeld) return;
  e.preventDefault();
  dragging = true;
  try {
    const outer = await appWindow.outerPosition();
    // outer.x/y are physical pixels; scale to logical using the monitor's
    // scale factor so we can compare with mouse coordinates which are in
    // logical CSS pixels relative to the screen.
    const scale = window.devicePixelRatio || 1;
    const winX = outer.x / scale;
    const winY = outer.y / scale;
    // e.screenX/Y are in CSS pixels (logical) on Windows for WebView2
    dragOffsetX = e.screenX - winX;
    dragOffsetY = e.screenY - winY;
    dbg('drag start, win=(' + winX + ',' + winY + ') offset=(' + dragOffsetX + ',' + dragOffsetY + ')');
  } catch (err) {
    dbg('drag start failed: ' + err.message);
    dragging = false;
  }
});

window.addEventListener('mousemove', (e) => {
  if (!dragging) return;
  const newX = Math.round(e.screenX - dragOffsetX);
  const newY = Math.round(e.screenY - dragOffsetY);
  invoke('set_window_position', { x: newX, y: newY }).catch(err => {
    dbg('set_window_position failed: ' + err.message);
  });
});

window.addEventListener('mouseup', () => {
  if (dragging) {
    dragging = false;
    dbg('drag end');
  }
});

// === Reaction ===
async function playReaction(source) {
  dbg('playReaction() from: ' + source);
  if (reacting) return;
  reacting = true;

  bubble.classList.remove('hidden', 'hide');
  bubble.classList.add('show');

  try {
    audio.currentTime = 0;
    await audio.play();
  } catch (e) {
    dbg('audio FAIL: ' + e.message);
  }

  dog.classList.remove('reaction-done');
  dog.classList.add('reaction');

  setTimeout(() => {
    dog.classList.remove('reaction');
    dog.classList.add('reaction-done');
    bubble.classList.remove('show');
    bubble.classList.add('hide');
    setTimeout(() => {
      bubble.classList.add('hidden');
      bubble.classList.remove('hide');
    }, 200);
    reacting = false;
  }, 2000);
}

// === Tauri event listener ===
async function setupListener() {
  try {
    const unlisten = await listen('action', (event) => {
      dbg(">>> 'action' event received, payload: " + JSON.stringify(event.payload));
      playReaction('tauri-event');
    });
    dbg('listen() registered OK');
    window.__ccpet_unlisten = unlisten;
  } catch (e) {
    dbg('listen() FAILED: ' + e.message);
  }
}

setupListener();
