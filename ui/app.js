const { invoke } = window.__TAURI__.core;
const { listen }  = window.__TAURI__.event;

// ---- State ------------------------------------------------------------------
let adapters     = [];
let liveIfaces   = [];
let liveConns    = [];
let usageData    = [];

// ---- Tab routing ------------------------------------------------------------
document.querySelectorAll('nav .tab').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('nav .tab').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById(btn.dataset.tab).classList.add('active');
    if (btn.dataset.tab === 'usage') refreshUsage();
  });
});

// ---- Live monitor events ----------------------------------------------------
listen('network-update', e => {
  liveIfaces = e.payload.interfaces || [];
  liveConns  = e.payload.connections || [];
  renderTraffic();
  renderConnections();
}).catch(console.error);

// ---- Dashboard --------------------------------------------------------------
function renderStatusCard() {
  const on = adapters.some(a => a.adguard_enabled);
  document.getElementById('status-card').innerHTML = `
    <div class="status-row">
      <span class="dot ${on ? 'on' : 'off'}"></span>
      <span class="status-label">AdGuard DNS ${on ? 'Active' : 'Inactive'}</span>
    </div>
    <p class="status-hint">
      ${on
        ? 'DNS queries are routed through 94.140.14.14 &mdash; ads and trackers blocked at DNS level.'
        : 'No adapters are using AdGuard DNS. Go to DNS Settings to enable it.'}
    </p>`;
}

function renderTraffic() {
  const el = document.getElementById('live-traffic');
  if (!liveIfaces.length) {
    el.innerHTML = '<p class="placeholder">Collecting data…</p>';
    return;
  }
  el.innerHTML = liveIfaces.map(i => `
    <div class="traffic-card">
      <div class="tc-name">${esc(i.interface_name)}</div>
      <div class="tc-rate rx">&#8595; ${fmtRate(i.rx_rate)}</div>
      <div class="tc-rate tx">&#8593; ${fmtRate(i.tx_rate)}</div>
      <div class="tc-total">
        RX total: ${fmtBytes(i.bytes_received)}<br>
        TX total: ${fmtBytes(i.bytes_sent)}
      </div>
    </div>`).join('');
}

// ---- DNS Settings -----------------------------------------------------------
async function loadAdapters() {
  try {
    adapters = await invoke('get_adapters');
    renderAdapters();
    renderStatusCard();
  } catch (err) {
    setDnsMsg(String(err), 'err');
  }
}

function renderAdapters() {
  const el = document.getElementById('adapter-list');
  if (!adapters.length) {
    el.innerHTML = '<p class="placeholder">No network adapters found.</p>';
    return;
  }
  el.innerHTML = adapters.map(a => `
    <div class="adapter-row">
      <span class="badge ${a.adguard_enabled ? 'on' : 'off'}">${a.adguard_enabled ? 'ON' : 'OFF'}</span>
      <span class="adapter-name">${esc(a.name)}</span>
      <button class="btn ${a.adguard_enabled ? 'btn-disable' : 'btn-enable'}"
              onclick="toggleDns(${JSON.stringify(a.name)}, ${!a.adguard_enabled})">
        ${a.adguard_enabled ? 'Disable' : 'Enable AdGuard DNS'}
      </button>
    </div>`).join('');
}

async function toggleDns(name, enable) {
  setDnsMsg('Applying…', '');
  try {
    await invoke(enable ? 'enable_dns' : 'disable_dns', { adapter: name });
    await loadAdapters();
    setDnsMsg(`AdGuard DNS ${enable ? 'enabled on' : 'disabled on'} “${name}”`, 'ok');
  } catch (err) {
    setDnsMsg(`Error: ${err}  (Try running as Administrator)`, 'err');
  }
}

function setDnsMsg(text, cls) {
  const el = document.getElementById('dns-msg');
  el.textContent = text;
  el.className = 'msg ' + cls;
}

document.getElementById('btn-refresh-adapters').addEventListener('click', loadAdapters);

// ---- Usage ------------------------------------------------------------------
async function refreshUsage() {
  const hours = parseInt(document.getElementById('sel-period').value, 10);
  const iface = document.getElementById('inp-iface').value.trim() || null;
  try {
    usageData = await invoke('query_usage', { interface: iface, since_hours: hours });
    drawChart();
    renderUsageTable();
  } catch (err) {
    console.error('query_usage:', err);
  }
}

