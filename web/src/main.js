import "./style.css";

import init, {
  fmt as wasmFmt,
  check as wasmCheck,
  stats as wasmStats,
  run as wasmRun
} from '../pkg/mu_wasm.js';

import muArenaMain from '../../apps/mu_arena/src/main.mu?raw';
import muDungeonMain from '../../apps/mu_dungeon/src/dungeon.mu?raw';
import signalReactorMain from '../../apps/signal_reactor/src/signal_reactor.mu?raw';

const demoSelect = document.getElementById('demo-select');
const sourceEl = document.getElementById('source');
const formattedEl = document.getElementById('formatted');
const stdoutEl = document.getElementById('stdout');
const stderrEl = document.getElementById('stderr');
const statsEl = document.getElementById('stats');
const stdinEl = document.getElementById('stdin');
const fuelEl = document.getElementById('fuel');
const fuelValueEl = document.getElementById('fuel-value');
const statusEl = document.getElementById('status');
const actionButtons = [
  document.getElementById('fmt-readable'),
  document.getElementById('fmt-compressed'),
  document.getElementById('check'),
  document.getElementById('run')
];

const demos = [
  {
    name: 'mu_arena main',
    src: muArenaMain,
    stdin: '',
    fuel: 5000000
  },
  {
    name: 'mu_dungeon dungeon',
    src: muDungeonMain,
    stdin: '',
    fuel: 250000
  },
  {
    name: 'signal_reactor (expected web effect rejection)',
    src: signalReactorMain,
    stdin: '',
    fuel: 300000
  },
  {
    name: 'io/readln snippet',
    src: '@web.demo{:io=core.io;E[main];F main:()->i32!{io}=v(a:s=c(readln),{c(println,a);0});}',
    stdin: 'hello from stdin',
    fuel: 50000
  }
];

function renderStats() {
  const readable = wasmStats(sourceEl.value, 'readable');
  const compressed = wasmStats(sourceEl.value, 'compressed');
  const ratio = readable.bytes === 0 ? 0 : (compressed.bytes * 100) / readable.bytes;

  statsEl.innerHTML = [
    `<strong>Readable</strong> bytes=${readable.bytes} tokens=${readable.tokens}`,
    `<strong>Compressed</strong> bytes=${compressed.bytes} tokens=${compressed.tokens}`,
    `<strong>Ratio</strong> ${ratio.toFixed(2)}%`,
    `<strong>Symtab</strong> size=${compressed.symtab_size}`,
    `<strong>#n width</strong> avg=${compressed.avg_ref_width.toFixed(2)} max=${compressed.max_ref_width}`
  ].join('<br/>');
}

function setStatus(message) {
  statusEl.textContent = message;
}

function setButtonsEnabled(enabled) {
  actionButtons.forEach((button) => {
    button.disabled = !enabled;
  });
}

function withActionStatus(label, fn) {
  try {
    fn();
    setStatus(`${label}: ok`);
  } catch (error) {
    const message = `Action failed (${label}): ${String(error)}`;
    stderrEl.textContent = message;
    setStatus(`${label}: failed`);
  }
}

function setDemo(index) {
  const demo = demos[index];
  sourceEl.value = demo.src;
  stdinEl.value = demo.stdin;
  fuelEl.value = String(demo.fuel);
  fuelValueEl.textContent = String(demo.fuel);
  formattedEl.value = '';
  stdoutEl.textContent = '';
  stderrEl.textContent = '';
  renderStats();
  setStatus(`Loaded demo: ${demo.name}`);
}

async function boot() {
  setButtonsEnabled(false);
  await init();
  setButtonsEnabled(true);
  setStatus('WASM ready');

  demos.forEach((demo, index) => {
    const option = document.createElement('option');
    option.value = String(index);
    option.textContent = demo.name;
    demoSelect.appendChild(option);
  });

  demoSelect.addEventListener('change', () => setDemo(Number(demoSelect.value)));

  fuelEl.addEventListener('input', () => {
    fuelValueEl.textContent = fuelEl.value;
  });

  sourceEl.addEventListener('input', () => {
    renderStats();
    setStatus('Source updated');
  });

  document.getElementById('fmt-readable').addEventListener('click', () => {
    withActionStatus('Format Readable', () => {
      formattedEl.value = wasmFmt(sourceEl.value, 'readable');
      stderrEl.textContent = '';
    });
  });

  document.getElementById('fmt-compressed').addEventListener('click', () => {
    withActionStatus('Format Compressed', () => {
      formattedEl.value = wasmFmt(sourceEl.value, 'compressed');
      stderrEl.textContent = '';
    });
  });

  document.getElementById('check').addEventListener('click', () => {
    withActionStatus('Check', () => {
      const result = wasmCheck(sourceEl.value);
      if (result && result.ok) {
        stderrEl.textContent = 'check ok';
        return;
      }
      const errors = Array.isArray(result?.errors) ? result.errors : [];
      stderrEl.textContent = errors
        .map((err) => `${err.code} ${err.line}:${err.col} ${err.msg}`)
        .join('\n');
      if (errors.length === 0) {
        stderrEl.textContent = 'check failed with unknown error';
      }
    });
  });

  document.getElementById('run').addEventListener('click', () => {
    withActionStatus('Run', () => {
      const response = wasmRun(sourceEl.value, Number(fuelEl.value), stdinEl.value || null);
      stdoutEl.textContent = response?.stdout ?? '';

      const diagnostics = [
        `exit_code=${response?.exit_code ?? 'unknown'}`,
        `fuel_used=${response?.fuel_used ?? 'unknown'}`,
        `trapped=${response?.trapped ?? 'unknown'}`,
        response?.trap_code ? `trap_code=${response.trap_code}` : ''
      ]
        .filter(Boolean)
        .join('\n');

      stderrEl.textContent = [diagnostics, response?.stderr ?? ''].filter(Boolean).join('\n\n');
    });
  });

  setDemo(0);
}

boot().catch((error) => {
  setButtonsEnabled(false);
  setStatus('WASM init failed');
  stderrEl.textContent = `WASM init failed: ${String(error)}`;
});