function drawChart() {
  const canvas = document.getElementById('usage-canvas');
  const dpr = window.devicePixelRatio || 1;
  canvas.width  = canvas.offsetWidth  * dpr;
  canvas.height = canvas.offsetHeight * dpr;
  const ctx = canvas.getContext('2d');
  ctx.scale(dpr, dpr);

  const W = canvas.offsetWidth;
  const H = canvas.offsetHeight;
  ctx.clearRect(0, 0, W, H);

  if (!usageData.length) {
    ctx.fillStyle = 'var(--text2)';
    ctx.font = '13px sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('No data recorded yet — data accumulates while the app runs.', W / 2, H / 2);
    return;
  }

  // Group by interface
  const groups = {};
  for (const s of usageData) {
    (groups[s.interface_name] = groups[s.interface_name] || []).push(s);
  }

  const allTs  = usageData.map(s => +new Date(s.timestamp));
  const minT   = Math.min(...allTs);
  const maxT   = Math.max(...allTs);
  const maxRate = Math.max(...usageData.map(s => s.rx_rate), 1);

  const pad = { t: 28, b: 32, l: 64, r: 16 };
  const gW  = W - pad.l - pad.r;
  const gH  = H - pad.t - pad.b;

  // Grid lines + Y labels
  ctx.font = '11px sans-serif';
  ctx.textAlign = 'right';
  ctx.textBaseline = 'middle';
  for (let i = 0; i <= 4; i++) {
    const y = pad.t + gH - (i / 4) * gH;
    ctx.fillStyle = '#4b5675';
    ctx.fillText(fmtBytes(Math.round((i / 4) * maxRate)) + '/s', pad.l - 6, y);
    ctx.strokeStyle = i === 0 ? '#2d3a52' : '#1c2538';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(pad.l, y);
    ctx.lineTo(pad.l + gW, y);
    ctx.stroke();
  }

  // Lines
  const palette = ['#60a5fa', '#fb923c', '#34d399', '#c084fc', '#fb7185', '#facc15'];
  const entries = Object.entries(groups);
  entries.forEach(([iface, pts], ci) => {
    const color = palette[ci % palette.length];
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.5;
    ctx.lineJoin = 'round';
    ctx.beginPath();
    pts.forEach((s, i) => {
      const t = +new Date(s.timestamp);
      const x = pad.l + ((t - minT) / ((maxT - minT) || 1)) * gW;
      const y = pad.t + gH - (s.rx_rate / maxRate) * gH;
      i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
    });
    ctx.stroke();

    // Legend
    const lx = pad.l + ci * 140;
    ctx.fillStyle = color;
    ctx.fillRect(lx, 8, 12, 12);
    ctx.fillStyle = '#cbd5e1';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.font = '11px sans-serif';
    ctx.fillText(iface, lx + 16, 14);
  });
}

function renderUsageTable() {
  const totals = {};
  for (const s of usageData) {
    if (!totals[s.interface_name]) totals[s.interface_name] = { rx: 0, tx: 0 };
    totals[s.interface_name].rx = Math.max(totals[s.interface_name].rx, s.bytes_received);
    totals[s.interface_name].tx = Math.max(totals[s.interface_name].tx, s.bytes_sent);
  }
  const tbody = document.querySelector('#usage-table tbody');
  const rows = Object.entries(totals).sort(([a], [b]) => a.localeCompare(b));
  tbody.innerHTML = rows.length
    ? rows.map(([name, { rx, tx }]) => `
        <tr>
          <td>${esc(name)}</td>
          <td>${fmtBytes(rx)}</td>
          <td>${fmtBytes(tx)}</td>
        </tr>`).join('')
    : '<tr class="empty-row"><td colspan="3">No data for this period.</td></tr>';
}

document.getElementById('btn-refresh-usage').addEventListener('click', refreshUsage);
document.getElementById('sel-period').addEventListener('change', refreshUsage);
window.addEventListener('resize', () => { if (usageData.length) drawChart(); });

// ---- Connections ------------------------------------------------------------
function renderConnections() {
  const filter = document.getElementById('inp-conn-filter').value.toLowerCase();
  const rows = filter
    ? liveConns.filter(c =>
        c.process_name.toLowerCase().includes(filter) ||
        c.local_addr.includes(filter) ||
        c.remote_addr.includes(filter))
    : liveConns;

  document.getElementById('conn-count').textContent =
    `${rows.length} connection${rows.length === 1 ? '' : 's'}`;

  const tbody = document.querySelector('#conn-table tbody');
  tbody.innerHTML = rows.length
    ? rows.map(c => `
        <tr>
          <td>${c.pid}</td>
          <td title="${esc(c.process_name)}">${esc(c.process_name)}</td>
          <td>${esc(c.protocol)}</td>
          <td title="${esc(c.local_addr)}">${esc(c.local_addr)}</td>
          <td title="${esc(c.remote_addr)}">${esc(c.remote_addr)}</td>
          <td>${esc(c.state)}</td>
        </tr>`).join('')
    : '<tr class="empty-row"><td colspan="6">No connections match.</td></tr>';
}

document.getElementById('inp-conn-filter').addEventListener('input', renderConnections);

// ---- Helpers ----------------------------------------------------------------
function fmtBytes(n) {
  n = Number(n) || 0;
  if (n >= 1073741824) return (n / 1073741824).toFixed(2) + ' GB';
  if (n >= 1048576)    return (n / 1048576).toFixed(2)    + ' MB';
  if (n >= 1024)       return (n / 1024).toFixed(1)       + ' KB';
  return n + ' B';
}

function fmtRate(r) { return fmtBytes(r) + '/s'; }

function esc(s) {
  return String(s ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

// ---- Init -------------------------------------------------------------------
loadAdapters();
